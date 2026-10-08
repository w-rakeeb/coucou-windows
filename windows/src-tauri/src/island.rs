// Island window: placement on the chosen display, the two window sizes
// (full panel / invisible wake strip), click-through and the cursor poll.
//
// There is no notch on a PC, so the island is a black shape drawn at the top
// centre of the main display inside a borderless, transparent, always-on-top
// window that never takes focus.

use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, Monitor, PhysicalPosition, PhysicalSize, WebviewWindow};

use windows::Win32::Foundation::{HWND, POINT};
use windows::core::BOOL;
use windows::Win32::Foundation::LPARAM;
use windows::Win32::System::Ole::RevokeDragDrop;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON};
use windows::Win32::UI::WindowsAndMessaging::{EnumChildWindows, GetClassNameW};
use windows::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW,
};

/// Logical size of the full window — the largest island view, like the macOS panel.
pub const PANEL_W: f64 = 720.0;
pub const PANEL_H: f64 = 320.0;
/// Logical size of the invisible strip that wakes the island when it is hidden.
pub const STRIP_W: f64 = 240.0;
pub const STRIP_H: f64 = 6.0;

pub const WINDOW_LABEL: &str = "island";

#[derive(Clone,Copy)]
struct NativeBounds { x:i32,y:i32,w:i32,h:i32 }
static EXPECTED_BOUNDS:Mutex<Option<NativeBounds>>=Mutex::new(None);
static ORIGINAL_PROC:AtomicIsize=AtomicIsize::new(0);
static DRAG_GATE:OnceLock<Arc<PollGate>>=OnceLock::new();

/// The island is not resizable. Keep late native placement requests from
/// replacing its saved envelope; Windows still owns movement during a drag.
pub fn protect_geometry(win:&WebviewWindow,gate:Arc<PollGate>) {
    use windows::Win32::UI::WindowsAndMessaging::GWLP_WNDPROC;
    let Some(hwnd)=hwnd_of(win) else{return};
    let old=unsafe{GetWindowLongPtrW(hwnd,GWLP_WNDPROC)};
    if old==0{return;}
    let _=DRAG_GATE.set(gate);
    ORIGINAL_PROC.store(old,Ordering::Relaxed);
    unsafe{SetWindowLongPtrW(hwnd,GWLP_WNDPROC,bounds_proc as *const () as isize);}
}

unsafe extern "system" fn bounds_proc(hwnd:HWND,msg:u32,w:windows::Win32::Foundation::WPARAM,l:LPARAM)->windows::Win32::Foundation::LRESULT {
    use windows::Win32::UI::WindowsAndMessaging::{CallWindowProcW,WM_WINDOWPOSCHANGING,WINDOWPOS,WNDPROC,SWP_NOSIZE,SWP_NOMOVE};
    if msg==WM_WINDOWPOSCHANGING && !DRAG_GATE.get().is_some_and(|g|g.moving.load(Ordering::Relaxed)) {
        let p=unsafe{&mut *(l.0 as *mut WINDOWPOS)};
        if p.x>-30000&&p.y>-30000 {
            if let Some(b)=*EXPECTED_BOUNDS.lock().unwrap() {
                if !p.flags.contains(SWP_NOMOVE){p.x=b.x;p.y=b.y;}
                if !p.flags.contains(SWP_NOSIZE)&&p.cx>0&&p.cy>0{p.cx=b.w;p.cy=b.h;}
            }
        }
    }
    let old:WNDPROC=unsafe{std::mem::transmute(ORIGINAL_PROC.load(Ordering::Relaxed))};
    unsafe{CallWindowProcW(old,hwnd,msg,w,l)}
}

/// Margin around the island that still counts as "on the island", in logical px.
/// Wider than the macOS 6 pt because a click must never be swallowed.
const HIT_MARGIN: f64 = 14.0;

#[derive(Serialize, Clone)]
pub struct CursorPayload {
    pub x: f64,
    pub y: f64,
}

#[derive(Serialize, Clone)]
pub struct ScreenInfo {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub scale: f64,
}

/// The island shape in window-logical coordinates, pushed by the front end.
/// The poll thread owns the click-through decision so it lands in the same 16 ms
/// tick as the cursor read — an IPC round trip here loses clicks.
#[derive(Clone, Copy, Default)]
pub struct IslandRect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

