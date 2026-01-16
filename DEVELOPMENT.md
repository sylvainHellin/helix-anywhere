# Development Guide

## Prerequisites

- Rust toolchain (install via [rustup](https://rustup.rs/))
- `cargo-bundle` for creating macOS app bundles:
  ```bash
  cargo install cargo-bundle
  ```

## Building

There are three ways to build and run the app, depending on your goal:

### Quick iteration (no app bundle)

Run directly from source without creating an `.app` bundle. Fastest for development:

```bash
cargo run --release
```

Or with debug logging:
```bash
RUST_LOG=debug cargo run --release
```

This compiles and runs the binary directly. No `.app` bundle is created.

### Local testing (app bundle)

Create a macOS `.app` bundle for testing the full app experience:

```bash
cargo bundle --release
```

This creates:
```
target/release/bundle/osx/Helix Anywhere.app
```

Install to `/Applications`:
```bash
cp -R "target/release/bundle/osx/Helix Anywhere.app" /Applications/
```

Or replace an existing installation:
```bash
rm -rf "/Applications/Helix Anywhere.app"
cp -R "target/release/bundle/osx/Helix Anywhere.app" /Applications/
```

**Note:** `cargo bundle` won't rebuild if nothing changed. Run `cargo build --release` first if you need to force a rebuild.

### GitHub release (clean build + zip)

For publishing a release, always start with a clean build to ensure everything is fresh:

```bash
cargo clean
cargo bundle --release
```

Then create the zip:
```bash
pushd target/release/bundle/osx && zip -r "Helix.Anywhere.app.zip" "Helix Anywhere.app" && popd
```

The release zip will be at `target/release/bundle/osx/Helix.Anywhere.app.zip`.

## Release Checklist

### 1. Update version numbers

Update the version in two places:

- `Cargo.toml`: `version = "X.Y.Z"`
- `src/menu_bar.rs`: Search for `helix-anywhere v` and update the version string

### 2. Build the release

```bash
cargo clean
cargo bundle --release
pushd target/release/bundle/osx && zip -r "Helix.Anywhere.app.zip" "Helix Anywhere.app" && popd
```

### 3. Commit, tag, and push

```bash
git add .
git commit -m "Release vX.Y.Z: <description>"
git tag vX.Y.Z
git push origin main --tags
```

### 4. Create GitHub release

Upload `target/release/bundle/osx/Helix.Anywhere.app.zip` to the GitHub release.

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
