// Coucou for Windows — app wiring and the commands the island calls.

mod claude;
mod api_chat;
mod codex;
mod codex_info;
mod codex_usage;
mod codex_hooks;
mod files;
mod hooks;
mod integrations;
mod island;
mod log;
mod pipe;
mod secrets;
mod settings;
mod tray;
mod win_user;

use std::os::windows::process::CommandExt;
use std::process::Command;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_autostart::{ManagerExt, MacosLauncher};

use claude::{Chat, ChatContext, ChatReply};
use files::DroppedFile;
use hooks::{HookPreview, HookStatus};
use island::{PollGate, ScreenInfo};
use pipe::Pending;
use settings::Settings;

/// Keeps spawned helpers from flashing a console window.
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub struct Shared {
    pub settings: Mutex<Settings>,
    pub gate: Arc<PollGate>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootInfo {
    settings: Settings,
    screen: ScreenInfo,
    version: String,
    hook_path: String,
}

#[tauri::command]
fn boot(app: AppHandle, shared: State<Shared>) -> BootInfo {
    let mut settings = shared.settings.lock().unwrap().clone();
    // The real state of ~/.claude/settings.json wins over whatever we stored.
    settings.hooks_installed = hooks::status().installed;
    settings.codex_hooks_installed = codex_hooks::status().installed;
    let screen = island::screen_info(&app, &settings.screen);
    BootInfo {
        settings,
        screen,
        version: env!("CARGO_PKG_VERSION").to_string(),
        hook_path: settings::hook_exe_path().to_string_lossy().to_string(),
    }
}

#[tauri::command]
fn save_settings(app: AppHandle, shared: State<Shared>, mut settings: Settings) {
    let (screen_changed, autostart_changed, tray_changed) = {
        let mut current = shared.settings.lock().unwrap();
        // Ignore placement edits from stale settings controls while locked.
        if current.position_locked && settings.position_locked {
            settings.screen = current.screen.clone();
            settings.free_placement = current.free_placement;
            settings.position_x = current.position_x;
            settings.position_y = current.position_y;
        }
        let screen_changed = current.screen != settings.screen || current.free_placement != settings.free_placement ||
            current.position_x != settings.position_x || current.position_y != settings.position_y || current.compact_scale != settings.compact_scale || current.expanded_scale != settings.expanded_scale;
        let autostart_changed = current.autostart != settings.autostart;
        let tray_changed = current.hide_tray_icon != settings.hide_tray_icon;
        *current = settings.clone();
        (screen_changed, autostart_changed, tray_changed)
    };
    if let Err(err) = settings::save(&settings) {
        eprintln!("[coucou] could not save settings: {err}");
    }
    if autostart_changed {
        let manager = app.autolaunch();
        let result = if settings.autostart { manager.enable() } else { manager.disable() };
        if let Err(err) = result {
            eprintln!("[coucou] autostart: {err}");
        }
    }
    if screen_changed {
        let collapsed = shared.gate.collapsed.load(Ordering::Relaxed);
        let height = *shared.gate.visible_height.lock().unwrap();
        island::apply_geometry(&app, &settings, collapsed, Some(height));
    }
    if let Some(win) = island::window(&app) { let _ = win.set_always_on_top(settings.always_on_top); }
    if tray_changed {
        if let Err(err) = tray::set_hidden(&app, settings.hide_tray_icon) {
            eprintln!("[coucou] tray visibility: {err}");
        }
    }
    // Keep the other window in step (island ⇄ settings window).
    let _ = app.emit("settings-changed", settings);
}

/// Hidden island → shrink the window to the invisible wake strip and park the
/// cursor poll; anything else → full panel and 60 Hz polling.
#[tauri::command]
fn set_collapsed(app: AppHandle, shared: State<Shared>, collapsed: bool, visible_height: Option<f64>) {
    if let Some(height)=visible_height { *shared.gate.visible_height.lock().unwrap()=height; }
    let pref = shared.settings.lock().unwrap().clone();
    shared.gate.collapsed.store(collapsed, Ordering::Relaxed);
    island::apply_geometry(&app, &pref, collapsed, visible_height);
    // The wake strip must always take the mouse, and a resize invalidates the flag.
    island::set_ignore_cursor(&app, false);
    shared.gate.forget_ignore_state();
    shared.gate.set_active(!collapsed);
}

/// The front end pushes the island shape; Rust decides click-through from it.
#[tauri::command]
fn set_island_rect(app: AppHandle, shared: State<Shared>, x: f64, y: f64, width: f64, height: f64) {
    let rect=island::IslandRect { x, y, w: width, h: height };
    shared.gate.set_rect(rect);
    let _=app; // No native window move on animation frames.
}

#[tauri::command]
fn focus_window(app: AppHandle, focused: bool) {
    let Some(win) = island::window(&app) else { return };
    island::set_activating(&win, focused);
    if focused {
        let _ = win.set_focus();
    }
}

#[tauri::command]
fn reposition(app: AppHandle, shared: State<Shared>) {
    let pref = shared.settings.lock().unwrap().clone();
    let collapsed = shared.gate.collapsed.load(Ordering::Relaxed);
    let height = *shared.gate.visible_height.lock().unwrap();
    island::apply_geometry(&app, &pref, collapsed, Some(height));
}

#[tauri::command]
fn open_url(url: String) {
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return;
    }
    let _ = Command::new("rundll32.exe")
        .args(["url.dll,FileProtocolHandler", &url])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn();
}