/// Wakes / parks the cursor poll thread so a hidden island costs literally nothing.
pub struct PollGate {
    active: Mutex<bool>,
    cv: Condvar,
    pub collapsed: AtomicBool,
    pub rect: Mutex<IslandRect>,
    pub visible_height: Mutex<f64>,
    pub moving: AtomicBool,
    /// Mirrors the window flag so we only call into Win32 when it changes.
    ignoring: AtomicBool,
}

impl PollGate {
    pub fn new() -> Self {
        Self {
            active: Mutex::new(false),
            cv: Condvar::new(),
            collapsed: AtomicBool::new(true),
            rect: Mutex::new(IslandRect::default()),
            visible_height: Mutex::new(160.0),
            moving: AtomicBool::new(false),
            ignoring: AtomicBool::new(false),
        }
    }

    pub fn set_rect(&self, rect: IslandRect) {
        *self.rect.lock().unwrap() = rect;
    }

    /// Forces the next poll tick to re-apply the flag (after a window resize).
    pub fn forget_ignore_state(&self) {
        self.ignoring.store(false, Ordering::Relaxed);
    }

    pub fn set_active(&self, on: bool) {
        let mut guard = self.active.lock().unwrap();
        *guard = on;
        self.cv.notify_all();
    }

    pub(crate) fn wait_until_active(&self) {
        let mut guard = self.active.lock().unwrap();
        while !*guard {
            guard = self.cv.wait(guard).unwrap();
        }
    }

    pub(crate) fn is_active(&self) -> bool {
        *self.active.lock().unwrap()
    }
}

pub fn window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(WINDOW_LABEL)
}

fn cursor_physical() -> Option<(f64, f64)> {
    let mut p = POINT::default();
    unsafe { GetCursorPos(&mut p).ok()? };
    Some((p.x as f64, p.y as f64))
}

/// Lets dropped files reach the app again.
///
/// wry installs its drop target by walking the webview's child windows **once**,
/// when the webview is created. WebView2 creates `Chrome_RenderWidgetHostHWND`
/// later and registers its own target on it; being the innermost window, that one
/// wins, and since the page has no HTML5 drop handler it refuses everything — the
/// "no drop" cursor, with nothing reaching Tauri. Revoking it makes OLE fall
/// through to the target wry registered on the parent widget, which is the one
/// that feeds Tauri's drag events.
///
/// Cheap and idempotent, so it is simply re-run whenever a drag might be starting.
pub fn unblock_webview_drops(app: &AppHandle) {
    for label in [WINDOW_LABEL, "settings"] {
        let Some(win) = app.get_webview_window(label) else { continue };
        let Some(hwnd) = hwnd_of(&win) else { continue };
        unsafe {
            let _ = EnumChildWindows(Some(hwnd), Some(revoke_render_widget), LPARAM(0));
        }
    }
}

unsafe extern "system" fn revoke_render_widget(hwnd: HWND, _: LPARAM) -> BOOL {
    let mut name = [0u16; 64];
    let len = unsafe { GetClassNameW(hwnd, &mut name) };
    if len > 0 {
        let class = String::from_utf16_lossy(&name[..len as usize]);
        if class == "Chrome_RenderWidgetHostHWND" {
            let _ = unsafe { RevokeDragDrop(hwnd) };
        }
    }
    true.into()
}

/// True while the left mouse button is held — the only signal we get that a
/// drag might be in flight before it reaches the window.
fn left_button_down() -> bool {
    unsafe { (GetAsyncKeyState(VK_LBUTTON.0 as i32) as u16 & 0x8000) != 0 }
}

fn monitor_contains(m: &Monitor, x: f64, y: f64) -> bool {
    let p = m.position();
    let s = m.size();
    x >= p.x as f64
        && x < (p.x + s.width as i32) as f64
        && y >= p.y as f64
        && y < (p.y + s.height as i32) as f64
}

/// The display the island lives on: the primary one, or the one under the cursor.
pub(crate) fn target_monitor(app: &AppHandle, pref: &str) -> Option<Monitor> {
    let monitors = app.available_monitors().ok()?;
    if let Some(id) = pref.strip_prefix("monitor:") {
        if let Some(m) = monitors.iter().find(|m| monitor_id(m) == id) { return Some(m.clone()); }
    }
    if pref == "cursor" {
        if let Some((cx, cy)) = cursor_physical() {
            if let Some(m) = monitors.iter().find(|m| monitor_contains(m, cx, cy)) {
                return Some(m.clone());
            }
        }
    }
    app.primary_monitor()
        .ok()
        .flatten()
        .or_else(|| monitors.into_iter().next())
}

