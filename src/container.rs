use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use tokio::process::Command as AsyncCommand;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameContainer {
    pub id: String,
    pub name: String,
    pub game_id: String,
    pub base_image: ContainerImage,
    pub wine_version: String,
    pub graphics_layers: Vec<String>,
    pub system_dependencies: Vec<String>,
    pub environment_variables: HashMap<String, String>,
    pub mount_points: Vec<MountPoint>,
    pub network_mode: NetworkMode,
    pub resource_limits: ResourceLimits,
    pub created_at: DateTime<Utc>,
    pub last_used: Option<DateTime<Utc>>,
    pub size_mb: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerImage {
    pub name: String,
    pub tag: String,
    pub digest: Option<String>,
    pub platform: String, // linux/amd64, linux/arm64
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountPoint {
    pub host_path: PathBuf,
    pub container_path: PathBuf,
    pub read_only: bool,
    pub bind_type: BindType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BindType {
    Bind,       // Direct bind mount
    Volume,     // Named volume
    TmpFs,      // Temporary filesystem
    DeviceNode, // Device access
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkMode {
    Bridge,         // Default bridge network
    Host,           // Host networking (for anti-cheat)
    None,           // No network access
    Custom(String), // Custom network name
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub memory_mb: Option<u64>,
    pub cpu_cores: Option<f32>,
    pub disk_mb: Option<u64>,
    pub gpu_access: bool,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            memory_mb: Some(4096), // 4GB default
            cpu_cores: None,       // No CPU limit
            disk_mb: Some(10240),  // 10GB default
            gpu_access: true,      // GPU access enabled
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerRuntime {
    pub runtime_type: RuntimeType,
    pub socket_path: Option<PathBuf>,
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuntimeType {
    Bolt,           // Gaming-optimized runtime (preferred)
    Podman,         // Rootless containers
    Docker,         // Traditional containers
    Containerd,     // Low-level runtime
    Custom(String), // User-defined runtime
}

pub struct ContainerManager {
    pub runtime: ContainerRuntime,
    pub containers: HashMap<String, GameContainer>,
    pub base_images: Vec<ContainerImage>,
}

impl ContainerManager {
    pub fn new(config_dir: PathBuf) -> Result<Self> {
        let runtime = Self::detect_runtime()?;
        let data_dir = config_dir.join("containers");
        std::fs::create_dir_all(&data_dir)?;

        Ok(Self {
            runtime: ContainerRuntime {
                runtime_type: runtime,
                socket_path: None,
                config_dir: config_dir.clone(),
                data_dir,
            },
            containers: HashMap::new(),
            base_images: Self::get_default_images(),
        })
    }

    fn detect_runtime() -> Result<RuntimeType> {
        // Try Bolt first (gaming-optimized)
        #[cfg(feature = "container-bolt")]
        {
            if let Ok(output) = Command::new("bolt").arg("--version").output() {
                if output.status.success() {
                    return Ok(RuntimeType::Bolt);
                }
            }
        }

        // Try Podman (rootless, secure)
        #[cfg(feature = "container-podman")]
        {
            if let Ok(output) = Command::new("podman").arg("version").output() {
                if output.status.success() {
                    return Ok(RuntimeType::Podman);
                }
            }
        }

        // Fall back to Docker (compatibility)
        #[cfg(feature = "container-docker")]
        {
            if let Ok(output) = Command::new("docker").arg("version").output() {
                if output.status.success() {
                    return Ok(RuntimeType::Docker);
                }
            }
        }

        Err(anyhow::anyhow!(
            "No container runtime found. Please install Bolt, Podman, or Docker.\n\
            For optimal gaming performance, install Bolt: https://bolt.dev/"
        ))
    }

    fn get_default_images() -> Vec<ContainerImage> {
        vec![
            ContainerImage {
                name: "ghcr.io/ghostforge/gaming-base".to_string(),
                tag: "ubuntu22.04".to_string(),
                digest: None,
                platform: "linux/amd64".to_string(),
            },
            ContainerImage {
                name: "ghcr.io/ghostforge/gaming-nvidia".to_string(),
                tag: "latest".to_string(),
                digest: None,
                platform: "linux/amd64".to_string(),
            },
            ContainerImage {
                name: "ghcr.io/ghostforge/gaming-steam".to_string(),
                tag: "proton".to_string(),
                digest: None,
                platform: "linux/amd64".to_string(),
            },
        ]
    }

    pub async fn create_container(&mut self, game: &crate::game::Game) -> Result<GameContainer> {
        let container_id = Uuid::new_v4().to_string();
        let container_name = format!("ghostforge-{}", game.name.replace(" ", "-").to_lowercase());

        // Select optimal base image based on game requirements
        let base_image = self.select_optimal_image(game).await?;

        // Determine Wine version based on game compatibility
        let wine_version = self.determine_wine_version(game).await?;

        // Configure graphics layers
        let graphics_layers = self.configure_graphics_layers(game).await?;

        // Set up mount points
        let mount_points = self.create_mount_points(game, &container_id)?;

        // Configure environment variables
        let mut environment_variables = HashMap::new();
        self.setup_wine_environment(&mut environment_variables, &wine_version);
        self.setup_graphics_environment(&mut environment_variables, &graphics_layers);
        self.setup_game_environment(&mut environment_variables, game);

        // Configure resource limits based on game requirements
        let resource_limits = self.calculate_resource_limits(game);

        let container = GameContainer {
            id: container_id,
            name: container_name,
            game_id: game.id.clone(),
            base_image,
            wine_version,
            graphics_layers,
            system_dependencies: self.determine_dependencies(game),
            environment_variables,
            mount_points,
            network_mode: self.determine_network_mode(game),
            resource_limits,
            created_at: Utc::now(),
            last_used: None,
            size_mb: None,
        };

        // Build the container image using the appropriate runtime
        match self.runtime.runtime_type {
            RuntimeType::Bolt => self.build_bolt_container(&container).await?,
            _ => self.build_container(&container).await?,
        }

        // Store container configuration
        self.containers
            .insert(container.id.clone(), container.clone());
        self.save_container_config(&container)?;

        Ok(container)
    }

    async fn select_optimal_image(&self, game: &crate::game::Game) -> Result<ContainerImage> {
        // Logic to select best base image based on:
        // - GPU requirements (NVIDIA vs AMD vs Intel)
        // - Game launcher (Steam vs native)
        // - Performance requirements

        if game.launcher.as_deref() == Some("steam") {
            return Ok(self
                .base_images
                .iter()
                .find(|img| img.name.contains("steam"))
                .unwrap_or(&self.base_images[0])
                .clone());
        }

        // Check for NVIDIA requirements
        let graphics_manager =
            crate::graphics::GraphicsManager::new(std::env::temp_dir().join("graphics_temp"))?;

        if let Ok(nvidia_features) = graphics_manager.detect_nvidia_features() {
            if nvidia_features.available {
                return Ok(self
                    .base_images
                    .iter()
                    .find(|img| img.name.contains("nvidia"))
                    .unwrap_or(&self.base_images[0])
                    .clone());
            }
        }

        // Default to base gaming image
        Ok(self.base_images[0].clone())
    }

    async fn determine_wine_version(&self, game: &crate::game::Game) -> Result<String> {
        // Use ProtonDB integration to determine best Wine version
        let protondb = crate::protondb::ProtonDBClient::new();

        if let Some(launcher_id) = &game.launcher_id {
            if let Ok(appid) = launcher_id.parse::<u32>() {
                if let Ok(compat_info) = protondb.get_compatibility_info(appid).await {
                    return Ok(compat_info.recommended_proton);
                }
            }
        }

        // Default to latest Proton GE
        Ok("GE-Proton8-26".to_string())
    }

    async fn configure_graphics_layers(&self, game: &crate::game::Game) -> Result<Vec<String>> {
        let graphics_manager =
            crate::graphics::GraphicsManager::new(std::env::temp_dir().join("graphics_temp"))?;

        let nvidia_features = graphics_manager
            .detect_nvidia_features()
            .unwrap_or_default();
        let recommendations = graphics_manager.recommend_for_game(&game.name, &nvidia_features);

        let layer_names = recommendations
            .iter()
            .map(|layer| format!("{:?}", layer))
            .collect();

        Ok(layer_names)
    }

    fn create_mount_points(
        &self,
        game: &crate::game::Game,
        container_id: &str,
    ) -> Result<Vec<MountPoint>> {
        let mut mounts = vec![
            // Game files
            MountPoint {
                host_path: game.install_path.clone(),
                container_path: PathBuf::from("/game"),
                read_only: true,
                bind_type: BindType::Bind,
            },
            // Wine prefix (persistent)
            MountPoint {
                host_path: self.runtime.data_dir.join(container_id).join("wine_prefix"),
                container_path: PathBuf::from("/wine_prefix"),
                read_only: false,
                bind_type: BindType::Volume,
            },
            // Save games (persistent)
            MountPoint {
                host_path: self.runtime.data_dir.join(container_id).join("saves"),
                container_path: PathBuf::from("/saves"),
                read_only: false,
                bind_type: BindType::Volume,
            },
            // GPU access
            MountPoint {
                host_path: PathBuf::from("/dev/dri"),
                container_path: PathBuf::from("/dev/dri"),
                read_only: false,
                bind_type: BindType::DeviceNode,
            },
            // Audio access
            MountPoint {
                host_path: PathBuf::from("/dev/snd"),
                container_path: PathBuf::from("/dev/snd"),
                read_only: false,
                bind_type: BindType::DeviceNode,
            },
        ];

        // Add X11 socket for graphics
        if let Ok(display) = std::env::var("DISPLAY") {
            mounts.push(MountPoint {
                host_path: PathBuf::from("/tmp/.X11-unix"),
                container_path: PathBuf::from("/tmp/.X11-unix"),
                read_only: false,
                bind_type: BindType::Bind,
            });
        }

        // Create necessary host directories
        for mount in &mounts {
            if matches!(mount.bind_type, BindType::Volume) {
                std::fs::create_dir_all(&mount.host_path)?;
            }
        }

        Ok(mounts)
    }

    fn setup_wine_environment(&self, env: &mut HashMap<String, String>, wine_version: &str) {
        env.insert("WINE_VERSION".to_string(), wine_version.to_string());
        env.insert("WINEPREFIX".to_string(), "/wine_prefix".to_string());
        env.insert(
            "WINEDLLOVERRIDES".to_string(),
            "mscoree,mshtml=".to_string(),
        );
        env.insert("WINE_LARGE_ADDRESS_AWARE".to_string(), "1".to_string());
    }

    fn setup_graphics_environment(
        &self,
        env: &mut HashMap<String, String>,
        graphics_layers: &[String],
    ) {
        for layer in graphics_layers {
            match layer.as_str() {
                "DXVK" => {
                    env.insert("DXVK_ENABLE_NVAPI".to_string(), "1".to_string());
                    env.insert(
                        "DXVK_CONFIG".to_string(),
                        "dxr,dxgi.nvapiHack=False".to_string(),
                    );
                }
                "VKD3DProton" => {
                    env.insert("VKD3D_CONFIG".to_string(), "dxr".to_string());
                }
                "NvidiaDlss" => {
                    env.insert("DXVK_ENABLE_NVAPI".to_string(), "1".to_string());
                    env.insert(
                        "DXVK_NVAPI_ALLOW_OTHER_DRIVERS".to_string(),
                        "0".to_string(),
                    );
                }
                "MangoHud" => {
                    env.insert("MANGOHUD".to_string(), "1".to_string());
                }
                _ => {}
            }
        }
    }

    fn setup_game_environment(&self, env: &mut HashMap<String, String>, game: &crate::game::Game) {
        // Add game-specific environment variables
        for (key, value) in &game.environment_variables {
            env.insert(key.clone(), value.clone());
        }

        // Set up display and audio
        if let Ok(display) = std::env::var("DISPLAY") {
            env.insert("DISPLAY".to_string(), display);
        }

        env.insert(
            "PULSE_RUNTIME_PATH".to_string(),
            "/run/user/1000/pulse".to_string(),
        );
        env.insert("XDG_RUNTIME_DIR".to_string(), "/run/user/1000".to_string());
    }

    fn calculate_resource_limits(&self, game: &crate::game::Game) -> ResourceLimits {
        // Determine resource limits based on game requirements
        let mut limits = ResourceLimits::default();

        // Adjust based on game genre/requirements
        let game_lower = game.name.to_lowercase();

        if game_lower.contains("cyberpunk") || game_lower.contains("metro") {
            // AAA games need more resources
            limits.memory_mb = Some(8192); // 8GB
            limits.disk_mb = Some(20480); // 20GB
        } else if game_lower.contains("indie") || game_lower.contains("retro") {
            // Indie games need less
            limits.memory_mb = Some(2048); // 2GB
            limits.disk_mb = Some(5120); // 5GB
        }

        limits
    }

    fn determine_dependencies(&self, game: &crate::game::Game) -> Vec<String> {
        let mut deps = vec![
            "wine".to_string(),
            "winetricks".to_string(),
            "cabextract".to_string(),
        ];

        // Add dependencies based on game requirements
        let game_lower = game.name.to_lowercase();

        if game_lower.contains("directx") || game_lower.contains("dx") {
            deps.push("dxvk".to_string());
        }

        if game_lower.contains("vulkan") {
            deps.push("vulkan-tools".to_string());
        }

        deps
    }

    fn determine_network_mode(&self, game: &crate::game::Game) -> NetworkMode {
        // Use host networking for games that need anti-cheat or low latency
        let game_lower = game.name.to_lowercase();

        if game_lower.contains("valorant")
            || game_lower.contains("apex")
            || game_lower.contains("fortnite")
            || game_lower.contains("call of duty")
        {
            return NetworkMode::Host;
        }

        NetworkMode::Bridge
    }

    async fn build_container(&self, container: &GameContainer) -> Result<()> {
        let dockerfile_content = self.generate_dockerfile(container)?;
        let build_context = self.runtime.data_dir.join(&container.id);

        std::fs::create_dir_all(&build_context)?;
        std::fs::write(build_context.join("Dockerfile"), dockerfile_content)?;

        // Build the container image
        let build_cmd = match self.runtime.runtime_type {
            RuntimeType::Podman => "podman",
            RuntimeType::Docker => "docker",
            _ => return Err(anyhow::anyhow!("Unsupported runtime")),
        };

        let output = AsyncCommand::new(build_cmd)
            .args(&[
                "build",
                "-t",
                &format!("ghostforge-{}", container.id),
                build_context.to_str().unwrap(),
            ])
            .output()
            .await?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Container build failed: {}", error));
        }

        Ok(())
    }

    fn generate_dockerfile(&self, container: &GameContainer) -> Result<String> {
        let mut dockerfile = String::new();

        dockerfile.push_str(&format!("FROM {}\n", container.base_image.name));
        dockerfile.push_str("USER root\n");

        // Install dependencies
        dockerfile.push_str("RUN apt-get update && apt-get install -y \\\n");
        for dep in &container.system_dependencies {
            dockerfile.push_str(&format!("    {} \\\n", dep));
        }
        dockerfile.push_str("    && rm -rf /var/lib/apt/lists/*\n");

        // Set up Wine
        dockerfile.push_str(&format!(
            "ENV WINE_VERSION={}\nENV WINEPREFIX=/wine_prefix\n",
            container.wine_version
        ));

        // Create necessary directories
        dockerfile.push_str("RUN mkdir -p /wine_prefix /game /saves\n");

        // Set up user
        dockerfile.push_str("RUN useradd -m -s /bin/bash gameuser\n");
        dockerfile.push_str("RUN chown -R gameuser:gameuser /wine_prefix /game /saves\n");
        dockerfile.push_str("USER gameuser\n");

        // Set working directory
        dockerfile.push_str("WORKDIR /game\n");

        Ok(dockerfile)
    }

    #[cfg(feature = "container-bolt")]
    async fn build_bolt_container(&self, container: &GameContainer) -> Result<()> {
        // Placeholder implementation for when Bolt becomes available
        println!(
            "🔥 Building Bolt gaming container for {}...",
            container.name
        );
        println!(
            "   📦 Gaming capsule: {}",
            self.determine_gaming_capsule(container)
        );
        println!(
            "   🎮 GPU support: {}",
            container.resource_limits.gpu_access
        );

        if let Some(memory_mb) = container.resource_limits.memory_mb {
            println!("   💾 Memory limit: {}Mi", memory_mb);
        }
        if let Some(cpu_cores) = container.resource_limits.cpu_cores {
            println!("   🖥️  CPU limit: {}", cpu_cores);
        }

        println!("   📁 Volumes: {} mounts", container.mount_points.len());
        println!(
            "   🌍 Environment: {} variables",
            container.environment_variables.len()
        );

        // TODO: Replace with actual Bolt runtime calls when available
        // let bolt_runtime = BoltRuntime::new().await?;
        // bolt_runtime.create_service(&container.id, service_config).await?;

        println!(
            "✅ Bolt gaming container created (placeholder): {}",
            container.name
        );
        println!("   ⚠️  Install Bolt runtime for actual functionality");
        Ok(())
    }

    #[cfg(not(feature = "container-bolt"))]
    async fn build_bolt_container(&self, _container: &GameContainer) -> Result<()> {
        Err(anyhow::anyhow!(
            "Bolt runtime feature not enabled. Compile with --features container-bolt"
        ))
    }

    fn determine_gaming_capsule(&self, container: &GameContainer) -> String {
        // Select optimal gaming capsule based on requirements
        if container.wine_version.contains("Proton") {
            "proton-ge".to_string()
        } else if container.base_image.name.contains("nvidia") {
            "wine-nvidia".to_string()
        } else if container.base_image.name.contains("steam") {
            "steam-runtime".to_string()
        } else {
            "wine-base".to_string()
        }
    }

    pub async fn launch_game(&self, container_id: &str, game: &crate::game::Game) -> Result<u32> {
        let container = self
            .containers
            .get(container_id)
            .ok_or_else(|| anyhow::anyhow!("Container not found: {}", container_id))?;

        match self.runtime.runtime_type {
            RuntimeType::Bolt => self.launch_bolt_game(container, game).await,
            _ => {
                let run_cmd = self.build_run_command(container, game)?;
                let child = AsyncCommand::new(&run_cmd[0]).args(&run_cmd[1..]).spawn()?;
                Ok(child.id().unwrap_or(0))
            }
        }
    }

    #[cfg(feature = "container-bolt")]
    async fn launch_bolt_game(
        &self,
        container: &GameContainer,
        game: &crate::game::Game,
    ) -> Result<u32> {
        println!("🎮 Launching {} with Bolt runtime...", game.name);

        // Create launch command for the game
        let game_cmd = vec![
            "wine".to_string(),
            game.executable.to_string_lossy().to_string(),
        ];

        println!("   📦 Container: {}", container.name);
        println!("   🎮 Command: {:?}", game_cmd);

        // TODO: Replace with actual Bolt runtime calls when available
        // let bolt_runtime = BoltRuntime::new().await?;
        // let process_id = bolt_runtime.start_service_with_command(&container.id, &game_cmd).await?;

        // Placeholder: return a fake PID for testing
        let fake_pid = 12345;
        println!(
            "🚀 {} launched with Bolt (placeholder PID: {})",
            game.name, fake_pid
        );
        println!("   ⚠️  Install Bolt runtime for actual game launching");
        Ok(fake_pid)
    }

    #[cfg(not(feature = "container-bolt"))]
    async fn launch_bolt_game(
        &self,
        _container: &GameContainer,
        _game: &crate::game::Game,
    ) -> Result<u32> {
        Err(anyhow::anyhow!("Bolt runtime feature not enabled"))
    }

    fn build_run_command(
        &self,
        container: &GameContainer,
        game: &crate::game::Game,
    ) -> Result<Vec<String>> {
        let mut cmd = vec![];

        match self.runtime.runtime_type {
            RuntimeType::Podman => cmd.push("podman".to_string()),
            RuntimeType::Docker => cmd.push("docker".to_string()),
            RuntimeType::Bolt => {
                return Err(anyhow::anyhow!("Use launch_bolt_game for Bolt runtime"));
            }
            _ => return Err(anyhow::anyhow!("Unsupported runtime")),
        };

        cmd.push("run".to_string());
        cmd.push("--rm".to_string()); // Remove container after exit
        cmd.push("--interactive".to_string());
        cmd.push("--tty".to_string());

        // Add resource limits
        if let Some(memory_mb) = container.resource_limits.memory_mb {
            cmd.push(format!("--memory={}m", memory_mb));
        }

        if let Some(cpu_cores) = container.resource_limits.cpu_cores {
            cmd.push(format!("--cpus={}", cpu_cores));
        }

        // Add mount points
        for mount in &container.mount_points {
            let mount_arg = format!(
                "{}:{}{}",
                mount.host_path.display(),
                mount.container_path.display(),
                if mount.read_only { ":ro" } else { "" }
            );
            cmd.push("--volume".to_string());
            cmd.push(mount_arg);
        }

        // Add environment variables
        for (key, value) in &container.environment_variables {
            cmd.push("--env".to_string());
            cmd.push(format!("{}={}", key, value));
        }

        // Add network configuration
        match container.network_mode {
            NetworkMode::Host => {
                cmd.push("--network=host".to_string());
            }
            NetworkMode::None => {
                cmd.push("--network=none".to_string());
            }
            NetworkMode::Custom(ref network) => {
                cmd.push(format!("--network={}", network));
            }
            NetworkMode::Bridge => {
                // Default, no additional args needed
            }
        }

        // Container image
        cmd.push(format!("ghostforge-{}", container.id));

        // Game executable command
        cmd.push("wine".to_string());
        cmd.push(
            game.executable
                .file_name()
                .ok_or_else(|| anyhow::anyhow!("Invalid executable path"))?
                .to_string_lossy()
                .to_string(),
        );

        // Add launch arguments
        for arg in &game.launch_arguments {
            cmd.push(arg.clone());
        }

        Ok(cmd)
    }

    fn save_container_config(&self, container: &GameContainer) -> Result<()> {
        let config_file = self.runtime.data_dir.join(format!("{}.json", container.id));
        let config_json = serde_json::to_string_pretty(container)?;
        std::fs::write(config_file, config_json)?;
        Ok(())
    }

    pub fn load_containers(&mut self) -> Result<()> {
        let entries = std::fs::read_dir(&self.runtime.data_dir)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map_or(false, |ext| ext == "json") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(container) = serde_json::from_str::<GameContainer>(&content) {
                        self.containers.insert(container.id.clone(), container);
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn cleanup_unused_containers(&mut self, days_threshold: u64) -> Result<u64> {
        let cutoff = Utc::now() - chrono::Duration::days(days_threshold as i64);
        let mut cleaned_count = 0;

        let containers_to_remove: Vec<String> = self
            .containers
            .iter()
            .filter(|(_, container)| {
                container
                    .last_used
                    .map_or(true, |last_used| last_used < cutoff)
            })
            .map(|(id, _)| id.clone())
            .collect();

        for container_id in containers_to_remove {
            self.remove_container(&container_id).await?;
            cleaned_count += 1;
        }

        Ok(cleaned_count)
    }

    pub async fn remove_container(&mut self, container_id: &str) -> Result<()> {
        match self.runtime.runtime_type {
            RuntimeType::Bolt => self.remove_bolt_container(container_id).await?,
            RuntimeType::Podman | RuntimeType::Docker => {
                let remove_cmd = match self.runtime.runtime_type {
                    RuntimeType::Podman => "podman",
                    RuntimeType::Docker => "docker",
                    _ => unreachable!(),
                };

                let _output = AsyncCommand::new(remove_cmd)
                    .args(&["rmi", &format!("ghostforge-{}", container_id)])
                    .output()
                    .await?;
            }
            _ => return Err(anyhow::anyhow!("Unsupported runtime")),
        }

        // Remove container data
        let container_dir = self.runtime.data_dir.join(container_id);
        if container_dir.exists() {
            std::fs::remove_dir_all(container_dir)?;
        }

        // Remove from memory
        self.containers.remove(container_id);

        Ok(())
    }

    #[cfg(feature = "container-bolt")]
    async fn remove_bolt_container(&self, container_id: &str) -> Result<()> {
        println!("🗑️ Removing Bolt container: {}", container_id);

        // TODO: Replace with actual Bolt runtime calls when available
        // let bolt_runtime = BoltRuntime::new().await?;
        // bolt_runtime.remove_service(container_id).await?;

        println!("✅ Bolt container removed (placeholder): {}", container_id);
        println!("   ⚠️  Install Bolt runtime for actual container management");
        Ok(())
    }

    #[cfg(not(feature = "container-bolt"))]
    async fn remove_bolt_container(&self, _container_id: &str) -> Result<()> {
        Err(anyhow::anyhow!("Bolt runtime feature not enabled"))
    }

    /// Get information about available container runtimes
    pub fn get_runtime_info(&self) -> RuntimeInfo {
        let mut available_runtimes = Vec::new();
        let mut recommendations = Vec::new();

        // Check Bolt availability
        #[cfg(feature = "container-bolt")]
        {
            if Command::new("bolt").arg("--version").output().is_ok() {
                available_runtimes.push("Bolt (gaming-optimized)".to_string());
            } else {
                recommendations.push("Install Bolt for optimal gaming performance".to_string());
            }
        }

        // Check Podman availability
        #[cfg(feature = "container-podman")]
        {
            if Command::new("podman").arg("version").output().is_ok() {
                available_runtimes.push("Podman (rootless)".to_string());
            }
        }

        // Check Docker availability
        #[cfg(feature = "container-docker")]
        {
            if Command::new("docker").arg("version").output().is_ok() {
                available_runtimes.push("Docker (compatible)".to_string());
            }
        }

        RuntimeInfo {
            current_runtime: format!("{:?}", self.runtime.runtime_type),
            available_runtimes,
            recommendations,
            bolt_optimized: matches!(self.runtime.runtime_type, RuntimeType::Bolt),
        }
    }

    /// Get detailed runtime status for troubleshooting
    pub async fn diagnose_runtime(&self) -> RuntimeDiagnostics {
        let mut diagnostics = RuntimeDiagnostics {
            runtime_type: format!("{:?}", self.runtime.runtime_type),
            version: "Unknown".to_string(),
            features: Vec::new(),
            issues: Vec::new(),
            suggestions: Vec::new(),
        };

        match self.runtime.runtime_type {
            RuntimeType::Bolt => {
                #[cfg(feature = "container-bolt")]
                {
                    if let Ok(output) = Command::new("bolt").arg("--version").output() {
                        diagnostics.version =
                            String::from_utf8_lossy(&output.stdout).trim().to_string();
                        diagnostics.features.push("Gaming optimization".to_string());
                        diagnostics.features.push("GPU passthrough".to_string());
                        diagnostics.features.push("Snapshot system".to_string());
                        diagnostics.features.push("QUIC networking".to_string());
                    } else {
                        diagnostics.issues.push("Bolt binary not found".to_string());
                        diagnostics
                            .suggestions
                            .push("Install Bolt from https://bolt.dev/".to_string());
                    }
                }
                #[cfg(not(feature = "container-bolt"))]
                {
                    diagnostics
                        .issues
                        .push("Bolt feature not compiled".to_string());
                    diagnostics
                        .suggestions
                        .push("Recompile with --features container-bolt".to_string());
                }
            }
            RuntimeType::Podman => {
                if let Ok(output) = Command::new("podman").arg("version").output() {
                    diagnostics.version = String::from_utf8_lossy(&output.stdout)
                        .lines()
                        .find(|l| l.contains("Version:"))
                        .unwrap_or("Unknown")
                        .to_string();
                    diagnostics.features.push("Rootless containers".to_string());
                    diagnostics.features.push("OCI compatible".to_string());
                } else {
                    diagnostics.issues.push("Podman not installed".to_string());
                }
            }
            RuntimeType::Docker => {
                if let Ok(output) = Command::new("docker").arg("--version").output() {
                    diagnostics.version =
                        String::from_utf8_lossy(&output.stdout).trim().to_string();
                    diagnostics.features.push("Wide compatibility".to_string());
                    diagnostics.features.push("Extensive ecosystem".to_string());
                } else {
                    diagnostics.issues.push("Docker not installed".to_string());
                }
            }
            _ => {
                diagnostics
                    .issues
                    .push("Unsupported runtime type".to_string());
            }
        }

        diagnostics
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeInfo {
    pub current_runtime: String,
    pub available_runtimes: Vec<String>,
    pub recommendations: Vec<String>,
    pub bolt_optimized: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeDiagnostics {
    pub runtime_type: String,
    pub version: String,
    pub features: Vec<String>,
    pub issues: Vec<String>,
    pub suggestions: Vec<String>,
}
