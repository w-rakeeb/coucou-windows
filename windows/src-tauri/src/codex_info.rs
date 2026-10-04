// Read-only metadata client. Never starts/resumes a thread or submits a turn.
use std::{collections::HashMap, io::{BufRead, BufReader, Read, Write, Seek, SeekFrom}, path::{Path, PathBuf},
    process::{Child, Command, Stdio}, sync::{mpsc, Arc, Mutex}, time::{Duration, Instant, SystemTime, UNIX_EPOCH}};
use std::os::windows::process::CommandExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Window {
    pub(crate) used_percent: f64,
    pub(crate) window_duration_mins: u64,
    pub(crate) resets_at: Option<u64>,
}
#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Info {
    primary: Option<Window>, secondary: Option<Window>, checked_at: u64,
    error: Option<String>, thread_id: Option<String>, title: Option<String>,
}
#[derive(Default)]
struct Cache { info: Info, checked: Option<Instant>, titles: HashMap<String, (Option<String>, Instant)> }
#[derive(Default)]
pub struct InfoCache(Arc<Mutex<Cache>>);

#[derive(Default)]
pub struct LiveCache(Mutex<HashMap<String,(Option<PathBuf>,Instant)>>);

fn find_session(root: &Path, id: &str, depth: usize) -> Option<PathBuf> {
    if depth > 3 { return None; }
    for entry in std::fs::read_dir(root).ok()?.take(50_000).filter_map(Result::ok) {
        let path = entry.path();
        if entry.file_type().ok()?.is_dir() {
            if let Some(found) = find_session(&path,id,depth+1) {return Some(found);}
        } else if entry.file_name().to_string_lossy().ends_with(&format!("{id}.jsonl")) {return Some(path);}
    }
    None
}

pub(crate) fn timestamp_seconds(text: &str) -> Option<u64> {
    if text.len()<20 || !text.ends_with('Z') {return None;}
    let y: u64=text.get(0..4)?.parse().ok()?;
    let month: usize=text.get(5..7)?.parse().ok()?;
    let day: u64=text.get(8..10)?.parse().ok()?;
    let hour: u64=text.get(11..13)?.parse().ok()?;
    let minute: u64=text.get(14..16)?.parse().ok()?;
    let second: u64=text.get(17..19)?.parse().ok()?;
    if !(1970..=2100).contains(&y) || !(1..=12).contains(&month) || day==0 || hour>23 || minute>59 || second>59 {return None;}
    let leap=|year| year%4==0 && (year%100!=0 || year%400==0);
    let months=[31,if leap(y){29}else{28},31,30,31,30,31,31,30,31,30,31];
    if day>months[month-1] {return None;}
    let days=(1970..y).map(|year| if leap(year){366}else{365}).sum::<u64>()+months[..month-1].iter().sum::<u64>()+day-1;
    Some(days*86400+hour*3600+minute*60+second)
}

fn live_report(line: &str) -> Option<Info> {
    let value: Value=serde_json::from_str(line).ok()?;
    if value.get("type")?.as_str()? != "event_msg" || value.pointer("/payload/type")?.as_str()? != "token_count" {return None;}
    let bucket=value.pointer("/payload/rate_limits")?;
    if bucket.get("limit_id").and_then(Value::as_str).is_some_and(|id|id!="codex") {return None;}
    let convert=|v:&Value|window(&json!({"usedPercent":v.get("used_percent"),"windowDurationMins":v.get("window_minutes"),"resetsAt":v.get("resets_at")}));
    let primary=bucket.get("primary").and_then(convert);
    let secondary=bucket.get("secondary").and_then(convert);
    if primary.is_none() && secondary.is_none() {return None;}
    Some(Info {primary,secondary,checked_at:timestamp_seconds(value.get("timestamp")?.as_str()?)?,..Info::default()})
}

#[tauri::command]
pub fn codex_live_limits(thread_ids: Vec<String>, live: tauri::State<'_,LiveCache>) -> Option<Info> {
    let root=std::env::var_os("CODEX_HOME").map(PathBuf::from).or_else(||std::env::var_os("USERPROFILE").map(|p|PathBuf::from(p).join(".codex")))?.join("sessions");
    let mut paths=live.0.lock().ok()?;
    let mut newest:Option<Info>=None;
    for id in thread_ids.into_iter().take(16) {
        if super::codex::chat_url(Some(&id)).is_err() {continue;}
        let entry=paths.entry(id.clone()).or_insert_with(||(None,Instant::now()-Duration::from_secs(10)));
        if entry.0.is_none() && entry.1.elapsed().as_secs()>=10 {entry.0=find_session(&root,&id,0);entry.1=Instant::now();}
        let Some(path)=&entry.0 else {continue;};
        let Ok(mut file)=std::fs::File::open(path) else {continue;};
        let Ok(size)=file.metadata().map(|m|m.len()) else {continue;};
        if file.seek(SeekFrom::Start(size.saturating_sub(512*1024))).is_err() {continue;}
        let mut bytes=Vec::new();if file.take(512*1024).read_to_end(&mut bytes).is_err(){continue;}
        for line in String::from_utf8_lossy(&bytes).lines().rev() {
            if !line.contains("\"token_count\"") || !line.contains("\"rate_limits\"") {continue;}
            if let Some(info)=live_report(line) {if newest.as_ref().is_none_or(|old|info.checked_at>old.checked_at){newest=Some(info);}break;}
        }
    }
    newest
}

