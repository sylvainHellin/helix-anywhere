use anyhow::{Context, Result};
use arboard::Clipboard;

/// Get text from the clipboard
pub fn get_text() -> Result<String> {
    let mut clipboard = Clipboard::new()
        .context("Failed to access clipboard")?;

    clipboard.get_text()
        .context("Failed to read text from clipboard")
}

/// Set text to the clipboard
pub fn set_text(text: &str) -> Result<()> {
    let mut clipboard = Clipboard::new()
        .context("Failed to access clipboard")?;

    clipboard.set_text(text.to_string())
        .context("Failed to write text to clipboard")
}
