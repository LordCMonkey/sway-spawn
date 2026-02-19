use clap::Parser;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::process::{Command, Output};
use toml;

/// Sway scratchpad manager - toggle applications between scratchpad and focus
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Application name to toggle (fish, python, julia, conda, keepassxc, obsidian)
    #[arg(value_name = "APP")]
    app_name: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct SwayWindow {
    #[serde(rename = "name")]
    title: Option<String>,
    app_id: Option<String>,
    focused: bool,
    window_properties: Option<WindowProperties>,
    #[serde(rename = "type")]
    window_type: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct WindowProperties {
    class: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
enum WindowIdentifier {
    Title(String),
    AppId(String),
    Class(String),
}

/// match against all WindowIdentifier wether the window fits the identifier
fn matches_identifier(w: &SwayWindow, identifier: &WindowIdentifier) -> bool {
    match identifier {
        WindowIdentifier::Title(title) => w
            .title
            .as_ref()
            .map(|t| t.eq_ignore_ascii_case(title))
            .unwrap_or(false),
        WindowIdentifier::AppId(app_id) => w
            .app_id
            .as_ref()
            .map(|a| a.eq_ignore_ascii_case(app_id))
            .unwrap_or(false),
        WindowIdentifier::Class(class) => w.window_properties.as_ref().map_or(false, |wp| {
            wp.class
                .as_ref()
                .map(|c| c.eq_ignore_ascii_case(class))
                .unwrap_or(false)
        }),
    }
}

#[derive(Debug, Clone, Deserialize)]
struct AppConfig {
    /// The command to execute
    command: String,
    /// Whether this is a terminal application
    is_terminal: bool,
    /// window identifier
    identifier: WindowIdentifier,
    /// Optional custom startup command override
    startup_override: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Spawn {
    terminal: String,
    apps: HashMap<String, AppConfig>,
}

impl Spawn {
    fn new() -> Self {
        let mut config_path = env::home_dir().unwrap();
        config_path.push(".config/spawn/spawn.toml");
        if config_path.is_file() {
            let contents = std::fs::read(config_path).unwrap();
            let spawn = toml::from_slice::<Spawn>(&contents).unwrap();
            return spawn;
        } else {
            eprintln!("Config at '{:?}' is not a file", config_path);
        }

        Self {
            terminal: "".to_string(),
            apps: HashMap::new(),
        }
    }

    /// Execute swaymsg command and return output
    fn swaymsg(&self, args: &[&str]) -> Result<Output, std::io::Error> {
        Command::new("swaymsg").args(args).output()
    }

    /// Get all windows from sway tree
    fn get_windows(&self) -> Result<Vec<SwayWindow>, Box<dyn std::error::Error>> {
        let output = self.swaymsg(&["-t", "get_tree"])?;
        let tree: serde_json::Value = serde_json::from_slice(&output.stdout)?;

        let mut windows = Vec::new();
        self.extract_windows(&tree, &mut windows);

        Ok(windows)
    }

    /// Recursively extract windows from sway tree
    fn extract_windows(&self, node: &serde_json::Value, windows: &mut Vec<SwayWindow>) {
        if let Some(window_type) = node.get("type").and_then(|t| t.as_str()) {
            if window_type == "floating_con" || window_type == "con" {
                if let Ok(window) = serde_json::from_value::<SwayWindow>(node.clone()) {
                    windows.push(window);
                }
            }
        }

        // Recurse into child nodes
        if let Some(nodes) = node.get("nodes").and_then(|n| n.as_array()) {
            for child in nodes {
                self.extract_windows(child, windows);
            }
        }

        // Recurse into floating nodes
        if let Some(floating) = node.get("floating_nodes").and_then(|n| n.as_array()) {
            for child in floating {
                self.extract_windows(child, windows);
            }
        }
    }

    fn is_running(&self, windows: &[SwayWindow], identifier: &WindowIdentifier) -> bool {
        windows.iter().any(|w| matches_identifier(w, identifier))
    }

    fn is_focused(&self, windows: &[SwayWindow], identifier: &WindowIdentifier) -> bool {
        windows.iter().any(|w| {
            if !w.focused {
                return false;
            }

            matches_identifier(w, identifier)
        })
    }

    /// Build the startup command for an application
    fn build_startup_command(&self, config: &AppConfig) -> String {
        // Use startup override if specified
        if let Some(ref override_cmd) = config.startup_override {
            return override_cmd.clone();
        }

        if config.is_terminal {
            if let WindowIdentifier::Title(ref title) = config.identifier {
                return format!(
                    "{} --title {} --command {}",
                    self.terminal, title, config.command
                );
            }
        }

        // Return command as-is for GUI applications
        config.command.clone()
    }

    /// Build sway criteria string for window selection
    fn build_criteria(&self, identifier: &WindowIdentifier) -> String {
        match identifier {
            WindowIdentifier::Title(title) => format!("[title=\"{}\"]", title),
            WindowIdentifier::AppId(app_id) => format!("[app_id=\"{}\"]", app_id),
            WindowIdentifier::Class(class) => format!("[class=\"{}\"]", class),
        }
    }

    fn focus_window(
        &self,
        identifier: &WindowIdentifier,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let criteria = self.build_criteria(identifier);
        let command = format!("{} focus", criteria);
        self.swaymsg(&[&command])?;
        Ok(())
    }

    fn move_to_scratchpad(
        &self,
        identifier: &WindowIdentifier,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let criteria = self.build_criteria(identifier);
        let command = format!("{} move scratchpad", criteria);
        self.swaymsg(&[&command])?;
        Ok(())
    }

    /// Main logic to handle window toggling
    fn handle_window(&self, app_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let config = self
            .apps
            .get(app_name)
            .ok_or_else(|| format!("Unknown application: {}", app_name))?;

        let windows = self.get_windows()?;

        if self.is_running(&windows, &config.identifier) {
            if self.is_focused(&windows, &config.identifier) {
                // Window is focused -> move to scratchpad
                self.move_to_scratchpad(&config.identifier)?;
            } else {
                // Window exists but not focused -> bring to focus
                self.focus_window(&config.identifier)?;
            }
        } else {
            // Window doesn't exist -> start it
            let cmd = self.build_startup_command(config);
            self.swaymsg(&["exec", &cmd])?;
        }

        Ok(())
    }
}

fn main() {
    let cli = Cli::parse();
    let spawn = Spawn::new();

    if let Err(e) = spawn.handle_window(&cli.app_name) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
