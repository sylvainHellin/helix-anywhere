// Suppress warnings from deprecated `cocoa` crate (would require migration to `objc2`)
#![allow(deprecated)]
// Suppress cfg warnings from `objc` crate's msg_send! macro
#![allow(unexpected_cfgs)]

mod clipboard;
mod config;
mod edit_session;
mod hotkey;
mod keystroke;
mod menu_bar;
mod terminal;

use anyhow::Result;
use config::Config;
use std::sync::{Arc, Mutex};
use std::thread;

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

    // Start hotkey listener in a separate thread
    let hotkey_thread = thread::spawn(move || {
        let config = config_for_hotkey.lock().unwrap();
        let hotkey_config = config.hotkey.clone();
        drop(config); // Release the lock

        let config_for_callback = config_for_hotkey.clone();

        let listener = match hotkey::HotkeyListener::from_config(&hotkey_config, move || {
            // Clone config data so we don't hold the lock during the edit session
            // This prevents deadlock when user tries to change settings while editing
            let config_snapshot = {
                let config = config_for_callback.lock().unwrap();
                config.clone()
            };
            if let Err(e) = edit_session::run_edit_session(&config_snapshot) {
                log::error!("Edit session failed: {}", e);
            }
        }) {
            Ok(l) => l,
            Err(e) => {
                log::error!("Failed to create hotkey listener: {}", e);
                return;
            }
        };

        if let Err(e) = listener.start() {
            log::error!("Hotkey listener failed: {}", e);
        }
    });

    log::info!("helix-anywhere is running. Press Cmd+Shift+; to edit selected text.");

    // Run the app event loop (blocking)
    menu_bar::run_app();

    // Wait for hotkey thread (this won't actually be reached due to run_app)
    let _ = hotkey_thread.join();

    Ok(())
}
