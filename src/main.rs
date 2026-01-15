// Suppress warnings from deprecated `cocoa` crate (would require migration to `objc2`)
#![allow(deprecated)]
// Suppress cfg warnings from `objc` crate's msg_send! macro
#![allow(unexpected_cfgs)]

mod clipboard;
mod config;
mod edit_session;
mod hotkey;
mod hotkey_recorder;
mod keystroke;
mod menu_bar;
mod terminal;

use anyhow::Result;
use config::Config;
use std::sync::{Arc, Mutex};

fn main() -> Result<()> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_secs()
        .init();

    log::info!("Starting helix-anywhere");

    // Load configuration
    let config = Config::load()?;
    log::info!("Config loaded: {:?}", config);

    // Wrap config in Arc<Mutex> for sharing
    let config = Arc::new(Mutex::new(config));
    let config_for_hotkey = config.clone();
    let config_for_menu = config.clone();

    // Initialize the macOS app
    menu_bar::init_app();

    // Create status bar item
    let _status_item = menu_bar::create_status_item(config_for_menu.clone(), move |cfg| {
        if let Err(e) = cfg.save() {
            log::error!("Failed to save config: {}", e);
        }
    })?;

    // Start hotkey listener with controller (supports runtime updates)
    let hotkey_config = {
        let cfg = config_for_hotkey.lock().unwrap();
        cfg.hotkey.clone()
    };

    let config_for_callback = config_for_hotkey.clone();
    let hotkey_controller = hotkey::start_hotkey_listener_with_controller(
        hotkey_config.clone(),
        move || {
            // Clone config data so we don't hold the lock during the edit session
            // This prevents deadlock when user tries to change settings while editing
            let config_snapshot = {
                let config = config_for_callback.lock().unwrap();
                config.clone()
            };
            if let Err(e) = edit_session::run_edit_session(&config_snapshot) {
                log::error!("Edit session failed: {}", e);
            }
        },
    );

    // Pass the controller to the menu system for hotkey updates
    menu_bar::set_hotkey_controller(hotkey_controller);

    let hotkey_display = hotkey::format_hotkey_display(&hotkey_config);
    log::info!(
        "helix-anywhere is running. Press {} to edit selected text.",
        hotkey_display
    );

    // Run the app event loop (blocking)
    menu_bar::run_app();

    Ok(())
}
