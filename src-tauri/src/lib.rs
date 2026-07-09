//! ARMtemp — native Snapdragon X temperature monitor (Tauri 2 backend).
//!
//! Wires the native PDH-backed sensor provider to a polling loop that emits a
//! `sensor-update` event each tick, manages the live tray icon, and exposes
//! IPC commands to the React frontend.
//!
//! The provider reads Windows PDH performance counters directly (see
//! `sensors/pdh.rs`) — no per-tick subprocess, no COM/WMI. All data is
//! strictly real; missing sources -> honest "—".

mod sensors;

use std::sync::Arc;
use std::time::Duration;

use sensors::{tray::decide, ChipProfile, PdhProvider, SensorSnapshot};
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, Submenu},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, WindowEvent,
};
use tauri_plugin_autostart::MacosLauncher;
use tokio::sync::Mutex;

/// Restores the main window from a minimized and/or hidden state and gives
/// it focus. Shared by the tray icon's left-click handler and the "Open
/// ARMtemp"/"Settings" menu items — `show()` alone does not un-minimize a
/// window that was merely minimized (as opposed to hidden via close-to-tray).
fn restore_main_window<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

/// Guards against launching two instances of the SAME version at once
/// (double-clicking the exe twice, or an autostart relaunch racing a manual
/// one) while still letting a DIFFERENT version run alongside — e.g. testing
/// a dev build next to an already-running production install. Keyed on
/// `CARGO_PKG_VERSION` via a named Win32 mutex rather than the bundle
/// identifier, which `tauri-plugin-single-instance` locks on (making it
/// version-blind and unable to satisfy this). Fails open: if the mutex API
/// itself errors, the launch proceeds rather than blocking the app over a
/// Win32 quirk.
fn acquire_single_instance_lock() -> bool {
    use windows::core::HSTRING;
    use windows::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS, HWND};
    use windows::Win32::System::Threading::CreateMutexW;
    use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONINFORMATION, MB_OK};

    let version = env!("CARGO_PKG_VERSION");
    let name = HSTRING::from(format!("armtemp-single-instance-{version}"));
    let handle = match unsafe { CreateMutexW(None, true, &name) } {
        Ok(h) => h,
        Err(_) => return true, // fail open
    };
    let already_running = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
    if !already_running {
        // `HANDLE` is a plain Copy newtype with no `Drop` impl — it is never
        // auto-closed, so simply not calling `CloseHandle` holds the mutex
        // for the process lifetime (Windows reclaims it on process exit).
        let _ = handle;
        return true;
    }

    // An autostart relaunch (`--minimized`) racing the already-running
    // instance should bow out quietly, not pop up a dialog.
    if std::env::args().any(|a| a == "--minimized") {
        return false;
    }
    unsafe {
        let _ = MessageBoxW(
            HWND::default(),
            &HSTRING::from(format!("ARMtemp {version} is already running.")),
            &HSTRING::from("ARMtemp"),
            MB_OK | MB_ICONINFORMATION,
        );
    }
    false
}

/// Plain-data app state — fully Send+Sync.
struct AppState {
    provider: PdhProvider,
    latest: Mutex<Option<SensorSnapshot>>,
    profile: Mutex<Option<ChipProfile>>,
    tick: Mutex<u64>,
    settings: Mutex<AppSettings>,
    /// Overheat edge-trigger: true = ready to fire on the next threshold
    /// crossing; set false right after firing, re-armed once the package
    /// temp drops `OVERHEAT_REARM_MARGIN_C` below the threshold. Prevents
    /// re-notifying (or re-sleeping/-shutting-down) every poll tick while hot.
    overheat_armed: Mutex<bool>,
    /// Tray-menu checkmark kept in sync with settings from `update_settings`.
    unit_c_item: CheckMenuItem<tauri::Wry>,
    /// Tray-menu checkmark kept in sync with mini-mode state, wherever it's
    /// toggled from (tray menu or the in-window Options menu) — see
    /// `set_mini_state`.
    mini_item: CheckMenuItem<tauri::Wry>,
}

