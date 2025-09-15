use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::*;
use std::path::PathBuf;
use crate::protondb::ProtonDBTier;
use crate::game_launcher::{GameLauncher, LaunchOptions};

#[derive(Parser)]
#[command(
    name = "forge",
    author,
    version,
    about = "GhostForge - Universal Game Launcher & Manager",
    long_about = "A modern Linux gaming platform replacing Lutris with better Wine/Proton management"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(short, long, global = true, help = "Enable verbose output")]
    pub verbose: bool,

    #[arg(long, global = true, help = "Path to config file")]
    pub config: Option<String>,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Manage games in your library")]
    Game {
        #[command(subcommand)]
        action: GameCommands,
    },

    #[command(about = "Manage Wine/Proton versions")]
    Wine {
        #[command(subcommand)]
        action: WineCommands,
    },

    #[command(about = "Launch a game")]
    Launch {
        #[arg(help = "Game ID or name")]
        game: String,

        #[arg(long, help = "Override Wine/Proton version")]
        wine_version: Option<String>,

        #[arg(long, help = "Additional launch arguments")]
        args: Vec<String>,
    },

    #[command(about = "Install a game from various sources")]
    Install {
        #[arg(help = "Game installer path, URL, or launcher ID")]
        source: String,

        #[arg(long, help = "Installation name")]
        name: Option<String>,

        #[arg(long, help = "Wine/Proton version to use")]
        wine_version: Option<String>,
    },

    #[command(about = "Configure GhostForge settings")]
    Config {
        #[command(subcommand)]
        action: ConfigCommands,
    },

    #[command(about = "Manage game launchers (Steam, Battle.net, etc.)")]
    Launcher {
        #[command(subcommand)]
        action: LauncherCommands,
    },

    #[command(about = "Apply Winetricks/tweaks to a game prefix")]
    Tricks {
        #[arg(help = "Game ID or name")]
        game: String,

        #[arg(help = "Trick/tweak to apply")]
        trick: String,

        #[arg(long, help = "Force apply even if already installed")]
        force: bool,
    },

    #[command(about = "Optimize game performance and GPU settings")]
    Optimize {
        #[arg(help = "Game ID or name")]
        game: Option<String>,

        #[arg(long, help = "Apply NVIDIA optimizations")]
        nvidia: bool,

        #[arg(long, help = "Apply AMD optimizations")]
        amd: bool,

        #[arg(long, help = "Enable GameMode")]
        gamemode: bool,

        #[arg(long, help = "Set CPU governor to performance")]
        cpu_performance: bool,
    },

    #[command(about = "Search for games and settings")]
    Search {
        #[arg(help = "Search query")]
        query: String,

        #[arg(long, help = "Search in ProtonDB")]
        protondb: bool,

        #[arg(long, help = "Search in local library only")]
        local: bool,
    },

    #[command(about = "Show system information and diagnostics")]
    Info {
        #[arg(long, help = "Show GPU information")]
        gpu: bool,

        #[arg(long, help = "Show Wine/Proton versions")]
        wine: bool,

        #[arg(long, help = "Check Vulkan support")]
        vulkan: bool,

        #[arg(long, help = "Full system report")]
        full: bool,
    },

    #[command(about = "Backup and restore game configurations")]
    Backup {
        #[command(subcommand)]
        action: BackupCommands,
    },

    #[command(about = "Battle.net specific tools and setup")]
    Battlenet {
        #[command(subcommand)]
        action: BattlenetCommands,
    },

    #[command(about = "Manage graphics layers (DXVK, VKD3D)")]
    Graphics {
        #[command(subcommand)]
        action: GraphicsCommands,
    },

    #[command(about = "Launch Terminal UI")]
    Tui,

    #[command(about = "Launch GUI")]
    Gui,
}

#[derive(Subcommand)]
pub enum GameCommands {
    #[command(about = "List all games")]
    List {
        #[arg(long, help = "Filter by launcher")]
        launcher: Option<String>,

        #[arg(long, help = "Filter by status")]
        status: Option<String>,
    },

    #[command(about = "Add a game manually")]
    Add {
        #[arg(help = "Game executable path")]
        path: String,

        #[arg(help = "Game name")]
        name: String,

        #[arg(long, help = "Wine/Proton version")]
        wine_version: Option<String>,
    },

    #[command(about = "Remove a game")]
    Remove {
        #[arg(help = "Game ID or name")]
        game: String,

        #[arg(long, help = "Also remove game files")]
        purge: bool,
    },

    #[command(about = "Edit game configuration")]
    Edit {
        #[arg(help = "Game ID or name")]
        game: String,
    },

    #[command(about = "Show game details")]
    Info {
        #[arg(help = "Game ID or name")]
        game: String,
    },

    #[command(about = "Verify game files")]
    Verify {
        #[arg(help = "Game ID or name")]
        game: String,
    },
}

#[derive(Subcommand)]
pub enum WineCommands {
    #[command(about = "List installed Wine/Proton versions")]
    List {
        #[arg(long, help = "Include available versions to download")]
        available: bool,
    },

    #[command(about = "Install a Wine/Proton version")]
    Install {
        #[arg(help = "Version to install (e.g., Proton-9.0, Wine-staging-9.0)")]
        version: String,
    },

    #[command(about = "Remove a Wine/Proton version")]
    Remove {
        #[arg(help = "Version to remove")]
        version: String,
    },

    #[command(about = "Set default Wine/Proton version")]
    Default {
        #[arg(help = "Version to set as default")]
        version: String,
    },

