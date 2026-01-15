# helix-anywhere - Publishing Guide

This document details the steps required to publish helix-anywhere on various platforms.

## Current Status (v0.1.0)

- [x] Core functionality complete
- [x] Menu bar icon working
- [x] Terminal support: Ghostty, WezTerm
- [x] Configuration system
- [x] README and LICENSE created
- [x] App bundle (.app) created
- [ ] GitHub repository setup
- [ ] Homebrew tap (future)
- [ ] crates.io (future)
- [ ] App bundle signed and notarized (future)

---

## 1. GitHub Repository

### Steps
1. Create repository `sylvainhellin/helix-anywhere` on GitHub
2. Push code:
   ```bash
   git init
   git add .
   git commit -m "Initial commit"
   git remote add origin git@github.com:sylvainhellin/helix-anywhere.git
   git push -u origin main
   ```
3. Create a release:
   - Build the app: `cargo bundle --release`
   - Create zip: `ditto -c -k --keepParent "target/release/bundle/osx/Helix Anywhere.app" dist/Helix.Anywhere.app.zip`
   - Go to GitHub > Releases > Create new release
   - Tag: `v0.1.0`
   - Upload `dist/Helix.Anywhere.app.zip`

---

## 2. Homebrew Tap

### Prerequisites
- GitHub release created with binaries
- SHA256 hashes from release script output

### Steps
1. Create repository `sylvainhellin/homebrew-helix-anywhere` on GitHub
2. Create `Formula/helix-anywhere.rb` with actual SHA256 hashes:
   ```ruby
   class HelixAnywhere < Formula
     desc "Edit text from any application using Helix editor"
     homepage "https://github.com/sylvainhellin/helix-anywhere"
     version "0.1.0"
     license "MIT"

     on_macos do
       on_arm do
         url "https://github.com/sylvainhellin/helix-anywhere/releases/download/v0.1.0/helix-anywhere-darwin-arm64.tar.gz"
         sha256 "ACTUAL_SHA256_FOR_ARM64"
       end
       on_intel do
         url "https://github.com/sylvainhellin/helix-anywhere/releases/download/v0.1.0/helix-anywhere-darwin-x86_64.tar.gz"
         sha256 "ACTUAL_SHA256_FOR_X86_64"
       end
     end

     depends_on :macos

     def install
       bin.install "helix-anywhere"
     end

     def caveats
       <<~EOS
         helix-anywhere requires Accessibility permissions.
         Grant in: System Settings > Privacy & Security > Accessibility

         Make sure Helix is installed: brew install helix
       EOS
     end
   end
   ```
3. Push to the homebrew-helix-anywhere repo
4. Test: `brew tap sylvainhellin/helix-anywhere && brew install helix-anywhere`

---

## 3. crates.io

### Prerequisites
- crates.io account (login via GitHub)
- API token from https://crates.io/me

### Steps
1. Login to crates.io:
   ```bash
   cargo login
   # Paste your API token
   ```

2. Verify package:
   ```bash
   cargo publish --dry-run
   ```

3. Publish:
   ```bash
   cargo publish
   ```

### Notes
- Package name `helix-anywhere` must be unique on crates.io
- Once published, versions cannot be overwritten
- Users install with: `cargo install helix-anywhere`

---

## 4. macOS App Bundle (.app)

### Current Status
- [x] cargo-bundle configured in Cargo.toml
- [x] AppIcon.icns created
- [ ] Code signing (optional but recommended)
- [ ] Notarization (required for distribution outside App Store)

### Build App Bundle
```bash
cargo bundle --release
# Output: target/release/bundle/osx/Helix Anywhere.app
```

### Code Signing (Optional)

#### Ad-hoc signing (for local testing):
```bash
codesign --deep --force --verify --verbose --sign - "target/release/bundle/osx/Helix Anywhere.app"
```

#### Developer ID signing (for distribution):
Requires Apple Developer account ($99/year)
```bash
codesign --deep --force --verify --verbose \
  --sign "Developer ID Application: Your Name (TEAM_ID)" \
  "target/release/bundle/osx/Helix Anywhere.app"
```

### Notarization (Required for Gatekeeper)
For apps distributed outside the App Store, Apple requires notarization:

1. Create app-specific password at appleid.apple.com
2. Submit for notarization:
   ```bash
   xcrun notarytool submit "Helix Anywhere.app.zip" \
     --apple-id "your@email.com" \
     --team-id "TEAM_ID" \
     --password "app-specific-password" \
     --wait
   ```
3. Staple the ticket:
   ```bash
   xcrun stapler staple "Helix Anywhere.app"
   ```

### Create DMG
```bash
# Install create-dmg
brew install create-dmg

# Create DMG
create-dmg \
  --volname "Helix Anywhere" \
  --window-pos 200 120 \
  --window-size 600 400 \
  --icon-size 100 \
  --icon "Helix Anywhere.app" 150 190 \
  --app-drop-link 450 190 \
  "Helix-Anywhere-0.1.0.dmg" \
  "target/release/bundle/osx/Helix Anywhere.app"
```

---

## 5. Mac App Store

### Status: NOT RECOMMENDED

The Mac App Store requires apps to run in a **sandbox**, which conflicts with helix-anywhere's requirements:

- **Accessibility permissions**: Required for simulating keystrokes
- **Launching external processes**: Required for opening terminals
- **File system access**: Required for temp files

These capabilities are incompatible with App Store sandboxing requirements.

**Recommended distribution**: Homebrew tap + direct DMG download

---

## Future Enhancements (Not blocking release)

- [ ] **Neovim/Vim support**: Add `editor` config option
- [ ] **App renaming**: Consider `edit-anywhere` if multi-editor
- [ ] **Better completion detection**: Replace file polling with filesystem watcher
- [ ] **Auto-update**: Implement Sparkle or similar for update notifications

---

## Quick Release Checklist (v0.1.0)

```
[ ] 1. Ensure all code is committed
[ ] 2. Update version in Cargo.toml if needed
[ ] 3. Build app: cargo bundle --release
[ ] 4. Create zip: ditto -c -k --keepParent "target/release/bundle/osx/Helix Anywhere.app" dist/Helix.Anywhere.app.zip
[ ] 5. Create GitHub release (tag: v0.1.0), upload dist/Helix.Anywhere.app.zip
```

## Future Release Checklist (with Homebrew/crates.io)

```
[ ] 1-5. Same as above
[ ] 6. Update homebrew formula with SHA256 hash
[ ] 7. Push to homebrew-helix-anywhere repo
[ ] 8. Test: brew tap sylvainhellin/helix-anywhere && brew install helix-anywhere
[ ] 9. Publish to crates.io: cargo publish
```
