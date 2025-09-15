//! Bolt Runtime Integration for GhostForge
//!
//! This module provides the bridge between GhostForge and the Bolt container runtime,
//! enabling advanced gaming container management with GPU passthrough, Wine/Proton
//! integration, and performance optimization.

#[cfg(feature = "container-bolt")]
use bolt::{BoltRuntime, BoltFileBuilder};
use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

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
    #[cfg(not(feature = "container-bolt"))]
    _phantom: std::marker::PhantomData<()>,
}

impl BoltGameManager {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            #[cfg(feature = "container-bolt")]
            runtime: Some(BoltRuntime::new().map_err(|e| anyhow::anyhow!("Failed to initialize Bolt runtime: {}", e))?),
            containers: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(None)),
            #[cfg(not(feature = "container-bolt"))]
            _phantom: std::marker::PhantomData,
        })
    }

    #[cfg(feature = "container-bolt")]
    pub async fn launch_game(&self, game_id: &str, config: &crate::game::Game) -> anyhow::Result<String> {
        let runtime = self.runtime.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Bolt runtime not initialized"))?;

        // Use real Bolt API to setup gaming environment
        let proton_version = config.wine_version.as_deref().unwrap_or("8.0");
        let os_image = "win10";

        // Setup gaming environment with Proton
        runtime.setup_gaming(Some(proton_version), Some(os_image)).await
            .map_err(|e| anyhow::anyhow!("Failed to setup gaming environment: {}", e))?;

        // Create container name
        let container_name = format!("ghostforge-{}", game_id);

        // Launch game container with optimizations
        let game_path = config.install_path.to_string_lossy();
        let executable_path = config.executable.to_string_lossy();

        let steam_url = if config.launcher.as_ref().map(|l| l.contains("steam")).unwrap_or(false) {
            format!("steam://run/{}", game_id)
        } else {
            format!("wine '{}'", executable_path)
        };

        // Use Bolt's gaming container with GPU passthrough
        runtime.run_container(
            "bolt://gaming-optimized:latest",
            Some(&container_name),
            &[], // ports
            &[format!("{}:/game", game_path)], // volumes
            &[
                "WINE_PREFIX=/wine-prefix".to_string(),
                format!("PROTON_VERSION={}", proton_version),
                "DISPLAY=:0".to_string(),
                "PULSE_RUNTIME_PATH=/run/user/1000/pulse".to_string(),
                "NVIDIA_VISIBLE_DEVICES=all".to_string(),
                "NVIDIA_DRIVER_CAPABILITIES=all".to_string(),
            ],
            true // detached
        ).await.map_err(|e| anyhow::anyhow!("Failed to launch gaming container: {}", e))?;

        // Launch the actual game
        runtime.launch_game(&steam_url, &[]).await
            .map_err(|e| anyhow::anyhow!("Failed to launch game: {}", e))?;

        // Create container tracking
        let game_container = GameContainer {
            id: container_name.clone(),
            name: config.name.clone(),
            game_id: game_id.to_string(),
            status: ContainerStatus::Running,
            created: Utc::now(),
            image: "bolt://gaming-optimized:latest".to_string(),
            ports: vec!["27015:27015".to_string()], // Common Steam port
            gpu_enabled: true,
            wine_version: config.wine_version.clone(),
            proton_version: Some(proton_version.to_string()),
            performance_profile: "Gaming".to_string(),
        };

        self.containers.write().insert(container_name.clone(), game_container);

        println!("🎮 Launched {} in Bolt gaming container: {}", config.name, container_name);
        Ok(container_name)
    }

    #[cfg(not(feature = "container-bolt"))]
    pub async fn launch_game(&self, game_id: &str, _config: &crate::game::Game) -> anyhow::Result<String> {
        Err(anyhow::anyhow!("Bolt support not compiled in. Enable the 'container-bolt' feature."))
    }

    #[cfg(feature = "container-bolt")]
    pub async fn stop_game(&self, container_id: &str) -> anyhow::Result<()> {
        let runtime = self.runtime.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Bolt runtime not initialized"))?;

        runtime.stop_container(container_id).await
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

    #[cfg(feature = "container-bolt")]
    pub async fn list_game_containers(&self) -> anyhow::Result<Vec<GameContainer>> {
        let runtime = self.runtime.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Bolt runtime not initialized"))?;

        let containers = runtime.list_containers(false).await
            .map_err(|e| anyhow::anyhow!("Failed to list containers: {}", e))?;

        let mut game_containers = Vec::new();
        let container_map = self.containers.read();

        for container in containers {
            if container.name.starts_with("ghostforge-") {
                if let Some(game_container) = container_map.get(&container.id) {
                    let mut updated_container = game_container.clone();
                    updated_container.status = match container.status.as_str() {
                        "running" => ContainerStatus::Running,
                        "exited" => ContainerStatus::Stopped,
                        "paused" => ContainerStatus::Paused,
                        _ => ContainerStatus::Error(container.status),
                    };
                    game_containers.push(updated_container);
                }
            }
        }

        Ok(game_containers)
    }

    #[cfg(not(feature = "container-bolt"))]
    pub async fn list_game_containers(&self) -> anyhow::Result<Vec<GameContainer>> {
        Ok(vec![])
    }

    #[cfg(feature = "container-bolt")]
    pub async fn get_system_metrics(&self) -> anyhow::Result<BoltSystemMetrics> {
        let runtime = self.runtime.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Bolt runtime not initialized"))?;

        let containers = runtime.list_containers(true).await
            .map_err(|e| anyhow::anyhow!("Failed to get container metrics: {}", e))?;

        let running_containers = containers.iter()
            .filter(|c| c.status == "running")
            .count();

        let metrics = BoltSystemMetrics {
            running_containers,
            total_containers: containers.len(),
            cpu_usage: 0.0, // TODO: Implement real metrics
            memory_usage: 0.0,
            gpu_usage: 0.0,
            network_activity: 0.0,
        };

        *self.metrics.write() = Some(metrics.clone());
        Ok(metrics)
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

    pub fn get_containers(&self) -> Vec<GameContainer> {
        self.containers.read().values().cloned().collect()
    }

    pub fn get_cached_metrics(&self) -> Option<BoltSystemMetrics> {
        self.metrics.read().clone()
    }

    #[cfg(feature = "container-bolt")]
    pub async fn create_game_environment(&self, game_name: &str, wine_version: &str) -> anyhow::Result<()> {
        let runtime = self.runtime.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Bolt runtime not initialized"))?;

        // Setup gaming environment
        runtime.setup_gaming(Some(wine_version), Some("win10")).await
            .map_err(|e| anyhow::anyhow!("Failed to setup gaming environment: {}", e))?;

        // Create high-performance gaming network
        let network_name = format!("{}-gaming-net", game_name.to_lowercase().replace(" ", "-"));
        runtime.create_network(&network_name, "bolt", Some("10.1.0.0/16")).await
            .map_err(|e| anyhow::anyhow!("Failed to create gaming network: {}", e))?;

        Ok(())
    }

    #[cfg(not(feature = "container-bolt"))]
    pub async fn create_game_environment(&self, _game_name: &str, _wine_version: &str) -> anyhow::Result<()> {
        Err(anyhow::anyhow!("Bolt support not compiled in"))
    }
}

impl Default for BoltGameManager {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            #[cfg(feature = "container-bolt")]
            runtime: None,
            containers: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(None)),
            #[cfg(not(feature = "container-bolt"))]
            _phantom: std::marker::PhantomData,
        })
    }
}