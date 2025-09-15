use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use serde::{Deserialize, Serialize};
use tokio::process::Command as AsyncCommand;
use std::sync::{Arc, Mutex};
use chrono::{DateTime, Utc};

#[derive(Debug)]
pub struct GameLauncher {
    pub running_games: Arc<Mutex<HashMap<String, RunningGame>>>,
    pub config: crate::config::Config,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunningGame {
    pub game_id: String,
    pub game_name: String,
    pub pid: Option<u32>,
    pub start_time: DateTime<Utc>,
    pub wine_prefix: Option<PathBuf>,
    pub proton_version: Option<String>,
    pub launcher_type: LauncherType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LauncherType {
    Native,
    Wine,
    Steam,
    Proton,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchOptions {
    pub wine_version: Option<String>,
    pub wine_prefix: Option<PathBuf>,
    pub environment_variables: HashMap<String, String>,
    pub launch_arguments: Vec<String>,
    pub working_directory: Option<PathBuf>,
    pub enable_dxvk: bool,
    pub enable_vkd3d: bool,
    pub enable_gamemode: bool,
    pub enable_mangohud: bool,
    pub enable_gamescope: bool,
    pub gamescope_options: Option<String>,
    pub nvidia_prime: bool,
    pub amd_prime: Option<u8>,
    pub cpu_affinity: Option<Vec<u32>>,
    pub nice_level: Option<i8>,
    pub pre_launch_script: Option<String>,
    pub post_launch_script: Option<String>,
}

impl Default for LaunchOptions {
    fn default() -> Self {
        Self {
            wine_version: None,
            wine_prefix: None,
            environment_variables: HashMap::new(),
            launch_arguments: Vec::new(),
            working_directory: None,
            enable_dxvk: true,
            enable_vkd3d: false,
            enable_gamemode: true,
            enable_mangohud: false,
            enable_gamescope: false,
            gamescope_options: None,
            nvidia_prime: false,
            amd_prime: None,
            cpu_affinity: None,
            nice_level: None,
            pre_launch_script: None,
            post_launch_script: None,
        }
    }
}

impl GameLauncher {
    pub fn new(config: crate::config::Config) -> Self {
        Self {
            running_games: Arc::new(Mutex::new(HashMap::new())),
            config,
        }
    }

    /// Launch a game with comprehensive environment setup
    pub async fn launch_game(&self, game: &crate::game::Game, options: LaunchOptions) -> Result<u32> {
        println!("🚀 Launching {}...", game.name);

        // Pre-launch script
        if let Some(script) = &options.pre_launch_script {
            self.run_script(script, "pre-launch").await?;
        }

        // Determine launcher type
        let launcher_type = self.determine_launcher_type(game, &options);

        // Build the launch command
        let mut cmd = match launcher_type {
            LauncherType::Native => self.build_native_command(game, &options)?,
            LauncherType::Wine => self.build_wine_command(game, &options).await?,
            LauncherType::Proton => self.build_proton_command(game, &options).await?,
            LauncherType::Steam => self.build_steam_command(game, &options)?,
            LauncherType::Custom => self.build_custom_command(game, &options)?,
        };

        // Set environment variables
        for (key, value) in &options.environment_variables {
            cmd.env(key, value);
        }

        // Set working directory
        if let Some(work_dir) = &options.working_directory {
            cmd.current_dir(work_dir);
        } else {
            cmd.current_dir(&game.install_path);
        }

        // Launch with proper I/O handling
        cmd.stdin(Stdio::null())
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());

        // Execute the command
        let mut child = cmd.spawn()?;
        let pid = child.id().unwrap_or(0);

        println!("✅ {} launched with PID {}", game.name, pid);

        // Register the running game
        let running_game = RunningGame {
            game_id: game.id.clone(),
            game_name: game.name.clone(),
            pid: Some(pid),
            start_time: Utc::now(),
            wine_prefix: options.wine_prefix.clone(),
            proton_version: options.wine_version.clone(),
            launcher_type,
        };

        {
            let mut running_games = self.running_games.lock().unwrap();
            running_games.insert(game.id.clone(), running_game);
        }

        // Monitor the game in the background
        let game_id = game.id.clone();
        let running_games_clone = Arc::clone(&self.running_games);
        let post_launch_script = options.post_launch_script.clone();

        tokio::spawn(async move {
            match child.wait().await {
                Ok(status) => {
                    let exit_code = status.code().unwrap_or(-1);
                    println!("🎮 Game {} exited with code {}", game_id, exit_code);

                    // Remove from running games
                    {
                        let mut running_games = running_games_clone.lock().unwrap();
                        running_games.remove(&game_id);
                    }

                    // Post-launch script
                    if let Some(script) = post_launch_script {
                        if let Err(e) = Self::run_script_blocking(&script, "post-launch") {
                            eprintln!("⚠️ Post-launch script failed: {}", e);
                        }
                    }
                },
                Err(e) => {
                    eprintln!("❌ Game {} process error: {}", game_id, e);
                    let mut running_games = running_games_clone.lock().unwrap();
                    running_games.remove(&game_id);
                }
            }
        });

        Ok(pid)
    }

    fn determine_launcher_type(&self, game: &crate::game::Game, options: &LaunchOptions) -> LauncherType {
        if game.launcher.as_deref() == Some("Steam") {
            LauncherType::Steam
        } else if options.wine_version.is_some() || game.wine_version.is_some() {
            if options.wine_version.as_ref().or(game.wine_version.as_ref())
                .map(|v| v.contains("Proton"))
                .unwrap_or(false) {
                LauncherType::Proton
            } else {
                LauncherType::Wine
            }
        } else if game.executable.to_str().unwrap_or("").ends_with(".exe") {
            LauncherType::Wine
        } else {
            LauncherType::Native
        }
    }

    fn build_native_command(&self, game: &crate::game::Game, options: &LaunchOptions) -> Result<AsyncCommand> {
        let mut cmd = AsyncCommand::new(&game.executable);

        // Add game arguments
        for arg in &game.launch_arguments {
            cmd.arg(arg);
        }

        // Add additional arguments
        for arg in &options.launch_arguments {
            cmd.arg(arg);
        }

        // Wrap with performance tools if enabled
        self.wrap_with_performance_tools(&mut cmd, options)?;

        Ok(cmd)
    }

    async fn build_wine_command(&self, game: &crate::game::Game, options: &LaunchOptions) -> Result<AsyncCommand> {
        let wine_version = options.wine_version.as_ref()
            .or(game.wine_version.as_ref())
            .cloned()
            .unwrap_or_else(|| "wine".to_string());

        // Find Wine binary
        let wine_bin = if wine_version == "wine" {
            PathBuf::from("wine")
        } else {
            self.find_wine_binary(&wine_version).await?
        };

        let mut cmd = AsyncCommand::new(wine_bin);

        // Set Wine prefix
        let prefix = options.wine_prefix.as_ref()
            .or(game.wine_prefix.as_ref())
            .cloned()
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .unwrap_or_default()
                    .join("Games")
                    .join(&game.name)
            });

        cmd.env("WINEPREFIX", &prefix);
        cmd.env("WINEARCH", "win64");

        // Enable DXVK if requested
        if options.enable_dxvk {
            cmd.env("DXVK_ASYNC", "1");
            cmd.env("DXVK_HUD", "compiler");
        }

        // Enable VKD3D if requested
        if options.enable_vkd3d {
            cmd.env("VKD3D_DEBUG", "warn");
        }

        // Add Wine-specific environment variables
        cmd.env("WINEDLLOVERRIDES", "winemenubuilder.exe=d");

        // Add the executable and arguments
        cmd.arg(&game.executable);
        for arg in &game.launch_arguments {
            cmd.arg(arg);
        }
        for arg in &options.launch_arguments {
            cmd.arg(arg);
        }

        // Wrap with performance tools
        self.wrap_with_performance_tools(&mut cmd, options)?;

        Ok(cmd)
    }

