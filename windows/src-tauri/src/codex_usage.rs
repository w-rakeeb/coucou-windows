// Read-only local telemetry. No thread resume, turn submission or credential reads.
use std::{collections::{HashMap, HashSet, hash_map::DefaultHasher}, hash::{Hash, Hasher},
    io::{BufRead, BufReader, Seek, SeekFrom}, path::{Path, PathBuf}, sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH}};
use serde::Serialize;
use serde_json::Value;
use super::codex_info::{Window, timestamp_seconds};

const MAX_LINE: usize = 8 * 1024 * 1024;
const HISTORY: u64 = 8 * 24 * 60 * 60;
// The service can round the same reset deadline a second or two differently.
const RESET_JITTER: u64 = 90;

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all="camelCase")]
pub struct Tokens {
    input_tokens: u64, cached_input_tokens: u64, output_tokens: u64,
    reasoning_output_tokens: u64, total_tokens: u64,
}
impl Tokens {
    fn parse(value: &Value) -> Option<Self> {
        Some(Self {total_tokens:value.get("total_tokens")?.as_u64()?,
            input_tokens:value.get("input_tokens").and_then(Value::as_u64).unwrap_or(0),
            cached_input_tokens:value.get("cached_input_tokens").and_then(Value::as_u64).unwrap_or(0),
            output_tokens:value.get("output_tokens").and_then(Value::as_u64).unwrap_or(0),
            reasoning_output_tokens:value.get("reasoning_output_tokens").and_then(Value::as_u64).unwrap_or(0)})
    }
    fn delta(&self, old: &Self) -> Self {
        Self {input_tokens:self.input_tokens.saturating_sub(old.input_tokens),
            cached_input_tokens:self.cached_input_tokens.saturating_sub(old.cached_input_tokens),
            output_tokens:self.output_tokens.saturating_sub(old.output_tokens),
            reasoning_output_tokens:self.reasoning_output_tokens.saturating_sub(old.reasoning_output_tokens),
            total_tokens:self.total_tokens.saturating_sub(old.total_tokens)}
    }
    fn add(&mut self, value: &Self) {
        self.input_tokens=self.input_tokens.saturating_add(value.input_tokens);
        self.cached_input_tokens=self.cached_input_tokens.saturating_add(value.cached_input_tokens);
        self.output_tokens=self.output_tokens.saturating_add(value.output_tokens);
        self.reasoning_output_tokens=self.reasoning_output_tokens.saturating_add(value.reasoning_output_tokens);
        self.total_tokens=self.total_tokens.saturating_add(value.total_tokens);
    }
}

