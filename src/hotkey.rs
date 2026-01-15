use crate::config::HotkeyConfig;
use anyhow::{Context, Result};
use core_foundation::runloop::{kCFRunLoopCommonModes, kCFRunLoopDefaultMode, CFRunLoop};
use core_graphics::event::{CGEventTapLocation, CGEventType};
use std::sync::atomic::{AtomicBool, Ordering};
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
pub struct HotkeyListener {
    key_code: u16,
    modifiers: u64,
    callback: Box<dyn Fn() + Send + Sync>,
    running: Arc<AtomicBool>,
}

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
}
