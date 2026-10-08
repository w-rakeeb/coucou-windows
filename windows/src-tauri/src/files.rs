// Dropped files are copied into %LOCALAPPDATA%\Coucou\inbox so the original is
// never touched and the copy survives the drag source going away.
// The inbox is swept of anything older than a week, as on macOS.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use serde::Serialize;
use base64::{Engine,engine::general_purpose::STANDARD};
use std::io::{Read,Write};
const MAX_BYTES:u64=25*1024*1024;

use crate::settings;

const KEEP_FOR: Duration = Duration::from_secs(7 * 24 * 60 * 60);

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DroppedFile {
    pub name: String,
    pub path: String,
    pub size: u64,
}

pub fn inbox_dir() -> PathBuf {
    settings::local_dir().join("inbox")
}

const DROP_VALID_FOR: Duration = Duration::from_secs(120);
const DROP_MAX_PENDING: usize = 64;

static DROPPED: std::sync::Mutex<Vec<(String, std::time::Instant)>> = std::sync::Mutex::new(Vec::new());

/// Records paths that came from a real drop.
pub fn allow_dropped<I: IntoIterator<Item = String>>(paths: I) {
    let mut list = DROPPED.lock().unwrap_or_else(|e| e.into_inner());
    let now = std::time::Instant::now();
    list.retain(|(_, at)| now.duration_since(*at) < DROP_VALID_FOR);
    for p in paths {
        list.push((p, now));
    }
    let excess = list.len().saturating_sub(DROP_MAX_PENDING);
    list.drain(..excess);
}

/// True (once) when `path` was delivered by a drop in the last couple of minutes.
fn take_dropped(path: &str) -> bool {
    let mut list = DROPPED.lock().unwrap_or_else(|e| e.into_inner());
    let now = std::time::Instant::now();
    list.retain(|(_, at)| now.duration_since(*at) < DROP_VALID_FOR);
    match list.iter().position(|(p, _)| p == path) {
        Some(i) => {
            list.remove(i);
            true
        }
        None => false,
    }
}


pub fn ingest(source: &str) -> Result<DroppedFile, String> {
    if !take_dropped(source) { return Err(crate::i18n::t("Only files dropped on the island can be added.")); }
    let src = Path::new(source);
    let meta = std::fs::metadata(src).map_err(|e| format!("cannot read {source}: {e}"))?;
    if meta.is_dir() {
        return Err("Folders can't be dropped yet.".into());
    }

    if meta.len()>MAX_BYTES{return Err("Choose a file smaller than 25 MB.".into());}
    let name=src.file_name().map(|n|n.to_string_lossy().to_string()).ok_or("Invalid file name")?;
    let mut input=std::fs::File::open(src).map_err(|e|e.to_string())?;
    let mut sample=vec![0;4096];let count=input.read(&mut sample).map_err(|e|e.to_string())?;
    validate(&name,&sample[..count])?;
    let (dest,mut output)=reserve(&name)?;
    let copy=std::fs::File::open(src).and_then(|mut file|std::io::copy(&mut file,&mut output));
    if let Err(e)=copy{drop(output);let _=std::fs::remove_file(&dest);return Err(format!("Cannot copy file: {e}"));}
    let _=output.set_modified(SystemTime::now());
    drop(output);sweep(&inbox_dir());
    Ok(DroppedFile{name,path:dest.to_string_lossy().to_string(),size:meta.len()})
}

/// Selected files arrive after the user chooses them in the Windows file picker.
pub fn receive(name:&str,data:&str)->Result<DroppedFile,String>{
    if data.len()>((MAX_BYTES as usize+2)/3)*4{return Err("Choose a file smaller than 25 MB.".into());}
    if name.is_empty()||Path::new(name).components().count()!=1||name=="."||name==".."{return Err("Invalid file name".into());}
    let bytes=STANDARD.decode(data).map_err(|_|"Could not read the selected file.")?;
    if bytes.len() as u64>MAX_BYTES{return Err("Choose a file smaller than 25 MB.".into());}
    validate(name,&bytes[..bytes.len().min(4096)])?;
    let (dest,mut output)=reserve(name)?;
    if let Err(e)=output.write_all(&bytes){drop(output);let _=std::fs::remove_file(&dest);return Err(e.to_string());}
    drop(output);sweep(&inbox_dir());
    Ok(DroppedFile{name:name.into(),path:dest.to_string_lossy().to_string(),size:bytes.len() as u64})
}