fn monitor_id(m: &Monitor) -> String {
    m.name().cloned().unwrap_or_else(|| format!("{}:{}", m.position().x, m.position().y))
}

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct MonitorChoice { id: String, label: String, width: u32, height: u32, x:i32, y:i32, work_x:i32, work_y:i32, work_width:u32, work_height:u32, scale:f64 }

#[tauri::command]
pub fn monitor_choices(app: AppHandle) -> Vec<MonitorChoice> {
    let primary = app.primary_monitor().ok().flatten();
    app.available_monitors().unwrap_or_default().iter().enumerate().map(|(i,m)| {
        let is_primary = primary.as_ref().is_some_and(|p| p.position() == m.position());
        MonitorChoice { id: format!("monitor:{}", monitor_id(m)),
            label: format!("Monitor {}{} · {} × {}", i + 1, if is_primary { " (primary)" } else { "" }, m.size().width, m.size().height),
            width: m.size().width, height: m.size().height, x:m.position().x, y:m.position().y, work_x:m.work_area().position.x, work_y:m.work_area().position.y, work_width:m.work_area().size.width, work_height:m.work_area().size.height, scale:m.scale_factor() }
    }).collect()
}

fn placement(mp: (i32,i32), ms: (u32,u32), pw: u32, visible_w: u32, visible_h: u32, free: bool, x: f64, y: f64) -> (i32,i32) {
    let fraction = |v: f64, fallback: f64| if v.is_finite() {v.clamp(0.0,1.0)} else {fallback};
    let center = if free {fraction(x,0.5)} else {0.5};
    let top = if free {fraction(y,0.0)} else {0.0};
    let left = ((ms.0.saturating_sub(visible_w)) as f64 * center + visible_w as f64 / 2.0 - pw as f64 / 2.0).round() as i32;
    let y = ((ms.1.saturating_sub(visible_h)) as f64 * top).round() as i32;
    (mp.0 + left, mp.1 + y)
}

pub fn screen_info(app: &AppHandle, pref: &str) -> ScreenInfo {
    match target_monitor(app, pref) {
        Some(m) => {
            let scale = m.scale_factor();
            let p = m.position();
            let s = m.size();
            ScreenInfo {
                x: p.x as f64 / scale,
                y: p.y as f64 / scale,
                width: s.width as f64 / scale,
                height: s.height as f64 / scale,
                scale,
            }
        }
        None => ScreenInfo { x: 0.0, y: 0.0, width: 1920.0, height: 1080.0, scale: 1.0 },
    }
}

/// Places and sizes the window. `collapsed` picks the wake strip instead of the panel.
pub fn apply_geometry(app: &AppHandle, settings: &crate::settings::Settings, collapsed: bool, visible_h: Option<f64>) {
    let Some(win) = window(app) else { return };
    let Some(m) = target_monitor(app, &settings.screen) else { return };

    let scale = m.scale_factor();
    let area=m.work_area();let mp=area.position;let ms=area.size;
    let compact=settings.compact_scale.clamp(0.75,1.5);let expanded=settings.expanded_scale.clamp(0.75,1.5);
    let envelope=compact.max(expanded);
    let (lw,lh,max_w,max_h)=if collapsed {(STRIP_W*compact,STRIP_H*compact,STRIP_W*compact,STRIP_H*compact)}
      else {(PANEL_W*envelope,PANEL_H*envelope,(640.0*expanded).max(288.0*compact),PANEL_H*expanded)};
    let pw=(lw*scale).round().max(1.0) as u32;let ph=(lh*scale).round().max(1.0) as u32;
    // Reserve all view sizes once. Visible children move within this fixed window.
    let (x,y)=placement((mp.x,mp.y),(ms.width,ms.height),pw,(max_w*scale).round() as u32,(max_h*scale).round() as u32,
        settings.free_placement,settings.position_x,settings.position_y);
    let _=visible_h;

    *EXPECTED_BOUNDS.lock().unwrap()=Some(NativeBounds{x,y,w:pw as i32,h:ph as i32});

    let _ = win.set_size(PhysicalSize::new(pw, ph));
    let _ = win.set_position(PhysicalPosition::new(x, y));
    // Moving across displays can rescale the window: re-assert the physical size.
    let _ = win.set_size(PhysicalSize::new(pw, ph));
    let _ = win.set_always_on_top(settings.always_on_top);
}

