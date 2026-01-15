use anyhow::Result;
use std::path::Path;
use std::process::{Child, Command};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Terminal {
    Ghostty,
    WezTerm,
    Kitty,
    Alacritty,
    ITerm,
    TerminalApp,
}

impl Terminal {
    /// Parse terminal name from string
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "ghostty" => Some(Terminal::Ghostty),
            "wezterm" => Some(Terminal::WezTerm),
            "kitty" => Some(Terminal::Kitty),
            "alacritty" => Some(Terminal::Alacritty),
            "iterm" | "iterm2" => Some(Terminal::ITerm),
            "terminal" | "terminal.app" => Some(Terminal::TerminalApp),
            _ => None,
        }
    }

    /// Get all supported terminals (shown in menu)
    pub fn all() -> Vec<Terminal> {
        vec![
            Terminal::Ghostty,
            Terminal::WezTerm,
            // TODO: Add support for these terminals in future versions
            // Terminal::Kitty,
            // Terminal::Alacritty,
            // Terminal::ITerm,
            // Terminal::TerminalApp,
        ]
    }

    /// Get display name for the terminal
    pub fn display_name(&self) -> &'static str {
        match self {
            Terminal::Ghostty => "Ghostty",
            Terminal::WezTerm => "WezTerm",
            Terminal::Kitty => "Kitty",
            Terminal::Alacritty => "Alacritty",
            Terminal::ITerm => "iTerm2",
            Terminal::TerminalApp => "Terminal.app",
        }
    }

    /// Get the config name for the terminal
    pub fn config_name(&self) -> &'static str {
        match self {
            Terminal::Ghostty => "ghostty",
            Terminal::WezTerm => "wezterm",
            Terminal::Kitty => "kitty",
            Terminal::Alacritty => "alacritty",
            Terminal::ITerm => "iterm",
            Terminal::TerminalApp => "terminal",
        }
    }

    /// Check if the terminal is installed
    pub fn is_installed(&self) -> bool {
        match self {
            Terminal::Ghostty => Path::new("/Applications/Ghostty.app").exists(),
            Terminal::WezTerm => Path::new("/Applications/WezTerm.app").exists(),
            Terminal::Kitty => Path::new("/Applications/kitty.app").exists(),
            Terminal::Alacritty => Path::new("/Applications/Alacritty.app").exists(),
            Terminal::ITerm => Path::new("/Applications/iTerm.app").exists(),
            Terminal::TerminalApp => Path::new("/System/Applications/Utilities/Terminal.app").exists(),
        }
    }

    /// Check if this terminal requires file polling to detect completion
    /// (Some terminals launched via `open` can't be waited on directly)
    pub fn needs_polling(&self) -> bool {
        matches!(self, Terminal::Ghostty | Terminal::ITerm | Terminal::TerminalApp)
    }

    /// Launch the terminal with helix editing the given file
    pub fn launch(&self, file_path: &Path, width: u32, height: u32) -> Result<Child> {
        let file_str = file_path.to_string_lossy();

        // Find helix binary (full path needed when running from .app bundle)
        let hx_path = find_helix()
            .ok_or_else(|| anyhow::anyhow!("Helix editor (hx) not found. Install with: brew install helix"))?;
        let hx_str = hx_path.to_string_lossy();

        match self {
            Terminal::Ghostty => {
                // On macOS, Ghostty doesn't support -e properly via `open --args`
                // Create a temporary shell script and tell Ghostty to run it
                let script_content = format!("#!/bin/bash\n\"{}\" \"{}\"\n", hx_str, file_str);
                let script_path = file_path.with_extension("sh");
                std::fs::write(&script_path, &script_content)
                    .map_err(|e| anyhow::anyhow!("Failed to create script: {}", e))?;

                // Make script executable
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let mut perms = std::fs::metadata(&script_path)
                        .map_err(|e| anyhow::anyhow!("Failed to get script metadata: {}", e))?
                        .permissions();
                    perms.set_mode(0o755);
                    std::fs::set_permissions(&script_path, perms)
                        .map_err(|e| anyhow::anyhow!("Failed to set script permissions: {}", e))?;
                }

                // Launch Ghostty with the script
                Command::new("open")
                    .arg("-na")
                    .arg("/Applications/Ghostty.app")
                    .arg("--args")
                    .arg("-e")
                    .arg(script_path.to_string_lossy().as_ref())
                    .spawn()
                    .map_err(|e| anyhow::anyhow!("Failed to launch Ghostty: {}", e))
            }
            Terminal::WezTerm => {
                // Use the CLI from within the .app bundle
                let wezterm_cli = "/Applications/WezTerm.app/Contents/MacOS/wezterm";

                // --always-new-process ensures we can wait for it to finish
                let child = Command::new(wezterm_cli)
                    .arg("start")
                    .arg("--always-new-process")
                    .arg("--")
                    .arg(hx_str.as_ref())
                    .arg(file_str.as_ref())
                    .spawn()
                    .map_err(|e| anyhow::anyhow!("Failed to launch WezTerm: {}", e))?;

                // Bring WezTerm to front using AppleScript
                std::thread::sleep(std::time::Duration::from_millis(200));
                let _ = Command::new("osascript")
                    .arg("-e")
                    .arg("tell application \"WezTerm\" to activate")
                    .spawn();

                Ok(child)
            }
            Terminal::Kitty => {
                // Use the CLI from within the .app bundle
                let kitty_cli = "/Applications/kitty.app/Contents/MacOS/kitty";

                Command::new(kitty_cli)
                    .arg("--override")
                    .arg(format!("initial_window_width={}c", width))
                    .arg("--override")
                    .arg(format!("initial_window_height={}c", height))
                    .arg(hx_str.as_ref())
                    .arg(file_str.as_ref())
                    .spawn()
                    .map_err(|e| anyhow::anyhow!("Failed to launch Kitty: {}", e))
            }
            Terminal::Alacritty => {
                // Use the CLI from within the .app bundle
                let alacritty_cli = "/Applications/Alacritty.app/Contents/MacOS/alacritty";

                Command::new(alacritty_cli)
                    .arg("-o")
                    .arg(format!("window.dimensions.columns={}", width))
                    .arg("-o")
                    .arg(format!("window.dimensions.lines={}", height))
                    .arg("-e")
                    .arg(hx_str.as_ref())
                    .arg(file_str.as_ref())
                    .spawn()
                    .map_err(|e| anyhow::anyhow!("Failed to launch Alacritty: {}", e))
            }
            Terminal::ITerm => {
                // Use AppleScript to launch iTerm with full path to hx
                let script = format!(
                    r#"
                    tell application "iTerm"
                        activate
                        create window with default profile command "{} {}"
                    end tell
                    "#,
                    hx_str.replace("\"", "\\\""),
                    file_str.replace("\"", "\\\"")
                );
                Command::new("osascript")
                    .arg("-e")
                    .arg(&script)
                    .spawn()
                    .map_err(|e| anyhow::anyhow!("Failed to launch iTerm: {}", e))
            }
            Terminal::TerminalApp => {
                // Use AppleScript to launch Terminal.app with full path to hx
                let script = format!(
                    r#"
                    tell application "Terminal"
                        activate
                        do script "{} {}; exit"
                    end tell
                    "#,
                    hx_str.replace("\"", "\\\""),
                    file_str.replace("\"", "\\\"")
                );
                Command::new("osascript")
                    .arg("-e")
                    .arg(&script)
                    .spawn()
                    .map_err(|e| anyhow::anyhow!("Failed to launch Terminal.app: {}", e))
            }
        }
    }
}

/// Find the helix editor binary in common locations
pub fn find_helix() -> Option<std::path::PathBuf> {
    let common_paths = [
        "/opt/homebrew/bin/hx",           // Homebrew on Apple Silicon
        "/usr/local/bin/hx",              // Homebrew on Intel
        &format!("{}/.cargo/bin/hx", std::env::var("HOME").unwrap_or_default()), // Cargo install
        "/usr/bin/hx",                    // System install
    ];

    for path in &common_paths {
        let p = std::path::PathBuf::from(path);
        if p.exists() {
            return Some(p);
        }
    }

    // Fallback: try PATH (works when run from terminal)
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .filter_map(|dir| {
                let full_path = dir.join("hx");
                if full_path.is_file() {
                    Some(full_path)
                } else {
                    None
                }
            })
            .next()
    })
}

/// Get list of installed terminals
#[allow(dead_code)]
pub fn get_installed_terminals() -> Vec<Terminal> {
    Terminal::all()
        .into_iter()
        .filter(|t| t.is_installed())
        .collect()
}