fn validate(name:&str,sample:&[u8])->Result<(),String>{
    let extension=Path::new(name).extension().and_then(|s|s.to_str()).unwrap_or("").to_lowercase();
    if ["pdf","png","jpg","jpeg","webp","gif"].contains(&extension.as_str()){return Ok(());}
    let text=std::str::from_utf8(sample).map(|_|true).unwrap_or_else(|e|e.error_len().is_none());
    if text&&!sample.contains(&0){Ok(())}else{Err("Choose a PDF, image, plain text or code file. Word documents must be exported as PDF first.".into())}
}

fn reserve(name:&str)->Result<(PathBuf,std::fs::File),String>{
    let dir=inbox_dir();std::fs::create_dir_all(&dir).map_err(|e|e.to_string())?;
    let source=Path::new(name);let stem=source.file_stem().and_then(|s|s.to_str()).unwrap_or("file");
    let extension=source.extension().map(|s|format!(".{}",s.to_string_lossy())).unwrap_or_default();
    for i in 1..10000{
        let dest=dir.join(if i==1{name.to_string()}else{format!("{stem} ({i}){extension}")});
        match std::fs::OpenOptions::new().create_new(true).write(true).open(&dest){
            Ok(file)=>return Ok((dest,file)),Err(e)if e.kind()==std::io::ErrorKind::AlreadyExists=>continue,Err(e)=>return Err(e.to_string())
        }
    }
    Err("Too many files with this name in the inbox.".into())
}

/// Drops anything copied here more than a week ago. `ingest` stamps every copy
/// with the time it landed, so this really is the age of the copy and not the
/// age of whatever the user happened to drag in.
fn sweep(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let now = SystemTime::now();
    for entry in entries.flatten() {
        let Ok(meta) = entry.metadata() else { continue };
        let Ok(copied) = meta.modified() else { continue };
        if now.duration_since(copied).map(|age| age > KEEP_FOR).unwrap_or(false) {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_upload_is_safe_and_keeps_bytes(){
        let name=format!("coucou-selected-{}.txt",std::process::id());
        let first=receive(&name,&STANDARD.encode("Selected ✓".as_bytes())).unwrap();
        let second=receive(&name,&STANDARD.encode(b"second")).unwrap();
        assert_ne!(first.path,second.path);assert_eq!(std::fs::read(&first.path).unwrap(),"Selected ✓".as_bytes());
        assert!(receive("../escape.txt","AA==").is_err());assert!(receive("bad.txt","not base64").is_err());
        assert!(receive("archive.docx",&STANDARD.encode([0x50,0x4b,0,0xff])).is_err());
        let _=std::fs::remove_file(&first.path);let _=std::fs::remove_file(&second.path);
    }

    #[test]
    fn ingest_copies_and_never_overwrites() {
        let tmp = std::env::temp_dir().join(format!("coucou-test-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let source = tmp.join("note.txt");
        std::fs::write(&source, b"hello").unwrap();

        assert!(ingest(source.to_str().unwrap()).is_err());
        allow_dropped([source.to_string_lossy().to_string()]);
        let first = ingest(source.to_str().unwrap()).unwrap();
        assert_eq!(first.name, "note.txt");
        assert_eq!(std::fs::read(&first.path).unwrap(), b"hello");

        // A second drop of the same name must not clobber the first copy.
        std::fs::write(&source, b"second").unwrap();
        allow_dropped([source.to_string_lossy().to_string()]);
        let second = ingest(source.to_str().unwrap()).unwrap();
        assert_ne!(first.path, second.path);
        assert_eq!(std::fs::read(&first.path).unwrap(), b"hello");
        assert_eq!(std::fs::read(&second.path).unwrap(), b"second");

        // Folders are refused rather than silently ignored.
        assert!(ingest(tmp.to_str().unwrap()).is_err());

        // An ancient source must not arrive already older than the sweep window.
        let old_source = tmp.join("ancient.txt");
        std::fs::write(&old_source, b"old").unwrap();
        let long_ago = SystemTime::now() - KEEP_FOR - Duration::from_secs(60 * 60);
        std::fs::File::options()
            .write(true)
            .open(&old_source)
            .unwrap()
            .set_modified(long_ago)
            .unwrap();
        allow_dropped([old_source.to_string_lossy().to_string()]);
        let aged = ingest(old_source.to_str().unwrap()).unwrap();
        assert!(
            Path::new(&aged.path).exists(),
            "a file copied just now was swept as if it were a week old"
        );
        let _ = std::fs::remove_file(&aged.path);

        let _ = std::fs::remove_file(&first.path);
        let _ = std::fs::remove_file(&second.path);
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