#[derive(Clone, Copy)]
struct SnapMonitor { x: f64, y: f64, w: f64, h: f64, dpi: f64 }
// Only a tiny correction at release, close to both edges of a corner.
fn snap_position(x: f64, y: f64, w: f64, h: f64, monitors: &[SnapMonitor]) -> (f64,f64) {
    let cx=x+w/2.0;let cy=y+h/2.0;
    let distance=|m:&SnapMonitor|(cx-cx.clamp(m.x,m.x+m.w)).powi(2)+(cy-cy.clamp(m.y,m.y+m.h)).powi(2);
    let Some(m)=monitors.iter().min_by(|a,b|distance(a).total_cmp(&distance(b))) else {return(x,y)};
    if w>m.w||h>m.h{return(x,y)}
    let right=m.x+m.w-w;let bottom=m.y+m.h-h;
    let edge_x=if(x-m.x).abs()<(x-right).abs(){m.x}else{right};
    let edge_y=if(y-m.y).abs()<(y-bottom).abs(){m.y}else{bottom};
    if(x-edge_x).abs()<=3.0*m.dpi&&(y-edge_y).abs()<=3.0*m.dpi {(edge_x,edge_y)}else{(x,y)}
}

/// Windows owns the drag loop. Only its final coordinates are saved.
#[tauri::command]
pub async fn drag_island(app: AppHandle) -> Result<(),String> {
    let shared=app.state::<crate::Shared>();
    let prefs=shared.settings.lock().unwrap().clone();
    if prefs.position_locked { return Err("Position is locked".into()); }
    let win = window(&app).ok_or("Island is unavailable")?;
    shared.gate.moving.store(true,Ordering::Relaxed);
    if let Err(e)=win.start_dragging(){shared.gate.moving.store(false,Ordering::Relaxed);return Err(e.to_string())}
    tauri::async_runtime::spawn_blocking(move || {
        std::thread::sleep(Duration::from_millis(150));
        while left_button_down() {std::thread::sleep(Duration::from_millis(30));}
        let result=capture_placement(&app);
        app.state::<crate::Shared>().gate.moving.store(false,Ordering::Relaxed);
        result
    }).await.map_err(|e|e.to_string())?
}

pub fn capture_placement(app:&AppHandle) -> Result<(),String> {
    let win=window(app).ok_or("Island is unavailable")?;
    let mut pos=win.outer_position().map_err(|e|e.to_string())?;
    let scale=win.scale_factor().unwrap_or(1.0);
    let shared=app.state::<crate::Shared>();
    let rect=*shared.gate.rect.lock().unwrap();
    let mut center_x=pos.x as f64+(rect.x+rect.w/2.0)*scale;
    let center_y=pos.y as f64+(rect.y+rect.h/2.0)*scale;
    let m=app.available_monitors().map_err(|e|e.to_string())?.into_iter().find(|m|monitor_contains(m,center_x,center_y)).or_else(||win.current_monitor().ok().flatten()).ok_or("Monitor is unavailable")?;
    let area=m.work_area();let mp=&area.position;let ms=&area.size;
    let visible_w=rect.w*scale;let visible_h=rect.h*scale;
    let prefs=shared.settings.lock().unwrap().clone();
    if prefs.position_locked {return Ok(());}
    let mut left=center_x-visible_w/2.0;let mut top=pos.y as f64+rect.y*scale;
    if prefs.edge_snap {
        (left,top)=snap_position(left,top,visible_w,visible_h,&[SnapMonitor{x:mp.x as f64,y:mp.y as f64,w:ms.width as f64,h:ms.height as f64,dpi:m.scale_factor()}]);
    }
    // Always keep the visible shape inside the chosen display, independently of assist.
    left=left.clamp(mp.x as f64,(mp.x as f64+ms.width as f64-visible_w).max(mp.x as f64));
    top=top.clamp(mp.y as f64,(mp.y as f64+ms.height as f64-visible_h).max(mp.y as f64));
    pos=PhysicalPosition::new((left-rect.x*scale).round() as i32,(top-rect.y*scale).round() as i32);
    let _=win.set_position(pos);center_x=left+visible_w/2.0;
    let mut settings=shared.settings.lock().unwrap();
    settings.screen=format!("monitor:{}",monitor_id(&m));settings.free_placement=true;
    settings.position_x=((center_x-visible_w/2.0-mp.x as f64)/(ms.width as f64-visible_w).max(1.0)).clamp(0.0,1.0);
    settings.position_y=((top-mp.y as f64)/(ms.height as f64-visible_h).max(1.0)).clamp(0.0,1.0);
    crate::settings::save(&settings).map_err(|e|e.to_string())?;
    let updated=settings.clone();drop(settings);
    apply_geometry(app,&updated,false,None);
    let _=app.emit("settings-changed",updated);
    Ok(())
}

