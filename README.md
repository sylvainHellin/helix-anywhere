# helix-anywhere

Edit text from any macOS application using [Helix](https://helix-editor.com/) editor.

![Demo](https://img.shields.io/badge/platform-macOS-blue) ![License](https://img.shields.io/badge/license-MIT-green)

## How it works

1. **Select text** in any application (Outlook, Slack, Teams, VS Code, etc.)
2. **Press `Cmd+Shift+;`** (configurable hotkey)
3. **Edit in Helix** - a terminal opens with your selected text
4. **Save and quit** (`:wq`) - edited text is pasted back automatically

If you quit without saving (`:q!`), the original text is preserved.

## Installation

### Direct Download

1. Download `Helix.Anywhere.app.zip` from the [latest release](https://github.com/sylvainhellin/helix-anywhere/releases/latest)
2. Unzip and drag **Helix Anywhere.app** to `/Applications`
3. Double-click to launch (you may need to right-click → Open on first launch)

### From source

```bash
git clone https://github.com/sylvainhellin/helix-anywhere
cd helix-anywhere
cargo bundle --release
cp -R "target/release/bundle/osx/Helix Anywhere.app" /Applications/
```

## Start at Login

To have Helix Anywhere start automatically when you log in:

1. Go to **System Settings → General → Login Items**
2. Click **+** under "Open at Login"
3. Select **Helix Anywhere** from Applications

## Requirements

- **macOS 11.0** or later
- **[Helix](https://helix-editor.com/)** editor installed (`brew install helix`)
- One of the supported terminals:
  - [Ghostty](https://ghostty.org/) (recommended)
  - [WezTerm](https://wezfurlong.org/wezterm/)

*Future versions may add support for Kitty, Alacritty, iTerm2, and Terminal.app.*

## Configuration

Configuration file location:
```
~/Library/Application Support/com.helix-anywhere.helix-anywhere/config.toml
```

### Default configuration

```toml
[hotkey]
modifiers = ["cmd", "shift"]
key = "semicolon"

[terminal]
name = "ghostty"  # or "wezterm"
width = 100
height = 30
```

### Available hotkey modifiers
- `cmd` / `command`
- `shift`
- `alt` / `option`
- `ctrl` / `control`

### Available keys
Letters (`a`-`z`), numbers (`0`-`9`), and special keys:
`semicolon`, `comma`, `period`, `slash`, `backslash`, `quote`, `grave`, `space`, `return`, `tab`, `escape`

## Permissions

The app requires **Accessibility permissions** to simulate copy/paste keystrokes.

On first run, macOS will prompt you to grant permissions. You can also enable them manually:

**System Settings → Privacy & Security → Accessibility → Helix Anywhere**

**Important:** If the hotkey doesn't work after installation:
1. Open **System Settings → Privacy & Security → Accessibility**
2. Find **Helix Anywhere** in the list
3. Toggle it **OFF** then **ON** again (or remove and re-add it)
4. Restart the app

## Usage Tips

- **Quick edit**: Select text, press hotkey, edit, `:wq` to save and paste back
- **Cancel**: Press `:q!` to quit without pasting (original text preserved)
- **Change terminal**: Click the menu bar icon → Terminal → select your preferred terminal

## Troubleshooting

### Hotkey not working (app launched from Spotlight/Finder)

The app needs **Accessibility permissions** to detect the hotkey. If the app works when launched from Terminal but not from Spotlight:

1. Open **System Settings → Privacy & Security → Accessibility**
2. If **Helix Anywhere** is already in the list:
   - Remove it (select it, click **-**)
   - Add it again (click **+**, navigate to `/Applications/Helix Anywhere.app`)
3. If it's not in the list:
   - Click **+** and add `/Applications/Helix Anywhere.app`
4. Make sure the toggle is **ON**
5. Restart the app

*Note: Each time the app is updated/rebuilt, you may need to re-grant permissions.*

### Hotkey not working (other causes)

1. Check if another app is using the same hotkey
2. Try a different hotkey in the config file

### Terminal not opening

1. Ensure the selected terminal is installed
2. For Ghostty: ensure it's in `/Applications/Ghostty.app`
3. Try switching to a different terminal in the menu

## License

MIT License - see [LICENSE](LICENSE) for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
