# GhostForge Integration with Bolt

Integration guide for [GhostForge](https://github.com/ghostforge) - A pure Rust alternative to Lutris that leverages Bolt container runtime for Wine/Proton gaming environments.

## Overview

GhostForge replaces Lutris as a gaming platform manager, using Bolt's advanced container runtime instead of Podman for superior performance, GPU management, and declarative gaming environment configuration.

## Architecture

```
ghostforge
├── Core Gaming Engine (Rust)
├── Game Library Management
├── Wine/Proton Integration
└── Bolt Runtime
    ├── Gaming Capsules
    ├── Wine Containers
    ├── GPU Passthrough
    └── Storage Management
```

## Integration Setup

### 1. GhostForge Boltfile Configuration

Create `Boltfile.toml` in your ghostforge project:

```toml
project = "ghostforge"

# Base Wine/Proton runtime environment
[services.proton-runtime]
build = "./containers/proton"
gpu.nvidia = true
gpu.amd = true
privileged = true
volumes = [
    "./games:/games",
    "./wine-prefixes:/prefixes",
    "/tmp/.X11-unix:/tmp/.X11-unix",
    "/dev/dri:/dev/dri",
    "/run/user/1000/pulse:/run/user/1000/pulse"
]
devices = ["/dev/input", "/dev/uinput"]
env.DISPLAY = ":0"
env.PULSE_RUNTIME_PATH = "/run/user/1000/pulse"
env.WINE_PREFIX = "/prefixes/default"

# Steam compatibility layer
[services.steam-runtime]
build = "./containers/steam"
gpu.nvidia = true
volumes = [
    "./steam:/steam",
    "./games:/games",
    "/tmp/.X11-unix:/tmp/.X11-unix"
]
env.STEAM_COMPAT_DATA_PATH = "/steam/steamapps/compatdata"
env.STEAM_COMPAT_CLIENT_INSTALL_PATH = "/steam"

# Game-specific runtime (example)
[services.cyberpunk-2077]
capsule = "gaming/proton-ge"
gpu.nvidia = true
memory_limit = "16Gi"
cpu_limit = "8"
volumes = [
    "./games/cyberpunk2077:/game",
    "./saves/cyberpunk2077:/saves",
    "/tmp/.X11-unix:/tmp/.X11-unix"
]
env.PROTON_VERSION = "GE-Proton8-26"
env.WINE_PREFIX = "/prefixes/cyberpunk2077"
env.DXVK_ENABLE = "1"
env.MANGOHUD = "1"

# Gaming service manager
[services.game-manager]
build = "./src"
ports = ["8090:8090"]
volumes = ["./config:/config"]
env.BOLT_ENDPOINT = "unix:///var/run/bolt.sock"

# Performance monitoring
[services.performance-monitor]
build = "./containers/monitor"
gpu.nvidia = true
volumes = [
    "/sys/class/drm:/sys/class/drm:ro",
    "/proc:/host/proc:ro"
]
env.MONITORING_INTERVAL = "1000"

[network]
driver = "quic"
encryption = true

[storage.games]
type = "zfs"
size = "2Ti"
compression = "lz4"

[storage.prefixes]
type = "overlay"
size = "500Gi"
snapshot_enabled = true
```

### 2. Container Definitions

#### Proton Runtime Container (`containers/proton/Dockerfile.bolt`)

```dockerfile
FROM bolt://ubuntu:22.04

# Install Wine dependencies
RUN apt-get update && apt-get install -y \
    wine \
    winetricks \
    xvfb \
    pulseaudio \
    mesa-utils \
    vulkan-tools \
    dxvk \
    && rm -rf /var/lib/apt/lists/*

# Install Proton-GE
RUN mkdir -p /opt/proton-ge && \
    wget -O- https://github.com/GloriousEggroll/proton-ge-custom/releases/latest/download/GE-Proton*.tar.gz | \
    tar -xz -C /opt/proton-ge --strip-components=1

# Setup gaming environment
COPY gaming-entrypoint.sh /usr/local/bin/
RUN chmod +x /usr/local/bin/gaming-entrypoint.sh

ENTRYPOINT ["/usr/local/bin/gaming-entrypoint.sh"]
```

#### Steam Runtime Container (`containers/steam/Dockerfile.bolt`)

```dockerfile
FROM bolt://steamcmd:latest

# Install Steam Runtime
RUN apt-get update && apt-get install -y \
    steam-runtime \
    steam-devices \
    && rm -rf /var/lib/apt/lists/*

COPY steam-launcher.sh /usr/local/bin/
RUN chmod +x /usr/local/bin/steam-launcher.sh

CMD ["/usr/local/bin/steam-launcher.sh"]
```

### 3. GhostForge Core Integration

```rust
// src/runtime/bolt_integration.rs
use bolt::{BoltRuntime, BoltFileBuilder, api::*, config::*};

pub struct BoltGameRuntime {
    runtime: BoltRuntime,
}

impl BoltGameRuntime {
    pub async fn new() -> bolt::Result<Self> {
        let runtime = BoltRuntime::new()?;
        Ok(Self { runtime })
    }

    pub async fn launch_game(&self, game_id: &str) -> bolt::Result<()> {
        // Setup gaming environment with GPU and audio
        let gaming_config = GamingConfig {
            gpu: Some(GpuConfig {
                nvidia: Some(NvidiaConfig {
                    device: Some(0),
                    dlss: Some(true),
                    raytracing: Some(true),
                    cuda: Some(false),
                }),
                amd: None,
                passthrough: Some(true),
            }),
            audio: Some(AudioConfig {
                system: "pipewire".to_string(),
                latency: Some("low".to_string()),
            }),
            wine: Some(WineConfig {
                version: None,
                proton: Some("8.0".to_string()),
                winver: Some("win10".to_string()),
                prefix: Some(format!("/prefixes/{}", game_id)),
            }),
            performance: Some(PerformanceConfig {
                cpu_governor: Some("performance".to_string()),
                nice_level: Some(-10),
                rt_priority: Some(50),
            }),
        };

        // Create gaming-optimized Boltfile
        let boltfile = BoltFileBuilder::new(&format!("ghostforge-{}", game_id))
            .add_gaming_service("game-container", "bolt://gaming-base", gaming_config)
            .build();

        // Deploy gaming environment
        self.runtime.config().save_boltfile(&boltfile)?;
        self.runtime.surge_up(&[], false, false).await?;

        // Launch the specific game
        self.runtime.launch_game(&format!("steam://run/{}", game_id), &[]).await?;

        println!("🎮 Game {} launched in Bolt runtime", game_id);
        Ok(())
    }

    pub async fn install_game(&self, game_path: &str, proton_version: &str) -> bolt::Result<()> {
        // Setup Proton for installation
        self.runtime.setup_gaming(Some(proton_version), Some("win10")).await?;

        // Create installation container
        self.runtime.run_container(
            "bolt://proton-installer:latest",
            Some("game-installer"),
            &[],
            &[
                format!("GAME_PATH={}", game_path),
                format!("PROTON_VERSION={}", proton_version),
                "WINE_PREFIX=/prefixes/new-game".to_string(),
            ],
            &[format!("{}:/install", game_path)],
            true
        ).await?;

        Ok(())
    }

    pub async fn create_gaming_network(&self) -> bolt::Result<()> {
        // Create high-performance gaming network with QUIC
        self.runtime.create_network("gaming-net", "bolt", Some("10.1.0.0/16")).await?;
        Ok(())
    }

    pub async fn list_game_containers(&self) -> bolt::Result<Vec<ContainerInfo>> {
        let containers = self.runtime.list_containers(false).await?;
        Ok(containers.into_iter()
            .filter(|c| c.name.contains("game-") || c.name.contains("ghostforge-"))
            .collect())
    }
}
```

### 4. Game Management System

```rust
// src/games/manager.rs
use bolt::{BoltRuntime, BoltFileBuilder, api::*, config::*};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct GameConfig {
    pub name: String,
    pub executable: String,
    pub proton_version: String,
    pub wine_prefix: String,
    pub gpu_requirements: GpuRequirements,
    pub performance_profile: PerformanceProfile,
}

#[derive(Serialize, Deserialize)]
pub struct GpuRequirements {
    pub nvidia: bool,
    pub amd: bool,
    pub vram_min: String,
    pub dx_version: String,
}

#[derive(Serialize, Deserialize)]
pub struct PerformanceProfile {
    pub memory_limit: String,
    pub cpu_limit: String,
    pub priority: i32,
}

pub struct GameManager {
    bolt_runtime: BoltGameRuntime,
    runtime: BoltRuntime,
}

impl GameManager {
    pub async fn new() -> bolt::Result<Self> {
        let bolt_runtime = BoltGameRuntime::new().await?;
        let runtime = BoltRuntime::new()?;

        Ok(Self { bolt_runtime, runtime })
    }

    pub async fn create_game_template(&self, config: GameConfig) -> bolt::Result<()> {
        // Create gaming configuration from game requirements
        let gaming_config = GamingConfig {
            gpu: Some(GpuConfig {
                nvidia: if config.gpu_requirements.nvidia {
                    Some(NvidiaConfig {
                        device: Some(0),
                        dlss: Some(true),
                        raytracing: Some(true),
                        cuda: Some(false),
                    })
                } else { None },
                amd: if config.gpu_requirements.amd {
                    Some(AmdConfig {
                        device: Some(0),
                        rocm: Some(true),
                    })
                } else { None },
                passthrough: Some(true),
            }),
            wine: Some(WineConfig {
                version: None,
                proton: Some(config.proton_version.clone()),
                winver: Some("win10".to_string()),
                prefix: Some(config.wine_prefix.clone()),
            }),
            performance: Some(PerformanceConfig {
                cpu_governor: Some("performance".to_string()),
                nice_level: Some(config.performance_profile.priority),
                rt_priority: Some(50),
            }),
            audio: Some(AudioConfig {
                system: "pipewire".to_string(),
                latency: Some("low".to_string()),
            }),
        };

        // Create game-specific Boltfile
        let service_name = config.name.to_lowercase().replace(" ", "-");
        let boltfile = BoltFileBuilder::new(&format!("ghostforge-{}", service_name))
            .add_gaming_service(&service_name, "bolt://gaming-base", gaming_config)
            .build();

        // Save the game configuration
        self.runtime.config().save_boltfile(&boltfile)?;
        println!("✅ Game template created for {}", config.name);

        Ok(())
    }

    pub async fn launch_with_mods(&self, game_id: &str, mods: Vec<String>) -> bolt::Result<()> {
        // Create dedicated network for modded game
        let mod_network = format!("{}-mods-net", game_id);
        self.runtime.create_network(&mod_network, "bolt", Some("10.2.0.0/16")).await?;

        // Install mods in isolated container first
        for mod_name in mods {
            self.install_mod(game_id, &mod_name).await?;
        }

        // Launch modded game with additional environment variables
        self.runtime.run_container(
            &format!("bolt://game-{}", game_id),
            Some(&format!("{}-modded", game_id)),
            &[],
            &[
                "MODS_ENABLED=1".to_string(),
                format!("MOD_DIRECTORY=/mods/{}", game_id),
            ],
            &[format!("./mods/{}:/mods/{}", game_id, game_id)],
            true
        ).await?;

        Ok(())
    }

    async fn install_mod(&self, game_id: &str, mod_name: &str) -> bolt::Result<()> {
        self.runtime.run_container(
            "bolt://mod-installer:latest",
            Some(&format!("mod-installer-{}-{}", game_id, mod_name)),
            &[],
            &[
                format!("GAME_ID={}", game_id),
                format!("MOD_NAME={}", mod_name),
            ],
            &[format!("./mods/{}:/output", game_id)],
            false
        ).await?;

        Ok(())
    }
}
```

### 5. Performance Optimization

```rust
// src/performance/optimizer.rs
use bolt::{BoltRuntime, BoltFileBuilder, api::*, config::*};

pub struct PerformanceOptimizer {
    runtime: BoltRuntime,
}

impl PerformanceOptimizer {
    pub fn new() -> bolt::Result<Self> {
        Ok(Self {
            runtime: BoltRuntime::new()?,
        })
    }

    pub async fn optimize_for_game(&self, game_id: &str) -> bolt::Result<()> {
        // Get system information for optimization
        let cpu_cores = num_cpus::get();
        let memory_gb = self.get_available_memory().await?;

        // Create optimized gaming configuration
        let optimized_gaming_config = GamingConfig {
            gpu: Some(GpuConfig {
                nvidia: Some(NvidiaConfig {
                    device: Some(0),
                    dlss: Some(true),
                    raytracing: Some(true),
                    cuda: Some(false),
                }),
                amd: None,
                passthrough: Some(true),
            }),
            performance: Some(PerformanceConfig {
                cpu_governor: Some("performance".to_string()),
                nice_level: Some(-20), // Highest priority
                rt_priority: Some(99), // Real-time scheduling
            }),
            audio: Some(AudioConfig {
                system: "pipewire".to_string(),
                latency: Some("ultra-low".to_string()),
            }),
            wine: Some(WineConfig {
                version: None,
                proton: Some("8.0".to_string()),
                winver: Some("win10".to_string()),
                prefix: Some(format!("/prefixes/{}", game_id)),
            }),
        };

        // Create optimized service configuration
        let service_name = format!("{}-optimized", game_id);
        let boltfile = BoltFileBuilder::new(&service_name)
            .add_gaming_service(&service_name, "bolt://gaming-optimized", optimized_gaming_config)
            .build();

        // Deploy optimized configuration
        self.runtime.config().save_boltfile(&boltfile)?;
        self.runtime.surge_up(&[], false, true).await?; // Force recreate for optimization

        // Create high-performance network
        self.runtime.create_network(
            &format!("{}-perf-net", game_id),
            "bolt",
            Some("10.3.0.0/16")
        ).await?;

        println!("🚀 Performance optimized for {}", game_id);
        println!("   CPU Cores: {} (using {})", cpu_cores, cpu_cores * 3 / 4);
        println!("   Memory: {}GB (using {}GB)", memory_gb, memory_gb * 2 / 3);

        Ok(())
    }

    pub async fn monitor_performance(&self, game_id: &str) -> bolt::Result<()> {
        // Get container status and metrics
        let containers = self.runtime.list_containers(false).await?;
        let game_container = containers.iter()
            .find(|c| c.name.contains(game_id))
            .ok_or_else(|| bolt::BoltError::Runtime(
                format!("Game container {} not found", game_id).into()
            ))?;

        println!("📊 Performance Monitoring for {}", game_id);
        println!("   Container: {} ({})", game_container.name, game_container.status);
        println!("   Image: {}", game_container.image);

        Ok(())
    }

    async fn get_available_memory(&self) -> bolt::Result<usize> {
        // Simplified memory detection - in real implementation,
        // you'd use system APIs to get actual memory
        Ok(16) // Default to 16GB
    }
}
```

## CLI Integration

GhostForge CLI commands using Bolt:

```bash
# Launch a game
ghostforge launch "Cyberpunk 2077"

# Install new game
ghostforge install --proton GE-8.26 --path ./games/newgame

# Create game snapshot
ghostforge snapshot create stable-config

# List running games
ghostforge ps

# Performance monitoring
ghostforge monitor --game "Cyberpunk 2077"

# Update Proton version
ghostforge proton update GE-8.30

# Mod management
ghostforge mod install nexusmods://1234 --game "Skyrim"
```

## Advanced Features

### 1. Multi-GPU Gaming

```toml
[services.dual-gpu-game]
gpu.nvidia = true
gpu.amd = true
env.GPU_PRIMARY = "nvidia"
env.GPU_SECONDARY = "amd"
env.PRIME_OFFLOAD = "1"
```

### 2. VR Gaming Support

```toml
[services.vr-runtime]
build = "./containers/openvr"
privileged = true
devices = ["/dev/hidraw*", "/dev/usb"]
volumes = ["/sys/devices:/sys/devices"]
env.OPENVR_ROOT = "/opt/openvr"
```

### 3. Wine Prefix Management

```rust
pub async fn create_wine_prefix(&self, name: &str, windows_version: &str) -> anyhow::Result<()> {
    let prefix_config = format!(
        r#"
        [services.prefix-{}]
        capsule = "wine/base"
        volumes = ["./prefixes/{}:/prefix"]
        env.WINE_PREFIX = "/prefix"
        env.WINEARCH = "win64"
        env.WINDOWS_VERSION = "{}"
        "#,
        name, name, windows_version
    );

    self.bolt_runtime.create_service_from_config(&prefix_config).await?;
    Ok(())
}
```

## Migration from Lutris/Bottles

### Configuration Migration

```rust
pub async fn migrate_lutris_games(&self, lutris_config_dir: &Path) -> bolt::Result<()> {
    let lutris_games = self.parse_lutris_configs(lutris_config_dir)?;

    for game in lutris_games {
        let bolt_config = GameConfig {
            name: game.name.clone(),
            executable: game.exe,
            proton_version: "GE-Proton8-26".to_string(),
            wine_prefix: format!("/prefixes/{}", game.name.to_lowercase()),
            gpu_requirements: GpuRequirements {
                nvidia: true,
                amd: false,
                vram_min: "4Gi".to_string(),
                dx_version: "11".to_string(),
            },
            performance_profile: PerformanceProfile {
                memory_limit: "8Gi".to_string(),
                cpu_limit: "4".to_string(),
                priority: -10,
            },
        };

        // Create gaming configuration for migration
        let gaming_config = GamingConfig {
            gpu: Some(GpuConfig {
                nvidia: Some(NvidiaConfig {
                    device: Some(0),
                    dlss: Some(false),
                    raytracing: Some(false),
                    cuda: Some(false),
                }),
                amd: None,
                passthrough: Some(true),
            }),
            wine: Some(WineConfig {
                version: None,
                proton: Some(bolt_config.proton_version.clone()),
                winver: Some("win10".to_string()),
                prefix: Some(bolt_config.wine_prefix.clone()),
            }),
            performance: Some(PerformanceConfig {
                cpu_governor: Some("performance".to_string()),
                nice_level: Some(bolt_config.performance_profile.priority),
                rt_priority: Some(50),
            }),
            audio: Some(AudioConfig {
                system: "pipewire".to_string(),
                latency: Some("low".to_string()),
            }),
        };

        // Create migrated game Boltfile
        let service_name = game.name.to_lowercase().replace(" ", "-");
        let boltfile = BoltFileBuilder::new(&format!("migrated-{}", service_name))
            .add_gaming_service(&service_name, "bolt://gaming-base", gaming_config)
            .build();

        self.runtime.config().save_boltfile(&boltfile)?;
        println!("✅ Migrated game: {}", game.name);
    }

    println!("🎮 Successfully migrated {} games from Lutris to Bolt", lutris_games.len());
    Ok(())
}
```

## Benefits of Bolt Integration

1. **Superior Performance**: Native container runtime optimized for gaming
2. **GPU Management**: Advanced GPU resource allocation and monitoring
3. **Snapshot System**: Safe game state management and mod testing
4. **Declarative Configuration**: Version-controlled gaming setups
5. **QUIC Networking**: Low-latency multiplayer gaming
6. **Wine/Proton Isolation**: Multiple Wine versions without conflicts
7. **Automatic Optimization**: Dynamic resource allocation based on game requirements

## Troubleshooting

- **Audio Issues**: Ensure PulseAudio socket is properly mounted
- **GPU Access**: Check device permissions and driver compatibility
- **Wine Prefix Corruption**: Use Bolt snapshots for quick recovery
- **Performance**: Monitor resource usage with built-in performance tools
- **Controller Support**: Verify uinput device access for game controllers

## Gaming Performance Monitoring

```rust
use bolt::{BoltRuntime, api::*};
use std::time::Duration;

pub struct GamePerformanceMonitor {
    runtime: BoltRuntime,
}

impl GamePerformanceMonitor {
    pub fn new() -> bolt::Result<Self> {
        Ok(Self {
            runtime: BoltRuntime::new()?,
        })
    }

    pub async fn monitor_game_performance(&self, game_id: &str) -> bolt::Result<()> {
        let containers = self.runtime.list_containers(false).await?;
        let game_container = containers.iter()
            .find(|c| c.name.contains(game_id))
            .ok_or_else(|| bolt::BoltError::Runtime(
                format!("Game container {} not found", game_id).into()
            ))?;

        println!("🎮 Starting performance monitoring for {}", game_id);
        println!("   Container: {}", game_container.name);

        // Spawn monitoring task
        let container_name = game_container.name.clone();
        let runtime = self.runtime.clone();

        tokio::spawn(async move {
            loop {
                match runtime.list_containers(false).await {
                    Ok(containers) => {
                        if let Some(container) = containers.iter()
                            .find(|c| c.name == container_name) {
                            println!("📊 {} - Status: {}, Ports: {:?}",
                                game_id, container.status, container.ports);
                        }
                    },
                    Err(e) => eprintln!("Error monitoring {}: {}", game_id, e),
                }

                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        });

        Ok(())
    }

    pub async fn get_surge_status(&self) -> bolt::Result<()> {
        let status = self.runtime.surge_status().await?;
        println!("🚀 Surge Status:");
        println!("   Services: {}", status.services.len());
        println!("   Networks: {}", status.networks.len());

        for service in &status.services {
            println!("   📦 Service: {} ({})", service.name, service.status);
        }

        for network in &status.networks {
            println!("   🌐 Network: {} ({})", network.name, network.driver);
        }

        Ok(())
    }
}
```