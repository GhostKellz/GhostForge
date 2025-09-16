//! Enhanced Bolt Runtime Integration for GhostForge
//!
//! This module provides comprehensive gaming container management with:
//! - ProtonDB integration for compatibility data
//! - GPU optimization (NVIDIA DLSS/Reflex)
//! - Community profile sharing
//! - Performance optimization profiles

#[cfg(feature = "container-bolt")]
use bolt::{BoltFileBuilder, BoltRuntime};
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use crate::protondb::{ProtonDBClient, ProtonDBTier};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameContainer {
    pub id: String,
    pub name: String,
    pub game_id: String,
    pub status: ContainerStatus,
    pub created: DateTime<Utc>,
    pub image: String,
    pub ports: Vec<String>,
    pub gpu_enabled: bool,
    pub wine_version: Option<String>,
    pub proton_version: Option<String>,
    pub performance_profile: String,
    pub steam_appid: Option<u32>,
    pub protondb_tier: Option<ProtonDBTier>,
    pub category: GameCategory,
    pub nvidia_config: Option<NvidiaConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GameCategory {
    Competitive,
    AAA,
    Indie,
    VR,
    Streaming,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NvidiaConfig {
    pub dlss_enabled: bool,
    pub reflex_enabled: bool,
    pub raytracing_enabled: bool,
    pub power_limit: Option<u32>,
    pub memory_clock_offset: Option<i32>,
    pub core_clock_offset: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationProfile {
    pub name: String,
    pub description: String,
    pub game_category: GameCategory,
    pub proton_version: Option<String>,
    pub wine_tricks: Vec<String>,
    pub launch_options: Vec<String>,
    pub nvidia_config: Option<NvidiaConfig>,
    pub cpu_governor: Option<String>,
    pub nice_level: Option<i32>,
    pub created: DateTime<Utc>,
    pub rating: f32,
    pub downloads: u32,
    pub author: String,
    pub compatible_games: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContainerStatus {
    Running,
    Stopped,
    Paused,
    Error(String),
    Creating,
    Updating,
}

#[derive(Debug, Clone)]
pub struct BoltSystemMetrics {
    pub running_containers: usize,
    pub total_containers: usize,
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub gpu_usage: f64,
    pub network_activity: f64,
}

pub struct BoltGameManager {
    #[cfg(feature = "container-bolt")]
    runtime: Option<BoltRuntime>,
    containers: Arc<RwLock<HashMap<String, GameContainer>>>,
    metrics: Arc<RwLock<Option<BoltSystemMetrics>>>,
    optimization_manager: OptimizationManager,
    drift_client: DriftClient,
    protondb_client: ProtonDBClient,
    #[cfg(not(feature = "container-bolt"))]
    _phantom: std::marker::PhantomData<()>,
}

/// Manages optimization profiles for games
pub struct OptimizationManager {
    profiles: Arc<RwLock<HashMap<String, OptimizationProfile>>>,
    profile_dir: std::path::PathBuf,
}

/// Client for community profile sharing (Drift registry simulation)
pub struct DriftClient {
    base_url: String,
    client: reqwest::Client,
    auth_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunityProfile {
    pub id: String,
    pub profile: OptimizationProfile,
    pub metadata: ProfileMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileMetadata {
    pub author: String,
    pub downloads: u64,
    pub rating: f32,
    pub reviews_count: u32,
    pub last_updated: DateTime<Utc>,
    pub game_compatibility: Vec<String>,
    pub gpu_vendor: Option<String>,
}

impl BoltGameManager {
    pub fn new() -> anyhow::Result<Self> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("Cannot find config directory"))?
            .join("ghostforge");

        let profile_dir = config_dir.join("profiles");
        std::fs::create_dir_all(&profile_dir)?;

        Ok(Self {
            #[cfg(feature = "container-bolt")]
            runtime: Some(
                BoltRuntime::new()
                    .map_err(|e| anyhow::anyhow!("Failed to initialize Bolt runtime: {}", e))?,
            ),
            containers: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(None)),
            optimization_manager: OptimizationManager::new(profile_dir.clone())?,
            drift_client: DriftClient::new(),
            protondb_client: ProtonDBClient::new(),
            #[cfg(not(feature = "container-bolt"))]
            _phantom: std::marker::PhantomData,
        })
    }

    /// Launch a game with ProtonDB integration and automatic optimization
    pub async fn launch_game_with_protondb(
        &self,
        game_id: &str,
        config: &crate::game::Game,
        steam_appid: Option<u32>,
    ) -> anyhow::Result<String> {
        println!("🚀 Launching {} with ProtonDB optimization...", config.name);

        // Get ProtonDB compatibility data
        let mut protondb_tier = None;
        let mut recommended_proton = None;

        if let Some(appid) = steam_appid {
            if let Ok(compatibility) = self.protondb_client.generate_compatibility_report(appid, &config.name).await {
                protondb_tier = Some(compatibility.tier.clone());
                recommended_proton = Some(compatibility.recommended_proton.clone());

                println!("📊 ProtonDB: {} - {}",
                    compatibility.tier_display,
                    compatibility.tier_description
                );

                // Apply ProtonDB recommendations
                for tip in &compatibility.compatibility_tips {
                    println!("💡 {}", tip);
                }
            }
        }

        // Detect game category for optimization
        let category = self.detect_game_category(config, steam_appid).await;

        // Get or create optimization profile
        let profile_name = format!("{}-optimized", game_id);
        let profile = self.optimization_manager.get_or_create_profile(
            &profile_name,
            &category,
            recommended_proton.as_deref(),
            protondb_tier.as_ref(),
        ).await?;

        // Launch with optimizations
        self.launch_game_optimized(game_id, config, &profile, steam_appid, protondb_tier).await
    }

    /// Launch game with full optimization pipeline
    async fn launch_game_optimized(
        &self,
        game_id: &str,
        config: &crate::game::Game,
        profile: &OptimizationProfile,
        steam_appid: Option<u32>,
        protondb_tier: Option<ProtonDBTier>,
    ) -> anyhow::Result<String> {
        #[cfg(feature = "container-bolt")]
        {
            self.launch_game_bolt(game_id, config, profile, steam_appid, protondb_tier).await
        }

        #[cfg(not(feature = "container-bolt"))]
        {
            Err(anyhow::anyhow!("Bolt support not compiled in"))
        }
    }

    #[cfg(feature = "container-bolt")]
    async fn launch_game_bolt(
        &self,
        game_id: &str,
        config: &crate::game::Game,
        profile: &OptimizationProfile,
        steam_appid: Option<u32>,
        protondb_tier: Option<ProtonDBTier>,
    ) -> anyhow::Result<String> {
        println!("🎮 Launching {} with profile: {}", config.name, profile.name);

        let runtime = self.runtime.as_ref().ok_or_else(|| anyhow::anyhow!("Bolt runtime not initialized"))?;

        // Use profile's Proton version or default
        let proton_version = profile.proton_version.as_deref().unwrap_or("GE-Proton8-26");

        // Setup gaming environment with profile optimizations
        runtime.setup_gaming(Some(proton_version), Some("win10")).await
            .map_err(|e| anyhow::anyhow!("Failed to setup gaming environment: {}", e))?;

        // Configure NVIDIA optimizations if available
        let nvidia_config = profile.nvidia_config.clone().unwrap_or_else(|| {
            self.create_nvidia_config_for_category(&profile.game_category)
        });

        // Apply system optimizations
        self.apply_system_optimizations(profile).await?;

        // Create container with optimizations
        let container_name = format!("ghostforge-{}", game_id);
        let env_vars = self.build_optimized_environment(config, profile, &nvidia_config);

        runtime.run_container(
            "bolt://gaming-optimized:latest",
            Some(&container_name),
            &[], // ports
            &[format!("{}:/game", config.install_path.to_string_lossy())], // volumes
            &env_vars,
            true, // detached
        ).await.map_err(|e| anyhow::anyhow!("Failed to launch container: {}", e))?;

        // Launch the game
        let launch_cmd = self.build_launch_command(config, profile);
        runtime.launch_game(&launch_cmd, &profile.launch_options)
            .await.map_err(|e| anyhow::anyhow!("Failed to launch game: {}", e))?;

        // Create enhanced container tracking
        let game_container = GameContainer {
            id: container_name.clone(),
            name: config.name.clone(),
            game_id: game_id.to_string(),
            status: ContainerStatus::Running,
            created: Utc::now(),
            image: "bolt://gaming-optimized:latest".to_string(),
            ports: vec![],
            gpu_enabled: true,
            wine_version: config.wine_version.clone(),
            proton_version: Some(proton_version.to_string()),
            performance_profile: profile.name.clone(),
            steam_appid,
            protondb_tier,
            category: profile.game_category.clone(),
            nvidia_config: Some(nvidia_config),
        };

        self.containers.write().insert(container_name.clone(), game_container);

        println!("✅ {} launched successfully with {} profile", config.name, profile.name);
        Ok(container_name)
    }

    /// Legacy launch method for backward compatibility
    pub async fn launch_game(
        &self,
        game_id: &str,
        config: &crate::game::Game,
    ) -> anyhow::Result<String> {
        // For backward compatibility, try to get Steam AppID and use enhanced launch
        let steam_appid = if let Some(launcher) = &config.launcher {
            if launcher.contains("steam") {
                self.protondb_client.get_steam_appid(&config.name).await.unwrap_or(None)
            } else {
                None
            }
        } else {
            None
        };

        self.launch_game_with_protondb(game_id, config, steam_appid).await
    }

    /// Detect game category for optimization
    async fn detect_game_category(&self, config: &crate::game::Game, steam_appid: Option<u32>) -> GameCategory {
        // Check game name for category hints
        let name_lower = config.name.to_lowercase();

        // Competitive games
        if name_lower.contains("counter-strike") || name_lower.contains("cs2") ||
           name_lower.contains("valorant") || name_lower.contains("overwatch") ||
           name_lower.contains("rocket league") || name_lower.contains("dota") ||
           name_lower.contains("league of legends") {
            return GameCategory::Competitive;
        }

        // VR games
        if name_lower.contains("vr") || name_lower.contains("virtual reality") ||
           name_lower.contains("half-life: alyx") || name_lower.contains("beat saber") {
            return GameCategory::VR;
        }

        // AAA games (common publishers/franchises)
        if name_lower.contains("call of duty") || name_lower.contains("battlefield") ||
           name_lower.contains("cyberpunk") || name_lower.contains("witcher") ||
           name_lower.contains("assassin's creed") || name_lower.contains("red dead") ||
           name_lower.contains("grand theft auto") || name_lower.contains("gta") {
            return GameCategory::AAA;
        }

        // Use ProtonDB data if available
        if let Some(appid) = steam_appid {
            if let Ok(Some(summary)) = self.protondb_client.get_game_summary(appid).await {
                // High-profile games with many reports are likely AAA
                if summary.total > 1000 {
                    return GameCategory::AAA;
                } else if summary.total > 100 {
                    return GameCategory::Indie;
                }
            }
        }

        GameCategory::Unknown
    }

    /// Create NVIDIA config based on game category
    fn create_nvidia_config_for_category(&self, category: &GameCategory) -> NvidiaConfig {
        match category {
            GameCategory::Competitive => NvidiaConfig {
                dlss_enabled: false, // Minimize latency
                reflex_enabled: true,
                raytracing_enabled: false,
                power_limit: Some(110),
                memory_clock_offset: Some(1000),
                core_clock_offset: Some(200),
            },
            GameCategory::AAA => NvidiaConfig {
                dlss_enabled: true,
                reflex_enabled: false,
                raytracing_enabled: true,
                power_limit: Some(100),
                memory_clock_offset: Some(500),
                core_clock_offset: Some(100),
            },
            GameCategory::VR => NvidiaConfig {
                dlss_enabled: true,
                reflex_enabled: false, // VR has different latency requirements
                raytracing_enabled: false, // Too demanding for VR
                power_limit: Some(110),
                memory_clock_offset: Some(800),
                core_clock_offset: Some(150),
            },
            _ => NvidiaConfig {
                dlss_enabled: false,
                reflex_enabled: false,
                raytracing_enabled: false,
                power_limit: Some(100),
                memory_clock_offset: Some(0),
                core_clock_offset: Some(0),
            },
        }
    }

    /// Apply system-level optimizations
    async fn apply_system_optimizations(&self, profile: &OptimizationProfile) -> anyhow::Result<()> {
        // Set CPU governor
        if let Some(governor) = &profile.cpu_governor {
            let _ = std::process::Command::new("cpupower")
                .args(&["frequency-set", "-g", governor])
                .status();
        }

        // Set process priority
        if let Some(nice) = profile.nice_level {
            let _ = std::process::Command::new("renice")
                .args(&["-n", &nice.to_string(), "-p", &std::process::id().to_string()])
                .status();
        }

        Ok(())
    }

    /// Build optimized environment variables
    fn build_optimized_environment(&self, config: &crate::game::Game, profile: &OptimizationProfile, nvidia_config: &NvidiaConfig) -> Vec<String> {
        let mut env_vars = vec![
            "DISPLAY=:0".to_string(),
            "NVIDIA_VISIBLE_DEVICES=all".to_string(),
        ];

        // NVIDIA optimizations
        if nvidia_config.dlss_enabled {
            env_vars.push("NVIDIA_DLSS_ENABLED=1".to_string());
        }
        if nvidia_config.reflex_enabled {
            env_vars.push("NVIDIA_REFLEX_ENABLED=1".to_string());
            env_vars.push("__GL_YIELD=USLEEP".to_string());
        }
        if nvidia_config.raytracing_enabled {
            env_vars.push("NVIDIA_RTX_ENABLED=1".to_string());
        }

        // Wine/Proton environment
        if let Some(proton_version) = &profile.proton_version {
            env_vars.push(format!("PROTON_VERSION={}", proton_version));
        }
        env_vars.push("WINEARCH=win64".to_string());
        env_vars.push(format!("WINEPREFIX=/wine-prefix/{}", config.name.replace(" ", "-")));

        // Apply profile launch options as environment variables
        for option in &profile.launch_options {
            if option.contains("=") {
                env_vars.push(option.clone());
            }
        }

        env_vars
    }

    /// Build launch command for game
    fn build_launch_command(&self, config: &crate::game::Game, _profile: &OptimizationProfile) -> String {
        if let Some(launcher) = &config.launcher {
            if launcher.contains("steam") {
                // Try to extract Steam AppID
                format!("steam steam://run/{}", config.name.replace(" ", ""))
            } else {
                format!("wine '{}'", config.executable.to_string_lossy())
            }
        } else {
            format!("wine '{}'", config.executable.to_string_lossy())
        }
    }

    /// Scan and optimize entire Steam library
    pub async fn scan_and_optimize_steam_library(&self) -> anyhow::Result<Vec<OptimizationProfile>> {
        println!("🔍 Scanning Steam library for optimization...");

        // This would scan Steam's library and create optimized profiles
        // For now, create some example profiles
        let mut created_profiles = Vec::new();

        // Create competitive gaming profile
        let competitive_profile = OptimizationProfile {
            name: "competitive-gaming".to_string(),
            description: "Optimized for competitive FPS games with minimal latency".to_string(),
            game_category: GameCategory::Competitive,
            proton_version: Some("GE-Proton8-26".to_string()),
            wine_tricks: vec!["vcrun2019".to_string()],
            launch_options: vec![
                "PROTON_NO_ESYNC=1".to_string(),
                "__GL_YIELD=USLEEP".to_string(),
            ],
            nvidia_config: Some(self.create_nvidia_config_for_category(&GameCategory::Competitive)),
            cpu_governor: Some("performance".to_string()),
            nice_level: Some(-15),
            created: Utc::now(),
            rating: 4.8,
            downloads: 0,
            author: "system".to_string(),
            compatible_games: vec!["Counter-Strike 2".to_string(), "Valorant".to_string()],
        };

        self.optimization_manager.save_profile(&competitive_profile).await?;
        created_profiles.push(competitive_profile);

        // Create AAA gaming profile
        let aaa_profile = OptimizationProfile {
            name: "aaa-gaming".to_string(),
            description: "Optimized for AAA games with DLSS and Ray Tracing".to_string(),
            game_category: GameCategory::AAA,
            proton_version: Some("GE-Proton8-26".to_string()),
            wine_tricks: vec!["vcrun2019".to_string(), "corefonts".to_string()],
            launch_options: vec![],
            nvidia_config: Some(self.create_nvidia_config_for_category(&GameCategory::AAA)),
            cpu_governor: Some("performance".to_string()),
            nice_level: Some(-10),
            created: Utc::now(),
            rating: 4.6,
            downloads: 0,
            author: "system".to_string(),
            compatible_games: vec!["Cyberpunk 2077".to_string(), "The Witcher 3".to_string()],
        };

        self.optimization_manager.save_profile(&aaa_profile).await?;
        created_profiles.push(aaa_profile);

        println!("✅ Created {} optimization profiles", created_profiles.len());
        Ok(created_profiles)
    }

    /// Get ProtonDB client for external use
    pub fn protondb(&self) -> &ProtonDBClient {
        &self.protondb_client
    }

    /// Get optimization manager for external use
    pub fn optimization_manager(&self) -> &OptimizationManager {
        &self.optimization_manager
    }

    /// Get drift client for community features
    pub fn drift_client(&self) -> &DriftClient {
        &self.drift_client
    }

    // Existing methods for compatibility...

    #[cfg(feature = "container-bolt")]
    pub async fn stop_game(&self, container_id: &str) -> anyhow::Result<()> {
        let runtime = self
            .runtime
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Bolt runtime not initialized"))?;

        runtime
            .stop_container(container_id)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to stop container: {}", e))?;

        // Update container status
        if let Some(container) = self.containers.write().get_mut(container_id) {
            container.status = ContainerStatus::Stopped;
        }

        Ok(())
    }

    #[cfg(not(feature = "container-bolt"))]
    pub async fn stop_game(&self, _container_id: &str) -> anyhow::Result<()> {
        Err(anyhow::anyhow!("Bolt support not compiled in"))
    }

    pub fn get_containers(&self) -> Vec<GameContainer> {
        self.containers.read().values().cloned().collect()
    }

    #[cfg(feature = "container-bolt")]
    pub async fn get_system_metrics(&self) -> anyhow::Result<BoltSystemMetrics> {
        // Simplified metrics for now
        Ok(BoltSystemMetrics {
            running_containers: self.containers.read().values().filter(|c| matches!(c.status, ContainerStatus::Running)).count(),
            total_containers: self.containers.read().len(),
            cpu_usage: 0.0,
            memory_usage: 0.0,
            gpu_usage: 0.0,
            network_activity: 0.0,
        })
    }

    #[cfg(not(feature = "container-bolt"))]
    pub async fn get_system_metrics(&self) -> anyhow::Result<BoltSystemMetrics> {
        Ok(BoltSystemMetrics {
            running_containers: 0,
            total_containers: 0,
            cpu_usage: 0.0,
            memory_usage: 0.0,
            gpu_usage: 0.0,
            network_activity: 0.0,
        })
    }

    pub fn get_cached_metrics(&self) -> Option<BoltSystemMetrics> {
        self.metrics.read().clone()
    }
}

// Implementation for OptimizationManager
impl OptimizationManager {
    pub fn new(profile_dir: std::path::PathBuf) -> anyhow::Result<Self> {
        std::fs::create_dir_all(&profile_dir)?;

        let mut profiles = HashMap::new();

        // Load existing profiles
        if let Ok(entries) = std::fs::read_dir(&profile_dir) {
            for entry in entries.flatten() {
                if let Some(ext) = entry.path().extension() {
                    if ext == "json" {
                        if let Ok(content) = std::fs::read_to_string(entry.path()) {
                            if let Ok(profile) = serde_json::from_str::<OptimizationProfile>(&content) {
                                profiles.insert(profile.name.clone(), profile);
                            }
                        }
                    }
                }
            }
        }

        Ok(Self {
            profiles: Arc::new(RwLock::new(profiles)),
            profile_dir,
        })
    }

    pub async fn get_or_create_profile(
        &self,
        name: &str,
        category: &GameCategory,
        proton_version: Option<&str>,
        protondb_tier: Option<&ProtonDBTier>,
    ) -> anyhow::Result<OptimizationProfile> {
        // Check if profile already exists
        if let Some(profile) = self.profiles.read().get(name) {
            return Ok(profile.clone());
        }

        // Create new profile based on category and ProtonDB data
        let wine_tricks = if let Some(tier) = protondb_tier {
            match tier {
                ProtonDBTier::Platinum => vec![],
                ProtonDBTier::Gold | ProtonDBTier::Silver => vec!["vcrun2019".to_string()],
                _ => vec!["vcrun2019".to_string(), "corefonts".to_string(), "dotnet48".to_string()],
            }
        } else {
            vec!["vcrun2019".to_string()]
        };

        let launch_options = if let Some(tier) = protondb_tier {
            match tier {
                ProtonDBTier::Platinum => vec![],
                ProtonDBTier::Gold => vec!["PROTON_USE_WINED3D=1".to_string()],
                _ => vec!["PROTON_USE_WINED3D=1".to_string(), "PROTON_NO_ESYNC=1".to_string()],
            }
        } else {
            vec![]
        };

        let profile = OptimizationProfile {
            name: name.to_string(),
            description: format!("Auto-generated profile for {:?} games", category),
            game_category: category.clone(),
            proton_version: proton_version.map(|s| s.to_string()).or_else(|| Some("GE-Proton8-26".to_string())),
            wine_tricks,
            launch_options,
            nvidia_config: None, // Will be set by caller
            cpu_governor: Some("performance".to_string()),
            nice_level: Some(-10),
            created: Utc::now(),
            rating: 0.0,
            downloads: 0,
            author: "auto".to_string(),
            compatible_games: vec![],
        };

        self.save_profile(&profile).await?;
        Ok(profile)
    }

    pub async fn save_profile(&self, profile: &OptimizationProfile) -> anyhow::Result<()> {
        let profile_file = self.profile_dir.join(format!("{}.json", profile.name));
        let json = serde_json::to_string_pretty(profile)?;
        std::fs::write(profile_file, json)?;

        self.profiles.write().insert(profile.name.clone(), profile.clone());
        Ok(())
    }

    pub fn list_profiles(&self) -> Vec<OptimizationProfile> {
        self.profiles.read().values().cloned().collect()
    }

    pub fn get_profile(&self, name: &str) -> Option<OptimizationProfile> {
        self.profiles.read().get(name).cloned()
    }

    pub async fn delete_profile(&self, name: &str) -> anyhow::Result<()> {
        let profile_file = self.profile_dir.join(format!("{}.json", name));
        if profile_file.exists() {
            std::fs::remove_file(profile_file)?;
        }
        self.profiles.write().remove(name);
        Ok(())
    }
}

// Implementation for DriftClient (community features)
impl DriftClient {
    pub fn new() -> Self {
        Self {
            base_url: "https://registry.ghostforge.dev".to_string(),
            client: reqwest::Client::new(),
            auth_token: None,
        }
    }

    pub async fn search_profiles(&self, query: &str, category: Option<&GameCategory>) -> anyhow::Result<Vec<CommunityProfile>> {
        // Simulate community profile search
        // In real implementation, this would query the Drift registry
        let mut profiles = Vec::new();

        // Mock some community profiles
        if query.to_lowercase().contains("competitive") ||
           (category.is_some() && matches!(category.unwrap(), GameCategory::Competitive)) {
            profiles.push(CommunityProfile {
                id: "esports-cs2".to_string(),
                profile: OptimizationProfile {
                    name: "esports-cs2".to_string(),
                    description: "Professional eSports configuration for Counter-Strike 2".to_string(),
                    game_category: GameCategory::Competitive,
                    proton_version: Some("GE-Proton8-26".to_string()),
                    wine_tricks: vec![],
                    launch_options: vec![
                        "PROTON_NO_ESYNC=1".to_string(),
                        "__GL_YIELD=USLEEP".to_string(),
                        "-high".to_string(),
                        "-threads 8".to_string(),
                    ],
                    nvidia_config: Some(NvidiaConfig {
                        dlss_enabled: false,
                        reflex_enabled: true,
                        raytracing_enabled: false,
                        power_limit: Some(115),
                        memory_clock_offset: Some(1200),
                        core_clock_offset: Some(250),
                    }),
                    cpu_governor: Some("performance".to_string()),
                    nice_level: Some(-20),
                    created: Utc::now(),
                    rating: 4.9,
                    downloads: 15420,
                    author: "pro_gamer_2024".to_string(),
                    compatible_games: vec!["Counter-Strike 2".to_string()],
                },
                metadata: ProfileMetadata {
                    author: "pro_gamer_2024".to_string(),
                    downloads: 15420,
                    rating: 4.9,
                    reviews_count: 342,
                    last_updated: Utc::now(),
                    game_compatibility: vec!["Counter-Strike 2".to_string()],
                    gpu_vendor: Some("nvidia".to_string()),
                },
            });
        }

        Ok(profiles)
    }

    pub async fn install_profile(&self, profile_id: &str, profile_dir: &std::path::Path) -> anyhow::Result<OptimizationProfile> {
        // Simulate downloading and installing a community profile
        let profiles = self.search_profiles(profile_id, None).await?;

        if let Some(community_profile) = profiles.into_iter().find(|p| p.id == profile_id) {
            let profile_file = profile_dir.join(format!("{}.json", community_profile.profile.name));
            let json = serde_json::to_string_pretty(&community_profile.profile)?;
            std::fs::write(profile_file, json)?;

            println!("✅ Installed community profile: {} ({}⭐ {} downloads)",
                community_profile.profile.name,
                community_profile.metadata.rating,
                community_profile.metadata.downloads
            );

            Ok(community_profile.profile)
        } else {
            Err(anyhow::anyhow!("Profile not found: {}", profile_id))
        }
    }

    pub async fn share_profile(&self, profile: &OptimizationProfile) -> anyhow::Result<String> {
        // Simulate uploading profile to community registry
        println!("🌍 Sharing profile '{}' to community...", profile.name);

        // In real implementation, this would upload to Drift registry
        // For now, just simulate success
        let profile_id = format!("{}-{}", profile.author, profile.name.replace(" ", "-"));

        println!("✅ Profile shared successfully! ID: {}", profile_id);
        Ok(profile_id)
    }

    pub async fn rate_profile(&self, profile_id: &str, rating: f32) -> anyhow::Result<()> {
        println!("⭐ Rated profile '{}' with {:.1}/5.0 stars", profile_id, rating);
        Ok(())
    }

    pub async fn get_trending_profiles(&self, limit: usize) -> anyhow::Result<Vec<CommunityProfile>> {
        // Return mock trending profiles
        let all_profiles = self.search_profiles("", None).await?;
        let mut trending = all_profiles;
        trending.sort_by(|a, b| b.metadata.downloads.cmp(&a.metadata.downloads));
        trending.truncate(limit);
        Ok(trending)
    }
}

impl Default for BoltGameManager {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| {
            let config_dir = dirs::config_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
                .join("ghostforge");
            let profile_dir = config_dir.join("profiles");
            let _ = std::fs::create_dir_all(&profile_dir);

            Self {
                #[cfg(feature = "container-bolt")]
                runtime: None,
                containers: Arc::new(RwLock::new(HashMap::new())),
                metrics: Arc::new(RwLock::new(None)),
                optimization_manager: OptimizationManager::new(profile_dir).unwrap_or_else(|_| {
                    OptimizationManager {
                        profiles: Arc::new(RwLock::new(HashMap::new())),
                        profile_dir: std::path::PathBuf::from("/tmp/profiles"),
                    }
                }),
                drift_client: DriftClient::new(),
                protondb_client: ProtonDBClient::new(),
                #[cfg(not(feature = "container-bolt"))]
                _phantom: std::marker::PhantomData,
            }
        })
    }
}