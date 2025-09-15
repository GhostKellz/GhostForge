//! Bolt Runtime Integration for GhostForge
//!
//! This module provides the bridge between GhostForge and the Bolt container runtime,
//! enabling advanced gaming container management with GPU passthrough, Wine/Proton
//! integration, and performance optimization.

#[cfg(feature = "container-bolt")]
use bolt::{BoltFileBuilder, BoltRuntime};
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

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
            runtime: Some(
                BoltRuntime::new()
                    .map_err(|e| anyhow::anyhow!("Failed to initialize Bolt runtime: {}", e))?,
            ),
            containers: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(None)),
            #[cfg(not(feature = "container-bolt"))]
            _phantom: std::marker::PhantomData,
        })
    }

    #[cfg(feature = "container-bolt")]
    pub async fn launch_game(
        &self,
        game_id: &str,
        config: &crate::game::Game,
    ) -> anyhow::Result<String> {
        let runtime = self
            .runtime
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Bolt runtime not initialized"))?;

        // Use real Bolt API to setup gaming environment
        let proton_version = config.wine_version.as_deref().unwrap_or("8.0");
        let os_image = "win10";

        // Setup gaming environment with Proton
        runtime
            .setup_gaming(Some(proton_version), Some(os_image))
            .await
            .map_err(|e| anyhow::anyhow!("Failed to setup gaming environment: {}", e))?;

        // Create container name
        let container_name = format!("ghostforge-{}", game_id);

        // Launch game container with optimizations
        let game_path = config.install_path.to_string_lossy();
        let executable_path = config.executable.to_string_lossy();

        let steam_url = if config
            .launcher
            .as_ref()
            .map(|l| l.contains("steam"))
            .unwrap_or(false)
        {
            format!("steam://run/{}", game_id)
        } else {
            format!("wine '{}'", executable_path)
        };

        // Use Bolt's gaming container with GPU passthrough
        runtime
            .run_container(
                "bolt://gaming-optimized:latest",
                Some(&container_name),
                &[],                               // ports
                &[format!("{}:/game", game_path)], // volumes
                &[
                    "WINE_PREFIX=/wine-prefix".to_string(),
                    format!("PROTON_VERSION={}", proton_version),
                    "DISPLAY=:0".to_string(),
                    "PULSE_RUNTIME_PATH=/run/user/1000/pulse".to_string(),
                    "NVIDIA_VISIBLE_DEVICES=all".to_string(),
                    "NVIDIA_DRIVER_CAPABILITIES=all".to_string(),
                ],
                true, // detached
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to launch gaming container: {}", e))?;

        // Launch the actual game
        runtime
            .launch_game(&steam_url, &[])
            .await
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

        self.containers
            .write()
            .insert(container_name.clone(), game_container);

        println!(
            "🎮 Launched {} in Bolt gaming container: {}",
            config.name, container_name
        );
        Ok(container_name)
    }

    #[cfg(not(feature = "container-bolt"))]
    pub async fn launch_game(
        &self,
        game_id: &str,
        _config: &crate::game::Game,
    ) -> anyhow::Result<String> {
        Err(anyhow::anyhow!(
            "Bolt support not compiled in. Enable the 'container-bolt' feature."
        ))
    }

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

    #[cfg(feature = "container-bolt")]
    pub async fn list_game_containers(&self) -> anyhow::Result<Vec<GameContainer>> {
        let runtime = self
            .runtime
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Bolt runtime not initialized"))?;

        let containers = runtime
            .list_containers(false)
            .await
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
        let runtime = self
            .runtime
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Bolt runtime not initialized"))?;

        let containers = runtime
            .list_containers(true)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get container metrics: {}", e))?;

        let running_containers = containers.iter().filter(|c| c.status == "running").count();

        // Get real system metrics
        let cpu_usage = self.get_cpu_usage().unwrap_or(0.0);
        let memory_usage = self.get_memory_usage().unwrap_or(0.0);
        let gpu_usage = self.get_gpu_usage().unwrap_or(0.0);
        let network_activity = self.get_network_activity().unwrap_or(0.0);

        let metrics = BoltSystemMetrics {
            running_containers,
            total_containers: containers.len(),
            cpu_usage: cpu_usage.into(),
            memory_usage: memory_usage.into(),
            gpu_usage: gpu_usage.into(),
            network_activity: network_activity.into(),
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
    pub async fn create_game_environment(
        &self,
        game_name: &str,
        wine_version: &str,
    ) -> anyhow::Result<()> {
        let runtime = self
            .runtime
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Bolt runtime not initialized"))?;

        // Setup gaming environment
        runtime
            .setup_gaming(Some(wine_version), Some("win10"))
            .await
            .map_err(|e| anyhow::anyhow!("Failed to setup gaming environment: {}", e))?;

        // Create high-performance gaming network
        let network_name = format!("{}-gaming-net", game_name.to_lowercase().replace(" ", "-"));
        runtime
            .create_network(&network_name, "bolt", Some("10.1.0.0/16"))
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create gaming network: {}", e))?;

        Ok(())
    }

    #[cfg(not(feature = "container-bolt"))]
    pub async fn create_game_environment(
        &self,
        _game_name: &str,
        _wine_version: &str,
    ) -> anyhow::Result<()> {
        Err(anyhow::anyhow!("Bolt support not compiled in"))
    }

    // Real system metrics implementation with Zen 3D V-Cache optimizations
    fn get_cpu_usage(&self) -> anyhow::Result<f32> {
        use std::fs::File;
        use std::io::{BufRead, BufReader};

        // Read /proc/stat for CPU usage
        let file = File::open("/proc/stat")?;
        let reader = BufReader::new(file);

        if let Some(line) = reader.lines().next() {
            let line = line?;
            if line.starts_with("cpu ") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 8 {
                    let user: u64 = parts[1].parse().unwrap_or(0);
                    let nice: u64 = parts[2].parse().unwrap_or(0);
                    let system: u64 = parts[3].parse().unwrap_or(0);
                    let idle: u64 = parts[4].parse().unwrap_or(0);
                    let iowait: u64 = parts[5].parse().unwrap_or(0);
                    let irq: u64 = parts[6].parse().unwrap_or(0);
                    let softirq: u64 = parts[7].parse().unwrap_or(0);

                    let total = user + nice + system + idle + iowait + irq + softirq;
                    let idle_total = idle + iowait;
                    let non_idle = total - idle_total;

                    if total > 0 {
                        return Ok((non_idle as f32 / total as f32) * 100.0);
                    }
                }
            }
        }
        Ok(0.0)
    }

    fn get_memory_usage(&self) -> anyhow::Result<f32> {
        use std::fs::File;
        use std::io::{BufRead, BufReader};

        // Read /proc/meminfo for memory usage
        let file = File::open("/proc/meminfo")?;
        let reader = BufReader::new(file);

        let mut mem_total = 0u64;
        let mut mem_free = 0u64;
        let mut mem_available = 0u64;

        for line in reader.lines() {
            let line = line?;
            if line.starts_with("MemTotal:") {
                mem_total = line
                    .split_whitespace()
                    .nth(1)
                    .unwrap_or("0")
                    .parse()
                    .unwrap_or(0);
            } else if line.starts_with("MemFree:") {
                mem_free = line
                    .split_whitespace()
                    .nth(1)
                    .unwrap_or("0")
                    .parse()
                    .unwrap_or(0);
            } else if line.starts_with("MemAvailable:") {
                mem_available = line
                    .split_whitespace()
                    .nth(1)
                    .unwrap_or("0")
                    .parse()
                    .unwrap_or(0);
            }
        }

        if mem_total > 0 {
            let used = mem_total - mem_available.max(mem_free);
            return Ok((used as f32 / mem_total as f32) * 100.0);
        }
        Ok(0.0)
    }

    fn get_gpu_usage(&self) -> anyhow::Result<f32> {
        // Try NVIDIA first
        if let Ok(output) = std::process::Command::new("nvidia-smi")
            .args(&[
                "--query-gpu=utilization.gpu",
                "--format=csv,noheader,nounits",
            ])
            .output()
        {
            if output.status.success() {
                let usage_str = String::from_utf8_lossy(&output.stdout);
                if let Ok(usage) = usage_str.trim().parse::<f32>() {
                    return Ok(usage);
                }
            }
        }

        // Try AMD
        if let Ok(output) = std::process::Command::new("radeontop")
            .args(&["-d", "-", "-l", "1"])
            .output()
        {
            if output.status.success() {
                let output_str = String::from_utf8_lossy(&output.stdout);
                // Parse radeontop output for GPU usage
                for line in output_str.lines() {
                    if line.contains("gpu") && line.contains("%") {
                        if let Some(percent_pos) = line.find('%') {
                            if let Some(space_pos) = line[..percent_pos].rfind(' ') {
                                if let Ok(usage) = line[space_pos + 1..percent_pos].parse::<f32>() {
                                    return Ok(usage);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(0.0)
    }

    fn get_network_activity(&self) -> anyhow::Result<f32> {
        use std::fs::File;
        use std::io::{BufRead, BufReader};

        // Read /proc/net/dev for network activity
        let file = File::open("/proc/net/dev")?;
        let reader = BufReader::new(file);

        let mut total_bytes = 0u64;

        for line in reader.lines() {
            let line = line?;
            if line.contains(':') && !line.contains("lo:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 10 {
                    // RX bytes (index 1) + TX bytes (index 9)
                    let rx_bytes: u64 = parts[1].parse().unwrap_or(0);
                    let tx_bytes: u64 = parts[9].parse().unwrap_or(0);
                    total_bytes += rx_bytes + tx_bytes;
                }
            }
        }

        // Convert to MB/s (this is a rough approximation)
        Ok((total_bytes as f32) / (1024.0 * 1024.0))
    }

    // AMD Zen 3D V-Cache optimizations for gaming performance
    pub fn optimize_for_zen3d_vcache(&self) -> anyhow::Result<()> {
        // Set CPU governor to performance for gaming
        let _ = std::process::Command::new("cpupower")
            .args(&["frequency-set", "-g", "performance"])
            .status();

        // Enable Zen 3D cache optimizations
        self.set_zen_cache_settings()?;

        // Configure CPU affinity for gaming threads
        self.optimize_cpu_affinity()?;

        Ok(())
    }

    fn set_zen_cache_settings(&self) -> anyhow::Result<()> {
        // Set L3 cache sharing mode for better gaming performance
        if std::path::Path::new("/sys/devices/system/cpu/cpu0/cache/index3/shared_cpu_map").exists()
        {
            // Optimize cache sharing for Zen 3D V-Cache
            let _ = std::fs::write("/proc/sys/vm/zone_reclaim_mode", "0");
            let _ = std::fs::write("/proc/sys/kernel/numa_balancing", "0");
        }

        // Set CPU scaling settings optimized for gaming
        for cpu_dir in std::fs::read_dir("/sys/devices/system/cpu/")? {
            let cpu_dir = cpu_dir?;
            if cpu_dir.file_name().to_string_lossy().starts_with("cpu") {
                let scaling_governor = cpu_dir.path().join("cpufreq/scaling_governor");
                if scaling_governor.exists() {
                    let _ = std::fs::write(&scaling_governor, "performance");
                }
            }
        }

        Ok(())
    }

    fn optimize_cpu_affinity(&self) -> anyhow::Result<()> {
        // Detect Zen 3D V-Cache CPUs and optimize core allocation
        if let Ok(cpuinfo) = std::fs::read_to_string("/proc/cpuinfo") {
            if cpuinfo.contains("AMD Ryzen")
                && (cpuinfo.contains("5800X3D")
                    || cpuinfo.contains("5900X3D")
                    || cpuinfo.contains("5950X3D"))
            {
                // For Zen 3D, prefer cores with 3D V-Cache for gaming threads
                println!("Detected AMD Zen 3D V-Cache CPU - optimizing for gaming performance");

                // Set process priority for better gaming performance
                let _ = std::process::Command::new("renice")
                    .args(&["-n", "-10", "-p", &std::process::id().to_string()])
                    .status();
            }
        }

        Ok(())
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
