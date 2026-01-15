
# Plan: Add Hotkey Customization Feature

## Overview
Add a "Hotkey" submenu to the menu bar (between Terminal and Quit) that allows users to record and set a custom hotkey for triggering the edit workflow.

## User Experience Flow
1. User clicks menu bar icon → hovers over "Hotkey" submenu
2. Submenu shows:
   - `Current: ⌘⇧;` (disabled, informational)
   - Separator
   - `Record New Hotkey...`
   - `Reset to Default (⌘⇧;)`
3. User clicks "Record New Hotkey..." → menu closes → notification: "Press your new hotkey..."
4. User presses new key combination → notification confirms → hotkey is active immediately

## Implementation Steps

### Step 1: Add Hotkey Display Formatting (src/hotkey.rs)
Add functions to convert hotkey config to display format using Unicode symbols:
- `⌘` (Command), `⇧` (Shift), `⌥` (Option), `⌃` (Control)
- `format_hotkey_display(&HotkeyConfig) -> String` (e.g., "⌘⇧;")
- Reverse mapping from key codes to display strings

### Step 2: Add Hotkey Recorder Module (src/hotkey_recorder.rs - new file)
Create a one-shot hotkey recorder using CGEventTap:
- `record_next_hotkey(callback: FnOnce(HotkeyConfig))` - captures next key+modifiers
- Spawns a temporary event tap thread
- Extracts key code and modifiers from the event
- Converts to HotkeyConfig and calls callback
- Auto-cleans up after capture
- Add timeout (10 seconds) with cancellation

### Step 3: Add Hotkey Controller for Restart Support (src/hotkey.rs)
Wrap listener in a controller that supports runtime updates:
```rust
pub struct HotkeyController {
    command_tx: Sender<HotkeyCommand>,
}

impl HotkeyController {
    pub fn update_hotkey(&self, config: HotkeyConfig);
    pub fn stop(&self);
}

pub fn start_hotkey_listener_with_controller<F>(
    initial_config: HotkeyConfig,
    callback: F,
) -> HotkeyController
```
- Uses channel-based communication
- Stop current listener → restart with new config

### Step 4: Add Hotkey Submenu (src/menu_bar.rs)
Add new submenu after Terminal section (before Quit separator):
- Add `static mut HOTKEY_SUBMENU: Option<id>` for storing reference
- Add `static mut HOTKEY_CONTROLLER: Option<HotkeyController>` for listener control
- Create submenu with:
  - Current hotkey display item (disabled)
  - "Record New Hotkey..." item with `sel!(recordHotkey:)` action
  - "Reset to Default (⌘⇧;)" item with `sel!(resetHotkey:)` action

### Step 5: Add Menu Action Handlers (src/menu_bar.rs)
In `register_menu_delegate_class()`, add:
- `extern "C" fn record_hotkey()`:
  - Show notification "Press your new hotkey..."
  - Call `record_next_hotkey()` with callback that:
    - Updates config
    - Saves to file
    - Updates controller
    - Updates menu display
    - Shows confirmation notification
- `extern "C" fn reset_hotkey()`:
  - Reset to default (cmd+shift+semicolon)
  - Update config, save, update controller, update menu

### Step 6: Add Helper Functions (src/menu_bar.rs)
- `update_hotkey_display(&HotkeyConfig)` - updates current hotkey menu item
- `show_notification(title, message)` - uses osascript for macOS notifications
- `pub fn set_hotkey_controller(controller: HotkeyController)` - called from main

### Step 7: Update Main Entry Point (src/main.rs)
- Replace direct `HotkeyListener` creation with `start_hotkey_listener_with_controller()`
- Call `menu_bar::set_hotkey_controller()` to pass controller to menu system

## Files to Modify

| File | Changes |
|------|---------|
| `src/hotkey.rs` | Add display formatting, HotkeyController, restart support |
| `src/hotkey_recorder.rs` | **New file** - CGEventTap-based one-shot recorder |
| `src/menu_bar.rs` | Add Hotkey submenu, action handlers, display updates |
| `src/main.rs` | Use HotkeyController, add mod declaration |

## Verification

1. **Basic recording**: Click "Record New Hotkey...", press a combination, verify menu shows new hotkey
2. **Hotkey works**: Verify the new hotkey triggers the edit workflow
3. **Old hotkey disabled**: Verify the old hotkey no longer triggers anything
4. **Persistence**: Quit and restart app, verify custom hotkey is preserved
5. **Reset**: Click "Reset to Default", verify reverts to ⌘⇧;
6. **Config file**: Check `~/.config/helix-anywhere/config.toml` is updated correctly

## Key Technical Details

- Hotkey recording uses the same CGEventTap pattern as the listener
- Controller uses `std::sync::mpsc::channel` for thread-safe stop/restart signaling
- Listener thread loop: create listener → wait for command → stop → (restart with new config OR exit)
- Reserved shortcuts (Cmd+Q, Cmd+Tab, etc.) should be rejected with warning notification
