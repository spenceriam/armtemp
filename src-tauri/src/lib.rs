//! ARMTEMP — native Snapdragon X temperature monitor (Tauri 2 backend).
//!
//! Wires the real PowerShell-backed sensor provider to a polling loop that emits
//! a `sensor-update` event each tick, manages the live tray icon, and exposes
//! IPC commands to the React frontend.
//!
//! The provider shells out to `Get-CimInstance` (the proven Phase 0 path),
//! which sidesteps the COM/IWbemServices interaction that fails under a Tauri
//! process. All data is strictly real; missing sources -> honest "—".

mod sensors;

use std::sync::Arc;
use std::time::Duration;

use sensors::{tray::{decide, TrayMode}, ChipProfile, PowerShellProvider, SensorSnapshot};
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, Submenu},
    tray::TrayIconBuilder,
    Emitter, Manager, WindowEvent,
};
use tauri_plugin_autostart::MacosLauncher;
use tokio::sync::Mutex;

/// Plain-data app state — fully Send+Sync.
struct AppState {
    provider: PowerShellProvider,
    latest: Mutex<Option<SensorSnapshot>>,
    profile: Mutex<Option<ChipProfile>>,
    tick: Mutex<u64>,
    settings: Mutex<AppSettings>,
}

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
}
fn default_c() -> String { "c".into() }
fn default_tray_mode() -> String { "average".into() }
fn default_tray_style() -> String { "rounded".into() }
fn default_overheat() -> f64 { 95.0 }

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            temp_unit: default_c(),
            tray_mode: default_tray_mode(),
            tray_style: default_tray_style(),
            overheat_on: false,
            overheat_threshold_c: default_overheat(),
        }
    }
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
    if let Some(snap) = state.latest.blocking_lock().clone() {
        update_tray(&app, &snap, &parsed);
    }
    Ok(())
}

/// Redraw the tray icon from the real snapshot + user settings.
fn update_tray(app: &tauri::AppHandle, snap: &SensorSnapshot, settings: &AppSettings) {
    let d = decide(snap, settings.tray_mode());
    let png = sensors::tray::blank_icon_png(d.color_rgb);
    if let Some(tray) = app.tray_by_id("main-tray") {
        let img = tauri::image::Image::new(&png, 32, 32);
        let _ = tray.set_icon(Some(img));
        let tooltip = match d.primary_value {
            Some(t) => format!("ARMTEMP — {}°", t),
            None => "ARMTEMP — (no sensor)".to_string(),
        };
        let _ = tray.set_tooltip(Some(tooltip));
    }
}

/// Background poll loop: every 2s read real sensors, emit an event, refresh the
/// tray, and honor overheat protection. PowerShell per-call, so a slightly
/// longer interval keeps the system calm.
fn start_poll_loop(app: tauri::AppHandle, state: Arc<AppState>) {
    tauri::async_runtime::spawn(async move {
        loop {
            // Run the (sync, blocking) PowerShell probe on a thread so it never
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

                let settings = state.settings.lock().await.clone();

                // Overheat protection (real notification on real threshold breach).
                if settings.overheat_on {
                    if let Some(p) = snap.package_c {
                        if p >= settings.overheat_threshold_c {
                            use tauri_plugin_notification::NotificationExt;
                            let _ = app
                                .notification()
                                .builder()
                                .title("ARMTEMP — overheat")
                                .body(format!(
                                    "Package reached {:.0}°C (threshold {:.0}°C).",
                                    p, settings.overheat_threshold_c
                                ))
                                .show();
                        }
                    }
                }

                update_tray(&app, &snap, &settings);
                *state.latest.lock().await = Some(snap.clone());
                let _ = app.emit("sensor-update", snap);
            }
            tokio::time::sleep(Duration::from_millis(2000)).await;
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

            // Initialize the real PowerShell-backed provider (detection runs now).
            let provider = PowerShellProvider::new();
            let profile = provider.profile().ok().flatten();
            eprintln!(
                "[armtemp] detected: {}",
                profile.as_ref().map(|p| p.name.as_ref()).unwrap_or("unknown")
            );
            let state = Arc::new(AppState {
                provider,
                latest: Mutex::new(None),
                profile: Mutex::new(profile),
                tick: Mutex::new(0),
                settings: Mutex::new(AppSettings::default()),
            });
            app.manage(state.clone());

            start_poll_loop(app.handle().clone(), state);
            Ok(())
        })
        .on_window_event(|window, event| {
            // Close-to-tray: intercept close on the main window and hide instead.
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_snapshot,
            get_profile,
            refresh_now,
            update_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running ARMTEMP");
}
