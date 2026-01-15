# Development Guide

## Prerequisites

- Rust toolchain (install via [rustup](https://rustup.rs/))
- `cargo-bundle` for creating macOS app bundles:
  ```bash
  cargo install cargo-bundle
  ```

## Building

### Debug build
```bash
cargo build
```

### Release build
```bash
cargo build --release
```

The binary will be at `target/release/helix-anywhere`.

## Creating the App Bundle

To create a proper macOS `.app` bundle:

```bash
cargo bundle --release
```

This creates the app at:
```
target/release/bundle/osx/Helix Anywhere.app
```

## Installing Locally

After bundling, install to `/Applications`:

```bash
cp -R "target/release/bundle/osx/Helix Anywhere.app" /Applications/
```

Or to replace an existing installation:

```bash
rm -rf "/Applications/Helix Anywhere.app"
cp -R "target/release/bundle/osx/Helix Anywhere.app" /Applications/
```

## Running for Development

### Run directly (without bundling)
```bash
cargo run --release
```

### Run with debug logging
```bash
RUST_LOG=debug cargo run --release
```

## Creating a Release

### 1. Update version numbers

Update the version in two places:

- `Cargo.toml`: `version = "X.Y.Z"`
- `src/menu_bar.rs`: Search for `helix-anywhere v` and update the version string

### 2. Build and bundle

```bash
cargo build --release
cargo bundle --release
```

### 3. Create the release zip

```bash
cd target/release/bundle/osx
zip -r Helix-Anywhere-vX.Y.Z.zip "Helix Anywhere.app"
mv Helix-Anywhere-vX.Y.Z.zip ../../../../
```

Or as a one-liner:
```bash
(cd target/release/bundle/osx && zip -r ../../../../Helix-Anywhere-vX.Y.Z.zip "Helix Anywhere.app")
```

### 4. Create git tag and release

```bash
git add .
git commit -m "Release vX.Y.Z: <description>"
git tag vX.Y.Z
git push origin main --tags
```

Then create a GitHub release and upload the zip file.

## Project Structure

```
src/
├── main.rs              # Entry point, app initialization
├── menu_bar.rs          # macOS menu bar UI
├── hotkey.rs            # Global hotkey listener
├── hotkey_recorder.rs   # Hotkey recording for customization
├── config.rs            # Configuration management
├── terminal.rs          # Terminal launcher implementations
├── edit_session.rs      # Edit workflow orchestration
├── clipboard.rs         # Clipboard operations
└── keystroke.rs         # Keyboard event simulation

assets/
├── logo_app.png         # Menu bar icon
└── AppIcon.icns         # Application icon
```

## Configuration File Location

The app stores its configuration at:
```
~/.config/helix-anywhere/config.toml
```

## Debugging

### Check if hotkey events are being detected
```bash
RUST_LOG=debug cargo run --release 2>&1 | grep -i hotkey
```

### Common issues

1. **Accessibility permissions**: The app needs accessibility permissions to detect global hotkeys. Grant in System Settings → Privacy & Security → Accessibility.

2. **Multiple instances**: Make sure only one instance is running. Kill all with:
   ```bash
   pkill -f helix-anywhere
   ```

3. **Permission caching**: If hotkeys stop working after rebuild, toggle the accessibility permission off and on again.
