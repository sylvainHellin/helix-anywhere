use crate::clipboard;
use crate::config::Config;
use crate::keystroke;
use crate::terminal::Terminal;
use anyhow::{bail, Context, Result};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::{Duration, SystemTime};
use tempfile::NamedTempFile;

/// Get the bundle identifier of the frontmost application
fn get_frontmost_app() -> Option<String> {
    let output = Command::new("osascript")
        .arg("-e")
        .arg(r#"tell application "System Events" to get bundle identifier of first application process whose frontmost is true"#)
        .output()
        .ok()?;

    if output.status.success() {
        let bundle_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !bundle_id.is_empty() {
            log::info!("Frontmost app: {}", bundle_id);
            return Some(bundle_id);
        }
    }
    None
}

/// Activate an application by its bundle identifier
fn activate_app(bundle_id: &str) -> Result<()> {
    let script = format!(
        r#"tell application id "{}" to activate"#,
        bundle_id
    );
    Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .output()
        .context("Failed to activate app")?;

    // Give the app time to come to front
    thread::sleep(Duration::from_millis(100));
    Ok(())
}

/// Run an edit session
///
/// 1. Simulate Cmd+C to copy selected text
/// 2. Get clipboard content
/// 3. Write to temp file
/// 4. Launch terminal with helix
/// 5. Wait for terminal to exit
/// 6. If content changed, paste back
pub fn run_edit_session(config: &Config) -> Result<()> {
    log::info!("Starting edit session");

    // Step 0: Remember the frontmost app so we can return to it
    let original_app = get_frontmost_app();

    // Step 1: Save current clipboard content (to restore if aborted)
    let original_clipboard = clipboard::get_text().ok();

    // Step 2: Simulate Cmd+C to copy selection
    keystroke::simulate_copy()
        .context("Failed to simulate copy")?;

    // Small delay to ensure clipboard is updated
    thread::sleep(Duration::from_millis(50));

    // Step 3: Get the selected text from clipboard
    let selected_text = clipboard::get_text()
        .context("Failed to read selected text from clipboard")?;

    if selected_text.is_empty() {
        log::warn!("No text selected, aborting edit session");
        // Restore original clipboard if we had one
        if let Some(orig) = original_clipboard {
            let _ = clipboard::set_text(&orig);
        }
        return Ok(());
    }

    log::info!("Captured {} characters of selected text", selected_text.len());

    // Step 4: Create temp file with the selected text
    let mut temp_file = NamedTempFile::with_suffix(".txt")
        .context("Failed to create temp file")?;

    temp_file
        .write_all(selected_text.as_bytes())
        .context("Failed to write to temp file")?;

    temp_file
        .flush()
        .context("Failed to flush temp file")?;

    let temp_path = temp_file.path().to_path_buf();
    log::info!("Created temp file: {:?}", temp_path);

    // Store original content hash for comparison
    let original_hash = hash_content(&selected_text);

    // Step 5: Launch terminal with helix
    let terminal = Terminal::from_name(&config.terminal.name)
        .context("Invalid terminal name in config")?;

    if !terminal.is_installed() {
        bail!(
            "Terminal '{}' is not installed. Please install it or change the terminal in config.",
            terminal.display_name()
        );
    }

    log::info!("Launching {} with helix", terminal.display_name());

    // Get file modification time before launch (for polling-based terminals)
    let original_mtime = fs::metadata(&temp_path)
        .and_then(|m| m.modified())
        .unwrap_or_else(|_| SystemTime::now());

    let mut child = terminal
        .launch(&temp_path, config.terminal.width, config.terminal.height)
        .context("Failed to launch terminal")?;

    // Step 6: Wait for terminal/helix to exit
    if terminal.needs_polling() {
        // For terminals launched via AppleScript or `open`, we can't wait on the child
        // Instead, poll the file for changes
        log::info!("Using file polling to detect edit completion (terminal uses AppleScript/open)");
        wait_for_file_change(&temp_path, original_mtime)?;
        log::info!("File change detected, edit session complete");
    } else {
        // For terminals with proper CLI support, we can wait on the child process
        let status = child.wait().context("Failed to wait for terminal")?;
        log::info!("Terminal exited with status: {:?}", status);
    }

    // Step 7: Read the edited content
    let edited_text = fs::read_to_string(&temp_path)
        .context("Failed to read edited file")?;

    // Trim trailing newline that Helix adds when saving
    let edited_text = edited_text.trim_end_matches('\n').to_string();

    let edited_hash = hash_content(&edited_text);

    // Step 8: Check if content changed
    if original_hash == edited_hash {
        log::info!("Content unchanged, not pasting back (user likely aborted)");
        // Restore original clipboard
        if let Some(orig) = original_clipboard {
            let _ = clipboard::set_text(&orig);
        }
        return Ok(());
    }

    log::info!("Content changed, pasting back {} characters", edited_text.len());

    // Step 9: Put edited text in clipboard
    clipboard::set_text(&edited_text)
        .context("Failed to set clipboard with edited text")?;

    // Step 10: Return focus to the original app
    if let Some(ref app_id) = original_app {
        log::info!("Restoring focus to original app: {}", app_id);
        activate_app(app_id)?;
    } else {
        // Fallback: small delay hoping focus returns naturally
        thread::sleep(Duration::from_millis(100));
    }

    // Step 11: Simulate Cmd+V to paste
    keystroke::simulate_paste()
        .context("Failed to simulate paste")?;

    log::info!("Edit session completed successfully");
    Ok(())
}

/// Simple hash function for content comparison
fn hash_content(content: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    hasher.finish()
}

/// Wait for the file to be modified or deleted
/// This is used for terminals that can't be waited on directly (Ghostty, iTerm, Terminal.app)
/// TODO: Replace with a more elegant solution (filesystem watcher, AppleScript check)
fn wait_for_file_change(path: &Path, original_mtime: SystemTime) -> Result<()> {
    const POLL_INTERVAL: Duration = Duration::from_millis(100);
    const TIMEOUT: Duration = Duration::from_secs(3600); // 1 hour timeout

    let start = std::time::Instant::now();

    // Small delay to let the terminal open
    thread::sleep(Duration::from_millis(500));

    loop {
        // Check timeout
        if start.elapsed() > TIMEOUT {
            bail!("Timeout waiting for edit to complete (1 hour)");
        }

        // Check if file was modified
        match fs::metadata(path) {
            Ok(metadata) => {
                if let Ok(mtime) = metadata.modified() {
                    if mtime > original_mtime {
                        // File was modified - user saved
                        return Ok(());
                    }
                }
            }
            Err(_) => {
                // File was deleted - user quit without saving or something went wrong
                // We'll let the caller handle this (it will fail to read the file)
                return Ok(());
            }
        }

        thread::sleep(POLL_INTERVAL);
    }
}