/// "Open terminal" opens the working folder in VS Code when `code` is on PATH,
/// and falls back to Explorer otherwise.
#[tauri::command]
fn open_in_vscode(path: Option<String>) -> bool {
    // No `cmd /C` anywhere near this. The path is a project folder chosen by
    // whoever is using Claude Code, and cmd would happily read `&`, `^` and `%`
    // in a folder name as syntax. Finding the launcher ourselves and handing the
    // path over as a separate argument keeps it a path.
    if let Some(code) = find_on_path("code") {
        let mut cmd = Command::new(code);
        if let Some(p) = path.as_deref().filter(|p| !p.is_empty()) {
            cmd.arg(p);
        }
        if cmd.creation_flags(CREATE_NO_WINDOW).spawn().is_ok() {
            return true;
        }
    }
    if let Some(p) = path.as_deref().filter(|p| !p.is_empty()) {
        let _ = Command::new("explorer").arg(p).spawn();
    }
    false
}

/// Our own `where`: walks %PATH% against %PATHEXT%, no shell involved.
/// Rust quotes arguments correctly for `.cmd`/`.bat` targets since 1.77, so
/// spawning `code.cmd` directly is safe.
fn find_on_path(stem: &str) -> Option<std::path::PathBuf> {
    let exts = std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into());
    let dirs = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&dirs) {
        for ext in exts.split(';').filter(|e| !e.is_empty()) {
            let candidate = dir.join(format!("{stem}{}", ext.to_lowercase()));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

#[tauri::command]
fn quit_app(app: AppHandle) {
    app.exit(0);
}

/// Tray → Pause. Paused means paused: the pollers stop talking to the network,
/// not just the island stopping showing things.
#[tauri::command]
fn set_paused(paused: bool) {
    integrations::set_paused(paused);
}

// ── Claude Code hooks ─────────────────────────────────────────────────────────

#[tauri::command]
fn hooks_status() -> HookStatus {
    hooks::status()
}

#[tauri::command]
fn open_project_folder(path: String) -> Result<(), String> {
    let folder = std::path::Path::new(&path);
    if !folder.is_absolute() || !folder.is_dir() { return Err("This project folder is unavailable.".into()); }
    Command::new("explorer.exe").arg(folder).creation_flags(CREATE_NO_WINDOW)
        .spawn().map_err(|_| "Couldn't open the project folder.".to_string())?;
    Ok(())
}

#[tauri::command]
fn codex_hooks_status() -> HookStatus { codex_hooks::status() }

#[tauri::command]
fn codex_hooks_preview(install: bool) -> Result<HookPreview, String> { codex_hooks::preview(install) }

#[tauri::command]
fn codex_hooks_apply(app: AppHandle, shared: State<Shared>, install: bool, fingerprint: String) -> Result<String, String> {
    let backup = codex_hooks::write(install, &fingerprint)?;
    let updated = {
        let mut current = shared.settings.lock().unwrap();
        current.codex_hooks_installed = codex_hooks::status().installed;
        let _ = settings::save(&current);
        current.clone()
    };
    let _ = app.emit("settings-changed", updated);
    Ok(backup)
}

/// Narrow setup CLI for scripted installations. It emits a reviewable JSON
/// report and requires the fingerprint from the preview to apply a change.
/// It never enables/trusts hooks on behalf of Codex.
pub fn setup_cli() -> bool {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) != Some("--codex-hooks") { return false; }
    let result: Result<serde_json::Value, String> = (|| {
        match args.get(1).map(String::as_str) {
            Some("status") => Ok(serde_json::to_value(codex_hooks::status()).unwrap()),
            Some("preview-install") => Ok(serde_json::to_value(codex_hooks::preview(true)?).unwrap()),
            Some("install") => {
                if !settings::hook_exe_path().is_file() { return Err("Relay is missing".into()); }
                let fingerprint = args.get(3).ok_or("Expected preview fingerprint")?;
                Ok(serde_json::json!({"backup":codex_hooks::write(true, fingerprint)?, "status":codex_hooks::status()}))
            }
            _ => Err("Expected status, preview-install, or install".into()),
        }
    })();
    let failed = result.is_err();
    let report = match result { Ok(v) => v, Err(e) => serde_json::json!({"error":e}) };
    let Some(output) = args.get(2) else { std::process::exit(2) };
    if std::fs::write(output, serde_json::to_vec_pretty(&report).unwrap()).is_err() { std::process::exit(2); }
    std::process::exit(if failed { 1 } else { 0 });
}

/// Returns the diff the user has to look at before anything is written.
#[tauri::command]
fn hooks_preview(install: bool) -> Result<HookPreview, String> {
    hooks::preview(install)
}

/// Only ever called from an explicit click in the settings window.
#[tauri::command]
fn hooks_apply(
    app: AppHandle,
    shared: State<Shared>,
    install: bool,
    fingerprint: String,
) -> Result<String, String> {
    // The fingerprint comes from the preview the user actually looked at, so a
    // settings.json that changed in between is refused rather than overwritten.
    let backup = hooks::write(install, &fingerprint)?;
    let updated = {
        let mut current = shared.settings.lock().unwrap();
        current.hooks_installed = install;
        let _ = settings::save(&current);
        current.clone()
    };
    let _ = app.emit("settings-changed", updated);
    Ok(backup)
}

#[tauri::command]
fn approval_decision(app: AppHandle, request_id: String, decision: String) {
    pipe::answer(&app, &request_id, &decision);
}

/// The island has the card on screen, so the long wait for a human may begin.
/// Until this arrives the relay only waits a few hundred milliseconds, which is
/// what stops a paused or unresponsive island from freezing Claude Code.
#[tauri::command]
fn approval_ack(app: AppHandle, request_id: String) {
    pipe::acknowledge(&app, &request_id);
}

/// Nobody can act on this request — the island is paused, or another card is
/// already up. Claude Code falls back to asking in the terminal immediately.
#[tauri::command]
fn approval_decline(app: AppHandle, request_id: String) {
    pipe::decline(&app, &request_id);
}

// ── Chat, files and secrets ───────────────────────────────────────────────────

/// One chat turn. The API key and any file bytes stay on the Rust side.
#[tauri::command]
async fn chat_send(
    shared: State<'_, Shared>,
    chat: State<'_, Chat>,
    api: State<'_, api_chat::ApiChat>,
    query: String,
    context: Option<ChatContext>,
) -> Result<ChatReply, String> {
    let prefs = shared.settings.lock().unwrap().clone();
    match prefs.chat_provider.as_str(){
        "anthropic" => claude::send(&chat, &prefs.model, query, context).await,
        "openai" => api_chat::send(&api,"openai",&prefs.openai_model,query,context).await,
        "openrouter" => api_chat::send(&api,"openrouter",&prefs.openrouter_model,query,context).await,
        _ => Err("Select a chat provider in Settings.".into())
    }
}

#[tauri::command]
async fn chat_reset(chat: State<'_, Chat>, api: State<'_, api_chat::ApiChat>) -> Result<(), String> {
    chat.reset();
    api.reset().await;
    Ok(())
}

/// Copies a dropped file into the inbox and reports its name back.
#[tauri::command]
async fn ingest_file(path: String) -> Result<DroppedFile, String> {
    tauri::async_runtime::spawn_blocking(move || files::ingest(&path)).await.map_err(|e|e.to_string())?
}

#[tauri::command]
async fn ingest_upload(name: String, data: String) -> Result<DroppedFile,String> {
    tauri::async_runtime::spawn_blocking(move || files::receive(&name,&data)).await.map_err(|e|e.to_string())?
}

/// The island may only ask whether a key exists — never read it.
#[tauri::command]
fn secret_present(key: String) -> bool {
    secrets::present(&key)
}

#[tauri::command]
fn secret_set(key: String, value: String) -> Result<(), String> {
    secrets::set(&key, &value)
}

#[tauri::command]
fn secret_clear(key: String) -> Result<(), String> {
    secrets::clear(&key)
}

/// Opens the configured n8n instance — the URL lives in the Credential Manager.
#[tauri::command]
fn open_n8n() {
    if let Some(url) = secrets::get("n8n-url") {
        open_url(url);
    }
}

/// Refresh buttons in the integration cards.
#[tauri::command]
async fn refresh_integration(app: AppHandle, id: String) {
    integrations::poll_once(app, &id).await;
}

/// Lets the island write to the same log as the Rust side.
#[tauri::command]
fn log_line(message: String) {
    log::line(format!("ui  {message}"));
}

// ── Settings window ───────────────────────────────────────────────────────────

/// WebView2 allows exactly one browser environment per app, and its options are
/// fixed by whichever webview is created first. Every window must therefore ask
/// for the *same* arguments as the island (see `additionalBrowserArgs` in
/// tauri.conf.json) — a mismatch makes the second window come up blank, with no
/// error anywhere.
const BROWSER_ARGS: &str = "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --autoplay-policy=no-user-gesture-required";

/// In a dev build the pages are served by Vite, so the second window needs the
/// absolute dev URL; a bundled build resolves it inside the app bundle.
fn settings_page_url(app: &AppHandle) -> WebviewUrl {
    #[cfg(dev)]
    if let Some(mut base) = app.config().build.dev_url.clone() {
        base.set_path("/settings.html");
        return WebviewUrl::External(base);
    }
    let _ = app;
    WebviewUrl::App("settings.html".into())
}

/// The settings window is created hidden at launch and only ever shown and
/// hidden afterwards. A WebView2 window created later — on the main thread or
/// not — silently comes up blank in this app, so the window that works is the
/// one that exists before the island's webview does.
fn create_settings_window(app: &AppHandle) {
    let url = settings_page_url(app);
    match WebviewWindowBuilder::new(app, "settings", url)
        .additional_browser_args(BROWSER_ARGS)
        .title("Settings — Coucou")
        .inner_size(560.0, 680.0)
        .min_inner_size(460.0, 600.0)
        .resizable(true)
        .visible(false)
        .center()
        .build()
    {
        Ok(win) => {
            protect_settings_bounds(&win);
            // Closing it must only hide it, or it could never be reopened.
            let hidden = win.clone();
            win.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = hidden.hide();
                }
            });
        }
        Err(err) => log::line(format!("settings window failed: {err}")),
    }
}

