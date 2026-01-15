# helix-anywhere - Architecture

## Overview

A standalone Rust macOS menu bar application that allows editing text from any GUI application using Helix editor.

**Workflow:**
1. User selects text in any application (Outlook, Slack, Teams, etc.)
2. Presses `Cmd+Shift+;` (configurable)
3. App copies selection, opens terminal with Helix
4. User edits text, saves and quits (`:wq`)
5. Edited text is automatically pasted back into the original app

If user quits without saving (`:q!`), the original text is preserved.

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                    helix-anywhere                           │
├─────────────────────────────────────────────────────────────┤
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐  │
│  │ Hotkey       │  │ Menu Bar     │  │ Edit Session     │  │
│  │ Listener     │  │ Controller   │  │ Manager          │  │
│  │              │  │              │  │                  │  │
│  │ - CGEventTap │  │ - NSStatusBar│  │ - Copy selection │  │
│  │ - Key codes  │  │ - Icon       │  │ - Create temp    │  │
│  │ - Modifiers  │  │ - Menu       │  │ - Launch terminal│  │
│  │              │  │ - Terminal   │  │ - Wait for exit  │  │
│  │              │  │   selection  │  │ - Paste back     │  │
│  └──────────────┘  └──────────────┘  └──────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## Components

### 1. Global Hotkey Registration (`hotkey.rs`)
- Uses `CGEventTap` from `core-graphics` crate
- Listens for configurable key combinations
- Requires Accessibility permissions

### 2. Menu Bar UI (`menu_bar.rs`)
- `NSStatusBar` with custom icon (template image)
- Terminal selection submenu
- Quit option

### 3. Clipboard Operations (`clipboard.rs`)
- Uses `arboard` crate
- Read/write text to system clipboard

### 4. Keystroke Simulation (`keystroke.rs`)
- Uses `CGEvent` API
- Simulates `Cmd+C` (copy) and `Cmd+V` (paste)
- Requires Accessibility permissions

### 5. Edit Session Manager (`edit_session.rs`)
- Orchestrates the full edit workflow
- Tracks original app for focus restoration
- Detects save vs abort via content hash comparison

### 6. Terminal Launcher (`terminal.rs`)
- Currently supports: Ghostty, WezTerm
- Future: Kitty, Alacritty, iTerm2, Terminal.app
- Different launch mechanisms per terminal
- File polling for terminals that can't be waited on directly

### 7. Configuration (`config.rs`)
- TOML configuration file
- Stored in `~/Library/Application Support/com.helix-anywhere.helix-anywhere/`

## Supported Terminals

| Terminal | Status | Launch Method | Wait Method |
|----------|--------|---------------|-------------|
| Ghostty | ✅ Supported | Shell script + `open -na` | File polling |
| WezTerm | ✅ Supported | `wezterm start --always-new-process` | Process wait |
| Kitty | 🔜 Future | `kitty` CLI | Process wait |
| Alacritty | 🔜 Future | `alacritty -e` | Process wait |
| iTerm2 | 🔜 Future | AppleScript | File polling |
| Terminal.app | 🔜 Future | AppleScript | File polling |

## Dependencies

```toml
cocoa = "0.26"          # macOS AppKit bindings
objc = "0.2"            # Objective-C runtime
core-foundation = "0.10" # Core Foundation types
core-graphics = "0.24"  # CGEvent API
arboard = "3.4"         # Clipboard
tempfile = "3.14"       # Temporary files
anyhow = "1.0"          # Error handling
log = "0.4"             # Logging
env_logger = "0.11"     # Log output
serde = "1.0"           # Serialization
toml = "0.8"            # Config format
directories = "5.0"     # Platform directories
```

## File Structure

```
helix-anywhere/
├── Cargo.toml
├── README.md
├── LICENSE
├── src/
│   ├── main.rs           # Entry point, app initialization
│   ├── hotkey.rs         # Global hotkey registration
│   ├── menu_bar.rs       # Status bar UI
│   ├── clipboard.rs      # Clipboard operations
│   ├── keystroke.rs      # Simulating Cmd+C/V
│   ├── edit_session.rs   # Core edit workflow
│   ├── config.rs         # Configuration management
│   └── terminal.rs       # Terminal detection & launching
├── assets/
│   ├── logo_app.png      # Menu bar icon (template)
│   └── AppIcon.icns      # App bundle icon
├── specs/
│   ├── ARCHITECTURE.md   # This file
│   └── PUBLISHING.md     # Publishing guide
├── Formula/
│   └── helix-anywhere.rb # Homebrew formula template
└── scripts/
    └── release.sh        # Release build script
```

## Permissions Required

The app requires **Accessibility permissions** for:
- Simulating `Cmd+C` to copy selected text
- Simulating `Cmd+V` to paste edited text back

Users must grant permission in:
**System Settings > Privacy & Security > Accessibility > helix-anywhere**
