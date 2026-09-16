pub mod pb {
    include!(concat!(env!("OUT_DIR"), "/mod.rs"));
}

mod collector_config;
mod common;
mod dish;
mod history;
mod router;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Manager, WindowEvent};

// macOS suspends (or heavily throttles) timers in apps that are hidden/not
// frontmost ("App Nap"), which stalls the background history collector for
// long stretches while the window is closed to the tray. Opting out here
// disables just App Nap for this process - it does NOT prevent the Mac
// itself from sleeping, so real system sleep still pauses everything as
// normal.
#[cfg(target_os = "macos")]
fn disable_app_nap() {
    use objc2_foundation::{NSActivityOptions, NSProcessInfo, NSString};

    let process_info = NSProcessInfo::processInfo();
    let reason = NSString::from_str("Continuous background dish/router history collection");
    let activity = process_info.beginActivityWithOptions_reason(
        NSActivityOptions::UserInitiatedAllowingIdleSystemSleep,
        &reason,
    );
    // Leak the activity token so it lives for the whole process - ending it
    // would let App Nap resume throttling the collector.
    std::mem::forget(activity);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(target_os = "macos")]
    disable_app_nap();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let show_item = MenuItem::with_id(app, "show", "Show Star Manager", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => app.exit(0),
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    _ => {}
                })
                .build(app)?;

            tauri::async_runtime::spawn(history::run_collector(app.handle().clone()));

            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the window hides it instead of quitting the app, so the
            // background history collector keeps running until the user
            // explicitly quits from the tray menu.
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            dish::default_dish_address,
            dish::fetch_starlink_snapshot,
            dish::fetch_starlink_section,
            dish::fetch_dish_obstruction_map,
            dish::fetch_dish_history,
            dish::apply_dish_config,
            dish::trigger_dish_action,
            router::default_router_address,
            router::fetch_router_snapshot,
            router::fetch_router_section,
            router::fetch_router_history,
            router::apply_wifi_config,
            router::trigger_router_action,
            collector_config::get_collector_config,
            collector_config::save_collector_config,
            history::query_events,
            history::query_samples,
            history::get_collector_status,
            history::clear_history,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