    async fn build_proton_command(&self, game: &crate::game::Game, options: &LaunchOptions) -> Result<AsyncCommand> {
        let proton_version = options.wine_version.as_ref()
            .or(game.wine_version.as_ref())
            .cloned()
            .unwrap_or_else(|| "GE-Proton".to_string());

        // Find Proton binary
        let proton_bin = self.find_proton_binary(&proton_version).await?;

        let mut cmd = AsyncCommand::new(proton_bin);

        // Set Steam compatibility data
        let prefix = options.wine_prefix.as_ref()
            .or(game.wine_prefix.as_ref())
            .cloned()
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .unwrap_or_default()
                    .join("Games")
                    .join(&game.name)
            });

        cmd.env("STEAM_COMPAT_DATA_PATH", &prefix);
        cmd.env("STEAM_COMPAT_CLIENT_INSTALL_PATH", "/home/user/.steam");

        // Proton-specific environment variables
        if options.enable_dxvk {
            cmd.env("PROTON_USE_DXVK", "1");
        }

        if options.enable_vkd3d {
            cmd.env("PROTON_USE_VKD3D", "1");
        }

        // Enable NVIDIA features if available
        if options.nvidia_prime {
            cmd.env("__NV_PRIME_RENDER_OFFLOAD", "1");
            cmd.env("__GLX_VENDOR_LIBRARY_NAME", "nvidia");
        }

        cmd.arg("run");
        cmd.arg(&game.executable);

        for arg in &game.launch_arguments {
            cmd.arg(arg);
        }
        for arg in &options.launch_arguments {
            cmd.arg(arg);
        }

        // Wrap with performance tools
        self.wrap_with_performance_tools(&mut cmd, options)?;

        Ok(cmd)
    }

    fn build_steam_command(&self, game: &crate::game::Game, _options: &LaunchOptions) -> Result<AsyncCommand> {
        let mut cmd = AsyncCommand::new("steam");

        if let Some(launcher_id) = &game.launcher_id {
            cmd.arg("-applaunch").arg(launcher_id);
        } else {
            return Err(anyhow::anyhow!("Steam game missing app ID"));
        }

        Ok(cmd)
    }

    fn build_custom_command(&self, game: &crate::game::Game, options: &LaunchOptions) -> Result<AsyncCommand> {
        // Use the executable directly
        let mut cmd = AsyncCommand::new(&game.executable);

        for arg in &game.launch_arguments {
            cmd.arg(arg);
        }
        for arg in &options.launch_arguments {
            cmd.arg(arg);
        }

        self.wrap_with_performance_tools(&mut cmd, options)?;

        Ok(cmd)
    }

    fn wrap_with_performance_tools(&self, cmd: &mut AsyncCommand, options: &LaunchOptions) -> Result<()> {
        // Build performance wrapper command
        let mut wrapper_parts = Vec::new();

        // CPU affinity
        if let Some(affinity) = &options.cpu_affinity {
            wrapper_parts.push("taskset".to_string());
            wrapper_parts.push("-c".to_string());
            wrapper_parts.push(affinity.iter().map(|cpu| cpu.to_string()).collect::<Vec<_>>().join(","));
        }

        // Nice level
        if let Some(nice) = options.nice_level {
            wrapper_parts.push("nice".to_string());
            wrapper_parts.push("-n".to_string());
            wrapper_parts.push(nice.to_string());
        }

        // GameMode
        if options.enable_gamemode && which::which("gamemoderun").is_ok() {
            wrapper_parts.push("gamemoderun".to_string());
        }

        // MangoHud
        if options.enable_mangohud && which::which("mangohud").is_ok() {
            wrapper_parts.push("mangohud".to_string());
        }

        // GameScope
        if options.enable_gamescope {
            wrapper_parts.push("gamescope".to_string());
            if let Some(gamescope_opts) = &options.gamescope_options {
                for opt in gamescope_opts.split_whitespace() {
                    wrapper_parts.push(opt.to_string());
                }
            } else {
                // Default GameScope options for better gaming
                wrapper_parts.extend([
                    "-W", "1920", "-H", "1080",
                    "-f", "--force-grab-cursor"
                ].iter().map(|s| s.to_string()));
            }
        }

        // NVIDIA Prime
        if options.nvidia_prime {
            cmd.env("__NV_PRIME_RENDER_OFFLOAD", "1");
            cmd.env("__GLX_VENDOR_LIBRARY_NAME", "nvidia");
        }

        // AMD Prime
        if let Some(gpu_id) = options.amd_prime {
            cmd.env("DRI_PRIME", gpu_id.to_string());
        }

        // If we have wrapper commands, we need to restructure the command
        if !wrapper_parts.is_empty() {
            // This is complex to implement with tokio::process::Command
            // For now, we'll set environment variables to enable the features
            // In a real implementation, you'd want to use a shell wrapper

            if options.enable_gamemode {
                cmd.env("LD_PRELOAD", "libgamemodeauto.so.0");
            }
        }

        Ok(())
    }

    async fn find_wine_binary(&self, wine_version: &str) -> Result<PathBuf> {
        // Check system wine first
        if wine_version == "wine" || wine_version == "system" {
            return Ok(PathBuf::from("wine"));
        }

        // Check in wine versions directory
        let wine_path = self.config.wine.wine_versions_path.join(wine_version).join("bin/wine");
        if wine_path.exists() {
            return Ok(wine_path);
        }

        // Fallback to system wine
        Ok(PathBuf::from("wine"))
    }

    async fn find_proton_binary(&self, proton_version: &str) -> Result<PathBuf> {
        // Check in Proton directory
        let proton_path = self.config.wine.wine_versions_path.join(proton_version).join("proton");
        if proton_path.exists() {
            return Ok(proton_path);
        }

        // Check Steam Proton locations
        let steam_locations = [
            dirs::home_dir().unwrap().join(".steam/steam/steamapps/common"),
            dirs::home_dir().unwrap().join(".local/share/Steam/steamapps/common"),
            PathBuf::from("/usr/share/steam/compatibilitytools.d"),
        ];

        for location in steam_locations {
            for entry in std::fs::read_dir(&location).unwrap_or_else(|_| std::fs::read_dir("/tmp").unwrap()) {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    let name = path.file_name().unwrap().to_str().unwrap();
                    if name.contains("Proton") || name.contains("proton") {
                        let proton_bin = path.join("proton");
                        if proton_bin.exists() {
                            return Ok(proton_bin);
                        }
                    }
                }
            }
        }

        Err(anyhow::anyhow!("Proton binary not found"))
    }

    pub fn get_running_games(&self) -> HashMap<String, RunningGame> {
        self.running_games.lock().unwrap().clone()
    }

    pub async fn stop_game(&self, game_id: &str) -> Result<()> {
        let running_game = {
            self.running_games.lock().unwrap().get(game_id).cloned()
        };

        if let Some(running_game) = running_game {
            if let Some(pid) = running_game.pid {
                println!("🛑 Stopping {}...", running_game.game_name);

                // Try graceful shutdown first
                if let Ok(_) = nix::sys::signal::kill(
                    nix::unistd::Pid::from_raw(pid as i32),
                    nix::sys::signal::Signal::SIGTERM
                ) {
                    // Wait a bit for graceful shutdown
                    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                }

                // Force kill if still running
                let _ = nix::sys::signal::kill(
                    nix::unistd::Pid::from_raw(pid as i32),
                    nix::sys::signal::Signal::SIGKILL
                );

                // Remove from running games
                self.running_games.lock().unwrap().remove(game_id);
                println!("✅ {} stopped", running_game.game_name);
            }
        }

        Ok(())
    }

    async fn run_script(&self, script: &str, script_type: &str) -> Result<()> {
        println!("📜 Running {} script...", script_type);

        let mut cmd = AsyncCommand::new("bash");
        cmd.arg("-c").arg(script);

        let status = cmd.status().await?;

        if status.success() {
            println!("✅ {} script completed successfully", script_type);
        } else {
            println!("⚠️ {} script exited with code {:?}", script_type, status.code());
        }

        Ok(())
    }

    fn run_script_blocking(script: &str, script_type: &str) -> Result<()> {
        println!("📜 Running {} script...", script_type);

        let status = Command::new("bash")
            .arg("-c")
            .arg(script)
            .status()?;

        if status.success() {
            println!("✅ {} script completed successfully", script_type);
        } else {
            println!("⚠️ {} script exited with code {:?}", script_type, status.code());
        }

        Ok(())
    }

    /// Update playtime for a game when it stops
    pub async fn update_game_playtime(&self, game_id: &str, game_lib: &crate::game::GameLibrary) -> Result<()> {
        if let Some(running_game) = self.running_games.lock().unwrap().get(game_id) {
            let playtime_minutes = Utc::now()
                .signed_duration_since(running_game.start_time)
                .num_minutes() as u64;

            if playtime_minutes > 0 {
                game_lib.update_playtime(game_id, playtime_minutes)?;
                println!("📊 Updated playtime: {} minutes", playtime_minutes);
            }
        }

        Ok(())
    }
}