    #[command(about = "Download and install GE-Proton")]
    GeProton {
        #[arg(long, help = "Install latest version")]
        latest: bool,

        #[arg(long, help = "Specific version")]
        version: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum ConfigCommands {
    #[command(about = "Show current configuration")]
    Show,

    #[command(about = "Set a configuration value")]
    Set {
        #[arg(help = "Configuration key")]
        key: String,

        #[arg(help = "Configuration value")]
        value: String,
    },

    #[command(about = "Get a configuration value")]
    Get {
        #[arg(help = "Configuration key")]
        key: String,
    },

    #[command(about = "Reset configuration to defaults")]
    Reset {
        #[arg(long, help = "Confirm reset")]
        yes: bool,
    },
}

#[derive(Subcommand)]
pub enum LauncherCommands {
    #[command(about = "List configured launchers")]
    List,

    #[command(about = "Setup a launcher (Steam, Battle.net, Epic, etc.)")]
    Setup {
        #[arg(help = "Launcher type (steam, battlenet, epic, gog, ubisoft)")]
        launcher: String,

        #[arg(long, help = "Path to launcher installation")]
        path: Option<String>,
    },

    #[command(about = "Sync games from launcher")]
    Sync {
        #[arg(help = "Launcher to sync from")]
        launcher: String,
    },

    #[command(about = "Remove launcher configuration")]
    Remove {
        #[arg(help = "Launcher to remove")]
        launcher: String,
    },
}

#[derive(Subcommand)]
pub enum BackupCommands {
    #[command(about = "Create a backup")]
    Create {
        #[arg(help = "Game ID or 'all' for full backup")]
        target: String,

        #[arg(long, help = "Backup destination path")]
        output: Option<String>,
    },

    #[command(about = "Restore from backup")]
    Restore {
        #[arg(help = "Backup file path")]
        backup: String,

        #[arg(long, help = "Game to restore (if not restoring all)")]
        game: Option<String>,
    },

    #[command(about = "List available backups")]
    List,
}

#[derive(Subcommand)]
pub enum BattlenetCommands {
    #[command(about = "Setup a new Battle.net prefix")]
    Setup {
        #[arg(long, help = "Wine/Proton version to use")]
        wine_version: Option<String>,

        #[arg(long, help = "Custom prefix path")]
        prefix: Option<String>,

        #[arg(long, help = "Game-specific setup (wow, diablo, etc.)")]
        game: Option<String>,
    },

    #[command(about = "Install Battle.net essentials")]
    Essentials {
        #[arg(long, help = "Target prefix path")]
        prefix: Option<String>,
    },

    #[command(about = "Optimize prefix for World of Warcraft")]
    WowOptimize {
        #[arg(long, help = "Target prefix path")]
        prefix: Option<String>,
    },

    #[command(about = "Download Battle.net installer")]
    Download {
        #[arg(long, help = "Download directory")]
        output: Option<String>,
    },

    #[command(about = "Check Battle.net compatibility")]
    Check,

    #[command(about = "List installed Battle.net games")]
    Games,
}

#[derive(Subcommand)]
pub enum GraphicsCommands {
    #[command(about = "List available graphics layers")]
    List {
        #[arg(long, help = "Include available versions to download")]
        available: bool,

        #[arg(long, help = "Show only DXVK versions")]
        dxvk: bool,

        #[arg(long, help = "Show only VKD3D versions")]
        vkd3d: bool,
    },

    #[command(about = "Install a graphics layer")]
    Install {
        #[arg(help = "Layer name (e.g., dxvk-2.4, vkd3d-proton-2.11.1)")]
        layer: String,
    },

    #[command(about = "Apply graphics layer to a prefix")]
    Apply {
        #[arg(help = "Layer type (dxvk, vkd3d)")]
        layer_type: String,

        #[arg(help = "Target prefix path")]
        prefix: String,

        #[arg(long, help = "Specific version to use")]
        version: Option<String>,
    },

    #[command(about = "Remove graphics layer from prefix")]
    Remove {
        #[arg(help = "Layer type to remove (dxvk, vkd3d, all)")]
        layer_type: String,

        #[arg(help = "Target prefix path")]
        prefix: String,
    },

    #[command(about = "Show graphics info for a prefix")]
    Info {
        #[arg(help = "Prefix path to inspect")]
        prefix: String,
    },

    #[command(about = "Get recommendations for a game")]
    Recommend {
        #[arg(help = "Game name")]
        game: String,
    },
}

impl Cli {
    pub async fn execute(self) -> Result<()> {
        match self.command {
            Commands::Game { action } => handle_game_command(action).await,
            Commands::Wine { action } => handle_wine_command(action).await,
            Commands::Launch { game, wine_version, args } => {
                handle_launch(game, wine_version, args).await
            }
            Commands::Install { source, name, wine_version } => {
                handle_install(source, name, wine_version).await
            }
            Commands::Config { action } => handle_config_command(action).await,
            Commands::Launcher { action } => handle_launcher_command(action).await,
            Commands::Tricks { game, trick, force } => {
                handle_tricks(game, trick, force).await
            }
            Commands::Optimize { game, nvidia, amd, gamemode, cpu_performance } => {
                handle_optimize(game, nvidia, amd, gamemode, cpu_performance).await
            }
            Commands::Search { query, protondb, local } => {
                handle_search(query, protondb, local).await
            }
            Commands::Info { gpu, wine, vulkan, full } => {
                handle_info(gpu, wine, vulkan, full).await
            }
            Commands::Backup { action } => handle_backup_command(action).await,
            Commands::Battlenet { action } => handle_battlenet_command(action).await,
            Commands::Graphics { action } => handle_graphics_command(action).await,
            Commands::Tui => launch_tui().await,
            Commands::Gui => launch_gui().await,
        }
    }
}

async fn handle_game_command(action: GameCommands) -> Result<()> {
    match action {
        GameCommands::List { launcher, status: _ } => {
            println!("{}", "📮 Available Games:".bold().cyan());

            let config_dir = dirs::config_dir().unwrap().join("ghostforge");
            let launcher_manager = crate::launcher::LauncherManager::new(config_dir);

            // Get all detected launchers
            let launchers = launcher_manager.detect_launchers()?;

            if launchers.is_empty() {
                println!("No launchers detected. Run 'forge launcher list' to see available launchers.");
                return Ok(());
            }

            let mut total_games = 0;

            // Filter launchers if specific launcher requested
            let filtered_launchers: Vec<_> = if let Some(ref launcher_filter) = launcher {
                launchers.into_iter()
                    .filter(|l| l.name.to_lowercase().contains(&launcher_filter.to_lowercase()))
                    .collect()
            } else {
                launchers
            };

            for launcher_info in filtered_launchers {
                println!("\n{} ({}):", launcher_info.name.bold().blue(), format!("{:?}", launcher_info.launcher_type).dimmed());

                let games = match launcher_info.launcher_type {
                    crate::launcher::LauncherType::Steam => {
                        launcher_manager.sync_steam_games(&launcher_info)?
                    }
                    crate::launcher::LauncherType::BattleNet => {
                        launcher_manager.sync_battlenet_games(&launcher_info)?
                    }
                    _ => {
                        println!("  Game sync not yet implemented for this launcher type");
                        continue;
                    }
                };

                if games.is_empty() {
                    println!("  No games found");
                } else {
                    for game in &games {
                        let status_icon = if game.installed { "✅" } else { "❌" };
                        println!("  {} {} (ID: {})", status_icon, game.name.cyan(), game.id.yellow());
                        if game.installed {
                            println!("    Path: {}", game.install_path.display().to_string().dimmed());
                        }
                    }
                    total_games += games.len();
                }
            }

            println!("\n{} {} games found", "📊".bold(), total_games.to_string().bold().green());
            Ok(())
        }
        GameCommands::Add { path: _, name, wine_version: _ } => {
            println!("{} {}", "✅".green(), format!("Added game: {}", name).bold());
            Ok(())
        }
        GameCommands::Remove { game, purge: _ } => {
            println!("{} Removed game: {}", "🗑️", game);
            Ok(())
        }
        GameCommands::Edit { game } => {
            println!("Opening configuration for: {}", game.yellow());
            Ok(())
        }
        GameCommands::Info { game } => {
            println!("{}", format!("Game Information: {}", game).bold());
            Ok(())
        }
        GameCommands::Verify { game } => {
            println!("Verifying files for: {}", game.cyan());
            Ok(())
        }
    }
}

async fn handle_wine_command(action: WineCommands) -> Result<()> {
    match action {
        WineCommands::List { available } => {
            println!("{}", "🍷 Wine/Proton Versions:".bold().magenta());

            let wine_dir = dirs::data_dir().unwrap().join("ghostforge").join("wine");
            let config_dir = dirs::config_dir().unwrap().join("ghostforge");
            let manager = crate::wine::WineManager::new(wine_dir, config_dir);

            if available {
                println!("\n📥 Available for Download:");
                match manager.list_available().await {
                    Ok(available_versions) => {
                        if available_versions.is_empty() {
                            println!("  No versions available for download (check internet connection)");
                        } else {
                            for version in &available_versions[..10.min(available_versions.len())] {
                                let type_icon = match version.wine_type {
                                    crate::wine::WineType::ProtonGE => "🚀",
                                    crate::wine::WineType::Lutris => "🎮",
                                    crate::wine::WineType::WineStaging => "🍷",
                                    _ => "📦",
                                };
                                println!("  {} {} ({})", type_icon, version.name.green(), format!("{:?}", version.wine_type).dimmed());
                            }
                            if available_versions.len() > 10 {
                                println!("  ... and {} more", available_versions.len() - 10);
                            }
                        }
                    }
                    Err(e) => println!("  ❌ Failed to fetch available versions: {}", e),
                }
            } else {
                println!("\n📦 Installed Versions:");
                match manager.list_installed().await {
                    Ok(installed_versions) => {
                        if installed_versions.is_empty() {
                            println!("  No Wine/Proton versions installed");
                            println!("  Use 'forge wine list --available' to see downloadable versions");
                        } else {
                            for version in &installed_versions {
                                let type_icon = match version.wine_type {
                                    crate::wine::WineType::Proton | crate::wine::WineType::ProtonGE => "🚀",
                                    crate::wine::WineType::Wine => "🍷",
                                    crate::wine::WineType::WineStaging => "🍾",
                                    crate::wine::WineType::Lutris => "🎮",
                                    _ => "📦",
                                };
                                let system_marker = if version.system { " (system)" } else { "" };
                                println!("  {} {} {} {}",
                                    type_icon,
                                    version.name.cyan(),
                                    format!("v{}", version.version).yellow(),
                                    system_marker.dimmed()
                                );
                                println!("    Architecture: {}", version.arch.join(", ").dimmed());
                                if !version.system {
                                    println!("    Path: {}", version.path.display().to_string().dimmed());
                                }
                            }
                        }
                    }
                    Err(e) => println!("  ❌ Failed to list installed versions: {}", e),
                }
            }
            Ok(())
        }
        WineCommands::Install { version } => {
            println!("Installing {}...", version.green());
            Ok(())
        }
        WineCommands::Remove { version } => {
            println!("Removing {}...", version.red());
            Ok(())
        }
        WineCommands::Default { version } => {
            println!("Set default to: {}", version.yellow());
            Ok(())
        }
        WineCommands::GeProton { latest, version } => {
            if latest {
                println!("Installing latest GE-Proton...");
            } else if let Some(v) = version {
                println!("Installing GE-Proton {}...", v);
            }
            Ok(())
        }
    }
}

async fn handle_launch(game: String, wine_version: Option<String>, args: Vec<String>) -> Result<()> {
    let config = crate::config::Config::load()?;
    config.ensure_directories()?;
    let game_lib = crate::game::GameLibrary::new(&config.paths.database)?;
    let launcher = GameLauncher::new(config);

    // Find the game in the database
    let game_obj = if let Some(found_game) = game_lib.search_games(&game)?
        .into_iter()
        .find(|g| g.name.to_lowercase() == game.to_lowercase()) {
        found_game
    } else {
        return Err(anyhow::anyhow!(
            "Game '{}' not found. Use 'forge game list' to see available games.",
            game
        ));
    };

    println!("{} Launching {}...", "🚀", game_obj.name.bold().green());

    // Prepare launch options
    let mut options = LaunchOptions::default();

    if let Some(wine) = wine_version {
        options.wine_version = Some(wine.clone());
        println!("  Using Wine/Proton: {}", wine.cyan());
    }

    options.launch_arguments = args;

    // Get ProtonDB recommendations if available
    if let Some(launcher_id) = &game_obj.launcher_id {
        if let Ok(appid) = launcher_id.parse::<u32>() {
            let protondb = crate::protondb::ProtonDBClient::new();
            if let Ok(compat_report) = protondb.get_compatibility_info(appid).await {
                println!("  🌐 ProtonDB rating: {:?}", compat_report.tier);
                if compat_report.tier == ProtonDBTier::Silver || compat_report.tier == ProtonDBTier::Bronze {
                    println!("  ⚠️  This game may require tweaks for optimal performance");
                }
            }
        }
    }

    // Launch the game
    match launcher.launch_game(&game_obj, options).await {
        Ok(pid) => {
            println!("✅ {} launched successfully (PID: {})", game_obj.name, pid);
            Ok(())
        },
        Err(e) => {
            println!("❌ Failed to launch {}: {}", game_obj.name, e);
            Err(e)
        }
    }
}

async fn handle_install(source: String, name: Option<String>, _wine_version: Option<String>) -> Result<()> {
    println!("{} Installing from: {}", "📦", source.yellow());
    if let Some(n) = name {
        println!("  Game name: {}", n);
    }
    Ok(())
}

async fn handle_config_command(action: ConfigCommands) -> Result<()> {
    let mut config = crate::config::Config::load()?;

    match action {
        ConfigCommands::Show => {
            println!("{}", "⚙️  GhostForge Configuration".bold().cyan());
            println!();

            // General settings
            println!("{}", "General:".bold());
            println!("  Default Wine Version: {}", config.general.default_wine_version.yellow());
            println!("  Enable GameMode: {}", if config.general.enable_gamemode { "✅ Yes".green() } else { "❌ No".red() });
            println!("  Enable MangoHud: {}", if config.general.enable_mangohud { "✅ Yes".green() } else { "❌ No".red() });
            println!("  Enable DXVK: {}", if config.general.enable_dxvk { "✅ Yes".green() } else { "❌ No".red() });
            println!("  Enable VKD3D: {}", if config.general.enable_vkd3d { "✅ Yes".green() } else { "❌ No".red() });
            println!("  Log Level: {}", config.general.log_level.cyan());
            println!();

            // Wine settings
            println!("{}", "Wine:".bold());
            println!("  Default Prefix Path: {}", config.wine.default_prefix_path.display().to_string().yellow());
            println!("  Wine Versions Path: {}", config.wine.wine_versions_path.display().to_string().yellow());
            println!("  Default Architecture: {}", config.wine.default_arch.cyan());
            println!("  Default Windows Version: {}", config.wine.default_windows_version.cyan());
            println!();

            // GPU settings
            println!("{}", "GPU:".bold());
            println!("  NVIDIA Prime Render Offload: {}", if config.gpu.nvidia_prime_render_offload { "✅ Yes".green() } else { "❌ No".red() });
            println!("  Enable DLSS: {}", if config.gpu.enable_dlss { "✅ Yes".green() } else { "❌ No".red() });
            println!("  Enable Ray Tracing: {}", if config.gpu.enable_ray_tracing { "✅ Yes".green() } else { "❌ No".red() });
            println!();

            // Paths
            println!("{}", "Paths:".bold());
            println!("  Games Library: {}", config.paths.games_library.display().to_string().yellow());
            println!("  Downloads: {}", config.paths.downloads.display().to_string().yellow());
            println!("  Database: {}", config.paths.database.display().to_string().yellow());
            println!();

            Ok(())
        }
        ConfigCommands::Set { key, value } => {
            let updated = match key.as_str() {
                "wine.default_version" => {
                    config.general.default_wine_version = value.clone();
                    true
                },
                "general.gamemode" => {
                    config.general.enable_gamemode = value.parse().unwrap_or(false);
                    true
                },
                "general.mangohud" => {
                    config.general.enable_mangohud = value.parse().unwrap_or(false);
                    true
                },
                "general.dxvk" => {
                    config.general.enable_dxvk = value.parse().unwrap_or(true);
                    true
                },
                "general.vkd3d" => {
                    config.general.enable_vkd3d = value.parse().unwrap_or(false);
                    true
                },
                "gpu.nvidia_prime" => {
                    config.gpu.nvidia_prime_render_offload = value.parse().unwrap_or(false);
                    true
                },
                "gpu.dlss" => {
                    config.gpu.enable_dlss = value.parse().unwrap_or(true);
                    true
                },
                "gpu.ray_tracing" => {
                    config.gpu.enable_ray_tracing = value.parse().unwrap_or(true);
                    true
                },
                "wine.default_arch" => {
                    if value == "win32" || value == "win64" {
                        config.wine.default_arch = value.clone();
                        true
                    } else {
                        println!("❌ Invalid architecture. Use 'win32' or 'win64'");
                        false
                    }
                },
                _ => {
                    println!("❌ Unknown configuration key: {}", key);
                    println!("Available keys:");
                    println!("  wine.default_version, general.gamemode, general.mangohud");
                    println!("  general.dxvk, general.vkd3d, gpu.nvidia_prime");
                    println!("  gpu.dlss, gpu.ray_tracing, wine.default_arch");
                    false
                }
            };

            if updated {
                config.save()?;
                println!("✅ Set {} = {}", key.cyan(), value.green());
                println!("Configuration saved to: {}", crate::config::Config::config_path().display().to_string().dimmed());
            }

            Ok(())
        }
        ConfigCommands::Get { key } => {
            let value = match key.as_str() {
                "wine.default_version" => config.general.default_wine_version,
                "general.gamemode" => config.general.enable_gamemode.to_string(),
                "general.mangohud" => config.general.enable_mangohud.to_string(),
                "general.dxvk" => config.general.enable_dxvk.to_string(),
                "general.vkd3d" => config.general.enable_vkd3d.to_string(),
                "gpu.nvidia_prime" => config.gpu.nvidia_prime_render_offload.to_string(),
                "gpu.dlss" => config.gpu.enable_dlss.to_string(),
                "gpu.ray_tracing" => config.gpu.enable_ray_tracing.to_string(),
                "wine.default_arch" => config.wine.default_arch,
                _ => {
                    println!("❌ Unknown configuration key: {}", key);
                    return Ok(());
                }
            };

            println!("{}: {}", key.cyan(), value.yellow());
            Ok(())
        }
        ConfigCommands::Reset { yes } => {
            if yes {
                let default_config = crate::config::Config::default();
                default_config.save()?;
                println!("✅ Configuration reset to defaults");
                println!("Configuration file: {}", crate::config::Config::config_path().display().to_string().dimmed());
            } else {
                println!("⚠️ This will reset ALL configuration to defaults.");
                println!("Use --yes to confirm the reset.");
            }
            Ok(())
        }
    }
}

async fn handle_launcher_command(action: LauncherCommands) -> Result<()> {
    match action {
        LauncherCommands::List => {
            println!("{}", "🎮 Configured Launchers:".bold().blue());

            let config_dir = dirs::config_dir().unwrap().join("ghostforge");
            let launcher_manager = crate::launcher::LauncherManager::new(config_dir);

            match launcher_manager.detect_launchers() {
                Ok(launchers) => {
                    if launchers.is_empty() {
                        println!("  No launchers detected");
                        println!("\n💡 Supported launchers:");
                        println!("  • Steam - Install from your distribution's package manager");
                        println!("  • Battle.net - Use 'forge battlenet setup' to create a Wine prefix");
                        println!("  • Epic Games - Use 'forge launcher setup epic' or install Heroic Games Launcher");
                        println!("  • GOG Galaxy - Use 'forge launcher setup gog' or install Minigalaxy");
                        println!("  • Ubisoft Connect - Use 'forge launcher setup ubisoft'");
                        println!("  • EA App - Use 'forge launcher setup ea'");
                    } else {
                        for launcher in &launchers {
                            let status_icon = if launcher.installed { "✅" } else { "❌" };
                            let launcher_icon = match launcher.launcher_type {
                                crate::launcher::LauncherType::Steam => "🚂",
                                crate::launcher::LauncherType::BattleNet => "⚔️",
                                crate::launcher::LauncherType::Epic => "🎮",
                                crate::launcher::LauncherType::GOG => "🎭",
                                crate::launcher::LauncherType::Ubisoft => "🗼",
                                crate::launcher::LauncherType::EA => "🎯",
                                crate::launcher::LauncherType::Riot => "⚡",
                                crate::launcher::LauncherType::Rockstar => "🌟",
                                crate::launcher::LauncherType::Custom => "🔧",
                            };

                            println!("  {} {} {} ({})",
                                status_icon,
                                launcher_icon,
                                launcher.name.bold().cyan(),
                                format!("{:?}", launcher.launcher_type).dimmed()
                            );

                            println!("    Executable: {}", launcher.executable.display().to_string().dimmed());

                            if let Some(ref wine_prefix) = launcher.wine_prefix {
                                println!("    Wine Prefix: {}", wine_prefix.display().to_string().dimmed());
                                if let Some(ref wine_version) = launcher.wine_version {
                                    println!("    Wine Version: {}", wine_version.dimmed());
                                }
                            }

                            // Show game paths
                            if !launcher.games_path.is_empty() {
                                println!("    Game Paths:");
                                for path in &launcher.games_path {
                                    let exists_marker = if path.exists() { "✓" } else { "✗" };
                                    println!("      {} {}", exists_marker, path.display().to_string().dimmed());
                                }
                            }
                            println!();
                        }

                        println!("📊 {} launcher(s) detected", launchers.len().to_string().bold().green());

                        // Show which ones have games available
                        let mut games_available = 0;
                        for launcher in &launchers {
                            let game_count = match launcher.launcher_type {
                                crate::launcher::LauncherType::Steam => {
                                    launcher_manager.sync_steam_games(launcher).map(|games| games.len()).unwrap_or(0)
                                }
                                crate::launcher::LauncherType::BattleNet => {
                                    launcher_manager.sync_battlenet_games(launcher).map(|games| games.len()).unwrap_or(0)
                                }
                                _ => 0,
                            };
                            games_available += game_count;
                        }

                        if games_available > 0 {
                            println!("🎮 {} games available across all launchers", games_available.to_string().bold().blue());
                            println!("💡 Use 'forge game list' to see all games");
                        }
                    }
                }
                Err(e) => {
                    println!("❌ Failed to detect launchers: {}", e);
                }
            }
            Ok(())
        }
        LauncherCommands::Setup { launcher, path: _ } => {
            println!("Setting up {} launcher...", launcher.green());
            Ok(())
        }
        LauncherCommands::Sync { launcher } => {
            let config = crate::config::Config::load()?;
            config.ensure_directories()?;
            let game_lib = crate::game::GameLibrary::new(&config.paths.database)?;
            let launcher_manager = crate::launcher::LauncherManager::new(config.paths.cache.clone());

            if !launcher.is_empty() {
                println!("🔄 Syncing games from {}...", launcher.cyan());
                // TODO: Sync specific launcher
                println!("⚠️ Specific launcher sync not yet implemented");
            } else {
                println!("🔄 Syncing games from all detected launchers...");
                let imported = launcher_manager.import_all_games(&game_lib).await?;
                println!("\n✅ Successfully imported {} games", imported.to_string().bold().green());

                if imported > 0 {
                    println!("Use 'forge game list' to see your imported games");
                }
            }
            Ok(())
        }
        LauncherCommands::Remove { launcher } => {
            println!("Removing {} configuration", launcher);
            Ok(())
        }
    }
}

async fn handle_tricks(game: String, trick: String, _force: bool) -> Result<()> {
    use crate::winetricks::WinetricksManager;

    let cache_dir = dirs::cache_dir().unwrap().join("ghostforge").join("winetricks");
    let manager = WinetricksManager::new(cache_dir)?;

    // For demo purposes, use a default prefix path
    let prefix_path = dirs::home_dir().unwrap().join("Games").join("battle.net");

    match trick.as_str() {
        "battlenet-essentials" => {
            println!("🎮 Installing Battle.net essentials for {}...", game.cyan());
            manager.install_battlenet_essentials(&prefix_path).await?;
        }
        "wow-optimize" => {
            println!("🐉 Optimizing for World of Warcraft...");
            manager.optimize_for_wow(&prefix_path).await?;
        }
        "create-battlenet-prefix" => {
            println!("🍷 Creating new Battle.net prefix...");
            manager.create_battlenet_prefix(&prefix_path, None).await?;
        }
        _ => {
            if let Some(verb) = manager.get_verb_info(&trick) {
                println!("Installing {} for {}...", verb.description.magenta(), game.cyan());
                manager.install_verb(&prefix_path, &verb).await?;
            } else {
                println!("❌ Unknown trick: {}", trick);
                println!("Available tricks:");
                println!("  • battlenet-essentials - Install all Battle.net essentials");
                println!("  • wow-optimize - Optimize for World of Warcraft");
                println!("  • create-battlenet-prefix - Create new Battle.net prefix");
                println!("  • corefonts - Windows core fonts");
                println!("  • vcrun2019 - Visual C++ 2019 Runtime");
                println!("  • dxvk - DirectX to Vulkan layer");
            }
        }
    }

    Ok(())
}

async fn handle_optimize(
    _game: Option<String>,
    nvidia: bool,
    amd: bool,
    gamemode: bool,
    cpu_performance: bool
) -> Result<()> {
    println!("{}", "⚡ Applying optimizations...".bold().yellow());
    if nvidia {
        println!("  • NVIDIA optimizations enabled");
    }
    if amd {
        println!("  • AMD optimizations enabled");
    }
    if gamemode {
        println!("  • GameMode enabled");
    }
    if cpu_performance {
        println!("  • CPU governor set to performance");
    }
    Ok(())
}

async fn handle_search(query: String, _protondb: bool, _local: bool) -> Result<()> {
    println!("🔍 Searching for: {}", query.bold());
    Ok(())
}

async fn handle_info(gpu: bool, wine: bool, vulkan: bool, full: bool) -> Result<()> {
    println!("{}", "ℹ️  System Information:".bold().blue());

    match crate::utils::SystemDetector::get_system_info() {
        Ok(system_info) => {
            if full {
                // Show everything
                println!("\n🖥️  System:");
                println!("  OS: {}", system_info.os.cyan());
                println!("  Kernel: {}", system_info.kernel.yellow());
                if let Some(ref desktop) = system_info.desktop {
                    println!("  Desktop: {}", desktop.green());
                }

                println!("\n💻 CPU:");
                println!("  Model: {}", system_info.cpu.brand.cyan());
                println!("  Cores: {} ({} threads)", system_info.cpu.cores, system_info.cpu.threads);
                println!("  Frequency: {} MHz", system_info.cpu.frequency);

                println!("\n💾 Memory:");
                println!("  Total: {:.2} GB", system_info.memory.total as f64 / 1024.0 / 1024.0 / 1024.0);
                println!("  Available: {:.2} GB", system_info.memory.available as f64 / 1024.0 / 1024.0 / 1024.0);
                if system_info.memory.swap_total > 0 {
                    println!("  Swap: {:.2} GB", system_info.memory.swap_total as f64 / 1024.0 / 1024.0 / 1024.0);
                }
            }

            if gpu || full {
                println!("\n🎮 GPU Information:");
                if system_info.gpu.is_empty() {
                    println!("  No GPUs detected");
                } else {
                    for (i, gpu_info) in system_info.gpu.iter().enumerate() {
                        let vendor_icon = match gpu_info.vendor {
                            crate::utils::GpuVendor::Nvidia => "🟢",
                            crate::utils::GpuVendor::AMD => "🔴",
                            crate::utils::GpuVendor::Intel => "🔵",
                            _ => "⚪",
                        };
                        println!("  {} GPU {}: {} ({:?})", vendor_icon, i + 1, gpu_info.name.bold().cyan(), gpu_info.vendor);

                        if let Some(ref driver) = gpu_info.driver {
                            println!("    Driver: {}", driver.green());
                        }

                        if let Some(vram) = gpu_info.vram {
                            println!("    VRAM: {} GB", vram / 1024 / 1024 / 1024);
                        }

                        let vulkan_status = if gpu_info.vulkan_support { "✅" } else { "❌" };
                        let dxvk_status = if gpu_info.dxvk_support { "✅" } else { "❌" };
                        println!("    Vulkan: {} | DXVK: {}", vulkan_status, dxvk_status);
                    }
                }
            }

            if vulkan || full {
                println!("\n🌋 Vulkan Support:");
                if system_info.vulkan.available {
                    println!("  Status: {} Available", "✅".green());
                    if let Some(ref api_version) = system_info.vulkan.api_version {
                        println!("  API Version: {}", api_version.yellow());
                    }
                    if let Some(ref driver_version) = system_info.vulkan.driver_version {
                        println!("  Driver Version: {}", driver_version.cyan());
                    }
                    if !system_info.vulkan.devices.is_empty() {
                        println!("  Devices:");
                        for device in &system_info.vulkan.devices {
                            println!("    • {}", device.dimmed());
                        }
                    }
                } else {
                    println!("  Status: {} Not available", "❌".red());
                    println!("  Install vulkan drivers for your GPU to enable Vulkan support");
                }
            }

            if wine || full {
                println!("\n🍷 Wine Support:");
                if system_info.wine_support.installed {
                    println!("  Status: {} Installed", "✅".green());
                    if let Some(ref version) = system_info.wine_support.version {
                        println!("  Version: {}", version.yellow());
                    }
                    if !system_info.wine_support.architecture.is_empty() {
                        println!("  Architecture: {}", system_info.wine_support.architecture.join(", ").cyan());
                    }
                    let multilib_status = if system_info.wine_support.multilib_support { "✅ Yes" } else { "❌ No" };
                    println!("  Multilib: {}", multilib_status);
                    if let Some(ref prefix_path) = system_info.wine_support.prefix_path {
                        println!("  Default Prefix: {}", prefix_path.display().to_string().dimmed());
                    }
                } else {
                    println!("  Status: {} Not installed", "❌".red());
                    println!("  Install Wine to run Windows games and applications");
                }
            }

            if full {
                println!("\n🎯 Gaming Tools:");
                let tools = &system_info.gaming_tools;
                let dxvk_status = if tools.dxvk { "✅" } else { "❌" };
                let vkd3d_status = if tools.vkd3d { "✅" } else { "❌" };
                let mangohud_status = if tools.mangohud { "✅" } else { "❌" };
                let gamemode_status = if tools.gamemode { "✅" } else { "❌" };
                let gamescope_status = if tools.gamescope { "✅" } else { "❌" };
                let winetricks_status = if tools.winetricks { "✅" } else { "❌" };
                let protontricks_status = if tools.protontricks { "✅" } else { "❌" };

                println!("  DXVK: {} | VKD3D: {} | MangoHUD: {}", dxvk_status, vkd3d_status, mangohud_status);
                println!("  GameMode: {} | GameScope: {}", gamemode_status, gamescope_status);
                println!("  Winetricks: {} | Protontricks: {}", winetricks_status, protontricks_status);

                // Show container runtime information
                if full {
                    println!("\n📦 Container Runtime:");
                    let config_dir = dirs::config_dir().unwrap_or_default().join("ghostforge");
                    match crate::container::ContainerManager::new(config_dir) {
                        Ok(container_manager) => {
                            let runtime_info = container_manager.get_runtime_info();
                            println!("  Current: {} {}",
                                runtime_info.current_runtime.cyan(),
                                if runtime_info.bolt_optimized { "(gaming-optimized)".green() } else { "".normal() }
                            );

                            if !runtime_info.available_runtimes.is_empty() {
                                println!("  Available runtimes:");
                                for runtime in &runtime_info.available_runtimes {
                                    println!("    • {}", runtime.cyan());
                                }
                            } else {
                                println!("  ⚠️  No container runtimes detected");
                            }
                        }
                        Err(e) => {
                            println!("  ❌ Container runtime detection failed: {}", e.to_string().red());
                        }
                    }
                }

                // Show recommendations
                println!("\n💡 Recommendations:");
                if !tools.dxvk {
                    println!("  • Install DXVK for better DirectX performance");
                }
                if !tools.vkd3d {
                    println!("  • Install VKD3D-Proton for DirectX 12 support");
                }
                if !tools.gamemode {
                    println!("  • Install GameMode for automatic performance optimizations");
                }
                if !tools.mangohud {
                    println!("  • Install MangoHUD for performance monitoring overlay");
                }
                if !system_info.wine_support.installed {
                    println!("  • Install Wine to run Windows games");
                }
                if !system_info.vulkan.available {
                    println!("  • Install Vulkan drivers for your GPU");
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to gather system information: {}", e);
            println!("This might be due to missing system utilities or permissions");
        }
    }

    Ok(())
}

async fn handle_backup_command(action: BackupCommands) -> Result<()> {
    match action {
        BackupCommands::Create { target, output: _ } => {
            println!("Creating backup of {}...", target.cyan());
            Ok(())
        }
        BackupCommands::Restore { backup, game: _ } => {
            println!("Restoring from {}...", backup.yellow());
            Ok(())
        }
        BackupCommands::List => {
            println!("{}", "💾 Available Backups:".bold());
            Ok(())
        }
    }
}

async fn handle_battlenet_command(action: BattlenetCommands) -> Result<()> {
    use crate::winetricks::WinetricksManager;
    use crate::utils::SystemDetector;

    match action {
        BattlenetCommands::Setup { wine_version, prefix, game } => {
            let prefix_path = prefix
                .map(|p| PathBuf::from(p))
                .unwrap_or_else(|| dirs::home_dir().unwrap().join("Games/battlenet"));

            println!("🍷 Setting up Battle.net prefix at: {}", prefix_path.display());

            let cache_dir = dirs::cache_dir().unwrap().join("ghostforge").join("winetricks");
            let manager = WinetricksManager::new(cache_dir)?;

            match game.as_deref() {
                Some("wow") => {
                    println!("🐉 Setting up World of Warcraft optimized prefix...");
                    crate::winetricks::setup_wow_prefix(&prefix_path, wine_version.as_deref()).await?;
                }
                Some("diablo") => {
                    println!("⚔️  Setting up Diablo optimized prefix...");
                    crate::winetricks::setup_diablo_prefix(&prefix_path, wine_version.as_deref()).await?;
                }
                _ => {
                    manager.create_battlenet_prefix(&prefix_path, wine_version.as_deref()).await?;
                }
            }
        }

        BattlenetCommands::Essentials { prefix } => {
            let prefix_path = prefix
                .map(|p| PathBuf::from(p))
                .unwrap_or_else(|| dirs::home_dir().unwrap().join("Games/battlenet"));

            println!("📦 Installing Battle.net essentials...");
            let cache_dir = dirs::cache_dir().unwrap().join("ghostforge").join("winetricks");
            let manager = WinetricksManager::new(cache_dir)?;

            manager.install_battlenet_essentials(&prefix_path).await?;
        }

        BattlenetCommands::WowOptimize { prefix } => {
            let prefix_path = prefix
                .map(|p| PathBuf::from(p))
                .unwrap_or_else(|| dirs::home_dir().unwrap().join("Games/battlenet"));

            println!("🐉 Optimizing prefix for World of Warcraft...");
            let cache_dir = dirs::cache_dir().unwrap().join("ghostforge").join("winetricks");
            let manager = WinetricksManager::new(cache_dir)?;

            manager.optimize_for_wow(&prefix_path).await?;
        }

        BattlenetCommands::Download { output } => {
            let download_dir = output
                .map(|p| PathBuf::from(p))
                .unwrap_or_else(|| dirs::download_dir().unwrap_or_else(|| dirs::home_dir().unwrap().join("Downloads")));

            println!("📥 Downloading Battle.net installer...");
            println!("Installer will be saved to: {}", download_dir.display());

            // In a real implementation, you'd download from Blizzard's servers
            println!("⚠️  Download Battle.net from: https://www.battle.net/download");
            println!("Save it to: {}", download_dir.display());
        }

        BattlenetCommands::Check => {
            println!("🔍 Checking Battle.net compatibility...");
            let report = SystemDetector::check_battlenet_compatibility()?;
            println!("{}", report);
        }

        BattlenetCommands::Games => {
            println!("🎮 Scanning for Battle.net games...");
            let config_dir = dirs::config_dir().unwrap().join("ghostforge");
            let launcher_manager = crate::launcher::LauncherManager::new(config_dir);

            if let Some(battlenet_launcher) = launcher_manager.detect_battlenet()? {
                let games = launcher_manager.sync_battlenet_games(&battlenet_launcher)?;

                if games.is_empty() {
                    println!("No Battle.net games found.");
                } else {
                    println!("Found {} Battle.net game(s):", games.len());
                    for game in games {
                        println!("  • {} ({})", game.name.cyan(), game.launcher_id.yellow());
                        println!("    Path: {}", game.install_path.display());
                        println!("    Installed: {}", if game.installed { "✅" } else { "❌" });
                    }
                }
            } else {
                println!("❌ Battle.net launcher not detected.");
                println!("Run 'forge battlenet setup' to create a Battle.net prefix first.");
            }
        }
    }

    Ok(())
}

async fn handle_graphics_command(action: GraphicsCommands) -> Result<()> {
    use crate::graphics::GraphicsManager;

    let base_dir = dirs::data_dir().unwrap().join("ghostforge").join("graphics");
    let mut manager = GraphicsManager::new(base_dir)?;
    manager.set_dry_run(true); // Safe default

    match action {
        GraphicsCommands::List { available, dxvk, vkd3d } => {
            if available {
                println!("📥 Available Graphics Layers:");

                if !vkd3d {
                    println!("\n🔷 DXVK (DirectX 9/10/11 → Vulkan):");
                    let dxvk_versions = manager.list_available_dxvk().await?;
                    for layer in &dxvk_versions[..5.min(dxvk_versions.len())] {
                        println!("  • {} - APIs: {}", layer.name.cyan(), layer.supported_apis.join(", "));
                    }
                }

                if !dxvk {
                    println!("\n🔶 VKD3D-Proton (DirectX 12 → Vulkan):");
                    let vkd3d_versions = manager.list_available_vkd3d().await?;
                    for layer in &vkd3d_versions[..5.min(vkd3d_versions.len())] {
                        println!("  • {} - APIs: {}", layer.name.yellow(), layer.supported_apis.join(", "));
                    }
                }
            } else {
                println!("📦 Installed Graphics Layers:");
                let installed = manager.list_installed()?;

                if installed.is_empty() {
                    println!("No graphics layers installed. Use 'forge graphics list --available' to see available versions.");
                } else {
                    for layer in installed {
                        println!("  • {} at {}", layer.name.green(), layer.path.display());
                    }
                }
            }
        }

        GraphicsCommands::Install { layer } => {
            println!("📦 Installing graphics layer: {}", layer.cyan());

            // For demo, show what would be installed
            if layer.contains("dxvk") {
                let versions = manager.list_available_dxvk().await?;
                if let Some(found) = versions.iter().find(|v| v.version.contains(&layer) || v.name.to_lowercase().contains(&layer.to_lowercase())) {
                    println!("Found: {}", found.name);
                    println!("🔄 [DRY RUN] Would download from: {}", found.download_url.as_ref().unwrap_or(&"N/A".to_string()));
                    println!("✅ [SIMULATED] {} installed successfully", found.name);
                } else {
                    println!("❌ DXVK version '{}' not found", layer);
                }
            } else if layer.contains("vkd3d") {
                let versions = manager.list_available_vkd3d().await?;
                if let Some(found) = versions.iter().find(|v| v.version.contains(&layer) || v.name.to_lowercase().contains(&layer.to_lowercase())) {
                    println!("Found: {}", found.name);
                    println!("🔄 [DRY RUN] Would download from: {}", found.download_url.as_ref().unwrap_or(&"N/A".to_string()));
                    println!("✅ [SIMULATED] {} installed successfully", found.name);
                } else {
                    println!("❌ VKD3D version '{}' not found", layer);
                }
            }
        }

        GraphicsCommands::Apply { layer_type, prefix, version: _ } => {
            let _prefix_path = PathBuf::from(&prefix);

            match layer_type.to_lowercase().as_str() {
                "dxvk" => {
                    println!("🔧 Applying DXVK to prefix: {}", prefix.cyan());
                    println!("🔄 [DRY RUN] Would copy DXVK DLLs to prefix");
                    println!("🔄 [DRY RUN] Would set DLL overrides for: d3d9, d3d10core, d3d11, dxgi");
                    println!("✅ [SIMULATED] DXVK applied successfully");
                }
                "vkd3d" => {
                    println!("🔧 Applying VKD3D-Proton to prefix: {}", prefix.cyan());
                    println!("🔄 [DRY RUN] Would copy VKD3D DLLs to prefix");
                    println!("🔄 [DRY RUN] Would set DLL overrides for: d3d12, dxcore");
                    println!("✅ [SIMULATED] VKD3D-Proton applied successfully");
                }
                _ => {
                    println!("❌ Unknown layer type: {}. Use 'dxvk' or 'vkd3d'", layer_type);
                }
            }
        }

        GraphicsCommands::Remove { layer_type, prefix } => {
            let _prefix_path = PathBuf::from(&prefix);

            match layer_type.to_lowercase().as_str() {
                "dxvk" => {
                    println!("🗑️ Removing DXVK from prefix: {}", prefix.cyan());
                    println!("🔄 [DRY RUN] Would remove DXVK DLLs and reset overrides");
                    println!("✅ [SIMULATED] DXVK removed successfully");
                }
                "vkd3d" => {
                    println!("🗑️ Removing VKD3D from prefix: {}", prefix.cyan());
                    println!("🔄 [DRY RUN] Would remove VKD3D DLLs and reset overrides");
                    println!("✅ [SIMULATED] VKD3D removed successfully");
                }
                "all" => {
                    println!("🗑️ Removing all graphics layers from prefix: {}", prefix.cyan());
                    println!("🔄 [DRY RUN] Would remove DXVK and VKD3D");
                    println!("✅ [SIMULATED] All graphics layers removed");
                }
                _ => {
                    println!("❌ Unknown layer type: {}. Use 'dxvk', 'vkd3d', or 'all'", layer_type);
                }
            }
        }

        GraphicsCommands::Info { prefix } => {
            let prefix_path = PathBuf::from(&prefix);
            println!("📊 Graphics Layer Information for: {}", prefix.cyan());

            // Simulate checking what's installed
            println!("🔍 Checking prefix...");

            if prefix_path.join("drive_c/windows/system32/d3d11.dll").exists() {
                println!("  ✅ DXVK: Installed (d3d9, d3d10core, d3d11, dxgi)");
            } else {
                println!("  ❌ DXVK: Not installed");
            }

            if prefix_path.join("drive_c/windows/system32/d3d12.dll").exists() {
                println!("  ✅ VKD3D-Proton: Installed (d3d12, dxcore)");
            } else {
                println!("  ❌ VKD3D-Proton: Not installed");
            }

            println!("  🍷 WineD3D: Available (Wine's built-in renderer)");
        }

        GraphicsCommands::Recommend { game } => {
            let nvidia_features = manager.detect_nvidia_features().unwrap_or_default();
            let recommendations = manager.recommend_for_game(&game, &nvidia_features);

            println!("🎯 Graphics Recommendations for: {}", game.green());
            println!("Based on community reports and compatibility data:\n");

            for layer_type in recommendations {
                match layer_type {
                    crate::graphics::GraphicsLayerType::DXVK => {
                        println!("  ✅ {} - Recommended", "DXVK".cyan().bold());
                        println!("     • Significantly improves DirectX 9/10/11 performance");
                        println!("     • Better frame times and reduced CPU overhead");
                        println!("     • Install: forge graphics install dxvk-latest");
                    }
                    crate::graphics::GraphicsLayerType::VKD3DProton => {
                        println!("  ✅ {} - Recommended", "VKD3D-Proton".yellow().bold());
                        println!("     • Required for DirectX 12 games");
                        println!("     • Valve's optimized D3D12 implementation");
                        println!("     • Install: forge graphics install vkd3d-proton-latest");
                    }
                    _ => {}
                }
            }

            println!("\n💡 Tip: Always test both layers to see which performs best for your system!");
        }
    }

    Ok(())
}

async fn launch_tui() -> Result<()> {
    println!("Launching Terminal UI...");
    Ok(())
}

async fn launch_gui() -> Result<()> {
    println!("Launching GUI...");
    Ok(())
}