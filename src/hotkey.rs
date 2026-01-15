use crate::config::HotkeyConfig;
use anyhow::{Context, Result};
use core_foundation::runloop::{kCFRunLoopCommonModes, kCFRunLoopDefaultMode, CFRunLoop};
use core_graphics::event::{CGEventTapLocation, CGEventType};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;

// macOS virtual key codes for common keys
pub fn key_code_from_string(key: &str) -> Option<u16> {
    match key.to_lowercase().as_str() {
        "a" => Some(0x00),
        "s" => Some(0x01),
        "d" => Some(0x02),
        "f" => Some(0x03),
        "h" => Some(0x04),
        "g" => Some(0x05),
        "z" => Some(0x06),
        "x" => Some(0x07),
        "c" => Some(0x08),
        "v" => Some(0x09),
        "b" => Some(0x0B),
        "q" => Some(0x0C),
        "w" => Some(0x0D),
        "e" => Some(0x0E),
        "r" => Some(0x0F),
        "y" => Some(0x10),
        "t" => Some(0x11),
        "1" => Some(0x12),
        "2" => Some(0x13),
        "3" => Some(0x14),
        "4" => Some(0x15),
        "6" => Some(0x16),
        "5" => Some(0x17),
        "=" => Some(0x18),
        "9" => Some(0x19),
        "7" => Some(0x1A),
        "-" => Some(0x1B),
        "8" => Some(0x1C),
        "0" => Some(0x1D),
        "]" => Some(0x1E),
        "o" => Some(0x1F),
        "u" => Some(0x20),
        "[" => Some(0x21),
        "i" => Some(0x22),
        "p" => Some(0x23),
        "l" => Some(0x25),
        "j" => Some(0x26),
        "'" | "quote" => Some(0x27),
        "k" => Some(0x28),
        ";" | "semicolon" => Some(0x29),
        "\\" | "backslash" => Some(0x2A),
        "," | "comma" => Some(0x2B),
        "/" | "slash" => Some(0x2C),
        "n" => Some(0x2D),
        "m" => Some(0x2E),
        "." | "period" => Some(0x2F),
        "`" | "grave" | "backtick" => Some(0x32),
        "space" => Some(0x31),
        "return" | "enter" => Some(0x24),
        "tab" => Some(0x30),
        "delete" | "backspace" => Some(0x33),
        "escape" | "esc" => Some(0x35),
        _ => None,
    }
}

// Raw modifier flag values (from CGEvent.h)
const FLAG_COMMAND: u64 = 0x00100000;
const FLAG_SHIFT: u64 = 0x00020000;
const FLAG_ALTERNATE: u64 = 0x00080000;
const FLAG_CONTROL: u64 = 0x00040000;

/// Mask for relevant modifier flags
const MODIFIER_MASK: u64 = FLAG_COMMAND | FLAG_SHIFT | FLAG_ALTERNATE | FLAG_CONTROL;

/// Convert modifier strings to raw flag bits
pub fn modifiers_from_config(modifiers: &[String]) -> u64 {
    let mut flags: u64 = 0;

    for modifier in modifiers {
        match modifier.to_lowercase().as_str() {
            "cmd" | "command" => flags |= FLAG_COMMAND,
            "shift" => flags |= FLAG_SHIFT,
            "alt" | "option" => flags |= FLAG_ALTERNATE,
            "ctrl" | "control" => flags |= FLAG_CONTROL,
            _ => log::warn!("Unknown modifier: {}", modifier),
        }
    }

    flags
}

/// Represents a registered hotkey
#[allow(dead_code)]
pub struct HotkeyListener {
    key_code: u16,
    modifiers: u64,
    callback: Box<dyn Fn() + Send + Sync>,
    running: Arc<AtomicBool>,
}