#[cfg(test)]
mod placement_tests {
    use super::*;
    #[test]
    fn placement_handles_second_display_edges_and_bad_saved_values() {
        assert_eq!(placement((-1920,0),(1920,1080),720,720,32,false,0.0,0.8),(-1320,0));
        assert_eq!(placement((-1920,0),(1920,1080),720,720,32,true,1.0,1.0),(-720,1048));
        assert_eq!(placement((-1920,0),(1920,1080),720,720,320,true,1.0,1.0),(-720,760));
        assert_eq!(placement((0,0),(1920,1080),720,720,32,true,f64::NAN,f64::INFINITY),(600,0));
        assert_eq!(placement((0,0),(1920,1080),720,288,32,true,0.0,1.0),(-216,1048));
    }
    #[test]
    fn corner_assist_only_corrects_three_pixels_on_release() {
        let monitors=[SnapMonitor{x:0.0,y:0.0,w:2560.0,h:1440.0,dpi:1.0},SnapMonitor{x:-1080.0,y:-313.0,w:1080.0,h:1920.0,dpi:1.0}];
        assert_eq!(snap_position(2.0,2.0,640.0,160.0,&monitors),(0.0,0.0));
        assert_eq!(snap_position(4.0,2.0,640.0,160.0,&monitors),(4.0,2.0));
        assert_eq!(snap_position(2.0,400.0,640.0,160.0,&monitors),(2.0,400.0));
        assert_eq!(snap_position(1918.0,1278.0,640.0,160.0,&monitors),(1920.0,1280.0));
        assert_eq!(snap_position(-1078.0,-311.0,288.0,32.0,&monitors),(-1080.0,-313.0));
        assert_eq!(snap_position(-290.0,1573.0,288.0,32.0,&monitors),(-288.0,1575.0));
        assert_eq!(snap_position(700.0,400.0,640.0,160.0,&monitors),(700.0,400.0));
        assert_eq!(snap_position(4.0,4.0,640.0,160.0,&[SnapMonitor{x:0.0,y:0.0,w:2560.0,h:1440.0,dpi:2.0}]),(0.0,0.0));
    }
}

fn hwnd_of(win: &WebviewWindow) -> Option<HWND> {
    let raw = win.hwnd().ok()?.0 as isize;
    if raw == 0 {
        return None;
    }
    Some(HWND(raw as *mut _))
}

/// WS_EX_NOACTIVATE keeps clicks from stealing focus; WS_EX_TOOLWINDOW keeps the
/// island out of Alt-Tab.
pub fn make_non_activating(win: &WebviewWindow) {
    let Some(hwnd) = hwnd_of(win) else { return };
    unsafe {
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let want = ex | WS_EX_NOACTIVATE.0 as isize | WS_EX_TOOLWINDOW.0 as isize;
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, want);
    }
}

/// Temporarily allow activation so a text field inside the island can be typed in.
pub fn set_activating(win: &WebviewWindow, activating: bool) {
    let Some(hwnd) = hwnd_of(win) else { return };
    unsafe {
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let want = if activating {
            ex & !(WS_EX_NOACTIVATE.0 as isize)
        } else {
            ex | WS_EX_NOACTIVATE.0 as isize
        };
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, want);
    }
}

/// Position, size and scale of the monitor the island lives on. Any change here
/// means the island has to be placed again.
fn current_screen_key(app: &AppHandle) -> Option<(i32, i32, u32, u32, u64)> {
    let pref = app
        .try_state::<crate::Shared>()
        .map(|s| s.settings.lock().unwrap().screen.clone())
        .unwrap_or_else(|| "primary".into());
    let m = target_monitor(app, &pref)?;
    let p = m.position();
    let size = m.size();
    Some((p.x, p.y, size.width, size.height, m.scale_factor().to_bits()))
}

