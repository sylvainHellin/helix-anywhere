//! Hotkey recorder module
//!
//! Provides a one-shot hotkey recording mechanism using CGEventTap.
//! When recording is started, the next key combination (modifiers + key)
//! will be captured and returned via a callback.

use crate::config::HotkeyConfig;
use crate::hotkey::{get_modifier_mask, key_code_to_config, modifiers_to_config};
use core_foundation::runloop::{kCFRunLoopCommonModes, kCFRunLoopDefaultMode, CFRunLoop};
use core_graphics::event::{CGEventTapLocation, CGEventType};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Timeout for recording (10 seconds)
const RECORDING_TIMEOUT: Duration = Duration::from_secs(10);

/// Start recording the next hotkey combination.
///
/// This function spawns a temporary event tap thread that captures the next
/// key press with modifiers. Once captured, the callback is called with the
/// resulting HotkeyConfig.
///
/// The recording will timeout after 10 seconds if no key is pressed.
///
/// # Arguments
/// * `on_recorded` - Callback called with the recorded HotkeyConfig
/// * `on_timeout` - Callback called if recording times out
/// * `on_error` - Callback called if recording fails (e.g., invalid key)
pub fn record_next_hotkey<F, T, E>(on_recorded: F, on_timeout: T, on_error: E)
where
    F: FnOnce(HotkeyConfig) + Send + 'static,
    T: FnOnce() + Send + 'static,
    E: FnOnce(String) + Send + 'static,
{
    std::thread::spawn(move || {
        if let Err(e) = record_hotkey_blocking(on_recorded, on_timeout) {
            on_error(e);
        }
    });
}

/// Internal blocking implementation of hotkey recording
fn record_hotkey_blocking<F, T>(on_recorded: F, on_timeout: T) -> Result<(), String>
where
    F: FnOnce(HotkeyConfig) + Send + 'static,
    T: FnOnce() + Send + 'static,
{
    use core_graphics::event::{CGEventTap, CGEventTapOptions, CGEventTapPlacement};

    let recorded = Arc::new(AtomicBool::new(false));
    let recorded_clone = recorded.clone();
    let start_time = Instant::now();

    // Channel to send the recorded hotkey
    let (tx, rx) = std::sync::mpsc::channel::<Option<HotkeyConfig>>();

    // Create event tap callback
    let callback = move |_proxy: core_graphics::event::CGEventTapProxy,
                         event_type: CGEventType,
                         event: &core_graphics::event::CGEvent|
          -> Option<core_graphics::event::CGEvent> {
        // Only process KeyDown events
        if !matches!(event_type, CGEventType::KeyDown) {
            return Some(event.clone());
        }

        // Check if already recorded
        if recorded_clone.load(Ordering::SeqCst) {
            return Some(event.clone());
        }

        // Get key code
        let key_code = event.get_integer_value_field(
            core_graphics::event::EventField::KEYBOARD_EVENT_KEYCODE,
        ) as u16;

        // Get modifier flags
        let event_flags = event.get_flags();
        let event_flags_raw: u64 = unsafe { std::mem::transmute(event_flags) };
        let modifiers = event_flags_raw & get_modifier_mask();

        // Ignore pure modifier key presses (no actual key)
        // Modifier-only key codes: Shift=56/60, Control=59/62, Option=58/61, Command=55/54
        let is_modifier_only = matches!(
            key_code,
            54 | 55 | 56 | 57 | 58 | 59 | 60 | 61 | 62 | 63
        );
        if is_modifier_only {
            return Some(event.clone());
        }

        // Convert to config format
        if let Some(key_name) = key_code_to_config(key_code) {
            let modifier_strings = modifiers_to_config(modifiers);

            // Require at least one modifier
            if modifier_strings.is_empty() {
                log::warn!("Hotkey recording: no modifiers pressed, ignoring");
                return Some(event.clone());
            }

            let config = HotkeyConfig {
                modifiers: modifier_strings,
                key: key_name,
            };

            recorded_clone.store(true, Ordering::SeqCst);
            let _ = tx.send(Some(config));

            // Consume the event
            return None;
        }

        // Unknown key code, let it pass through
        Some(event.clone())
    };

    // Create the event tap
    let tap = CGEventTap::new(
        CGEventTapLocation::Session,
        CGEventTapPlacement::HeadInsertEventTap,
        CGEventTapOptions::Default,
        vec![CGEventType::KeyDown],
        callback,
    )
    .ok()
    .ok_or_else(|| {
        "Failed to create event tap. Make sure Accessibility permissions are granted.".to_string()
    })?;

    // Enable the tap
    tap.enable();

    // Add to run loop
    let source = tap
        .mach_port
        .create_runloop_source(0)
        .ok()
        .ok_or_else(|| "Failed to create run loop source".to_string())?;

    let run_loop = CFRunLoop::get_current();
    run_loop.add_source(&source, unsafe { kCFRunLoopCommonModes });

    log::info!("Hotkey recording started, waiting for key press...");

    // Run the loop with timeout checking
    while !recorded.load(Ordering::SeqCst) {
        // Check timeout
        if start_time.elapsed() > RECORDING_TIMEOUT {
            log::info!("Hotkey recording timed out");
            on_timeout();
            return Ok(());
        }

        // Run loop for a short interval
        CFRunLoop::run_in_mode(
            unsafe { kCFRunLoopDefaultMode },
            Duration::from_millis(100),
            false,
        );
    }

    // Get the recorded hotkey
    if let Ok(Some(config)) = rx.try_recv() {
        log::info!("Hotkey recorded: {:?}", config);
        on_recorded(config);
    }

    Ok(())
}

/// Check if a hotkey combination is reserved by the system
/// Returns Some(reason) if reserved, None if available
#[allow(dead_code)]
pub fn is_reserved_hotkey(config: &HotkeyConfig) -> Option<&'static str> {
    let has_cmd = config.modifiers.iter().any(|m| m == "cmd" || m == "command");
    let only_cmd = config.modifiers.len() == 1 && has_cmd;

    if only_cmd {
        match config.key.to_lowercase().as_str() {
            "q" => Some("Cmd+Q is reserved for Quit"),
            "w" => Some("Cmd+W is reserved for Close Window"),
            "h" => Some("Cmd+H is reserved for Hide"),
            "m" => Some("Cmd+M is reserved for Minimize"),
            "tab" => Some("Cmd+Tab is reserved for App Switcher"),
            "space" => Some("Cmd+Space is reserved for Spotlight"),
            _ => None,
        }
    } else {
        None
    }
}