pub fn show_settings_window(app: &AppHandle) {
    let Some(win) = app.get_webview_window("settings") else {
        log::line("settings window missing");
        return;
    };
    let prefs=app.state::<Shared>().settings.lock().unwrap().clone();
    if let Some(parent)=island::window(app) {
        if let (Ok(origin),Some(m))=(parent.outer_position(),island::target_monitor(app,&prefs.screen)) {
            let shared=app.state::<Shared>();let r=*shared.gate.rect.lock().unwrap();
            let dpi=parent.scale_factor().unwrap_or(1.0);let work=m.work_area();let target_dpi=m.scale_factor();
            let parent_rect=(origin.x as f64+r.x*dpi,origin.y as f64+r.y*dpi,r.w*dpi,r.h*dpi);
            let preferred_w=if prefs.settings_interface=="v1" {580.0}else{660.0};
            let w=(preferred_w*target_dpi).min(work.size.width as f64-24.0*target_dpi).max(300.0);
            let h=(760.0*target_dpi).min(work.size.height as f64-64.0*target_dpi).max(300.0);
            if let (Ok(inner),Ok(outer),Ok(raw))=(win.inner_size(),win.outer_size(),win.hwnd()) {
                let panel_w=w.round() as i32+(outer.width as i32-inner.width as i32).max(0);
                let panel_h=h.round() as i32+(outer.height as i32-inner.height as i32).max(0);
                let (x,y)=settings_position(parent_rect,(work.position.x as f64,work.position.y as f64,work.size.width as f64,work.size.height as f64),(panel_w as f64,panel_h as f64),12.0*target_dpi);
                *SETTINGS_BOUNDS.lock().unwrap()=Some(SettingsBounds{x,y,w:panel_w,h:panel_h,min_w:(460.0*target_dpi) as i32+(panel_w-w as i32),min_h:((600.0*target_dpi) as i32+(panel_h-h as i32)).min(panel_h)});
                let _=win.unminimize();let _=win.show();
                let handle=raw.0 as isize;let popup=win.clone();
                // Size and position the HWND atomically, instead of queued size/move
                // requests that can reuse the hidden window's stale short bounds.
                let _=win.run_on_main_thread(move || {
                    use windows::Win32::UI::WindowsAndMessaging::{SetWindowPos,SWP_NOZORDER,SWP_NOACTIVATE};
                    let hwnd=windows::Win32::Foundation::HWND(handle as *mut _);
                    let result=unsafe{SetWindowPos(hwnd,None,x,y,panel_w,panel_h,SWP_NOZORDER|SWP_NOACTIVATE)};
                    if let Err(e)=result{log::line(format!("settings popup bounds failed: {e}"));}
                    let _=popup.set_focus();
                });
                return;
            }
        }
    }
    let _ = win.unminimize();
    let _ = win.show();
    let _ = win.set_focus();
}

