use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use regex::Regex;
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Launcher {
    pub name: String,
    pub launcher_type: LauncherType,
    pub path: PathBuf,
    pub executable: PathBuf,
    pub config_path: PathBuf,
    pub games_path: Vec<PathBuf>,
    pub installed: bool,
    pub wine_prefix: Option<PathBuf>,
    pub wine_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LauncherType {
    Steam,
    BattleNet,
    Epic,
    GOG,
    Ubisoft,
    EA,
    Riot,
    Rockstar,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LauncherGame {
    pub id: String,
    pub name: String,
    pub launcher: LauncherType,
    pub launcher_id: String,
    pub install_path: PathBuf,
    pub executable: Option<PathBuf>,
    pub launch_command: String,
    pub icon: Option<PathBuf>,
    pub installed: bool,
}

pub struct LauncherManager {
    config_dir: PathBuf,
}

impl LauncherManager {
    pub fn new(config_dir: PathBuf) -> Self {
        Self { config_dir }
    }

    pub fn detect_launchers(&self) -> Result<Vec<Launcher>> {
        let mut launchers = Vec::new();

        // Detect Steam
        if let Some(steam) = self.detect_steam()? {
            launchers.push(steam);
        }

        // Detect Battle.net
        if let Some(battlenet) = self.detect_battlenet()? {
            launchers.push(battlenet);
        }

        // Detect Epic Games
        if let Some(epic) = self.detect_epic()? {
            launchers.push(epic);
        }

        // Detect GOG Galaxy
        if let Some(gog) = self.detect_gog()? {
            launchers.push(gog);
        }

        // Detect Ubisoft Connect
        if let Some(ubisoft) = self.detect_ubisoft()? {
            launchers.push(ubisoft);
        }

        // Detect EA App
        if let Some(ea) = self.detect_ea()? {
            launchers.push(ea);
        }

        Ok(launchers)
    }

    fn detect_steam(&self) -> Result<Option<Launcher>> {
        let possible_paths = vec![
            dirs::home_dir().unwrap().join(".local/share/Steam"),
            dirs::home_dir().unwrap().join(".steam/steam"),
            PathBuf::from("/usr/share/steam"),
            PathBuf::from("/opt/steam"),
        ];

        for path in possible_paths {
            if path.exists() {
                let executable = path.join("steam.sh")
                    .exists()
                    .then(|| path.join("steam.sh"))
                    .or_else(|| {
                        which::which("steam").ok()
                    });

                if let Some(exec) = executable {
                    return Ok(Some(Launcher {
                        name: "Steam".to_string(),
                        launcher_type: LauncherType::Steam,
                        path: path.clone(),
                        executable: exec,
                        config_path: path.join("config"),
                        games_path: vec![
                            path.join("steamapps/common"),
                            path.join("steamapps"),
                        ],
                        installed: true,
                        wine_prefix: None,
                        wine_version: None,
                    }));
                }
            }
        }

        Ok(None)
    }

    pub fn detect_battlenet(&self) -> Result<Option<Launcher>> {
        let possible_prefixes = vec![
            dirs::home_dir().unwrap().join(".wine"),
            dirs::home_dir().unwrap().join("Games/battlenet"),
            dirs::home_dir().unwrap().join(".local/share/lutris/runners/wine/battlenet"),
        ];

        for prefix in possible_prefixes {
            let battle_net_path = prefix.join("drive_c/Program Files (x86)/Battle.net");
            if battle_net_path.exists() {
                let executable = battle_net_path.join("Battle.net.exe");
                if executable.exists() {
                    return Ok(Some(Launcher {
                        name: "Battle.net".to_string(),
                        launcher_type: LauncherType::BattleNet,
                        path: battle_net_path.clone(),
                        executable,
                        config_path: prefix.join("drive_c/ProgramData/Battle.net"),
                        games_path: vec![
                            prefix.join("drive_c/Program Files (x86)"),
                            prefix.join("drive_c/Program Files"),
                        ],
                        installed: true,
                        wine_prefix: Some(prefix),
                        wine_version: None,
                    }));
                }
            }
        }

        Ok(None)
    }

    fn detect_epic(&self) -> Result<Option<Launcher>> {
        let possible_prefixes = vec![
            dirs::home_dir().unwrap().join(".wine"),
            dirs::home_dir().unwrap().join("Games/epic-games-store"),
            dirs::home_dir().unwrap().join(".local/share/lutris/runners/wine/epic"),
        ];

        for prefix in possible_prefixes {
            let epic_path = prefix.join("drive_c/Program Files (x86)/Epic Games/Launcher");
            if epic_path.exists() {
                let executable = epic_path.join("Portal/Binaries/Win32/EpicGamesLauncher.exe");
                if executable.exists() {
                    return Ok(Some(Launcher {
                        name: "Epic Games".to_string(),
                        launcher_type: LauncherType::Epic,
                        path: epic_path.clone(),
                        executable,
                        config_path: prefix.join("drive_c/ProgramData/Epic"),
                        games_path: vec![
                            prefix.join("drive_c/Program Files/Epic Games"),
                        ],
                        installed: true,
                        wine_prefix: Some(prefix),
                        wine_version: None,
                    }));
                }
            }
        }

        // Check for Heroic Games Launcher (native Linux Epic/GOG client)
        if let Ok(heroic) = which::which("heroic") {
            let config_path = dirs::config_dir().unwrap().join("heroic");
            if config_path.exists() {
                return Ok(Some(Launcher {
                    name: "Heroic (Epic/GOG)".to_string(),
                    launcher_type: LauncherType::Epic,
                    path: config_path.clone(),
                    executable: heroic,
                    config_path,
                    games_path: vec![dirs::home_dir().unwrap().join("Games/Heroic")],
                    installed: true,
                    wine_prefix: None,
                    wine_version: None,
                }));
            }
        }

        Ok(None)
    }

    fn detect_gog(&self) -> Result<Option<Launcher>> {
        // Check for native GOG Galaxy in Wine
        let possible_prefixes = vec![
            dirs::home_dir().unwrap().join(".wine"),
            dirs::home_dir().unwrap().join("Games/gog-galaxy"),
        ];

        for prefix in possible_prefixes {
            let gog_path = prefix.join("drive_c/Program Files (x86)/GOG Galaxy");
            if gog_path.exists() {
                let executable = gog_path.join("GalaxyClient.exe");
                if executable.exists() {
                    return Ok(Some(Launcher {
                        name: "GOG Galaxy".to_string(),
                        launcher_type: LauncherType::GOG,
                        path: gog_path.clone(),
                        executable,
                        config_path: prefix.join("drive_c/ProgramData/GOG.com"),
                        games_path: vec![
                            prefix.join("drive_c/GOG Games"),
                        ],
                        installed: true,
                        wine_prefix: Some(prefix),
                        wine_version: None,
                    }));
                }
            }
        }

        // Check for Minigalaxy (native Linux GOG client)
        if let Ok(minigalaxy) = which::which("minigalaxy") {
            let config_path = dirs::config_dir().unwrap().join("minigalaxy");
            if config_path.exists() {
                return Ok(Some(Launcher {
                    name: "Minigalaxy".to_string(),
                    launcher_type: LauncherType::GOG,
                    path: config_path.clone(),
                    executable: minigalaxy,
                    config_path,
                    games_path: vec![dirs::home_dir().unwrap().join("GOG Games")],
                    installed: true,
                    wine_prefix: None,
                    wine_version: None,
                }));
            }
        }

        Ok(None)
    }

    fn detect_ubisoft(&self) -> Result<Option<Launcher>> {
        let possible_prefixes = vec![
            dirs::home_dir().unwrap().join(".wine"),
            dirs::home_dir().unwrap().join("Games/ubisoft-connect"),
        ];

        for prefix in possible_prefixes {
            let ubisoft_path = prefix.join("drive_c/Program Files (x86)/Ubisoft/Ubisoft Game Launcher");
            if ubisoft_path.exists() {
                let executable = ubisoft_path.join("UbisoftConnect.exe");
                if executable.exists() {
                    return Ok(Some(Launcher {
                        name: "Ubisoft Connect".to_string(),
                        launcher_type: LauncherType::Ubisoft,
                        path: ubisoft_path.clone(),
                        executable,
                        config_path: prefix.join("drive_c/Program Files (x86)/Ubisoft"),
                        games_path: vec![
                            prefix.join("drive_c/Program Files (x86)/Ubisoft"),
                        ],
                        installed: true,
                        wine_prefix: Some(prefix),
                        wine_version: None,
                    }));
                }
            }
        }

        Ok(None)
    }

    fn detect_ea(&self) -> Result<Option<Launcher>> {
        let possible_prefixes = vec![
            dirs::home_dir().unwrap().join(".wine"),
            dirs::home_dir().unwrap().join("Games/ea-app"),
        ];

        for prefix in possible_prefixes {
            let ea_path = prefix.join("drive_c/Program Files/Electronic Arts/EA Desktop");
            if ea_path.exists() {
                let executable = ea_path.join("EA Desktop.exe");
                if executable.exists() {
                    return Ok(Some(Launcher {
                        name: "EA App".to_string(),
                        launcher_type: LauncherType::EA,
                        path: ea_path.clone(),
                        executable,
                        config_path: prefix.join("drive_c/ProgramData/Electronic Arts"),
                        games_path: vec![
                            prefix.join("drive_c/Program Files/EA Games"),
                        ],
                        installed: true,
                        wine_prefix: Some(prefix),
                        wine_version: None,
                    }));
                }
            }
        }

        Ok(None)
    }

    pub fn sync_steam_games(&self, steam_launcher: &Launcher) -> Result<Vec<LauncherGame>> {
        let mut games = Vec::new();
        let libraryfolders_vdf = steam_launcher.config_path.parent()
            .unwrap()
            .join("steamapps/libraryfolders.vdf");

        if !libraryfolders_vdf.exists() {
            return Ok(games);
        }

        // Parse Steam library folders
        let content = fs::read_to_string(&libraryfolders_vdf)?;
        let library_paths = self.parse_steam_libraries(&content)?;

        for library_path in library_paths {
            let steamapps_dir = library_path.join("steamapps");
            if !steamapps_dir.exists() {
                continue;
            }

            // Parse .acf files for game information
            for entry in fs::read_dir(&steamapps_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("acf") {
                    if let Ok(game) = self.parse_steam_acf(&path) {
                        games.push(game);
                    }
                }
            }
        }

        Ok(games)
    }

    fn parse_steam_libraries(&self, content: &str) -> Result<Vec<PathBuf>> {
        let mut paths = vec![
            dirs::home_dir().unwrap().join(".local/share/Steam"),
        ];

        // Simple regex to find path entries in libraryfolders.vdf
        let re = Regex::new(r#""path"\s+"([^"]+)""#)?;
        for cap in re.captures_iter(content) {
            if let Some(path_str) = cap.get(1) {
                paths.push(PathBuf::from(path_str.as_str()));
            }
        }

        Ok(paths)
    }

    fn parse_steam_acf(&self, acf_path: &Path) -> Result<LauncherGame> {
        let content = fs::read_to_string(acf_path)?;

        let app_id = self.extract_acf_value(&content, "appid")?;
        let name = self.extract_acf_value(&content, "name")?;
        let install_dir = self.extract_acf_value(&content, "installdir")?;

        let steamapps_dir = acf_path.parent().unwrap();
        let install_path = steamapps_dir.join("common").join(&install_dir);

        Ok(LauncherGame {
            id: format!("steam_{}", app_id),
            name,
            launcher: LauncherType::Steam,
            launcher_id: app_id.clone(),
            install_path: install_path.clone(),
            executable: None, // Will be determined later
            launch_command: format!("steam://rungameid/{}", app_id),
            icon: None,
            installed: install_path.exists(),
        })
    }

    fn extract_acf_value(&self, content: &str, key: &str) -> Result<String> {
        let pattern = format!(r#""{}"\s+"([^"]+)""#, key);
        let re = Regex::new(&pattern)?;

        if let Some(cap) = re.captures(content) {
            if let Some(value) = cap.get(1) {
                return Ok(value.as_str().to_string());
            }
        }

        Err(anyhow::anyhow!("Key '{}' not found in ACF file", key))
    }

    pub fn sync_battlenet_games(&self, battlenet_launcher: &Launcher) -> Result<Vec<LauncherGame>> {
        let mut games = Vec::new();

        if let Some(prefix) = &battlenet_launcher.wine_prefix {
            // World of Warcraft
            let wow_paths = vec![
                prefix.join("drive_c/Program Files (x86)/World of Warcraft"),
                prefix.join("drive_c/Program Files/World of Warcraft"),
            ];

            for wow_path in wow_paths {
                if wow_path.exists() {
                    // Check for different WoW versions
                    let versions = vec![
                        ("_retail_", "World of Warcraft"),
                        ("_classic_era_", "World of Warcraft Classic"),
                        ("_classic_", "World of Warcraft Classic"),
                    ];

                    for (folder, name) in versions {
                        let version_path = wow_path.join(folder);
                        if version_path.exists() {
                            games.push(LauncherGame {
                                id: format!("battlenet_wow_{}", folder.trim_matches('_')),
                                name: name.to_string(),
                                launcher: LauncherType::BattleNet,
                                launcher_id: "wow".to_string(),
                                install_path: version_path.clone(),
                                executable: Some(version_path.join("Wow.exe")),
                                launch_command: format!("battlenet://launch/wow"),
                                icon: None,
                                installed: true,
                            });
                        }
                    }
                }
            }

            // Other Blizzard games
            let blizzard_games = vec![
                ("Diablo IV", "drive_c/Program Files (x86)/Diablo IV", "Diablo IV.exe", "d4"),
                ("Diablo III", "drive_c/Program Files (x86)/Diablo III", "Diablo III.exe", "d3"),
                ("Overwatch", "drive_c/Program Files (x86)/Overwatch", "Overwatch.exe", "pro"),
                ("Hearthstone", "drive_c/Program Files (x86)/Hearthstone", "Hearthstone.exe", "hs"),
                ("StarCraft II", "drive_c/Program Files (x86)/StarCraft II", "StarCraft II.exe", "s2"),
            ];

            for (name, path, exe, code) in blizzard_games {
                let game_path = prefix.join(path);
                if game_path.exists() {
                    games.push(LauncherGame {
                        id: format!("battlenet_{}", code),
                        name: name.to_string(),
                        launcher: LauncherType::BattleNet,
                        launcher_id: code.to_string(),
                        install_path: game_path.clone(),
                        executable: Some(game_path.join(exe)),
                        launch_command: format!("battlenet://launch/{}", code),
                        icon: None,
                        installed: true,
                    });
                }
            }
        }

        Ok(games)
    }

    pub fn launch_game(&self, game: &LauncherGame, wine_cmd: Option<String>) -> Result<()> {
        match game.launcher {
            LauncherType::Steam => {
                // Use Steam URL protocol
                Command::new("xdg-open")
                    .arg(&game.launch_command)
                    .spawn()?;
            }
            LauncherType::BattleNet | LauncherType::Epic | LauncherType::GOG |
            LauncherType::Ubisoft | LauncherType::EA => {
                // Launch through Wine
                if let Some(wine) = wine_cmd {
                    if let Some(exe) = &game.executable {
                        Command::new(wine)
                            .arg(exe)
                            .current_dir(&game.install_path)
                            .spawn()?;
                    }
                } else {
                    return Err(anyhow::anyhow!("Wine command not specified for Windows game"));
                }
            }
            _ => {
                return Err(anyhow::anyhow!("Launcher type not supported"));
            }
        }

        Ok(())
    }

    pub fn setup_launcher(&self, launcher_type: LauncherType, _path: Option<PathBuf>) -> Result<Launcher> {
        // Implementation for manually setting up a launcher
        match launcher_type {
            LauncherType::Steam => {
                if let Some(steam) = self.detect_steam()? {
                    return Ok(steam);
                }
                Err(anyhow::anyhow!("Steam not found. Please install Steam first."))
            }
            LauncherType::BattleNet => {
                // Download and install Battle.net installer
                println!("Battle.net setup requires Wine. Please ensure Wine is installed.");
                // TODO: Implement Battle.net installer download and setup
                Err(anyhow::anyhow!("Battle.net auto-setup not yet implemented"))
            }
            _ => {
                Err(anyhow::anyhow!("Launcher setup not yet implemented for {:?}", launcher_type))
            }
        }
    }

    /// Import games from all detected launchers into the database
    pub async fn import_all_games(&self, game_lib: &crate::game::GameLibrary) -> Result<u32> {
        let launchers = self.detect_launchers()?;
        let mut total_imported = 0;

        for launcher in launchers {
            println!("🔄 Syncing games from {}...", launcher.name);
            let imported = self.import_launcher_games(&launcher, game_lib).await?;
            total_imported += imported;
            println!("✅ Imported {} games from {}", imported, launcher.name);
        }

        Ok(total_imported)
    }

    /// Import games from a specific launcher
    pub async fn import_launcher_games(&self, launcher: &Launcher, game_lib: &crate::game::GameLibrary) -> Result<u32> {
        let games = match launcher.launcher_type {
            LauncherType::Steam => self.import_steam_games(launcher, game_lib).await?,
            LauncherType::BattleNet => self.import_battlenet_games(launcher, game_lib).await?,
            LauncherType::Epic => self.import_epic_games(launcher, game_lib).await?,
            LauncherType::GOG => self.import_gog_games(launcher, game_lib).await?,
            _ => 0,
        };

        Ok(games)
    }

    async fn import_steam_games(&self, launcher: &Launcher, game_lib: &crate::game::GameLibrary) -> Result<u32> {
        let steam_games = self.sync_steam_games(launcher)?;
        let mut imported_count = 0;

        for steam_game in steam_games {
            // Convert LauncherGame to Game
            let game = crate::game::Game {
                id: steam_game.id.clone(),
                name: steam_game.name.clone(),
                executable: steam_game.executable.unwrap_or_else(|| {
                    // Try to find the main executable
                    self.find_game_executable(&steam_game.install_path)
                        .unwrap_or_else(|| steam_game.install_path.join("game.exe"))
                }),
                install_path: steam_game.install_path.clone(),
                launcher: Some("Steam".to_string()),
                launcher_id: Some(steam_game.launcher_id.clone()),
                wine_version: None, // Steam games typically don't need Wine
                wine_prefix: None,
                icon: steam_game.icon.clone(),
                banner: None,
                launch_arguments: vec![], // Steam handles launch arguments
                environment_variables: vec![],
                pre_launch_script: None,
                post_launch_script: None,
                categories: vec!["Steam".to_string()],
                tags: vec![],
                playtime_minutes: 0, // Will be updated from Steam if possible
                last_played: None,
                installed_date: chrono::Utc::now(),
                favorite: false,
                hidden: false,
                notes: None,
            };

            // Check if game already exists
            if game_lib.get_game(&game.id)?.is_none() {
                game_lib.add_game(&game)?;
                imported_count += 1;

                // Try to get ProtonDB data for the game
                if let Ok(appid) = steam_game.launcher_id.parse::<u32>() {
                    let _ = self.fetch_and_cache_protondb_data(appid, &game.name).await;
                }
            }
        }

        Ok(imported_count)
    }

    async fn import_battlenet_games(&self, launcher: &Launcher, game_lib: &crate::game::GameLibrary) -> Result<u32> {
        let battlenet_games = self.sync_battlenet_games(launcher)?;
        let mut imported_count = 0;

        for bn_game in battlenet_games {
            let game = crate::game::Game {
                id: bn_game.id.clone(),
                name: bn_game.name.clone(),
                executable: bn_game.executable.unwrap_or_else(|| {
                    self.find_game_executable(&bn_game.install_path)
                        .unwrap_or_else(|| bn_game.install_path.join("game.exe"))
                }),
                install_path: bn_game.install_path.clone(),
                launcher: Some("Battle.net".to_string()),
                launcher_id: Some(bn_game.launcher_id.clone()),
                wine_version: Some("GE-Proton".to_string()), // Default for Battle.net games
                wine_prefix: launcher.wine_prefix.clone(),
                icon: bn_game.icon.clone(),
                banner: None,
                launch_arguments: vec![],
                environment_variables: vec![
                    ("DXVK_ASYNC".to_string(), "1".to_string()),
                    ("WINEDLLOVERRIDES".to_string(), "winemenubuilder.exe=d".to_string()),
                ],
                pre_launch_script: None,
                post_launch_script: None,
                categories: vec!["Battle.net".to_string(), "Blizzard".to_string()],
                tags: vec![],
                playtime_minutes: 0,
                last_played: None,
                installed_date: chrono::Utc::now(),
                favorite: false,
                hidden: false,
                notes: Some(format!(
                    "Imported from Battle.net. Prefix: {}",
                    launcher.wine_prefix.as_ref().unwrap_or(&std::path::PathBuf::from("unknown")).display()
                )),
            };

            if game_lib.get_game(&game.id)?.is_none() {
                game_lib.add_game(&game)?;
                imported_count += 1;
            }
        }

        Ok(imported_count)
    }

    async fn import_epic_games(&self, _launcher: &Launcher, _game_lib: &crate::game::GameLibrary) -> Result<u32> {
        // TODO: Implement Epic Games import
        println!("Epic Games import not yet implemented");
        Ok(0)
    }

    async fn import_gog_games(&self, _launcher: &Launcher, _game_lib: &crate::game::GameLibrary) -> Result<u32> {
        // TODO: Implement GOG Galaxy import
        println!("GOG Galaxy import not yet implemented");
        Ok(0)
    }

    /// Find the main executable for a game
    fn find_game_executable(&self, install_path: &std::path::Path) -> Option<std::path::PathBuf> {
        if !install_path.exists() {
            return None;
        }

        // Common executable names
        let common_names = [
            "game.exe", "main.exe", "launcher.exe", "start.exe",
            "play.exe", "run.exe", "client.exe"
        ];

        // Check for common names first
        for name in &common_names {
            let exe_path = install_path.join(name);
            if exe_path.exists() {
                return Some(exe_path);
            }
        }

        // Find any .exe file in the install directory
        if let Ok(entries) = std::fs::read_dir(install_path) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("exe") {
                        // Prefer non-uninstaller executables
                        let name = path.file_name().unwrap().to_str().unwrap().to_lowercase();
                        if !name.contains("unins") && !name.contains("setup") && !name.contains("install") {
                            return Some(path);
                        }
                    }
                }
            }
        }

        None
    }

    async fn fetch_and_cache_protondb_data(&self, steam_appid: u32, game_name: &str) -> Result<()> {
        let protondb = crate::protondb::ProtonDBClient::new();
        let cache_dir = dirs::cache_dir()
            .unwrap_or_default()
            .join("ghostforge")
            .join("protondb");

        match protondb.get_game_summary(steam_appid).await {
            Ok(Some(_summary)) => {
                protondb.cache_game_data(steam_appid, &cache_dir).await?;
                println!("  📊 Cached ProtonDB data for {}", game_name);
            },
            Ok(None) => {
                println!("  ⚠️ No ProtonDB data found for {}", game_name);
            },
            Err(e) => {
                println!("  ⚠️ Failed to fetch ProtonDB data for {}: {}", game_name, e);
            }
        }

        Ok(())
    }
}