/// Once an overheat action fires, require the package temp to drop this many
/// °C below the threshold before it can fire again.
const OVERHEAT_REARM_MARGIN_C: f64 = 5.0;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct AppSettings {
    #[serde(rename = "tempUnit", default = "default_c")]
    temp_unit: String,
    #[serde(rename = "trayStyle", default = "default_tray_style")]
    tray_style: String,
    #[serde(rename = "overheatOn", default)]
    overheat_on: bool,
    #[serde(rename = "overheatThreshold", default = "default_overheat")]
    overheat_threshold_c: f64,
    #[serde(rename = "overheatAction", default = "default_overheat_action")]
    overheat_action: String,
    #[serde(rename = "closeToTray", default = "default_true")]
    close_to_tray: bool,
    #[serde(rename = "hideWhenMinimized", default)]
    hide_when_minimized: bool,
    #[serde(rename = "pollingIntervalMs", default = "default_polling_ms")]
    polling_interval_ms: u64,
    #[serde(rename = "trayOn", default = "default_true")]
    tray_on: bool,
    #[serde(rename = "trayTooltipAllCores", default = "default_true")]
    tray_tooltip_all_cores: bool,
    #[serde(rename = "trayBoldFont", default = "default_true")]
    tray_bold_font: bool,
    #[serde(rename = "trayDegreeSymbol", default)]
    tray_degree_symbol: bool,
    #[serde(rename = "taskbarOn", default = "default_true")]
    taskbar_on: bool,
    #[serde(rename = "taskbarAccent", default = "default_true")]
    taskbar_accent: bool,
    /// UI theme: "system" | "dark" | "light". Mirrored from the frontend so
    /// Rust can drive the NATIVE title-bar chrome (see update_settings).
    #[serde(rename = "theme", default = "default_theme")]
    theme: String,
}
fn default_theme() -> String { "dark".into() }
fn default_c() -> String { "c".into() }
fn default_tray_style() -> String { "plain".into() }
fn default_overheat() -> f64 { 95.0 }
fn default_overheat_action() -> String { "notify".into() }
fn default_true() -> bool { true }
fn default_polling_ms() -> u64 { 1500 }
/// Polling interval is user-controlled but must stay sane — never busy-loop
/// the PDH query, never sleep so long the UI feels dead.
const MIN_POLL_MS: u64 = 500;
const MAX_POLL_MS: u64 = 10_000;

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            temp_unit: default_c(),
            tray_style: default_tray_style(),
            overheat_on: false,
            overheat_threshold_c: default_overheat(),
            overheat_action: default_overheat_action(),
            close_to_tray: true,
            hide_when_minimized: false,
            polling_interval_ms: default_polling_ms(),
            tray_on: true,
            tray_tooltip_all_cores: true,
            tray_bold_font: true,
            tray_degree_symbol: false,
            taskbar_on: true,
            taskbar_accent: true,
            theme: default_theme(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum OverheatAction {
    Notify,
    Sleep,
    Shutdown,
}

impl AppSettings {
    fn tray_style(&self) -> sensors::tray::TrayStyle {
        match self.tray_style.as_str() {
            "badge" => sensors::tray::TrayStyle::Badge,
            "plain" => sensors::tray::TrayStyle::Plain,
            _ => sensors::tray::TrayStyle::Rounded,
        }
    }
    fn unit_is_f(&self) -> bool {
        self.temp_unit.eq_ignore_ascii_case("f")
    }
    fn overheat_action(&self) -> OverheatAction {
        match self.overheat_action.as_str() {
            "sleep" => OverheatAction::Sleep,
            "shutdown" => OverheatAction::Shutdown,
            _ => OverheatAction::Notify,
        }
    }
    fn polling_interval(&self) -> Duration {
        Duration::from_millis(self.polling_interval_ms.clamp(MIN_POLL_MS, MAX_POLL_MS))
    }
}

// ---------- IPC commands ----------

#[tauri::command]
fn get_snapshot(state: tauri::State<'_, Arc<AppState>>) -> Option<SensorSnapshot> {
    state.latest.blocking_lock().clone()
}

#[tauri::command]
fn get_profile(state: tauri::State<'_, Arc<AppState>>) -> serde_json::Value {
    let p = state.profile.blocking_lock().clone();
    match p {
        Some(p) => serde_json::json!({
            "name": p.name, "model": p.model, "tjmax_c": p.tjmax_c,
            "base_ghz": p.base_ghz, "boost_ghz": p.boost_ghz,
            "tdp_w": p.tdp_w, "cores": p.total_cores(),
        }),
        None => serde_json::json!({}),
    }
}

#[tauri::command]
fn exit_app(app: tauri::AppHandle) {
    app.exit(0);
}

/// Plain-text dump of every raw CPU identity signal plus how the chip was
/// matched — see issue #2, where the only way to diagnose a misdetected
/// machine we don't own was asking the reporter for more screenshots.
#[tauri::command]
fn get_detection_report(state: tauri::State<'_, Arc<AppState>>) -> String {
    state
        .provider
        .detection_report()
        .unwrap_or_else(|e| format!("detection report unavailable: {e}"))
}

/// Keeps the tray menu's "Mini-mode" checkmark honest. Mini mode is ephemeral
/// UI state (not persisted settings), so the frontend reports it here on
/// every toggle rather than routing it through `update_settings`.
#[tauri::command]
fn set_mini_state(mini: bool, state: tauri::State<'_, Arc<AppState>>) {
    let _ = state.mini_item.set_checked(mini);
}

#[tauri::command]
fn refresh_now(state: tauri::State<'_, Arc<AppState>>, app: tauri::AppHandle) {
    if let Ok(mut s) = state.provider.snapshot() {
        let mut tick = state.tick.blocking_lock();
        *tick += 1;
        s.tick = *tick;
        drop(tick);
        let settings = state.settings.blocking_lock().clone();
        update_tray(&app, &s, &settings);
        *state.latest.blocking_lock() = Some(s.clone());
        let _ = app.emit("sensor-update", s);
    }
}

#[tauri::command]
fn update_settings(
    settings: serde_json::Value,
    state: tauri::State<'_, Arc<AppState>>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let parsed: AppSettings = serde_json::from_value(settings).unwrap_or_default();
    *state.settings.blocking_lock() = parsed.clone();

    // Keep the tray's own checkmark honest — it's driven by settings now,
    // not frozen at its build-time default.
    let _ = state.unit_c_item.set_checked(!parsed.unit_is_f());

    if let Some(tray) = app.tray_by_id("main-tray") {
        let _ = tray.set_visible(parsed.tray_on);
    }

    // Native title-bar chrome follows the app theme. Owned by Rust (not JS)
    // so failures are LOGGED instead of silently swallowed.
    if let Some(w) = app.get_webview_window("main") {
        let theme = match parsed.theme.as_str() {
            "light" => Some(tauri::Theme::Light),
            "dark" => Some(tauri::Theme::Dark),
            _ => None, // "system" — follow the OS
        };
        if let Err(e) = w.set_theme(theme) {
            eprintln!("[armtemp] set_theme failed: {e}");
        }
    }

    if let Some(snap) = state.latest.blocking_lock().clone() {
        update_tray(&app, &snap, &parsed);
    }
    Ok(())
}

/// Windows accent blue — matches `ACCENT` in `src/app/theme.ts`.
const ACCENT_RGB: (u8, u8, u8) = (0, 120, 212);

fn unit_convert(c_val: f64, is_f: bool) -> i32 {
    if is_f { (c_val * 9.0 / 5.0 + 32.0).round() as i32 } else { c_val.round() as i32 }
}

/// The Windows SYSTEM theme (taskbar/tray surfaces — distinct from the app
/// theme setting): true = light taskbar. Read fresh on every tray redraw so
/// a system theme switch is picked up on the next poll tick.
fn system_uses_light_theme() -> bool {
    use windows::core::HSTRING;
    use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};
    unsafe {
        let subkey = HSTRING::from(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize");
        let value = HSTRING::from("SystemUsesLightTheme");
        let mut data: u32 = 0;
        let mut size: u32 = std::mem::size_of::<u32>() as u32;
        let st = RegGetValueW(
            HKEY_CURRENT_USER,
            &subkey,
            &value,
            RRF_RT_REG_DWORD,
            None,
            Some(&mut data as *mut u32 as *mut core::ffi::c_void),
            Some(&mut size),
        );
        st.0 == 0 && data == 1
    }
}

/// Theme-adaptive monochrome for the default (Plain-style) tray icon: white
/// text on a dark taskbar, near-black on a light one.
fn mono_color() -> (u8, u8, u8) {
    if system_uses_light_theme() { (25, 25, 25) } else { (255, 255, 255) }
}

/// Darken a temperature color ~28% on a light taskbar so yellow/green stay
/// legible against the light notification-area background; left unchanged on
/// a dark taskbar where the existing scale already reads well.
fn adapt_for_taskbar(rgb: (u8, u8, u8)) -> (u8, u8, u8) {
    if system_uses_light_theme() {
        let (r, g, b) = rgb;
        ((r as f32 * 0.72) as u8, (g as f32 * 0.72) as u8, (b as f32 * 0.72) as u8)
    } else {
        rgb
    }
}

/// Build a tray/overlay image for `value` using the native-font renderer.
/// `None` draws an honest dash — never a fabricated number. `degree` draws a
/// small superscript `°` next to a real reading only — the no-reading dash
/// never gets one. The degree mark is rendered in its own reserved column
/// (see `render_number_rgba`) rather than appended to the digit string, so
/// turning it on doesn't force the digits to shrink to fit a 3rd glyph.
fn number_image(
    value: Option<i32>,
    fg: (u8, u8, u8),
    plate: Option<((u8, u8, u8), sensors::tray::TrayStyle)>,
    bold: bool,
    degree: bool,
) -> tauri::image::Image<'static> {
    let size = sensors::tray_render::tray_icon_size();
    let text = match value {
        Some(v) => v.to_string(),
        None => "-".to_string(),
    };
    let show_degree = degree && value.is_some();
    let rgba = sensors::tray_render::render_number_rgba(&text, fg, plate, size, bold, show_degree);
    tauri::image::Image::new_owned(rgba, size, size)
}