#[allow(dead_code)]
impl HotkeyListener {
    /// Create a new hotkey listener from config
    pub fn from_config<F>(config: &HotkeyConfig, callback: F) -> Result<Self>
    where
        F: Fn() + Send + Sync + 'static,
    {
        let key_code = key_code_from_string(&config.key)
            .with_context(|| format!("Unknown key: {}", config.key))?;

        let modifiers = modifiers_from_config(&config.modifiers);

        Ok(Self {
            key_code,
            modifiers,
            callback: Box::new(callback),
            running: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Start listening for the hotkey (blocking)
    /// This should be called from a dedicated thread
    pub fn start(&self) -> Result<()> {
        use core_graphics::event::{CGEventTap, CGEventTapOptions, CGEventTapPlacement};

        self.running.store(true, Ordering::SeqCst);

        let key_code = self.key_code;
        let target_modifiers = self.modifiers;
        let running = self.running.clone();

        // Create a channel to send hotkey events
        let (tx, rx) = std::sync::mpsc::channel::<()>();

        // Spawn the callback handler thread
        let callback = unsafe {
            // This is safe because we ensure the listener outlives the thread
            std::mem::transmute::<&(dyn Fn() + Send + Sync), &'static (dyn Fn() + Send + Sync)>(
                self.callback.as_ref(),
            )
        };

        std::thread::spawn(move || {
            while let Ok(()) = rx.recv() {
                callback();
            }
        });

        // Create event tap callback
        let tx_clone = tx.clone();
        let callback = move |_proxy: core_graphics::event::CGEventTapProxy,
                             event_type: CGEventType,
                             event: &core_graphics::event::CGEvent|
              -> Option<core_graphics::event::CGEvent> {
            // KeyDown = 10
            if matches!(event_type, CGEventType::KeyDown) {
                let event_key_code = event.get_integer_value_field(
                    core_graphics::event::EventField::KEYBOARD_EVENT_KEYCODE,
                ) as u16;

                // Get flags and extract the raw bits
                let event_flags = event.get_flags();
                let event_flags_raw: u64 = unsafe { std::mem::transmute(event_flags) };

                // Mask to only relevant modifier flags
                let event_mods = event_flags_raw & MODIFIER_MASK;
                let target_mods = target_modifiers & MODIFIER_MASK;

                if event_key_code == key_code && event_mods == target_mods {
                    log::info!("Hotkey triggered!");
                    let _ = tx_clone.send(());
                    // Consume the event (don't pass it to other apps)
                    return None;
                }
            }
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
        .context("Failed to create event tap. Make sure Accessibility permissions are granted.")?;

        // Enable the tap
        tap.enable();

        // Add to run loop
        let source = tap
            .mach_port
            .create_runloop_source(0)
            .ok()
            .context("Failed to create run loop source")?;

        let run_loop = CFRunLoop::get_current();
        run_loop.add_source(&source, unsafe { kCFRunLoopCommonModes });

        log::info!(
            "Hotkey listener started (key_code: 0x{:02X}, modifiers: 0x{:08X})",
            self.key_code,
            self.modifiers
        );

        // Run the loop
        while running.load(Ordering::SeqCst) {
            CFRunLoop::run_in_mode(
                unsafe { kCFRunLoopDefaultMode },
                std::time::Duration::from_secs(1),
                false,
            );
        }

        Ok(())
    }

    /// Stop the listener
    #[allow(dead_code)]
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// Get a reference to the running flag
    pub fn running_flag(&self) -> Arc<AtomicBool> {
        self.running.clone()
    }
}

// ============================================================================
// Hotkey Controller (supports runtime updates)
// ============================================================================

/// Command type for controlling the hotkey listener
pub enum HotkeyCommand {
    Stop,
    Restart(HotkeyConfig),
}

/// Controller for the hotkey listener that allows runtime updates
pub struct HotkeyController {
    command_tx: Sender<HotkeyCommand>,
}

impl HotkeyController {
    /// Update the hotkey configuration (will restart the listener)
    pub fn update_hotkey(&self, config: HotkeyConfig) {
        log::info!("Updating hotkey to: {:?}", config);
        if let Err(e) = self.command_tx.send(HotkeyCommand::Restart(config)) {
            log::error!("Failed to send hotkey update command: {}", e);
        }
    }

    /// Stop the hotkey listener
    #[allow(dead_code)]
    pub fn stop(&self) {
        if let Err(e) = self.command_tx.send(HotkeyCommand::Stop) {
            log::error!("Failed to send stop command: {}", e);
        }
    }
}

/// Start the hotkey listener with a controller for runtime management.
///
/// This spawns a thread that runs the hotkey listener and can restart it
/// when the hotkey configuration changes.
///
/// # Arguments
/// * `initial_config` - The initial hotkey configuration
/// * `callback` - The callback to run when the hotkey is triggered
///
/// # Returns
/// A HotkeyController that can be used to update or stop the listener
pub fn start_hotkey_listener_with_controller<F>(
    initial_config: HotkeyConfig,
    callback: F,
) -> HotkeyController
where
    F: Fn() + Send + Sync + Clone + 'static,
{
    let (tx, rx) = channel::<HotkeyCommand>();

    std::thread::spawn(move || {
        let mut current_config = initial_config;

        'outer: loop {
            log::info!(
                "Starting hotkey listener with config: {:?}",
                current_config
            );

            // Set up the listener components manually to integrate command checking
            let key_code = match key_code_from_string(&current_config.key) {
                Some(k) => k,
                None => {
                    log::error!("Unknown key: {}", current_config.key);
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    continue;
                }
            };
            let target_modifiers = modifiers_from_config(&current_config.modifiers);

            // Create channel for hotkey events
            let (hotkey_tx, hotkey_rx) = channel::<()>();

            // Spawn callback handler thread
            let callback_clone = callback.clone();
            std::thread::spawn(move || {
                while let Ok(()) = hotkey_rx.recv() {
                    callback_clone();
                }
            });

            // Create event tap
            use core_graphics::event::{CGEventTap, CGEventTapOptions, CGEventTapPlacement};

            let hotkey_tx_clone = hotkey_tx.clone();
            let tap_callback = move |_proxy: core_graphics::event::CGEventTapProxy,
                                     event_type: CGEventType,
                                     event: &core_graphics::event::CGEvent|
                  -> Option<core_graphics::event::CGEvent> {
                if matches!(event_type, CGEventType::KeyDown) {
                    let event_key_code = event.get_integer_value_field(
                        core_graphics::event::EventField::KEYBOARD_EVENT_KEYCODE,
                    ) as u16;

                    let event_flags = event.get_flags();
                    let event_flags_raw: u64 = unsafe { std::mem::transmute(event_flags) };
                    let event_mods = event_flags_raw & MODIFIER_MASK;
                    let target_mods = target_modifiers & MODIFIER_MASK;

                    if event_key_code == key_code && event_mods == target_mods {
                        log::info!("Hotkey triggered!");
                        let _ = hotkey_tx_clone.send(());
                        return None;
                    }
                }
                Some(event.clone())
            };

            let tap = match CGEventTap::new(
                CGEventTapLocation::Session,
                CGEventTapPlacement::HeadInsertEventTap,
                CGEventTapOptions::Default,
                vec![CGEventType::KeyDown],
                tap_callback,
            )
            .ok()
            {
                Some(t) => t,
                None => {
                    log::error!("Failed to create event tap. Make sure Accessibility permissions are granted.");
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    continue;
                }
            };

            tap.enable();

            let source = match tap.mach_port.create_runloop_source(0).ok() {
                Some(s) => s,
                None => {
                    log::error!("Failed to create run loop source");
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    continue;
                }
            };

            let run_loop = CFRunLoop::get_current();
            run_loop.add_source(&source, unsafe { kCFRunLoopCommonModes });

            log::info!(
                "Hotkey listener started (key_code: 0x{:02X}, modifiers: 0x{:08X})",
                key_code,
                target_modifiers
            );

            // Run loop with periodic command checking
            loop {
                // Run the event loop for a short time
                CFRunLoop::run_in_mode(
                    unsafe { kCFRunLoopDefaultMode },
                    std::time::Duration::from_millis(100),
                    false,
                );

                // Check for commands (non-blocking)
                match rx.try_recv() {
                    Ok(HotkeyCommand::Stop) => {
                        log::info!("Stopping hotkey listener");
                        break 'outer;
                    }
                    Ok(HotkeyCommand::Restart(new_config)) => {
                        log::info!("Restarting hotkey listener with new config");
                        current_config = new_config;
                        break; // Break inner loop to restart with new config
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                        // No command, continue running
                    }
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        log::info!("Hotkey controller channel closed, stopping listener");
                        break 'outer;
                    }
                }
            }
        }

        log::info!("Hotkey management thread exiting");
    });

    HotkeyController { command_tx: tx }
}

// ============================================================================
// Display formatting functions
// ============================================================================

/// Convert a key code back to a display string
#[allow(dead_code)]
pub fn key_code_to_display(key_code: u16) -> Option<String> {
    match key_code {
        0x00 => Some("A".to_string()),
        0x01 => Some("S".to_string()),
        0x02 => Some("D".to_string()),
        0x03 => Some("F".to_string()),
        0x04 => Some("H".to_string()),
        0x05 => Some("G".to_string()),
        0x06 => Some("Z".to_string()),
        0x07 => Some("X".to_string()),
        0x08 => Some("C".to_string()),
        0x09 => Some("V".to_string()),
        0x0B => Some("B".to_string()),
        0x0C => Some("Q".to_string()),
        0x0D => Some("W".to_string()),
        0x0E => Some("E".to_string()),
        0x0F => Some("R".to_string()),
        0x10 => Some("Y".to_string()),
        0x11 => Some("T".to_string()),
        0x12 => Some("1".to_string()),
        0x13 => Some("2".to_string()),
        0x14 => Some("3".to_string()),
        0x15 => Some("4".to_string()),
        0x16 => Some("6".to_string()),
        0x17 => Some("5".to_string()),
        0x18 => Some("=".to_string()),
        0x19 => Some("9".to_string()),
        0x1A => Some("7".to_string()),
        0x1B => Some("-".to_string()),
        0x1C => Some("8".to_string()),
        0x1D => Some("0".to_string()),
        0x1E => Some("]".to_string()),
        0x1F => Some("O".to_string()),
        0x20 => Some("U".to_string()),
        0x21 => Some("[".to_string()),
        0x22 => Some("I".to_string()),
        0x23 => Some("P".to_string()),
        0x25 => Some("L".to_string()),
        0x26 => Some("J".to_string()),
        0x27 => Some("'".to_string()),
        0x28 => Some("K".to_string()),
        0x29 => Some(";".to_string()),
        0x2A => Some("\\".to_string()),
        0x2B => Some(",".to_string()),
        0x2C => Some("/".to_string()),
        0x2D => Some("N".to_string()),
        0x2E => Some("M".to_string()),
        0x2F => Some(".".to_string()),
        0x32 => Some("`".to_string()),
        0x31 => Some("Space".to_string()),
        0x24 => Some("↵".to_string()),
        0x30 => Some("⇥".to_string()),
        0x33 => Some("⌫".to_string()),
        0x35 => Some("⎋".to_string()),
        _ => None,
    }
}

/// Convert a key name to display symbol
pub fn key_name_to_display(key: &str) -> String {
    match key.to_lowercase().as_str() {
        "semicolon" | ";" => ";".to_string(),
        "comma" | "," => ",".to_string(),
        "period" | "." => ".".to_string(),
        "slash" | "/" => "/".to_string(),
        "backslash" | "\\" => "\\".to_string(),
        "quote" | "'" => "'".to_string(),
        "grave" | "backtick" | "`" => "`".to_string(),
        "space" => "Space".to_string(),
        "return" | "enter" => "↵".to_string(),
        "tab" => "⇥".to_string(),
        "delete" | "backspace" => "⌫".to_string(),
        "escape" | "esc" => "⎋".to_string(),
        other => other.to_uppercase(),
    }
}

/// Convert modifier flags to display string with Unicode symbols
pub fn modifiers_to_display(modifiers: u64) -> String {
    let mut result = String::new();
    // Order: Control, Option, Shift, Command (standard macOS order)
    if modifiers & FLAG_CONTROL != 0 {
        result.push('⌃');
    }
    if modifiers & FLAG_ALTERNATE != 0 {
        result.push('⌥');
    }
    if modifiers & FLAG_SHIFT != 0 {
        result.push('⇧');
    }
    if modifiers & FLAG_COMMAND != 0 {
        result.push('⌘');
    }
    result
}

/// Convert modifier config strings to display string
pub fn modifiers_config_to_display(modifiers: &[String]) -> String {
    let flags = modifiers_from_config(modifiers);
    modifiers_to_display(flags)
}

/// Format a HotkeyConfig for display (e.g., "⌘⇧;")
pub fn format_hotkey_display(config: &HotkeyConfig) -> String {
    let mod_str = modifiers_config_to_display(&config.modifiers);
    let key_str = key_name_to_display(&config.key);
    format!("{}{}", mod_str, key_str)
}

/// Convert modifier flags back to config strings
pub fn modifiers_to_config(modifiers: u64) -> Vec<String> {
    let mut result = Vec::new();
    if modifiers & FLAG_COMMAND != 0 {
        result.push("cmd".to_string());
    }
    if modifiers & FLAG_SHIFT != 0 {
        result.push("shift".to_string());
    }
    if modifiers & FLAG_ALTERNATE != 0 {
        result.push("alt".to_string());
    }
    if modifiers & FLAG_CONTROL != 0 {
        result.push("ctrl".to_string());
    }
    result
}

/// Convert a key code back to config string
pub fn key_code_to_config(key_code: u16) -> Option<String> {
    match key_code {
        0x00 => Some("a".to_string()),
        0x01 => Some("s".to_string()),
        0x02 => Some("d".to_string()),
        0x03 => Some("f".to_string()),
        0x04 => Some("h".to_string()),
        0x05 => Some("g".to_string()),
        0x06 => Some("z".to_string()),
        0x07 => Some("x".to_string()),
        0x08 => Some("c".to_string()),
        0x09 => Some("v".to_string()),
        0x0B => Some("b".to_string()),
        0x0C => Some("q".to_string()),
        0x0D => Some("w".to_string()),
        0x0E => Some("e".to_string()),
        0x0F => Some("r".to_string()),
        0x10 => Some("y".to_string()),
        0x11 => Some("t".to_string()),
        0x12 => Some("1".to_string()),
        0x13 => Some("2".to_string()),
        0x14 => Some("3".to_string()),
        0x15 => Some("4".to_string()),
        0x16 => Some("6".to_string()),
        0x17 => Some("5".to_string()),
        0x18 => Some("=".to_string()),
        0x19 => Some("9".to_string()),
        0x1A => Some("7".to_string()),
        0x1B => Some("-".to_string()),
        0x1C => Some("8".to_string()),
        0x1D => Some("0".to_string()),
        0x1E => Some("]".to_string()),
        0x1F => Some("o".to_string()),
        0x20 => Some("u".to_string()),
        0x21 => Some("[".to_string()),
        0x22 => Some("i".to_string()),
        0x23 => Some("p".to_string()),
        0x25 => Some("l".to_string()),
        0x26 => Some("j".to_string()),
        0x27 => Some("'".to_string()),
        0x28 => Some("k".to_string()),
        0x29 => Some("semicolon".to_string()),
        0x2A => Some("backslash".to_string()),
        0x2B => Some("comma".to_string()),
        0x2C => Some("slash".to_string()),
        0x2D => Some("n".to_string()),
        0x2E => Some("m".to_string()),
        0x2F => Some("period".to_string()),
        0x32 => Some("grave".to_string()),
        0x31 => Some("space".to_string()),
        0x24 => Some("return".to_string()),
        0x30 => Some("tab".to_string()),
        0x33 => Some("backspace".to_string()),
        0x35 => Some("escape".to_string()),
        _ => None,
    }
}

/// Get the modifier mask constant (for use in recorder)
pub const fn get_modifier_mask() -> u64 {
    MODIFIER_MASK
}
