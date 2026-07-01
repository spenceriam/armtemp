//! ARMTEMP — native Snapdragon X temperature monitor (Tauri 2 backend).
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

use sensors::{tray::{decide, TrayMode}, ChipProfile, PdhProvider, SensorSnapshot};
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, Submenu},
    tray::TrayIconBuilder,
    Emitter, Manager, WindowEvent,
};
use tauri_plugin_autostart::MacosLauncher;
use tokio::sync::Mutex;

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
    /// Tray-menu checkmarks kept in sync with settings from `update_settings`.
    mode_a_item: CheckMenuItem<tauri::Wry>,
    unit_c_item: CheckMenuItem<tauri::Wry>,
}

/// Once an overheat action fires, require the package temp to drop this many
/// °C below the threshold before it can fire again.
const OVERHEAT_REARM_MARGIN_C: f64 = 5.0;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct AppSettings {
    #[serde(rename = "tempUnit", default = "default_c")]
    temp_unit: String,
    #[serde(rename = "trayMode", default = "default_tray_mode")]
    tray_mode: String,
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
    #[serde(rename = "taskbarOn", default = "default_true")]
    taskbar_on: bool,
    #[serde(rename = "taskbarMode", default = "default_taskbar_mode")]
    taskbar_mode: String,
    #[serde(rename = "taskbarAccent", default = "default_true")]
    taskbar_accent: bool,
    /// UI theme: "system" | "dark" | "light". Mirrored from the frontend so
    /// Rust can drive the NATIVE title-bar chrome (see update_settings).
    #[serde(rename = "theme", default = "default_theme")]
    theme: String,
}
fn default_theme() -> String { "dark".into() }
fn default_c() -> String { "c".into() }
fn default_tray_mode() -> String { "average".into() }
fn default_tray_style() -> String { "rounded".into() }
fn default_overheat() -> f64 { 95.0 }
fn default_overheat_action() -> String { "notify".into() }
fn default_true() -> bool { true }
fn default_polling_ms() -> u64 { 1500 }
fn default_taskbar_mode() -> String { "per-core".into() }
/// Polling interval is user-controlled but must stay sane — never busy-loop
/// the PDH query, never sleep so long the UI feels dead.
const MIN_POLL_MS: u64 = 500;
const MAX_POLL_MS: u64 = 10_000;

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            temp_unit: default_c(),
            tray_mode: default_tray_mode(),
            tray_style: default_tray_style(),
            overheat_on: false,
            overheat_threshold_c: default_overheat(),
            overheat_action: default_overheat_action(),
            close_to_tray: true,
            hide_when_minimized: false,
            polling_interval_ms: default_polling_ms(),
            tray_on: true,
            tray_tooltip_all_cores: true,
            taskbar_on: true,
            taskbar_mode: default_taskbar_mode(),
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum TaskbarMode {
    PerCore,
    Average,
}

