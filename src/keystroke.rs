use anyhow::{Context, Result};
use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation, CGKeyCode};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use std::thread;
use std::time::Duration;

// macOS virtual key codes
const KEY_C: CGKeyCode = 0x08;
const KEY_V: CGKeyCode = 0x09;

/// Simulate a key press with command modifier
fn simulate_key_with_command(key_code: CGKeyCode) -> Result<()> {
    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .ok()
        .context("Failed to create event source")?;

    // Key down
    let key_down = CGEvent::new_keyboard_event(source.clone(), key_code, true)
        .ok()
        .context("Failed to create key down event")?;
    key_down.set_flags(CGEventFlags::CGEventFlagCommand);
    key_down.post(CGEventTapLocation::HID);

    // Small delay between down and up
    thread::sleep(Duration::from_millis(10));

    // Key up
    let key_up = CGEvent::new_keyboard_event(source, key_code, false)
        .ok()
        .context("Failed to create key up event")?;
    key_up.set_flags(CGEventFlags::CGEventFlagCommand);
    key_up.post(CGEventTapLocation::HID);

    Ok(())
}

/// Simulate Cmd+C (copy)
pub fn simulate_copy() -> Result<()> {
    log::debug!("Simulating Cmd+C");
    simulate_key_with_command(KEY_C)?;
    // Give the system time to process the copy
    thread::sleep(Duration::from_millis(100));
    Ok(())
}

/// Simulate Cmd+V (paste)
pub fn simulate_paste() -> Result<()> {
    log::debug!("Simulating Cmd+V");
    simulate_key_with_command(KEY_V)?;
    Ok(())
}