/// Average per-core LOAD as a single tooltip line (load is genuinely
/// per-core; there is no per-core temperature on this firmware — see
/// `decide()`). A one-line-per-core list was tried first but overflows the
/// OS tooltip's length limit on higher-core-count chips (truncates around
/// Core #3 on a 12-core X1E); one averaged line always fits.
fn avg_load_line(snap: &SensorSnapshot) -> Option<String> {
    let loads: Vec<f64> = snap.cores.iter().filter_map(|c| c.load).collect();
    if loads.is_empty() {
        return None;
    }
    let avg = loads.iter().sum::<f64>() / loads.len() as f64;
    Some(format!("Avg load: {}%", avg.round() as i32))
}

fn set_tray_icon(app: &tauri::AppHandle, id: &str, img: tauri::image::Image<'static>, tooltip: &str) {
    if let Some(tray) = app.tray_by_id(id) {
        if let Err(e) = tray.set_icon(Some(img)) {
            eprintln!("[armtemp] tray '{id}' set_icon failed: {e}");
        }
        let _ = tray.set_tooltip(Some(tooltip));
    }
}

/// Redraw the tray icon (and, on Windows, the taskbar overlay badge) from the
/// real snapshot + user settings. There is exactly one tray icon: it always
/// shows the single honest CPU temperature — there is no per-core sensor on
/// this firmware to drive a per-core tray mode.
fn update_tray(app: &tauri::AppHandle, snap: &SensorSnapshot, settings: &AppSettings) {
    let is_f = settings.unit_is_f();
    let unit = if is_f { "F" } else { "C" };
    let style = settings.tray_style();

    let d = decide(snap);
    let shown = d.primary_value.map(|c| unit_convert(c as f64, is_f));
    // Default (Plain) icon: bare digits, no plate, colored to complement
    // the taskbar theme. The opt-in Rounded/Badge styles keep a
    // temperature-colored plate under white digits.
    let (fg, plate) = match style {
        sensors::tray::TrayStyle::Plain => (mono_color(), None),
        _ => ((255, 255, 255), Some((adapt_for_taskbar(d.color_rgb), style))),
    };
    let img = number_image(shown, fg, plate, settings.tray_bold_font, settings.tray_degree_symbol);

    let header = match shown {
        Some(t) => format!("ARMtemp — {t}°{unit}"),
        None => "ARMtemp — (no sensor)".to_string(),
    };
    let tooltip = if settings.tray_tooltip_all_cores {
        match avg_load_line(snap) {
            Some(load_line) => format!("{header}\n{load_line}"),
            None => header,
        }
    } else {
        header
    };
    set_tray_icon(app, "main-tray", img, &tooltip);

    update_taskbar_overlay(app, snap, settings, is_f);
}