impl AppSettings {
    fn tray_mode(&self) -> TrayMode {
        match self.tray_mode.as_str() {
            "all" => TrayMode::All,
            "average" => TrayMode::Average,
            "package" => TrayMode::Package,
            _ => TrayMode::Highest,
        }
    }
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
    fn taskbar_mode(&self) -> TaskbarMode {
        match self.taskbar_mode.as_str() {
            "average" => TaskbarMode::Average,
            _ => TaskbarMode::PerCore,
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

    // Keep the tray's own checkmarks honest — they're driven by settings now,
    // not frozen at their build-time default.
    let _ = state.mode_a_item.set_checked(parsed.tray_mode == "average");
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

/// Redraw the tray icon (and, on Windows, the taskbar overlay badge) from the
/// real snapshot + user settings.
const TRAY_ICON_PX: u32 = 48;
const TASKBAR_OVERLAY_PX: u32 = 32;
/// Windows accent blue — matches `ACCENT` in `src/app/theme.ts`.
const ACCENT_RGB: (u8, u8, u8) = (0, 120, 212);

fn unit_convert(c_val: f64, is_f: bool) -> i32 {
    if is_f { (c_val * 9.0 / 5.0 + 32.0).round() as i32 } else { c_val.round() as i32 }
}

fn update_tray(app: &tauri::AppHandle, snap: &SensorSnapshot, settings: &AppSettings) {
    let d = decide(snap, settings.tray_mode());
    let is_f = settings.unit_is_f();
    let unit = if is_f { "F" } else { "C" };
    let shown = d.primary_value.map(|c| unit_convert(c as f64, is_f));

    let png = sensors::tray::number_icon_png(shown, d.color_rgb, settings.tray_style(), TRAY_ICON_PX);
    if let Some(tray) = app.tray_by_id("main-tray") {
        let img = tauri::image::Image::new(&png, TRAY_ICON_PX, TRAY_ICON_PX);
        let _ = tray.set_icon(Some(img));

        let header = match shown {
            Some(t) => format!("ARMTEMP — {t}°{unit}"),
            None => "ARMTEMP — (no sensor)".to_string(),
        };
        let tooltip = if settings.tray_tooltip_all_cores {
            let mut lines = vec![header];
            for c in &snap.cores {
                let core_val = c.temp_c.map(|v| format!("{}°{unit}", unit_convert(v, is_f)));
                lines.push(format!("Core #{}: {}", c.index, core_val.as_deref().unwrap_or("—")));
            }
            lines.join("\n")
        } else {
            header
        };
        let _ = tray.set_tooltip(Some(tooltip));
    }

    // Windows taskbar overlay badge — same digit renderer, smaller + no plate
    // (Plain-style) so it reads clearly at the small overlay size.
    #[cfg(target_os = "windows")]
    if let Some(w) = app.get_webview_window("main") {
        if settings.taskbar_on {
            let (val_c, temp_color) = match settings.taskbar_mode() {
                TaskbarMode::Average => (snap.average_c, snap.average_c.map(|v| sensors::tray::temp_color(v, snap.tjmax_c))),
                TaskbarMode::PerCore => (snap.package_c, snap.package_c.map(|v| sensors::tray::temp_color(v, snap.tjmax_c))),
            };
            let color = if settings.taskbar_accent { ACCENT_RGB } else { temp_color.unwrap_or((140, 140, 140)) };
            let val = val_c.map(|v| unit_convert(v, is_f));
            let png = sensors::tray::number_icon_png(val, color, sensors::tray::TrayStyle::Badge, TASKBAR_OVERLAY_PX);
            let img = tauri::image::Image::new(&png, TASKBAR_OVERLAY_PX, TASKBAR_OVERLAY_PX);
            let _ = w.set_overlay_icon(Some(img));
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
                .title("ARMTEMP — overheat")
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
            let open = MenuItem::with_id(app, "open", "Open ARMTEMP", true, None::<&str>)?;
            let settings_item = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
            let mini = MenuItem::with_id(app, "mini", "Mini-mode", true, None::<&str>)?;

            // Tray-mode submenu (Average is the default per user spec).
            let mode_h = MenuItem::with_id(app, "mode_h", "Highest core", true, None::<&str>)?;
            let mode_a = CheckMenuItem::with_id(app, "mode_a", "Average", true, true, None::<&str>)?;
            let mode_all = MenuItem::with_id(app, "mode_all", "All cores", true, None::<&str>)?;
            let mode_pkg = MenuItem::with_id(app, "mode_pkg", "Package", true, None::<&str>)?;
            let mode_menu = Submenu::with_items(
                app,
                "Tray mode",
                true,
                &[&mode_h, &mode_a, &mode_all, &mode_pkg],
            )?;

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
                    &mode_menu,
                    &unit_menu,
                    &sep2,
                    &refresh,
                    &exit,
                ],
            )?;
            let _tray = TrayIconBuilder::with_id("main-tray")
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "settings" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.emit("open-settings", ());
                            let _ = w.set_focus();
                        }
                    }
                    "mini" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.emit("toggle-mini", ());
                        }
                    }
                    // Tray-mode quick switches — tell the frontend to persist + apply.
                    "mode_h" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.emit("set-tray-mode", "highest");
                        }
                    }
                    "mode_a" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.emit("set-tray-mode", "average");
                        }
                    }
                    "mode_all" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.emit("set-tray-mode", "all");
                        }
                    }
                    "mode_pkg" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.emit("set-tray-mode", "package");
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
                "[armtemp] detected: {}",
                profile.as_ref().map(|p| p.name).unwrap_or("unknown")
            );
            let state = Arc::new(AppState {
                provider,
                latest: Mutex::new(None),
                profile: Mutex::new(profile),
                tick: Mutex::new(0),
                settings: Mutex::new(AppSettings::default()),
                overheat_armed: Mutex::new(true),
                mode_a_item: mode_a.clone(),
                unit_c_item: unit_c.clone(),
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
            exit_app
        ])
        .run(tauri::generate_context!())
        .expect("error while running ARMTEMP");
}
