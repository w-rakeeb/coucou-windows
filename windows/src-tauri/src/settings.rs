// Preferences, stored as plain JSON in %APPDATA%\Coucou\settings.json.
// No secret ever lands here — API keys live in the Windows Credential Manager.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    #[serde(default = "default_settings_interface")]
    pub settings_interface: String,
    #[serde(default = "default_chat_provider")]
    pub chat_provider: String,
    #[serde(default = "default_openai_model")]
    pub openai_model: String,
    #[serde(default = "default_openrouter_model")]
    pub openrouter_model: String,
    #[serde(default = "default_visible")]
    pub always_on_top: bool,
    #[serde(default)]
    pub position_locked: bool,
    #[serde(default = "default_visible")]
    pub edge_snap: bool,
    #[serde(default)]
    pub pet_color: String,
    #[serde(default = "default_scale")]
    pub compact_scale: f64,
    #[serde(default = "default_scale")]
    pub expanded_scale: f64,
    #[serde(default = "default_visible")]
    pub remember_placement: bool,
    #[serde(default = "default_visible")]
    pub expanded_reset: bool,
    #[serde(default = "default_visible")]
    pub compact_limits: bool,
    #[serde(default = "default_visible")]
    pub compact_activity: bool,
    #[serde(default)]
    pub compact_reset: bool,
    #[serde(default)]
    pub keep_expanded: bool,
    #[serde(default)]
    pub keep_minimized: bool,
    #[serde(default = "default_hide_delay")]
    pub minimize_hide_interval: f64,
    #[serde(default)]
    pub free_placement: bool,
    #[serde(default = "default_position_x")]
    pub position_x: f64,
    #[serde(default)]
    pub position_y: f64,
    pub sound_enabled: bool,
    pub sound_volume: f64,
    pub auto_close_interval: f64,
    pub absence_interval: f64,
    pub active_integrations: Vec<String>,
    /// "primary" = the main display, "cursor" = whichever display the mouse is on.
    pub screen: String,
    pub autostart: bool,
    #[serde(default)]
    pub hide_tray_icon: bool,
    pub hooks_installed: bool,
    #[serde(default)]
    pub codex_hooks_installed: bool,
    /// Claude model used by the chat. Changeable in the settings window.
    /// Defaulted explicitly so a settings.json written by an older build still loads.
    #[serde(default = "default_model")]
    pub model: String,
}

fn default_model() -> String {
    crate::claude::DEFAULT_MODEL.to_string()
}

fn default_visible() -> bool { true }
fn default_settings_interface() -> String { "v2".into() }
fn default_chat_provider() -> String { "anthropic".into() }
fn default_openai_model() -> String { "gpt-4.1-mini".into() }
fn default_openrouter_model() -> String { "openai/gpt-4.1-mini".into() }
fn default_scale() -> f64 { 1.0 }
fn default_hide_delay() -> f64 { 60.0 }
fn default_position_x() -> f64 { 0.5 }

impl Default for Settings {
    fn default() -> Self {
        Self {
            settings_interface: default_settings_interface(),
            chat_provider: default_chat_provider(),
            openai_model: default_openai_model(),
            openrouter_model: default_openrouter_model(),
            always_on_top: true,
            position_locked: false,
            edge_snap: true,
            pet_color: String::new(),
            compact_scale: 1.0,
            expanded_scale: 1.0,
            remember_placement: true,
            expanded_reset: true,
            compact_limits: true,
            compact_activity: true,
            compact_reset: false,
            keep_expanded: false,
            keep_minimized: false,
            minimize_hide_interval: 60.0,
            free_placement: false,
            position_x: 0.5,
            position_y: 0.0,
            sound_enabled: true,
            sound_volume: 0.12,
            auto_close_interval: 15.0,
            absence_interval: 180.0,
            active_integrations: vec![
                "integration_resend".into(),
                "integration_n8n".into(),
                "integration_vercel".into(),
                "integration_github".into(),
            ],
            screen: "primary".into(),
            autostart: false,
            hide_tray_icon: false,
            hooks_installed: false,
            codex_hooks_installed: false,
            model: default_model(),
        }
    }
}