fn executable() -> Option<PathBuf> {
    // Prefer the desktop app's native binary, then the native CLI installation.
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        let root = PathBuf::from(local).join("OpenAI/Codex/bin");
        let mut candidates: Vec<_> = std::fs::read_dir(root).ok().into_iter().flatten()
            .filter_map(Result::ok).map(|entry| entry.path().join("codex.exe"))
            .filter(|path| path.is_file()).collect();
        candidates.sort_by_key(|path| std::fs::metadata(path).ok().and_then(|m| m.modified().ok()));
        if let Some(path) = candidates.pop() { return Some(path); }
    }
    if let Some(roaming) = std::env::var_os("APPDATA") {
        let path = PathBuf::from(roaming).join("npm/node_modules/@openai/codex/node_modules/@openai/codex-win32-x64/vendor/x86_64-pc-windows-msvc/bin/codex.exe");
        if path.is_file() { return Some(path); }
    }
    std::env::var_os("PATH").and_then(|paths| std::env::split_paths(&paths)
        .map(|path| path.join("codex.exe")).find(|path| path.is_file()))
}

struct Client { child: Child, lines: mpsc::Receiver<Value>, id: u64, deadline: Instant }
impl Client {
    fn start() -> Result<Self, String> {
        let exe = executable().ok_or("Codex is not installed.")?;
        let mut child = Command::new(exe).args(["app-server", "--listen", "stdio://"])
            .creation_flags(super::CREATE_NO_WINDOW).stdin(Stdio::piped()).stdout(Stdio::piped())
            .stderr(Stdio::null()).spawn().map_err(|_| "Couldn't read Codex account information.")?;
        let stdout = child.stdout.take().ok_or("Codex metadata connection failed.")?;
        let (tx, lines) = mpsc::channel();
        std::thread::spawn(move || {
            // Bound both total output and the reader's lifetime to this child.
            for line in BufReader::new(stdout.take(2 * 1024 * 1024)).lines() {
                let Ok(line) = line else { break };
                if let Ok(value) = serde_json::from_str(&line) { if tx.send(value).is_err() { break; } }
            }
        });
        let mut client = Self { child, lines, id: 0, deadline: Instant::now() + Duration::from_secs(15) };
        client.call("initialize", json!({"clientInfo":{"name":"coucou_metadata","version":"0.1.2"},"capabilities":{"experimentalApi":true}}))?;
        client.write(json!({"method":"initialized"}))?;
        Ok(client)
    }
    fn write(&mut self, message: Value) -> Result<(), String> {
        let input = self.child.stdin.as_mut().ok_or("Codex metadata connection closed.")?;
        writeln!(input, "{message}").and_then(|_| input.flush()).map_err(|_| "Codex metadata connection closed.".into())
    }
    fn call(&mut self, method: &str, params: Value) -> Result<Value, String> {
        self.id += 1;
        self.write(json!({"id":self.id,"method":method,"params":params}))?;
        loop {
            let wait = self.deadline.saturating_duration_since(Instant::now());
            let message = self.lines.recv_timeout(wait).map_err(|_| "Codex information is temporarily unavailable.")?;
            if message.get("id").and_then(Value::as_u64) != Some(self.id) { continue; }
            // Do not expose server errors, authentication data, or stderr to the UI/log.
            if message.get("error").is_some() { return Err("Codex information is unavailable. Check that Codex is signed in.".into()); }
            return message.get("result").cloned().ok_or("Codex metadata response was incomplete.".into());
        }
    }
}
impl Drop for Client { fn drop(&mut self) { let _ = self.child.kill(); let _ = self.child.wait(); } }