static SETTINGS_ORIGINAL_PROC: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);
#[derive(Clone,Copy)]
struct SettingsBounds{x:i32,y:i32,w:i32,h:i32,min_w:i32,min_h:i32}
static SETTINGS_BOUNDS:Mutex<Option<SettingsBounds>>=Mutex::new(None);

fn protect_settings_bounds(win:&tauri::WebviewWindow) {
    use windows::Win32::UI::WindowsAndMessaging::{GetWindowLongPtrW,SetWindowLongPtrW,GWLP_WNDPROC};
    let Ok(raw)=win.hwnd() else{return};
    let hwnd=windows::Win32::Foundation::HWND(raw.0 as *mut _);
    // Keep the existing Tauri/WebView2 procedure in the chain.
    let old=unsafe{GetWindowLongPtrW(hwnd,GWLP_WNDPROC)};
    if old==0{return;}
    SETTINGS_ORIGINAL_PROC.store(old,Ordering::Relaxed);
    unsafe{SetWindowLongPtrW(hwnd,GWLP_WNDPROC,settings_bounds_proc as *const () as isize);}
}
unsafe extern "system" fn settings_bounds_proc(hwnd:windows::Win32::Foundation::HWND,msg:u32,w:windows::Win32::Foundation::WPARAM,l:windows::Win32::Foundation::LPARAM)->windows::Win32::Foundation::LRESULT {
    use windows::Win32::UI::WindowsAndMessaging::{CallWindowProcW,WM_WINDOWPOSCHANGING,WINDOWPOS,WNDPROC,SWP_NOSIZE,SWP_NOMOVE};
    if msg==WM_WINDOWPOSCHANGING {
        let p=unsafe{&mut *(l.0 as *mut WINDOWPOS)};
        if !p.flags.contains(SWP_NOSIZE)&&p.cx>0&&p.cy>0&&p.x>-30000&&p.y>-30000 {
            if let Some(b)=*SETTINGS_BOUNDS.lock().unwrap() {
                // Programmatic SetWindowPos calls can bypass Windows' normal
                // minimum-size checks. Discard stale, undersized popup bounds.
                if p.cx<b.min_w||p.cy<b.min_h {
                    p.x=b.x;p.y=b.y;p.cx=b.w;p.cy=b.h;p.flags&=!(SWP_NOSIZE|SWP_NOMOVE);
                }
            }
        }
    }
    let old:WNDPROC=unsafe{std::mem::transmute(SETTINGS_ORIGINAL_PROC.load(Ordering::Relaxed))};
    unsafe{CallWindowProcW(old,hwnd,msg,w,l)}
}