/// Windows taskbar overlay badge — same native-font renderer as the tray,
/// always drawn as a colored Badge plate so it reads at the tiny overlay size.
fn update_taskbar_overlay(app: &tauri::AppHandle, snap: &SensorSnapshot, settings: &AppSettings, is_f: bool) {
    #[cfg(target_os = "windows")]
    if let Some(w) = app.get_webview_window("main") {
        if settings.taskbar_on {
            let val_c = snap.package_c;
            let temp_color = snap.package_c.map(|v| sensors::tray::temp_color(v, snap.tjmax_c));
            let color = if settings.taskbar_accent { ACCENT_RGB } else { temp_color.unwrap_or((140, 140, 140)) };
            let val = val_c.map(|v| unit_convert(v, is_f));
            // Overlay badge never shows the degree symbol — there's no room
            // for it at that size — but bold still follows the setting.
            let img = number_image(val, (255, 255, 255), Some((color, sensors::tray::TrayStyle::Badge)), settings.tray_bold_font, false);
            if let Err(e) = w.set_overlay_icon(Some(img)) {
                eprintln!("[armtemp] taskbar overlay failed: {e}");
            }
        } else {
            let _ = w.set_overlay_icon(None);
        }
    }
}

/// Fire the user's chosen overheat action. Sleep/Shutdown are real OS actions
/// gated entirely behind the user's own Settings → Overheat protection
/// toggle + threshold — never triggered without that explicit opt-in.
fn fire_overheat_action(app: &tauri::AppHandle, action: OverheatAction, package_c: f64, threshold_c: f64) {
    match action {
        OverheatAction::Notify => {
            use tauri_plugin_notification::NotificationExt;
            let _ = app
                .notification()
                .builder()
                .title("ARMtemp — overheat")
                .body(format!("Package reached {package_c:.0}°C (threshold {threshold_c:.0}°C)."))
                .show();
        }
        OverheatAction::Sleep => {
            use windows::Win32::Foundation::BOOLEAN;
            use windows::Win32::System::Power::SetSuspendState;
            unsafe {
                let _ = SetSuspendState(BOOLEAN(0), BOOLEAN(0), BOOLEAN(0));
            }
        }
        OverheatAction::Shutdown => {
            // A few seconds' grace (abortable with `shutdown /a`) rather than
            // an instant power-off.
            let _ = std::process::Command::new("shutdown").args(["/s", "/t", "5"]).spawn();
        }
    }
}