#[derive(Clone, Default, Serialize)]
#[serde(rename_all="camelCase")]
pub struct Session {
    thread_id: String, model: Option<String>, reasoning_effort: Option<String>, branch: Option<String>,
    total: Option<Tokens>, last: Option<Tokens>, last_update_tokens: Option<u64>,
    context_used: Option<u64>, model_context_window: Option<u64>, updated_at: Option<u64>,
}
#[derive(Clone)]
struct Event { at: u64, fingerprint: u64, tokens: Tokens, primary_reset: Option<u64>, secondary_reset: Option<u64> }
#[derive(Default)]
struct Cursor {
    offset: u64, partial: Vec<u8>, oversized: bool, previous: Option<Tokens>,
    session: Session, events: Vec<Event>,
}
impl Cursor {
    fn line(&mut self, line: &[u8], cutoff: u64) {
        let Ok(text)=std::str::from_utf8(line) else {return;};
        // Avoid deserializing messages, images, code or tool outputs.
        if !text.contains("\"session_meta\"") && !text.contains("\"turn_context\"") && !text.contains("\"token_count\"") {return;}
        let Ok(value)=serde_json::from_str::<Value>(text) else {return;};
        let Some(payload)=value.get("payload") else {return;};
        match value.get("type").and_then(Value::as_str) {
            Some("session_meta") => {
                if let Some(id)=payload.get("id").and_then(Value::as_str) {self.session.thread_id=id.into();}
                self.session.branch=payload.pointer("/git/branch").and_then(Value::as_str).map(str::to_owned);
            }
            Some("turn_context") => {
                self.session.model=payload.get("model").and_then(Value::as_str).map(str::to_owned);
                self.session.reasoning_effort=payload.get("effort").and_then(Value::as_str).map(str::to_owned);
            }
            Some("event_msg") if payload.get("type").and_then(Value::as_str)==Some("token_count") => {
                let Some(info)=payload.get("info").filter(|v|!v.is_null()) else {return;};
                let Some(total)=info.get("total_token_usage").and_then(Tokens::parse) else {return;};
                let Some(last)=info.get("last_token_usage").and_then(Tokens::parse) else {return;};
                let delta=match &self.previous {
                    Some(old) if total.total_tokens>=old.total_tokens => total.delta(old),
                    // A fork/truncated history can start with an inherited total.
                    _ => last.clone(),
                };
                self.previous=Some(total.clone());
                self.session.total=Some(total);
                self.session.context_used=Some(last.total_tokens);
                self.session.last=Some(last);
                self.session.model_context_window=info.get("model_context_window").and_then(Value::as_u64).filter(|n|*n>0);
                let at=value.get("timestamp").and_then(Value::as_str).and_then(timestamp_seconds);
                self.session.updated_at=at;
                if delta.total_tokens==0 {return;}
                self.session.last_update_tokens=Some(delta.total_tokens);
                let Some(at)=at.filter(|at|*at>=cutoff) else {return;};
                let Some(quota)=payload.get("rate_limits").filter(|q|!q.is_null()) else {return;};
                if quota.get("limit_id").and_then(Value::as_str).is_some_and(|id|id!="codex") {return;}
                let mut hash=DefaultHasher::new();
                value.get("timestamp").and_then(Value::as_str).hash(&mut hash);
                info.to_string().hash(&mut hash);
                self.events.push(Event {at,fingerprint:hash.finish(),tokens:delta,
                    primary_reset:quota.pointer("/primary/resets_at").and_then(Value::as_u64),
                    secondary_reset:quota.pointer("/secondary/resets_at").and_then(Value::as_u64)});
            }
            _=>{}
        }
    }
    fn bytes(&mut self, bytes: &[u8], cutoff: u64) {
        for fragment in bytes.split_inclusive(|b|*b==b'\n') {
            let complete=fragment.last()==Some(&b'\n');
            if !self.oversized {
                if self.partial.len()+fragment.len()>MAX_LINE {self.partial.clear();self.oversized=true;}
                else {self.partial.extend_from_slice(fragment);}
            }
            if complete {
                if !self.oversized {let line=std::mem::take(&mut self.partial);self.line(&line,cutoff);}
                self.oversized=false;
            }
        }
    }
    fn update(&mut self, path: &Path, cutoff: u64) -> std::io::Result<()> {
        let mut file=std::fs::File::open(path)?;
        let size=file.metadata()?.len();
        if size<self.offset {*self=Self::default();}
        if size==self.offset {return Ok(());}
        file.seek(SeekFrom::Start(self.offset))?;
        let mut reader=BufReader::new(file);
        loop {
            let bytes=reader.fill_buf()?;
            if bytes.is_empty(){break;}
            let consumed=bytes.len();
            self.bytes(bytes,cutoff);self.offset+=consumed as u64;
            reader.consume(consumed);
        }
        self.events.retain(|event|event.at>=cutoff);
        Ok(())
    }
}

#[derive(Default)]
struct Cache {files:HashMap<PathBuf,Cursor>}
#[derive(Default)]
pub struct UsageCache(Arc<Mutex<Cache>>);

