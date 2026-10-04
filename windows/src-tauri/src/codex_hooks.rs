// Codex lifecycle hooks. Install/remove only our individual handlers, with a
// preview fingerprint and byte-for-byte backup. Codex's own trust review remains
// mandatory; this module never writes trust records or changes approval policy.
use std::path::{Path, PathBuf};
use serde_json::{json, Value};
use crate::{hooks::{self, HookPreview, HookStatus}, settings};

pub const EVENTS: &[(&str, u64)] = &[
    ("SessionStart", 3), ("SessionEnd", 3), ("UserPromptSubmit", 3),
    ("PreToolUse", 3), ("PostToolUse", 3), ("PermissionRequest", 120),
    ("Stop", 3), ("Interrupt", 3), ("SubagentStart", 3), ("SubagentStop", 3),
];

pub fn config_dir() -> PathBuf {
    std::env::var_os("CODEX_HOME").map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(std::env::var_os("USERPROFILE").unwrap_or_default()).join(".codex")
    })
}

pub fn path() -> PathBuf { config_dir().join("hooks.json") }

fn command(exe: &str, event: &str) -> String {
    // A quoted executable by itself is a string expression in PowerShell.
    // CALL lets cmd keep paths with spaces executable under either host shell.
    format!("cmd.exe /d /c call \"{exe}\" --codex {event}")
}

fn read_at(path: &Path) -> Result<Vec<u8>, String> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(bytes),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(format!("Can't read {}: {e}", path.display())),
    }
}

fn parse(bytes: &[u8], path: &Path) -> Result<Value, String> {
    let value = hooks::parse_settings(bytes, &path.display().to_string())?;
    if let Some(events) = value.get("hooks") {
        let events = events.as_object().ok_or("hooks must be an object; nothing was changed")?;
        for (event, groups) in events {
            let groups = groups.as_array().ok_or_else(|| format!("{event} hooks must be an array"))?;
            for group in groups {
                if !group.is_object() || !group.get("hooks").map(Value::is_array).unwrap_or(false) {
                    return Err(format!("Invalid {event} hook group; nothing was changed"));
                }
            }
        }
    }
    Ok(value)
}

fn is_ours(handler: &Value) -> bool {
    // Do not remove foreign handlers just because they share a matcher group.
    handler.get("command").and_then(Value::as_str).map(|c| {
        let lower = c.replace('\\', "/").to_lowercase();
        lower.contains("/coucou-hook.exe\" --codex ")
    }).unwrap_or(false)
}

pub(crate) fn merged(existing: &Value, install: bool) -> Value {
    let mut next = existing.clone();
    if next.get("hooks").is_none() { next["hooks"] = json!({}); }
    let events = next["hooks"].as_object_mut().expect("validated hooks");
    let mut emptied = Vec::new();
    for (event, groups) in events.iter_mut() {
        if let Some(groups) = groups.as_array_mut() {
            let mut removed_ours = false;
            groups.retain_mut(|group| {
                let Some(handlers) = group.get_mut("hooks").and_then(Value::as_array_mut) else { return true };
                let had_ours = handlers.iter().any(is_ours);
                removed_ours |= had_ours;
                handlers.retain(|h| !is_ours(h));
                !had_ours || !handlers.is_empty()
            });
            if removed_ours && groups.is_empty() { emptied.push(event.clone()); }
        }
    }
    for event in emptied { events.remove(&event); }
    if install {
        let exe = settings::hook_exe_path().to_string_lossy().replace('\\', "/");
        for (event, timeout) in EVENTS {
            let list = events.entry(*event).or_insert_with(|| json!([]));
            list.as_array_mut().expect("validated group list").push(json!({
                "hooks": [{"type": "command", "command": command(&exe, event), "timeout": timeout}]
            }));
        }
    } else if events.is_empty() {
        next.as_object_mut().unwrap().remove("hooks");
    }
    next
}

pub fn status() -> HookStatus {
    let p = path();
    let exe = settings::hook_exe_path().to_string_lossy().replace('\\', "/");
    let installed = read_at(&p).and_then(|bytes| parse(&bytes, &p)).ok().map(|v| {
        EVENTS.iter().all(|(event, _)| v["hooks"][*event].as_array().map(|groups| {
            let expected = command(&exe, event);
            groups.iter().any(|g| g["hooks"].as_array().map(|h| h.iter().any(|handler| handler["command"].as_str() == Some(expected.as_str()))).unwrap_or(false))
        }).unwrap_or(false))
    }).unwrap_or(false);
    let relay = settings::hook_exe_path();
    HookStatus { installed, settings_path: p.display().to_string(), hook_path: relay.display().to_string(), hook_ready: relay.is_file() }
}