fn window(value: &Value) -> Option<Window> {
    let used = value.get("usedPercent")?.as_f64()?;
    if !used.is_finite() { return None; }
    Some(Window { used_percent: used.clamp(0.0, 100.0),
        window_duration_mins: value.get("windowDurationMins")?.as_u64()?,
        resets_at: value.get("resetsAt").and_then(Value::as_u64) })
}
fn limits(value: &Value) -> (Option<Window>, Option<Window>) {
    let bucket = value.get("rateLimitsByLimitId").and_then(|v| v.get("codex"))
        .filter(|v| !v.is_null()).or_else(|| value.get("rateLimits"));
    let Some(bucket) = bucket else { return (None, None) };
    (bucket.get("primary").and_then(window), bucket.get("secondary").and_then(window))
}
fn thread_title(value: &Value) -> Option<String> {
    let name = value.get("thread")?.get("name")?.as_str()?.trim();
    if name.is_empty() { None } else { Some(name.chars().take(120).collect()) }
}
fn read(cache: Arc<Mutex<Cache>>, thread_id: Option<String>, force: bool) -> Result<Info, String> {
    let mut cache = cache.lock().map_err(|_| "Codex metadata cache is unavailable.")?;
    let ttl = 30;
    let refresh = cache.checked.map_or(true, |t| t.elapsed().as_secs() >= ttl || (force && t.elapsed().as_secs() >= 3));
    let read_title = thread_id.as_ref().is_some_and(|id| cache.titles.get(id).map_or(true, |(_, at)| at.elapsed().as_secs() >= 300));
    if refresh || read_title {
        let mut client = Client::start();
        if refresh {
            let result = client.as_mut().map_err(|e| e.clone()).and_then(|c| c.call("account/rateLimits/read", json!({})));
            cache.checked = Some(Instant::now());
            match result {
                Ok(value) => {
                    let (primary, secondary) = limits(&value);
                    cache.info.primary = primary; cache.info.secondary = secondary;
                    cache.info.checked_at = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
                    cache.info.error = if cache.info.primary.is_none() && cache.info.secondary.is_none() { Some("Codex did not provide usage limits.".into()) } else { None };
                }
                Err(error) => { cache.info.primary = None; cache.info.secondary = None; cache.info.error = Some(error); }
            }
        }
        if read_title {
            if let Some(id) = &thread_id {
                let title = client.as_mut().ok().and_then(|c| c.call("thread/read", json!({"threadId":id,"includeTurns":false})).ok()).and_then(|v| thread_title(&v));
                cache.titles.insert(id.clone(), (title, Instant::now()));
            }
        }
    }
    let mut info = cache.info.clone();
    info.title = thread_id.as_ref().and_then(|id| cache.titles.get(id)).and_then(|(title, _)| title.clone());
    info.thread_id = thread_id;
    Ok(info)
}

#[tauri::command]
pub async fn codex_info(thread_id: Option<String>, force: Option<bool>, cache: tauri::State<'_, InfoCache>) -> Result<Info, String> {
    if let Some(id) = &thread_id { super::codex::chat_url(Some(id))?; }
    let cache = cache.0.clone();
    tauri::async_runtime::spawn_blocking(move || read(cache, thread_id, force.unwrap_or(false)))
        .await.map_err(|_| "Couldn't read Codex information.".to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn live_reports_use_only_codex_quota_and_valid_utc_timestamps() {
        let line=json!({"type":"event_msg","timestamp":"2026-10-01T08:26:12.216Z","payload":{"type":"token_count","rate_limits":{"limit_id":"codex","primary":{"used_percent":19,"window_minutes":300,"resets_at":123},"secondary":{"used_percent":85,"window_minutes":10080,"resets_at":456}}}});
        let info=live_report(&line.to_string()).unwrap();
        assert_eq!(info.checked_at,1790843172);
        assert_eq!(info.primary.unwrap().used_percent,19.0);
        assert_eq!(info.secondary.unwrap().used_percent,85.0);
        let mut unrelated=line.clone();unrelated["payload"]["rate_limits"]["limit_id"]=json!("codex_other");
        assert!(live_report(&unrelated.to_string()).is_none());
        assert!(live_report("partial line").is_none());
        assert!(timestamp_seconds("2026-02-30T00:00:00Z").is_none());
        assert_eq!(timestamp_seconds("1970-01-01T00:00:00Z"),Some(0));
    }
    #[test] fn codex_bucket_takes_priority_and_missing_windows_stay_unknown() {
        let value = json!({"rateLimits":{"primary":{"usedPercent":1,"windowDurationMins":300}},"rateLimitsByLimitId":{"codex":{"primary":{"usedPercent":52,"windowDurationMins":300,"resetsAt":123},"secondary":null}}});
        let (primary, secondary) = limits(&value);
        assert_eq!(primary.unwrap().used_percent, 52.0); assert!(secondary.is_none());
        assert!(limits(&json!({})).0.is_none());
        assert!(window(&json!({"windowDurationMins":300})).is_none());
    }
    #[test] fn titles_use_the_real_name_instead_of_folder_or_first_prompt() {
        assert_eq!(thread_title(&json!({"thread":{"name":" Coucou Codex Support ","preview":"So I installed","cwd":"D:/so"}})).as_deref(), Some("Coucou Codex Support"));
        assert!(thread_title(&json!({"thread":{"name":null,"preview":"So I installed"}})).is_none());
    }
}