#[tauri::command]
fn open_settings_window(app: AppHandle) {
    show_settings_window(&app);
}

fn settings_position(parent:(f64,f64,f64,f64),work:(f64,f64,f64,f64),panel:(f64,f64),gap:f64)->(i32,i32){
    let(x,y,w,h)=parent;let(mx,my,mw,mh)=work;let(pw,ph)=panel;
    let max_x=(mx+mw-pw-gap).max(mx+gap);let max_y=(my+mh-ph-gap).max(my+gap);
    let fits=|(cx,cy):(f64,f64)|cx>=mx+gap&&cy>=my+gap&&cx+pw<=mx+mw-gap&&cy+ph<=my+mh-gap;
    // Prefer below, aligned with the island's right edge, then the available sides.
    let candidates=[((x+w-pw).clamp(mx+gap,max_x),y+h+gap),(x+w+gap,y.clamp(my+gap,max_y)),
        (x-pw-gap,y.clamp(my+gap,max_y)),((x+w-pw).clamp(mx+gap,max_x),y-ph-gap)];
    let chosen=candidates.iter().copied().find(|&c|fits(c)).unwrap_or((x.clamp(mx+gap,max_x),(y+h+gap).clamp(my+gap,max_y)));
    (chosen.0.round() as i32,chosen.1.round() as i32)
}

