pub mod pb {
    include!(concat!(env!("OUT_DIR"), "/mod.rs"));
}

mod common;
mod dish;
mod router;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
