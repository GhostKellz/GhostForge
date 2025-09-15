use anyhow::Result;
use dirs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub general: GeneralConfig,
    pub wine: WineConfig,
    pub gpu: GpuConfig,
    pub paths: PathsConfig,
    pub launchers: LaunchersConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub default_wine_version: String,
    pub enable_gamemode: bool,
    pub enable_mangohud: bool,
    pub enable_fsync: bool,
    pub enable_esync: bool,
    pub enable_dxvk: bool,
    pub enable_vkd3d: bool,
    pub cpu_governor: String,
    pub log_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WineConfig {
    pub default_prefix_path: PathBuf,
    pub wine_versions_path: PathBuf,
    pub winetricks_path: Option<PathBuf>,
    pub dxvk_versions_path: PathBuf,
    pub vkd3d_versions_path: PathBuf,
    pub default_arch: String,            // win32 or win64
    pub default_windows_version: String, // win10, win7, etc.
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuConfig {
    pub nvidia_prime_render_offload: bool,
    pub amd_vulkan_icd: String,
    pub intel_vulkan_icd: String,
    pub enable_nvapi: bool,
    pub enable_nvidia_ngx: bool,
    pub enable_dlss: bool,
    pub enable_ray_tracing: bool,
    pub vram_limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathsConfig {
    pub games_library: PathBuf,
    pub downloads: PathBuf,
    pub backups: PathBuf,
    pub logs: PathBuf,
    pub cache: PathBuf,
    pub database: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchersConfig {
    pub steam: Option<LauncherConfig>,
    pub battlenet: Option<LauncherConfig>,
    pub epic: Option<LauncherConfig>,
    pub gog: Option<LauncherConfig>,
    pub ubisoft: Option<LauncherConfig>,
    pub ea: Option<LauncherConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LauncherConfig {
    pub enabled: bool,
    pub path: PathBuf,
    pub wine_version: Option<String>,
    pub prefix_path: Option<PathBuf>,
    pub auto_sync: bool,
}

impl Default for Config {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/home/user"));
        let _config_dir = dirs::config_dir()
            .unwrap_or_else(|| home.join(".config"))
            .join("ghostforge");
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| home.join(".local/share"))
            .join("ghostforge");
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| home.join(".cache"))
            .join("ghostforge");

        Self {
            general: GeneralConfig {
                default_wine_version: "system".to_string(),
                enable_gamemode: true,
                enable_mangohud: false,
                enable_fsync: true,
                enable_esync: true,
                enable_dxvk: true,
                enable_vkd3d: true,
                cpu_governor: "ondemand".to_string(),
                log_level: "info".to_string(),
            },
            wine: WineConfig {
                default_prefix_path: data_dir.join("prefixes"),
                wine_versions_path: data_dir.join("wine-versions"),
                winetricks_path: None,
                dxvk_versions_path: data_dir.join("dxvk-versions"),
                vkd3d_versions_path: data_dir.join("vkd3d-versions"),
                default_arch: "win64".to_string(),
                default_windows_version: "win10".to_string(),
            },
            gpu: GpuConfig {
                nvidia_prime_render_offload: false,
                amd_vulkan_icd: "radv".to_string(),
                intel_vulkan_icd: "anv".to_string(),
                enable_nvapi: true,
                enable_nvidia_ngx: true,
                enable_dlss: true,
                enable_ray_tracing: true,
                vram_limit: None,
            },
            paths: PathsConfig {
                games_library: home.join("Games"),
                downloads: home.join("Downloads/ghostforge"),
                backups: data_dir.join("backups"),
                logs: data_dir.join("logs"),
                cache: cache_dir,
                database: data_dir.join("ghostforge.db"),
            },
            launchers: LaunchersConfig {
                steam: None,
                battlenet: None,
                epic: None,
                gog: None,
                ubisoft: None,
                ea: None,
            },
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path();

        if config_path.exists() {
            let contents = std::fs::read_to_string(&config_path)?;
            let config: Config = toml::from_str(&contents)?;
            Ok(config)
        } else {
            let config = Config::default();
            config.save()?;
            Ok(config)
        }
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path();

        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let contents = toml::to_string_pretty(self)?;
        std::fs::write(&config_path, contents)?;

        Ok(())
    }

    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| dirs::home_dir().unwrap().join(".config"))
            .join("ghostforge")
            .join("config.toml")
    }

    pub fn ensure_directories(&self) -> Result<()> {
        std::fs::create_dir_all(&self.paths.games_library)?;
        std::fs::create_dir_all(&self.paths.downloads)?;
        std::fs::create_dir_all(&self.paths.backups)?;
        std::fs::create_dir_all(&self.paths.logs)?;
        std::fs::create_dir_all(&self.paths.cache)?;
        std::fs::create_dir_all(&self.wine.default_prefix_path)?;
        std::fs::create_dir_all(&self.wine.wine_versions_path)?;
        std::fs::create_dir_all(&self.wine.dxvk_versions_path)?;
        std::fs::create_dir_all(&self.wine.vkd3d_versions_path)?;

        if let Some(parent) = self.paths.database.parent() {
            std::fs::create_dir_all(parent)?;
        }

        Ok(())
    }

    pub fn get_launcher(&self, name: &str) -> Option<&LauncherConfig> {
        match name.to_lowercase().as_str() {
            "steam" => self.launchers.steam.as_ref(),
            "battlenet" | "battle.net" => self.launchers.battlenet.as_ref(),
            "epic" => self.launchers.epic.as_ref(),
            "gog" => self.launchers.gog.as_ref(),
            "ubisoft" | "uplay" => self.launchers.ubisoft.as_ref(),
            "ea" | "origin" => self.launchers.ea.as_ref(),
            _ => None,
        }
    }
}