/// Emits `cursor` (window-logical coordinates) at ~60 Hz while the island is
/// visible. Parked on a condvar the rest of the time.
pub fn spawn_cursor_poll(app: AppHandle, gate: Arc<PollGate>) {
    std::thread::spawn(move || {
        let mut was_down = false;
        // Remembered across wakes so a display change while hidden is noticed the
        // moment the island comes back.
        let mut last_screen: Option<(i32, i32, u32, u32, u64)> = None;
        loop {
            gate.wait_until_active();
            let mut last = (f64::MIN, f64::MIN);
            let mut ticks: u32 = 0;
            while gate.is_active() {
                std::thread::sleep(Duration::from_millis(16));

                // Monitors get plugged in, unplugged, rearranged and rescaled, and
                // an island pinned to coordinates that no longer exist is an island
                // nobody can reach. Checked about twice a second — the cursor poll
                // is already running, so this costs one monitor query.
                ticks = ticks.wrapping_add(1);
                if ticks % 30 == 0 {
                    let now = current_screen_key(&app);
                    if now.is_some() && now != last_screen {
                        let first = last_screen.is_none();
                        last_screen = now;
                        if !first {
                            crate::log::line("display layout changed — repositioning".to_string());
                            let _ = app.emit_to(WINDOW_LABEL, "screen-changed", ());
                        }
                    }
                }

                let Some(win) = window(&app) else { continue };
                let Ok(origin) = win.outer_position() else { continue };
                let scale = win.scale_factor().unwrap_or(1.0);
                let Some((cx, cy)) = cursor_physical() else { continue };
                let x = (cx - origin.x as f64) / scale;
                let y = (cy - origin.y as f64) / scale;
                let size = match win.inner_size() {
                    Ok(s) => (s.width as f64 / scale, s.height as f64 / scale),
                    Err(_) => (PANEL_W, PANEL_H),
                };
                if (x - last.0).abs() < 1.0 && (y - last.1).abs() < 1.0 {
                    continue;
                }
                last = (x, y);

                // Click-through: the window only takes the mouse over the island
                // shape. A small entry margin means the flag is already off by the
                // time a moving cursor reaches a button.
                let r = *gate.rect.lock().unwrap();
                let on_island = r.w > 0.0
                    && x >= r.x - HIT_MARGIN
                    && x <= r.x + r.w + HIT_MARGIN
                    && y >= r.y - HIT_MARGIN
                    && y <= r.y + r.h + HIT_MARGIN;

                // A file being dragged has to be able to find us. WS_EX_TRANSPARENT
                // — what click-through is on Windows — hides the window from
                // WindowFromPoint, so OLE finds no drop target and shows the "no
                // drop" cursor. macOS has no such problem: AppKit delivers drags to
                // registered destinations whatever ignoresMouseEvents says. So while
                // a button is held anywhere over the panel, the whole panel takes
                // the mouse, which also makes the drop zone as forgiving as the Mac's.
                // A press may be the start of a drag: make sure the drop target is
                // ours before the file arrives.
                let down = left_button_down();
                if down && !was_down {
                    let handle = app.clone();
                    let _ = app.run_on_main_thread(move || unblock_webview_drops(&handle));
                }
                was_down = down;

                let dragging = down
                    && x >= 0.0
                    && x <= size.0
                    && y >= 0.0
                    && y <= size.1;

                let accept = on_island || dragging;
                if gate.ignoring.load(Ordering::Relaxed) == accept {
                    gate.ignoring.store(!accept, Ordering::Relaxed);
                    let _ = win.set_ignore_cursor_events(!accept);
                }

                let _ = win.emit("cursor", CursorPayload { x, y });
            }
        }
    });
}

pub fn set_ignore_cursor(app: &AppHandle, ignore: bool) {
    if let Some(win) = window(app) {
        let _ = win.set_ignore_cursor_events(ignore);
    }
}

/// Re-arm click-through after changing the native window envelope.
pub fn refresh_click_through(app: &AppHandle, gate: &PollGate) {
    gate.forget_ignore_state();
    if let Some(win) = window(app) { let _ = win.set_ignore_cursor_events(false); }
}