#[cfg(test)]
mod settings_position_tests{
    use super::*;
    #[test]
    fn popup_uses_available_sides_and_stays_in_work_area(){
        assert_eq!(settings_position((10.0,900.0,640.0,160.0),(0.0,0.0,2560.0,1400.0),(560.0,700.0),12.0),(662,688));
        assert_eq!(settings_position((1900.0,900.0,640.0,160.0),(0.0,0.0,2560.0,1400.0),(560.0,700.0),12.0),(1328,688));
        assert_eq!(settings_position((10.0,100.0,640.0,160.0),(0.0,0.0,2560.0,1400.0),(560.0,700.0),12.0),(90,272));
        assert_eq!(settings_position((1900.0,100.0,640.0,160.0),(0.0,0.0,2560.0,1400.0),(560.0,700.0),12.0),(1980,272));
        assert_eq!(settings_position((-1000.0,-300.0,736.0,184.0),(-1080.0,-313.0,1080.0,1880.0),(560.0,700.0),12.0),(-824,-104));
        assert_eq!(settings_position((-1000.0,1350.0,736.0,184.0),(-1080.0,-313.0,1080.0,1880.0),(560.0,700.0),12.0),(-824,638));
    }
}

pub fn run() {
    let loaded = settings::load();
    let gate = Arc::new(PollGate::new());

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            let _ = app.emit_to(island::WINDOW_LABEL, "tray", "open".to_string());
        }))
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))
        .manage(Shared {
            settings: Mutex::new(loaded.clone()),
            gate: gate.clone(),
        })
        .manage(Pending::default())
        .manage(Chat::default())
        .manage(api_chat::ApiChat::default())
        .manage(codex_info::InfoCache::default())
        .manage(codex_info::LiveCache::default())
        .manage(codex_usage::UsageCache::default())
        .invoke_handler(tauri::generate_handler![
            boot,
            save_settings,
            island::drag_island,
            set_collapsed,
            set_island_rect,
            focus_window,
            reposition,
            island::monitor_choices,
            open_url,
            open_in_vscode,
            open_project_folder,
            codex::open_codex_chat,
            codex_info::codex_info,
            codex_info::codex_live_limits,
            codex_usage::codex_telemetry,
            quit_app,
            hooks_status,
            codex_hooks_status,
            codex_hooks_preview,
            codex_hooks_apply,
            hooks_preview,
            hooks_apply,
            approval_decision,
            approval_ack,
            approval_decline,
            log_line,
            chat_send,
            chat_reset,
            ingest_file,
            ingest_upload,
            secret_present,
            secret_set,
            secret_clear,
            refresh_integration,
            open_n8n,
            open_settings_window,
            set_paused,
        ])
        .setup(move |app| {
            let handle = app.handle().clone();
            tray::set_hidden(&handle, loaded.hide_tray_icon)?;
            // Before the island: see create_settings_window.
            create_settings_window(&handle);

            if let Some(win) = island::window(&handle) {
                island::make_non_activating(&win);
                island::protect_geometry(&win,gate.clone());
                island::apply_geometry(&handle, &loaded, false, None);
                let _ = win.show();
            }
            gate.collapsed.store(false, Ordering::Relaxed);
            gate.set_active(true);
            island::spawn_cursor_poll(handle.clone(), gate.clone());

            log::line(format!("--- Coucou {} started ---", env!("CARGO_PKG_VERSION")));
            hooks::ensure_hook_exe(&handle);
            pipe::start(handle.clone());
            integrations::start(handle.clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Coucou");
}
