
    use log::{info, error};
    use std::collections::BTreeMap;
    use std::env;
    use std::path::Path;
    use std::process::Command;

    // Struct to hold pertinent information for each system monitor candidate app
    #[derive(Debug, Clone)]
    pub struct DesktopApp {
        pub name: String,
        exec: String,
        categories: Vec<String>,
        keywords: Vec<String>,
        desktop_file_path: String,
    }

    #[derive(Debug, PartialEq, Clone, Copy)]
    enum RuntimeEnvironment {
        Flatpak,
        Native,
    }

    fn detect_runtime_environment() -> RuntimeEnvironment {
        match env::var("FLATPAK_ID") {
            Ok(_) => RuntimeEnvironment::Flatpak,
            Err(_) => RuntimeEnvironment::Native,
        }
    }

    fn get_desktop_files_in_dir(
        environment: RuntimeEnvironment,
        dir: &str,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        match environment {
            RuntimeEnvironment::Native => {
                if !Path::new(dir).exists() {
                    return Ok(Vec::new());
                }

                let output = Command::new("find")
                    .arg(dir)
                    .arg("-name")
                    .arg("*.desktop")
                    .output()?;

                if !output.status.success() {
                    return Ok(Vec::new());
                }

                let stdout = String::from_utf8_lossy(&output.stdout);

                Ok(stdout.lines().map(|s| s.to_string()).collect())
            }
            RuntimeEnvironment::Flatpak => {
                // Use flatpak-spawn --host for access to host filesystem
                let output = Command::new("flatpak-spawn")
                    .arg("--host")
                    .arg("find")
                    .arg(dir)
                    .arg("-name")
                    .arg("*.desktop")
                    .output()?;

                if !output.status.success() {
                    return Ok(Vec::new());
                }

                let stdout = String::from_utf8_lossy(&output.stdout);
                Ok(stdout.lines().map(|s| s.to_string()).collect())
            }
        }
    }

    pub fn get_desktop_applications() -> BTreeMap<String, DesktopApp> {
        let desktop_dirs = vec![
            "/usr/local/share/applications".to_string(),
            format!(
                "{}/.local/share/applications",
                env::var("HOME").unwrap_or_default()
            ),
            "/var/lib/flatpak/exports/share/applications".to_string(),
            format!(
                "{}/.local/share/flatpak/exports/share/applications",
                env::var("HOME").unwrap_or_default()
            ),
            "/usr/share/applications".to_string(), // Last priority
        ];

        // Use BTreeMap to avoid duplicates.
        // If a file is found first in .local it won't be overwritten by /usr/..
        let mut candidates = BTreeMap::new();
        let environment = detect_runtime_environment();
        for dir in desktop_dirs {
            if let Ok(entries) = get_desktop_files_in_dir(environment, &dir) {
                for entry in entries {
                    match parse_desktop_file(environment, &entry) {
                        Ok(o) => _ = candidates.entry(o.name.clone()).or_insert(o),
                        Err(_e) => (), //info!("Error: {}", e),
                    };
                }
            }
        }

        for c in &candidates {
            info!("Found System Monitor: {} {}", c.1.name, c.1.desktop_file_path);
        }

        candidates
    }

    // Parse a .desktop file using appropriate access method
    fn parse_desktop_file(
        environment: RuntimeEnvironment,
        file_path: &str,
    ) -> Result<DesktopApp, Box<dyn std::error::Error>> {
        let content = match environment {
            RuntimeEnvironment::Native => std::fs::read_to_string(file_path)?,
            RuntimeEnvironment::Flatpak => {
                let output = Command::new("flatpak-spawn")
                    .arg("--host")
                    .arg("cat")
                    .arg(file_path)
                    .output()?;

                if !output.status.success() {
                    return Err("Failed to read desktop file".into());
                }

                String::from_utf8_lossy(&output.stdout).to_string()
            }
        };

        let mut name = String::new();
        let mut exec = String::new();
        let mut categories = Vec::new();
        let mut keywords = Vec::new();
        let mut in_desktop_entry = false;

        for line in content.lines() {
            let line = line.trim();

            if line == "[Desktop Entry]" {
                in_desktop_entry = true;
                continue;
            }

            if line.starts_with('[') && line != "[Desktop Entry]" {
                in_desktop_entry = false;
                continue;
            }

            if !in_desktop_entry {
                continue;
            }

            if let Some(_value) = line.strip_prefix("X-CosmicApplet=true") {
                return Err("Applet not app".into());
            } else if let Some(value) = line.strip_prefix("Name=") {
                name = value.to_string();
            } else if let Some(value) = line.strip_prefix("Exec=") {
                exec = value.to_string();
            } else if let Some(value) = line.strip_prefix("Categories=") {
                categories = value
                    .split(';')
                    .map(|s| s.trim().to_string().to_lowercase())
                    .filter(|s| !s.is_empty())
                    .collect();
            } else if let Some(value) = line.strip_prefix("Keywords=") {
                keywords = value
                    .split(';')
                    .map(|s| s.trim().to_string().to_lowercase())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
        }

        if name.is_empty() && exec.is_empty() {
            return Err("Invalid desktop file".into());
        }

        let app = DesktopApp {
            name,
            exec,
            categories,
            keywords,
            desktop_file_path: file_path.to_string(),
        };

        if !is_system_monitor_app(&app) {
            return Err("desktop file not a system monitor app".into());
        }

        Ok(app)
    }

    fn command_exists_on_host(cmd: &str, env: RuntimeEnvironment) -> bool {
        let result = match env {
            RuntimeEnvironment::Native => Command::new("which").arg(cmd).output(),
            RuntimeEnvironment::Flatpak => Command::new("flatpak-spawn")
                .args(["--host", "which", cmd])
                .output(),
        };

        result.map(|o| o.status.success()).unwrap_or(false)
    }

    pub fn launch_desktop_app(app: &DesktopApp) {
        fn clean_exec_command(exec: &str) -> Vec<String> {
            exec.split_whitespace()
                .filter(|arg| !arg.starts_with('%')) // Remove placeholders like %u, %f
                .map(|s| s.to_string())
                .collect()
        }

        let env = detect_runtime_environment();

        // Extract the desktop file ID: basename without ".desktop"
        let desktop_file_id = Path::new(&app.desktop_file_path)
            .file_name()
            .and_then(|f| f.to_str())
            .map(|s| s.trim_end_matches(".desktop"))
            .unwrap_or("");

        // Check for gtk4-launch and gtk-launch in that order
        let launcher = if command_exists_on_host("gtk4-launch", env) {
            Some("gtk4-launch")
        } else if command_exists_on_host("gtk-launch", env) {
            Some("gtk-launch")
        } else {
            None
        };

        if let Some(launcher_cmd) = launcher {
            let result = match env {
                RuntimeEnvironment::Native => {
                    Command::new(launcher_cmd).arg(desktop_file_id).spawn()
                }
                RuntimeEnvironment::Flatpak => Command::new("flatpak-spawn")
                    .args(["--host", launcher_cmd, desktop_file_id])
                    .spawn(),
            };

            match result {
                Ok(_) => info!("Launched with {}: {}", launcher_cmd, app.name),
                Err(e) => error!(
                    "Failed to launch '{}' with {}: {}",
                    app.name, launcher_cmd, e
                ),
            }
        } else {
            // Fall back to Exec
            let cmd_parts = clean_exec_command(&app.exec);
            if cmd_parts.is_empty() {
                error!("No valid executable command found.");
                return;
            }

            let (cmd, args) = (&cmd_parts[0], &cmd_parts[1..]);
            let mut command = if env == RuntimeEnvironment::Flatpak {
                let mut c = Command::new("flatpak-spawn");
                c.arg("--host").arg(cmd).args(args);
                c
            } else {
                let mut c = Command::new(cmd);
                c.args(args);
                c
            };

            match command.spawn() {
                Ok(_) => info!("Launched manually: {}", app.name),
                Err(e) => error!("Failed to launch manually '{}': {}", app.name, e),
            }
        }
    }
   
    /// Check if an application is likely a system monitor
    fn is_system_monitor_app(app: &DesktopApp) -> bool {
        let name_lower = app.name.to_lowercase();

        if app.categories.iter().any(|s| s == "system")
            && app.categories.iter().any(|s| s == "monitor")
        {
            return true;
        }

        if app.keywords.iter().any(|s| s == "system") && app.keywords.iter().any(|s| s == "monitor")
        {
            return true;
        }

        // Check name system monitor keywords
        let keywords = vec![
            "system monitor",
            "task manager",
            "activity monitor",
            "htop",
            "btop",
            "observatory",
            "mission center",
            "ksysguard",
        ];

        for keyword in keywords {
            if name_lower.contains(keyword) {
                return true;
            }
        }

        false
    }