/// %APPDATA%\Coucou
pub fn config_dir() -> PathBuf {
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("Coucou")
}

/// %LOCALAPPDATA%\Coucou — where coucou-hook.exe and the log live.
pub fn local_dir() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("Coucou")
}

pub fn hook_exe_path() -> PathBuf {
    local_dir().join("bin").join("coucou-hook.exe")
}

fn settings_path() -> PathBuf {
    config_dir().join("settings.json")
}

pub fn load() -> Settings {
    match std::fs::read(settings_path()) {
        Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_default(),
        Err(_) => Settings::default(),
    }
}

pub fn save(settings: &Settings) -> std::io::Result<()> {
    let dir = config_dir();
    std::fs::create_dir_all(&dir)?;
    let saved = saved_preferences(settings);
    let json = serde_json::to_vec_pretty(&saved)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(settings_path(), json)
}

fn saved_preferences(settings: &Settings) -> Settings {
    let mut saved=settings.clone();
    if !saved.remember_placement && !saved.position_locked { saved.free_placement=false; saved.position_x=0.5; saved.position_y=0.0; }
    saved
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn forgetting_position_preserves_other_preferences() {
        let mut prefs=Settings::default();prefs.free_placement=true;prefs.position_x=0.8;prefs.position_y=0.6;prefs.compact_scale=1.3;prefs.pet_color="#cc88ff".into();
        assert_eq!(saved_preferences(&prefs).position_x,0.8);
        prefs.remember_placement=false;
        let saved=saved_preferences(&prefs);
        assert!(!saved.free_placement);assert_eq!((saved.position_x,saved.position_y),(0.5,0.0));
        assert_eq!(saved.compact_scale,1.3);assert_eq!(saved.pet_color,"#cc88ff");assert_eq!(prefs.position_x,0.8);
        prefs.position_locked=true;
        assert_eq!((saved_preferences(&prefs).position_x,saved_preferences(&prefs).position_y),(0.8,0.6));
    }
    #[test]
    fn compact_preferences_migrate_and_preserve_explicit_off() {
        let mut old = serde_json::to_value(Settings::default()).unwrap();
        old.as_object_mut().unwrap().remove("compactLimits");
        old.as_object_mut().unwrap().remove("compactActivity");
        for key in ["settingsInterface","chatProvider","openaiModel","openrouterModel","alwaysOnTop","positionLocked","edgeSnap","petColor","compactScale","expandedScale","rememberPlacement","expandedReset","compactReset","keepExpanded","keepMinimized","minimizeHideInterval","freePlacement","positionX","positionY"] {old.as_object_mut().unwrap().remove(key);}
        let migrated: Settings = serde_json::from_value(old).unwrap();
        assert!(migrated.compact_limits && migrated.compact_activity);
        assert!(!migrated.compact_reset && !migrated.keep_expanded && !migrated.keep_minimized && !migrated.free_placement);
        assert_eq!(migrated.minimize_hide_interval,60.0);
        assert_eq!((migrated.position_x,migrated.position_y),(0.5,0.0));
        assert!(migrated.pet_color.is_empty());
        assert_eq!((migrated.compact_scale,migrated.expanded_scale),(1.0,1.0));
        assert!(migrated.remember_placement && migrated.expanded_reset);
        assert!(migrated.always_on_top && migrated.edge_snap && !migrated.position_locked);
        assert_eq!(migrated.chat_provider,"anthropic");
        assert_eq!(migrated.openai_model,"gpt-4.1-mini");
        let mut off = migrated;
        off.compact_limits = false;
        off.compact_activity = false;
        off.always_on_top = false;
        off.position_locked = true;
        off.edge_snap = false;
        off.chat_provider = "openrouter".into();
        off.openrouter_model = "example/model".into();
        let loaded: Settings = serde_json::from_slice(&serde_json::to_vec(&off).unwrap()).unwrap();
        assert!(!loaded.compact_limits && !loaded.compact_activity);
        assert!(!loaded.always_on_top && loaded.position_locked && !loaded.edge_snap);
        assert_eq!((loaded.chat_provider.as_str(),loaded.openrouter_model.as_str()),("openrouter","example/model"));
    }
}