#[derive(Default, Serialize)]
#[serde(rename_all="camelCase")]
pub struct WindowUsage {
    #[serde(flatten)] tokens: Tokens,
    responses: usize, sessions: usize, resets_at: u64, expired: bool,
}
#[derive(Default, Serialize)]
#[serde(rename_all="camelCase")]
pub struct Telemetry {
    primary: Option<WindowUsage>, secondary: Option<WindowUsage>,
    sessions: HashMap<String, Session>, checked_at: u64,
}
fn sum_window<'a>(window: Option<&Window>, weekly: bool, cursors: impl Iterator<Item=&'a Cursor>, now: u64) -> Option<WindowUsage> {
    let window=window?;
    let reset=window.resets_at?;
    if window.window_duration_mins==0 || window.window_duration_mins>14*24*60 {return None;}
    let start=reset.saturating_sub(window.window_duration_mins*60).saturating_sub(RESET_JITTER);
    let mut result=WindowUsage {resets_at:reset,expired:now>=reset,..WindowUsage::default()};
    if result.expired {return Some(result);}
    let mut seen=HashSet::new();let mut sessions=HashSet::new();
    let mut cursors:Vec<_>=cursors.collect();
    // Stable ordering chooses the original session before its copied forks.
    cursors.sort_by(|a,b|a.session.thread_id.cmp(&b.session.thread_id));
    for cursor in cursors {
        for event in &cursor.events {
            let event_reset=if weekly {event.secondary_reset} else {event.primary_reset};
            if event.at<start || event.at>now || event.at>=reset || !event_reset.is_some_and(|end|end.abs_diff(reset)<=RESET_JITTER) {continue;}
            // Forks/archives can contain copies of the same historical events.
            if seen.insert(event.fingerprint) {
                result.tokens.add(&event.tokens);result.responses+=1;
                sessions.insert(&cursor.session.thread_id);
            }
        }
    }
    result.sessions=sessions.len();Some(result)
}
fn collect(root: &Path, depth: usize, paths: &mut Vec<PathBuf>) {
    if depth>3 || paths.len()>=10_000 {return;}
    let Ok(entries)=std::fs::read_dir(root) else {return;};
    for entry in entries.filter_map(Result::ok) {
        let Ok(kind)=entry.file_type() else {continue;};
        if kind.is_symlink(){continue;}
        if kind.is_dir(){collect(&entry.path(),depth+1,paths);}
        else if entry.path().extension().is_some_and(|ext|ext=="jsonl") {paths.push(entry.path());}
        if paths.len()>=10_000 {break;}
    }
}
fn read(root: &Path, cache: &mut Cache, ids: &[String], primary: Option<&Window>, secondary: Option<&Window>, now: u64) -> Telemetry {
    let cutoff=now.saturating_sub(HISTORY);
    let mut paths=Vec::new();collect(&root.join("sessions"),0,&mut paths);collect(&root.join("archived_sessions"),0,&mut paths);
    paths.sort();
    // Old inactive logs cannot contribute to a current allowance window.
    paths.retain(|path|std::fs::metadata(path).ok().and_then(|m|m.modified().ok()).and_then(|t|t.duration_since(UNIX_EPOCH).ok()).is_some_and(|at|at.as_secs()>=cutoff)
        || ids.iter().any(|id|path.file_name().is_some_and(|name|name.to_string_lossy().ends_with(&format!("{id}.jsonl")))));
    let present:HashSet<_>=paths.iter().cloned().collect();cache.files.retain(|path,_|present.contains(path));
    for path in paths {let _=cache.files.entry(path.clone()).or_default().update(&path,cutoff);}
    let mut telemetry=Telemetry {primary:sum_window(primary,false,cache.files.values(),now),
        secondary:sum_window(secondary,true,cache.files.values(),now),checked_at:now,..Telemetry::default()};
    for cursor in cache.files.values() {
        if ids.contains(&cursor.session.thread_id) {
            let old=telemetry.sessions.get(&cursor.session.thread_id);
            if old.is_none_or(|old|old.updated_at<=cursor.session.updated_at) {telemetry.sessions.insert(cursor.session.thread_id.clone(),cursor.session.clone());}
        }
    }
    telemetry
}
#[tauri::command]
pub async fn codex_telemetry(thread_ids: Vec<String>, primary: Option<Window>, secondary: Option<Window>, cache: tauri::State<'_,UsageCache>) -> Result<Telemetry,String> {
    let root=std::env::var_os("CODEX_HOME").map(PathBuf::from).or_else(||std::env::var_os("USERPROFILE").map(|p|PathBuf::from(p).join(".codex"))).ok_or("Codex data directory is unavailable.")?;
    let ids:Vec<_>=thread_ids.into_iter().filter(|id|!id.is_empty() && super::codex::chat_url(Some(id)).is_ok()).take(64).collect();
    let cache=cache.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let now=SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
        let mut cache=cache.lock().map_err(|_|"Codex token metadata is temporarily unavailable.")?;
        Ok(read(&root,&mut cache,&ids,primary.as_ref(),secondary.as_ref(),now))
    }).await.map_err(|_|"Couldn't read local Codex token metadata.".to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn event(at: &str, total: u64, last: u64, reset: u64, weekly: u64) -> Vec<u8> {
        let usage=|n|json!({"input_tokens":n-10,"cached_input_tokens":n/2,"output_tokens":10,"reasoning_output_tokens":5,"total_tokens":n});
        format!("{}\n",json!({"timestamp":at,"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":usage(total),"last_token_usage":usage(last),"model_context_window":258400},"rate_limits":{"limit_id":"codex","primary":{"resets_at":reset},"secondary":{"resets_at":weekly}}}})).into_bytes()
    }
    fn quota(end:u64,mins:u64)->Window {Window{used_percent:20.,window_duration_mins:mins,resets_at:Some(end)}}
    #[test] fn counts_deltas_without_adding_cached_or_reasoning_twice() {
        let at=timestamp_seconds("2026-10-04T05:00:00Z").unwrap();let reset=at+3600;
        let mut cursor=Cursor::default();cursor.session.thread_id="one".into();
        cursor.bytes(&event("2026-10-04T05:00:00Z",100,100,reset,reset+86400),0);
        cursor.bytes(&event("2026-10-04T05:01:00Z",250,150,reset,reset+86400),0);
        cursor.bytes(&event("2026-10-04T05:02:00Z",250,150,reset,reset+86400),0);
        let result=sum_window(Some(&quota(reset,300)),false,[&cursor].into_iter(),at+600).unwrap();
        assert_eq!(result.tokens.total_tokens,250);assert_eq!(result.responses,2);
        assert_eq!(cursor.session.total.as_ref().unwrap().total_tokens,250);
        assert_eq!(cursor.session.context_used,Some(150));assert_eq!(cursor.session.model_context_window,Some(258400));
    }
    #[test] fn independent_resets_expiry_jitter_and_copied_fork_history() {
        let at=timestamp_seconds("2026-10-04T05:00:00Z").unwrap();let reset=at+3600;let weekly=at+86400;
        let mut a=Cursor::default();a.session.thread_id="one".into();
        a.bytes(&event("2026-10-04T05:00:00Z",100,100,reset,weekly),0);
        a.bytes(&event("2026-10-04T05:01:00Z",200,100,reset+1,weekly),0);
        let mut fork=Cursor::default();fork.session.thread_id="fork".into();
        fork.bytes(&event("2026-10-04T05:00:00Z",100,100,reset,weekly),0);
        assert_eq!(sum_window(Some(&quota(reset+2,300)),false,[&a,&fork].into_iter(),at+600).unwrap().tokens.total_tokens,200);
        assert_eq!(sum_window(Some(&quota(reset+18000,300)),false,[&a].into_iter(),at+600).unwrap().tokens.total_tokens,0);
        assert_eq!(sum_window(Some(&quota(weekly,10080)),true,[&a].into_iter(),at+600).unwrap().tokens.total_tokens,200);
        let expired=sum_window(Some(&quota(reset,300)),false,[&a].into_iter(),reset).unwrap();assert!(expired.expired);assert_eq!(expired.tokens.total_tokens,0);
        assert_eq!(sum_window(Some(&quota(weekly+604800,10080)),true,[&a].into_iter(),at+600).unwrap().tokens.total_tokens,0);
        assert!(sum_window(None,false,[&a].into_iter(),at).is_none());
    }
    #[test] fn partial_lines_wait_for_completion_and_counter_restarts_use_last_response() {
        let mut cursor=Cursor::default();let line=event("2026-10-04T05:00:00Z",10000,100,9999999999,9999999999);
        cursor.bytes(&line[..line.len()/2],0);assert!(cursor.session.total.is_none());
        cursor.bytes(&line[line.len()/2..],0);assert_eq!(cursor.events[0].tokens.total_tokens,100);
        cursor.bytes(&event("2026-10-04T05:01:00Z",150,150,9999999999,9999999999),0);
        assert_eq!(cursor.events[1].tokens.total_tokens,150);
        cursor.bytes(b"not JSON\n",0);assert_eq!(cursor.events.len(),2);
    }
    #[test] fn unrelated_metered_buckets_and_rate_only_events_do_not_inflate_tokens() {
        let mut cursor=Cursor::default();
        let mut value:Value=serde_json::from_slice(&event("2026-10-04T05:00:00Z",100,100,9999999999,9999999999)).unwrap();
        value["payload"]["rate_limits"]["limit_id"]=json!("codex_other");cursor.line(value.to_string().as_bytes(),0);
        assert!(cursor.events.is_empty());assert!(cursor.session.total.is_some());
        value["payload"]["info"]=Value::Null;cursor.line(value.to_string().as_bytes(),0);assert!(cursor.events.is_empty());
    }
    #[test] fn incremental_files_restart_archive_and_truncation_keep_correct_totals() {
        use std::io::Write;
        let at=timestamp_seconds("2026-10-04T05:00:00Z").unwrap();let reset=at+3600;
        let root=std::env::temp_dir().join(format!("coucou-token-test-{}-{}",std::process::id(),SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
        let dir=root.join("sessions/2026/10/04");std::fs::create_dir_all(&dir).unwrap();
        let path=dir.join("rollout-one.jsonl");
        let mut bytes=b"{\"type\":\"session_meta\",\"payload\":{\"id\":\"one\",\"git\":{\"branch\":\"main\"}}}\n{\"type\":\"turn_context\",\"payload\":{\"model\":\"gpt-6.1-sol\",\"effort\":\"medium\"}}\n".to_vec();
        bytes.extend(event("2026-10-04T05:00:00Z",100,100,reset,reset+86400));std::fs::write(&path,&bytes).unwrap();
        let mut cache=Cache::default();let ids=vec!["one".to_string()];
        let first=read(&root,&mut cache,&ids,Some(&quota(reset,300)),None,at+600);
        assert_eq!(first.primary.unwrap().tokens.total_tokens,100);assert_eq!(first.sessions["one"].reasoning_effort.as_deref(),Some("medium"));
        let offset=cache.files[&path].offset;
        let again=read(&root,&mut cache,&ids,Some(&quota(reset,300)),None,at+600);assert_eq!(again.primary.unwrap().tokens.total_tokens,100);assert_eq!(cache.files[&path].offset,offset);
        let mut append=std::fs::OpenOptions::new().append(true).open(&path).unwrap();append.write_all(&event("2026-10-04T05:01:00Z",250,150,reset,reset+86400)).unwrap();drop(append);
        assert_eq!(read(&root,&mut cache,&ids,Some(&quota(reset,300)),None,at+600).primary.unwrap().tokens.total_tokens,250);
        assert_eq!(read(&root,&mut Cache::default(),&ids,Some(&quota(reset,300)),None,at+600).primary.unwrap().tokens.total_tokens,250);
        let archives=root.join("archived_sessions");std::fs::create_dir_all(&archives).unwrap();std::fs::rename(&path,archives.join("rollout-one.jsonl")).unwrap();
        assert_eq!(read(&root,&mut cache,&ids,Some(&quota(reset,300)),None,at+600).primary.unwrap().tokens.total_tokens,250);
        let mut cursor=Cursor::default();let truncated=root.join("truncated.jsonl");std::fs::write(&truncated,&bytes).unwrap();cursor.update(&truncated,0).unwrap();
        std::fs::write(&truncated,event("2026-10-04T05:03:00Z",50,50,reset,reset+86400)).unwrap();cursor.update(&truncated,0).unwrap();assert_eq!(cursor.events.len(),1);assert_eq!(cursor.session.total.unwrap().total_tokens,50);
        std::fs::remove_dir_all(root).unwrap();
    }
}
