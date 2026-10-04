// Opening a chat is separate from submitting a message: the desktop protocol
// supports chat links, while the hook relay has no chat-send connection.
use std::os::windows::process::CommandExt;
use std::process::Command;

pub(crate) fn chat_url(thread_id: Option<&str>) -> Result<String, String> {
    let Some(id) = thread_id.filter(|id| !id.is_empty()) else {
        return Ok("codex://".into());
    };
    let parts: Vec<&str> = id.split('-').collect();
    if parts.len() != 5 || parts.iter().zip([8, 4, 4, 4, 12]).any(|(part, length)| {
        part.len() != length || !part.bytes().all(|b| b.is_ascii_hexdigit())
    }) {
        return Err("The session has no valid Codex chat ID.".into());
    }
    Ok(format!("codex://threads/{id}"))
}

#[tauri::command]
pub fn open_codex_chat(thread_id: Option<String>) -> Result<(), String> {
    let url = chat_url(thread_id.as_deref())?;
    Command::new("rundll32.exe")
        .args(["url.dll,FileProtocolHandler", &url])
        .creation_flags(super::CREATE_NO_WINDOW)
        .spawn().map_err(|error| format!("Couldn't open Codex: {error}"))?;
    super::log::line(&format!("open Codex {url}"));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::chat_url;
    #[test]
    fn accepts_a_chat_id_and_rejects_url_or_shell_syntax() {
        let id = "11111111-2222-3333-4444-555555555555";
        assert_eq!(chat_url(Some(id)).unwrap(), format!("codex://threads/{id}"));
        assert_eq!(chat_url(None).unwrap(), "codex://");
        for invalid in ["legacy", "../../../settings", "new?prompt=test", "id & echo test", "https://example.com", "11111111-2222-3333-4444-555555555555?prompt=test"] {
            assert!(chat_url(Some(invalid)).is_err());
        }
    }
}