pub fn preview(install: bool) -> Result<HookPreview, String> {
    let p = path();
    let bytes = read_at(&p)?;
    let current = parse(&bytes, &p)?;
    let next = merged(&current, install);
    Ok(HookPreview {
        diff: hooks::unified_diff(&serde_json::to_string_pretty(&current).unwrap(), &serde_json::to_string_pretty(&next).unwrap()),
        backup: p.with_file_name(format!("hooks.json.bak-coucou-{}", hooks::stamp())).display().to_string(),
        settings_path: p.display().to_string(), fingerprint: hooks::fingerprint(&bytes),
    })
}

pub fn write(install: bool, fingerprint: &str) -> Result<String, String> {
    write_at(&path(), install, fingerprint)
}

fn write_at(p: &Path, install: bool, fingerprint: &str) -> Result<String, String> {
    let bytes = read_at(p)?;
    let current = parse(&bytes, p)?;
    if hooks::fingerprint(&bytes) != fingerprint {
        return Err("Codex hooks changed since the preview. Review the new diff; nothing was written.".into());
    }
    let next = merged(&current, install);
    if next == current { return Ok("No changes needed.".into()); }
    let dir = p.parent().ok_or("No hooks directory")?;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let backup = p.with_file_name(format!("hooks.json.bak-coucou-{}-{}", hooks::stamp(), std::process::id()));
    if p.exists() {
        let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(&backup).map_err(|e| format!("Backup failed: {e}"))?;
        use std::io::Write;
        file.write_all(&bytes).map_err(|e| format!("Backup failed: {e}"))?;
        file.sync_all().map_err(|e| e.to_string())?;
    }
    let temp = p.with_extension(format!("json.coucou-{}", std::process::id()));
    let text = format!("{}\n", serde_json::to_string_pretty(&next).unwrap());
    std::fs::write(&temp, text).map_err(|e| e.to_string())?;
    // Re-check after preparing the replacement, so intervening edits are refused.
    if read_at(p)? != bytes { let _ = std::fs::remove_file(&temp); return Err("Codex hooks changed during installation; nothing was written.".into()); }
    if let Err(e) = std::fs::rename(&temp, p) {
        let _ = std::fs::remove_file(&temp);
        return Err(format!("Could not replace hooks: {e}"));
    }
    Ok(if bytes.is_empty() { "Created hooks.json; there was no previous content.".into() } else { backup.display().to_string() })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_foreign_handlers_even_in_a_shared_group() {
        let ours = json!({"type":"command", "command":"\"C:/test/coucou-hook.exe\" --codex Stop"});
        let foreign = json!({"type":"command", "command":"my-notifier.exe"});
        let before = json!({"description":"Keep me", "hooks":{"Stop":[{"matcher":"*", "hooks":[ours, foreign.clone()]}], "Notification":[]}});
        let after = merged(&before, true);
        assert_eq!(after["description"], "Keep me");
        assert_eq!(after["hooks"]["Stop"][0]["hooks"], json!([foreign]));
        assert_eq!(merged(&after, true), after, "reinstall must not duplicate hooks");
        let removed = merged(&after, false);
        assert_eq!(removed["hooks"]["Stop"][0]["hooks"], json!([{"type":"command", "command":"my-notifier.exe"}]));
        assert_eq!(removed["hooks"]["Notification"], json!([]));
    }
    #[test]
    fn rejects_invalid_shapes_instead_of_overwriting_them() {
        for raw in [b"{\"hooks\":[]}".as_slice(), b"{\"hooks\":{\"Stop\":{}}}", b"{\"hooks\":{\"Stop\":[{}]}}"] {
            assert!(parse(raw, Path::new("hooks.json")).is_err());
        }
    }
    #[test]
    fn file_install_is_backed_up_and_stale_preview_is_refused() {
        let dir = std::env::temp_dir().join(format!("coucou-codex-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("hooks.json");
        let original = b"{\"description\":\"original\"}";
        std::fs::write(&p, original).unwrap();
        let backup = write_at(&p, true, &hooks::fingerprint(original)).unwrap();
        assert_eq!(std::fs::read(backup).unwrap(), original);
        assert!(write_at(&p, false, &hooks::fingerprint(original)).unwrap_err().contains("changed"));
        assert!(std::fs::read_to_string(&p).unwrap().contains("--codex"));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