/// Background poll loop: read real sensors on the user's chosen cadence, emit
/// an event, refresh the tray/taskbar, and honor overheat protection.
fn start_poll_loop(app: tauri::AppHandle, state: Arc<AppState>) {
    tauri::async_runtime::spawn(async move {
        loop {
            let settings = state.settings.lock().await.clone();

            // Run the (sync, blocking) PDH query on a thread so it never
            // stalls the async runtime.
            let snap = tokio::task::spawn_blocking({
                let provider = state.provider.clone();
                move || provider.snapshot().ok()
            })
            .await
            .ok()
            .flatten();

            if let Some(mut snap) = snap {
                let mut tick = state.tick.lock().await;
                *tick += 1;
                snap.tick = *tick;
                drop(tick);

                // Overheat protection: edge-triggered with hysteresis so a
                // sustained overheat fires once, not every tick.
                if settings.overheat_on {
                    if let Some(p) = snap.package_c {
                        let mut armed = state.overheat_armed.lock().await;
                        if p >= settings.overheat_threshold_c {
                            if *armed {
                                *armed = false;
                                fire_overheat_action(&app, settings.overheat_action(), p, settings.overheat_threshold_c);
                            }
                        } else if p <= settings.overheat_threshold_c - OVERHEAT_REARM_MARGIN_C {
                            *armed = true;
                        }
                    }
                }

                update_tray(&app, &snap, &settings);
                *state.latest.lock().await = Some(snap.clone());
                let _ = app.emit("sensor-update", snap);
            }
            tokio::time::sleep(settings.polling_interval()).await;
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    if !acquire_single_instance_lock() {
        return;
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .setup(|app| {
            // Tray context menu.
            let open = MenuItem::with_id(app, "open", "Open ARMtemp", true, None::<&str>)?;
            let settings_item = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
            let mini = CheckMenuItem::with_id(app, "mini", "Mini-mode", true, false, None::<&str>)?;

            // Unit submenu (°C / °F quick toggle — also in Settings → General).
            let unit_c = CheckMenuItem::with_id(app, "unit_c", "°C", true, true, None::<&str>)?;
            let unit_f = MenuItem::with_id(app, "unit_f", "°F", true, None::<&str>)?;
            let unit_menu = Submenu::with_items(app, "Unit", true, &[&unit_c, &unit_f])?;

            let refresh = MenuItem::with_id(app, "refresh", "Refresh sensors", true, None::<&str>)?;
            let exit = MenuItem::with_id(app, "exit", "Exit", true, None::<&str>)?;
            let sep1 = tauri::menu::PredefinedMenuItem::separator(app)?;
            let sep2 = tauri::menu::PredefinedMenuItem::separator(app)?;
            let menu = Menu::with_items(
                app,
                &[
                    &open,
                    &mini,
                    &settings_item,
                    &sep1,
                    &unit_menu,
                    &sep2,
                    &refresh,
                    &exit,
                ],
            )?;
            // Seed with a dash (no reading yet) in theme-adaptive monochrome
            // so the tray's identity is "the number" from the very first
            // frame — never the generic app logo.
            let _tray = TrayIconBuilder::with_id("main-tray")
                .icon(number_image(None, mono_color(), None, true, false))
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                        restore_main_window(tray.app_handle());
                    }
                })
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => restore_main_window(app),
                    "settings" => {
                        restore_main_window(app);
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.emit("open-settings", ());
                        }
                    }
                    "mini" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.emit("toggle-mini", ());
                        }
                    }
                    // Unit quick toggle.
                    "unit_c" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.emit("set-unit", "C");
                        }
                    }
                    "unit_f" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.emit("set-unit", "F");
                        }
                    }
                    "refresh" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.emit("refresh-sensors", ());
                        }
                    }
                    "exit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;

            // Initialize the native PDH-backed provider (detection runs now).
            let provider = PdhProvider::new();
            let profile = provider.profile().ok().flatten();
            eprintln!(
                "[armtemp] detected: {} {}",
                profile.as_ref().map(|p| p.name).unwrap_or("unknown"),
                profile.as_ref().map(|p| p.model).unwrap_or("")
            );
            let state = Arc::new(AppState {
                provider,
                latest: Mutex::new(None),
                profile: Mutex::new(profile),
                tick: Mutex::new(0),
                settings: Mutex::new(AppSettings::default()),
                overheat_armed: Mutex::new(true),
                unit_c_item: unit_c.clone(),
                mini_item: mini.clone(),
            });
            app.manage(state.clone());

            // Honor `--minimized` (passed by the autostart plugin — see the
            // `tauri_plugin_autostart::init` call below) by hiding the main
            // window immediately instead of showing it and then hiding it.
            if std::env::args().any(|a| a == "--minimized") {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.hide();
                }
            }

            // Dialog deep-links: `--settings` / `--overheat` / `--about` open
            // the corresponding dialog once the frontend has mounted (small
            // delay so the event listeners exist).
            let dialog_event = std::env::args().find_map(|a| match a.as_str() {
                "--settings" => Some("open-settings"),
                "--overheat" => Some("open-overheat"),
                "--about" => Some("open-about"),
                _ => None,
            });
            if let Some(event) = dialog_event {
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(Duration::from_millis(1500)).await;
                    if let Some(w) = handle.get_webview_window("main") {
                        let _ = w.emit(event, ());
                    }
                });
            }

            start_poll_loop(app.handle().clone(), state);
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() != "main" {
                return;
            }
            match event {
                // Close-to-tray: intercept close on the main window and hide
                // instead, unless the user turned that setting off (in which
                // case letting the close proceed exits the app normally).
                WindowEvent::CloseRequested { api, .. } => {
                    let close_to_tray = window
                        .app_handle()
                        .try_state::<Arc<AppState>>()
                        .map(|s| s.settings.blocking_lock().close_to_tray)
                        .unwrap_or(true);
                    if close_to_tray {
                        api.prevent_close();
                        let _ = window.hide();
                    }
                }
                // Hide-when-minimized: only acted on when the user opted in.
                WindowEvent::Resized(_) => {
                    let hide_when_minimized = window
                        .app_handle()
                        .try_state::<Arc<AppState>>()
                        .map(|s| s.settings.blocking_lock().hide_when_minimized)
                        .unwrap_or(false);
                    if hide_when_minimized && window.is_minimized().unwrap_or(false) {
                        let _ = window.hide();
                    }
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_snapshot,
            get_profile,
            refresh_now,
            update_settings,
            exit_app,
            set_mini_state,
            get_detection_report
        ])
        .run(tauri::generate_context!())
        .expect("error while running ARMtemp");
}
