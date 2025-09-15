use anyhow::Result;
use flate2::read::GzDecoder;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use tar::Archive;

// NVIDIA-specific structures
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NvidiaFeatures {
    pub available: bool,
    pub gpu_name: Option<String>,
    pub driver_version: Option<String>,
    pub vram_mb: Option<u32>,
    pub supports_rtx: bool,
    pub supports_dlss: bool,
    pub supports_nvapi: bool,
    pub optimus_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NvidiaGameSettings {
    pub prime_render_offload: bool,
    pub dlss_enabled: bool,
    pub rtx_enabled: bool,
    pub environment_vars: HashMap<String, String>,
}

impl NvidiaGameSettings {
    pub fn available(&self) -> bool {
        !self.environment_vars.is_empty()
    }
}

// GameScope configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameScopeConfig {
    pub width: u32,
    pub height: u32,
    pub refresh_rate: Option<u32>,
    pub fullscreen: bool,
    pub borderless: bool,
    pub adaptive_sync: bool,
    pub hdr: bool,
    pub force_grab_cursor: bool,
    pub steam_integration: bool,
    pub upscaling: GameScopeUpscaling,
    pub scaling_filter: GameScopeFilter,
}

impl Default for GameScopeConfig {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            refresh_rate: None,
            fullscreen: false,
            borderless: true,
            adaptive_sync: true,
            hdr: false,
            force_grab_cursor: false,
            steam_integration: false,
            upscaling: GameScopeUpscaling::None,
            scaling_filter: GameScopeFilter::Linear,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameScopeUpscaling {
    None,
    FSR, // FidelityFX Super Resolution
    NIS, // NVIDIA Image Scaling
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameScopeFilter {
    Nearest,
    Linear,
    FSR,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphicsManager {
    pub dxvk_dir: PathBuf,
    pub vkd3d_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub dry_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphicsLayer {
    pub name: String,
    pub version: String,
    pub layer_type: GraphicsLayerType,
    pub path: PathBuf,
    pub installed: bool,
    pub download_url: Option<String>,
    pub checksum: Option<String>,
    pub supported_apis: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GraphicsLayerType {
    DXVK,        // DirectX 9/10/11 to Vulkan
    VKD3D,       // Direct3D 12 to Vulkan
    DXVKDGPU,    // DXVK for older GPUs
    VKD3DProton, // Valve's VKD3D-Proton
    DXVKNVAPI,   // NVIDIA-specific features
    WineD3D,     // Wine's built-in D3D
    GameScope,   // Valve's GameScope compositor
    NvidiaDlss,  // NVIDIA DLSS
    NvidiaRtx,   // NVIDIA RTX features
    MangoHud,    // Performance overlay
    GameMode,    // System optimizations
}

impl GraphicsManager {
    pub fn new(base_dir: PathBuf) -> Result<Self> {
        let dxvk_dir = base_dir.join("dxvk");
        let vkd3d_dir = base_dir.join("vkd3d");
        let cache_dir = base_dir.join("cache");

        fs::create_dir_all(&dxvk_dir)?;
        fs::create_dir_all(&vkd3d_dir)?;
        fs::create_dir_all(&cache_dir)?;

        Ok(Self {
            dxvk_dir,
            vkd3d_dir,
            cache_dir,
            dry_run: true, // Safe default
        })
    }

    pub fn set_dry_run(&mut self, dry_run: bool) {
        self.dry_run = dry_run;
    }

    pub async fn list_available_dxvk(&self) -> Result<Vec<GraphicsLayer>> {
        let mut versions = Vec::new();

        // Fetch DXVK releases from GitHub
        let client = reqwest::Client::new();
        let response = client
            .get("https://api.github.com/repos/doitsujin/dxvk/releases")
            .header("User-Agent", "GhostForge")
            .send()
            .await?;

        let releases: Vec<serde_json::Value> = response.json().await?;

        for release in releases.iter().take(10) {
            if let Some(tag) = release["tag_name"].as_str() {
                if let Some(assets) = release["assets"].as_array() {
                    for asset in assets {
                        if let Some(name) = asset["name"].as_str() {
                            if name.ends_with(".tar.gz") && name.contains("dxvk") {
                                versions.push(GraphicsLayer {
                                    name: format!("DXVK {}", tag),
                                    version: tag.to_string(),
                                    layer_type: GraphicsLayerType::DXVK,
                                    path: self.dxvk_dir.join(format!("dxvk-{}", tag)),
                                    installed: false,
                                    download_url: asset["browser_download_url"]
                                        .as_str()
                                        .map(String::from),
                                    checksum: None,
                                    supported_apis: vec![
                                        "d3d9".to_string(),
                                        "d3d10core".to_string(),
                                        "d3d11".to_string(),
                                    ],
                                });
                                break;
                            }
                        }
                    }
                }
            }
        }

        Ok(versions)
    }

    pub async fn list_available_vkd3d(&self) -> Result<Vec<GraphicsLayer>> {
        let mut versions = Vec::new();

        // VKD3D-Proton (Valve's version)
        let client = reqwest::Client::new();
        let response = client
            .get("https://api.github.com/repos/HansKristian-Work/vkd3d-proton/releases")
            .header("User-Agent", "GhostForge")
            .send()
            .await?;

        let releases: Vec<serde_json::Value> = response.json().await?;

        for release in releases.iter().take(10) {
            if let Some(tag) = release["tag_name"].as_str() {
                if let Some(assets) = release["assets"].as_array() {
                    for asset in assets {
                        if let Some(name) = asset["name"].as_str() {
                            if name.ends_with(".tar.xz") && name.contains("vkd3d-proton") {
                                versions.push(GraphicsLayer {
                                    name: format!("VKD3D-Proton {}", tag),
                                    version: tag.to_string(),
                                    layer_type: GraphicsLayerType::VKD3DProton,
                                    path: self.vkd3d_dir.join(format!("vkd3d-proton-{}", tag)),
                                    installed: false,
                                    download_url: asset["browser_download_url"]
                                        .as_str()
                                        .map(String::from),
                                    checksum: None,
                                    supported_apis: vec!["d3d12".to_string()],
                                });
                                break;
                            }
                        }
                    }
                }
            }
        }

        Ok(versions)
    }

    pub fn list_installed(&self) -> Result<Vec<GraphicsLayer>> {
        let mut installed = Vec::new();

        // Check installed DXVK versions
        if self.dxvk_dir.exists() {
            for entry in fs::read_dir(&self.dxvk_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    let name = path.file_name().unwrap().to_str().unwrap();
                    if name.starts_with("dxvk-") {
                        let version = name.replace("dxvk-", "");
                        installed.push(GraphicsLayer {
                            name: format!("DXVK {}", version),
                            version,
                            layer_type: GraphicsLayerType::DXVK,
                            path: path.clone(),
                            installed: true,
                            download_url: None,
                            checksum: None,
                            supported_apis: vec![
                                "d3d9".to_string(),
                                "d3d10core".to_string(),
                                "d3d11".to_string(),
                            ],
                        });
                    }
                }
            }
        }

        // Check installed VKD3D versions
        if self.vkd3d_dir.exists() {
            for entry in fs::read_dir(&self.vkd3d_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    let name = path.file_name().unwrap().to_str().unwrap();
                    if name.starts_with("vkd3d-proton-") {
                        let version = name.replace("vkd3d-proton-", "");
                        installed.push(GraphicsLayer {
                            name: format!("VKD3D-Proton {}", version),
                            version,
                            layer_type: GraphicsLayerType::VKD3DProton,
                            path: path.clone(),
                            installed: true,
                            download_url: None,
                            checksum: None,
                            supported_apis: vec!["d3d12".to_string()],
                        });
                    }
                }
            }
        }

        Ok(installed)
    }

    pub async fn install_layer(&self, layer: &GraphicsLayer) -> Result<()> {
        if layer.installed {
            return Err(anyhow::anyhow!("Layer already installed"));
        }

        if let Some(url) = &layer.download_url {
            println!("📦 Downloading {}...", layer.name);

            let client = reqwest::Client::new();
            let response = client.get(url).send().await?;
            let total_size = response.content_length().unwrap_or(0);

            let pb = ProgressBar::new(total_size);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                    .unwrap()
                    .progress_chars("#>-"),
            );

            let tmp_file = self.cache_dir.join(format!("{}.tmp", layer.name));
            let mut file = fs::File::create(&tmp_file)?;
            let mut downloaded = 0u64;
            let mut stream = response.bytes_stream();

            while let Some(chunk) = stream.next().await {
                let chunk = chunk?;
                file.write_all(&chunk)?;
                downloaded += chunk.len() as u64;
                pb.set_position(downloaded);
            }

            pb.finish_with_message("Download complete");

            // Extract
            println!("📂 Extracting {}...", layer.name);
            self.extract_graphics_layer(&tmp_file, &layer.path, &layer.layer_type)?;

            // Clean up
            fs::remove_file(&tmp_file)?;

            println!("✅ {} installed successfully", layer.name);
        }

        Ok(())
    }

    fn extract_graphics_layer(
        &self,
        archive_path: &Path,
        destination: &Path,
        _layer_type: &GraphicsLayerType,
    ) -> Result<()> {
        fs::create_dir_all(destination)?;

        if archive_path.to_str().unwrap().ends_with(".tar.gz") {
            let tar_gz = fs::File::open(archive_path)?;
            let tar = GzDecoder::new(tar_gz);
            let mut archive = Archive::new(tar);
            archive.unpack(destination)?;
        } else if archive_path.to_str().unwrap().ends_with(".tar.xz") {
            Command::new("tar")
                .args(&[
                    "-xf",
                    archive_path.to_str().unwrap(),
                    "-C",
                    destination.to_str().unwrap(),
                ])
                .status()?;
        }

        Ok(())
    }

    pub fn install_to_prefix(&self, layer: &GraphicsLayer, prefix_path: &Path) -> Result<()> {
        if self.dry_run {
            println!(
                "🔄 [DRY RUN] Would install {} to prefix: {}",
                layer.name,
                prefix_path.display()
            );
            return Ok(());
        }

        match layer.layer_type {
            GraphicsLayerType::DXVK => self.install_dxvk_to_prefix(layer, prefix_path),
            GraphicsLayerType::VKD3DProton => self.install_vkd3d_to_prefix(layer, prefix_path),
            _ => Err(anyhow::anyhow!(
                "Unsupported layer type for prefix installation"
            )),
        }
    }

    fn install_dxvk_to_prefix(&self, layer: &GraphicsLayer, prefix_path: &Path) -> Result<()> {
        println!("🔧 Installing DXVK to prefix: {}", prefix_path.display());

        // Copy DXVK DLLs to system32 and syswow64
        let system32_path = prefix_path.join("drive_c/windows/system32");
        let syswow64_path = prefix_path.join("drive_c/windows/syswow64");

        fs::create_dir_all(&system32_path)?;
        fs::create_dir_all(&syswow64_path)?;

        let dxvk_dlls = vec![
            ("d3d9.dll", "x64"),
            ("d3d10core.dll", "x64"),
            ("d3d11.dll", "x64"),
            ("dxgi.dll", "x64"),
        ];

        for (dll_name, arch) in dxvk_dlls {
            let src_path = layer.path.join(arch).join(dll_name);
            let dest_path = if arch == "x64" {
                system32_path.join(dll_name)
            } else {
                syswow64_path.join(dll_name)
            };

            if src_path.exists() {
                fs::copy(&src_path, &dest_path)?;
                println!("  ✅ Installed {}", dll_name);
            }
        }

        // Set DLL overrides
        self.set_dxvk_dll_overrides(prefix_path)?;

        Ok(())
    }

    fn install_vkd3d_to_prefix(&self, layer: &GraphicsLayer, prefix_path: &Path) -> Result<()> {
        println!(
            "🔧 Installing VKD3D-Proton to prefix: {}",
            prefix_path.display()
        );

        let system32_path = prefix_path.join("drive_c/windows/system32");
        fs::create_dir_all(&system32_path)?;

        let vkd3d_dlls = vec!["d3d12.dll", "dxcore.dll"];

        for dll_name in vkd3d_dlls {
            let src_path = layer.path.join("x64").join(dll_name);
            let dest_path = system32_path.join(dll_name);

            if src_path.exists() {
                fs::copy(&src_path, &dest_path)?;
                println!("  ✅ Installed {}", dll_name);
            }
        }

        // Set VKD3D DLL overrides
        self.set_vkd3d_dll_overrides(prefix_path)?;

        Ok(())
    }

    fn set_dxvk_dll_overrides(&self, prefix_path: &Path) -> Result<()> {
        let overrides = vec![
            ("d3d9", "native"),
            ("d3d10core", "native"),
            ("d3d11", "native"),
            ("dxgi", "native"),
        ];

        for (dll, mode) in overrides {
            if self.dry_run {
                println!("🔄 [DRY RUN] Would set DLL override: {} = {}", dll, mode);
            } else {
                Command::new("wine")
                    .env("WINEPREFIX", prefix_path)
                    .args(&[
                        "reg",
                        "add",
                        "HKEY_CURRENT_USER\\Software\\Wine\\DllOverrides",
                        "/v",
                        dll,
                        "/d",
                        mode,
                        "/f",
                    ])
                    .output()?;
                println!("  ✅ Set {} override to {}", dll, mode);
            }
        }

        Ok(())
    }

    fn set_vkd3d_dll_overrides(&self, prefix_path: &Path) -> Result<()> {
        let overrides = vec![("d3d12", "native"), ("dxcore", "native")];

        for (dll, mode) in overrides {
            if self.dry_run {
                println!("🔄 [DRY RUN] Would set DLL override: {} = {}", dll, mode);
            } else {
                Command::new("wine")
                    .env("WINEPREFIX", prefix_path)
                    .args(&[
                        "reg",
                        "add",
                        "HKEY_CURRENT_USER\\Software\\Wine\\DllOverrides",
                        "/v",
                        dll,
                        "/d",
                        mode,
                        "/f",
                    ])
                    .output()?;
                println!("  ✅ Set {} override to {}", dll, mode);
            }
        }

        Ok(())
    }

    pub fn remove_from_prefix(
        &self,
        layer_type: GraphicsLayerType,
        prefix_path: &Path,
    ) -> Result<()> {
        if self.dry_run {
            println!(
                "🔄 [DRY RUN] Would remove {:?} from prefix: {}",
                layer_type,
                prefix_path.display()
            );
            return Ok(());
        }

        match layer_type {
            GraphicsLayerType::DXVK => self.remove_dxvk_from_prefix(prefix_path),
            GraphicsLayerType::VKD3DProton => self.remove_vkd3d_from_prefix(prefix_path),
            _ => Err(anyhow::anyhow!("Unsupported layer type for removal")),
        }
    }

    fn remove_dxvk_from_prefix(&self, prefix_path: &Path) -> Result<()> {
        println!("🗑️ Removing DXVK from prefix...");

        let system32_path = prefix_path.join("drive_c/windows/system32");
        let syswow64_path = prefix_path.join("drive_c/windows/syswow64");

        let dxvk_dlls = vec!["d3d9.dll", "d3d10core.dll", "d3d11.dll", "dxgi.dll"];

        for dll_name in dxvk_dlls {
            let dll_64 = system32_path.join(dll_name);
            let dll_32 = syswow64_path.join(dll_name);

            if dll_64.exists() {
                fs::remove_file(&dll_64)?;
                println!("  🗑️ Removed {}", dll_name);
            }
            if dll_32.exists() {
                fs::remove_file(&dll_32)?;
            }

            // Reset DLL override to builtin
            Command::new("wine")
                .env("WINEPREFIX", prefix_path)
                .args(&[
                    "reg",
                    "add",
                    "HKEY_CURRENT_USER\\Software\\Wine\\DllOverrides",
                    "/v",
                    &dll_name.replace(".dll", ""),
                    "/d",
                    "builtin",
                    "/f",
                ])
                .output()?;
        }

        Ok(())
    }

    fn remove_vkd3d_from_prefix(&self, prefix_path: &Path) -> Result<()> {
        println!("🗑️ Removing VKD3D-Proton from prefix...");

        let system32_path = prefix_path.join("drive_c/windows/system32");
        let vkd3d_dlls = vec!["d3d12.dll", "dxcore.dll"];

        for dll_name in vkd3d_dlls {
            let dll_path = system32_path.join(dll_name);
            if dll_path.exists() {
                fs::remove_file(&dll_path)?;
                println!("  🗑️ Removed {}", dll_name);
            }

            // Reset DLL override to builtin
            Command::new("wine")
                .env("WINEPREFIX", prefix_path)
                .args(&[
                    "reg",
                    "add",
                    "HKEY_CURRENT_USER\\Software\\Wine\\DllOverrides",
                    "/v",
                    &dll_name.replace(".dll", ""),
                    "/d",
                    "builtin",
                    "/f",
                ])
                .output()?;
        }

        Ok(())
    }

    pub fn get_prefix_graphics_info(&self, prefix_path: &Path) -> Result<HashMap<String, String>> {
        let mut info = HashMap::new();

        let system32_path = prefix_path.join("drive_c/windows/system32");

        // Check for DXVK
        if system32_path.join("d3d11.dll").exists() {
            info.insert("DXVK".to_string(), "Installed".to_string());
        }

        // Check for VKD3D
        if system32_path.join("d3d12.dll").exists() {
            info.insert("VKD3D".to_string(), "Installed".to_string());
        }

        // Check Wine's built-in D3D
        if !info.contains_key("DXVK") && !info.contains_key("VKD3D") {
            info.insert("WineD3D".to_string(), "Active".to_string());
        }

        Ok(info)
    }

    pub fn recommend_for_game(
        &self,
        game_name: &str,
        nvidia_features: &NvidiaFeatures,
    ) -> Vec<GraphicsLayerType> {
        let game_lower = game_name.to_lowercase();
        let mut recommendations = Vec::new();

        // Base graphics layer recommendations
        match game_lower.as_str() {
            name if name.contains("world of warcraft") => {
                recommendations.push(GraphicsLayerType::DXVK);
            }
            name if name.contains("cyberpunk") || name.contains("metro exodus") => {
                recommendations.push(GraphicsLayerType::DXVK);
                recommendations.push(GraphicsLayerType::VKD3DProton);
            }
            name if name.contains("diablo") => {
                recommendations.push(GraphicsLayerType::DXVK);
            }
            _ => {
                recommendations.push(GraphicsLayerType::DXVK); // DXVK is generally recommended
            }
        }

        // Add NVIDIA-specific recommendations
        if nvidia_features.available {
            recommendations.push(GraphicsLayerType::MangoHud); // Performance monitoring
            recommendations.push(GraphicsLayerType::GameMode); // System optimizations

            if nvidia_features.supports_dlss {
                recommendations.push(GraphicsLayerType::NvidiaDlss);
            }

            if nvidia_features.supports_rtx {
                recommendations.push(GraphicsLayerType::NvidiaRtx);
            }

            // GameScope for newer games or those that benefit from upscaling
            if game_lower.contains("cyberpunk")
                || game_lower.contains("metro")
                || game_lower.contains("control")
                || game_lower.contains("battlefield")
            {
                recommendations.push(GraphicsLayerType::GameScope);
            }
        }

        recommendations
    }

    // NVIDIA-specific methods
    pub fn detect_nvidia_features(&self) -> Result<NvidiaFeatures> {
        let mut features = NvidiaFeatures::default();

        // Check for NVIDIA GPU using nvidia-smi
        if let Ok(output) = Command::new("nvidia-smi")
            .args(&[
                "--query-gpu=name,driver_version,memory.total",
                "--format=csv,noheader,nounits",
            ])
            .output()
        {
            if output.status.success() {
                let output_str = String::from_utf8_lossy(&output.stdout);
                let lines: Vec<&str> = output_str.trim().split('\n').collect();

                if let Some(first_gpu) = lines.first() {
                    let parts: Vec<&str> = first_gpu.split(", ").collect();
                    if parts.len() >= 3 {
                        features.available = true;
                        features.gpu_name = Some(parts[0].trim().to_string());
                        features.driver_version = Some(parts[1].trim().to_string());
                        if let Ok(vram) = parts[2].trim().parse::<u32>() {
                            features.vram_mb = Some(vram);
                        }

                        // Detect RTX support based on GPU name
                        if let Some(ref name) = features.gpu_name {
                            features.supports_rtx = name.to_uppercase().contains("RTX")
                                || name.to_uppercase().contains("QUADRO RTX")
                                || name.to_uppercase().contains("TITAN RTX");

                            // DLSS support is available on RTX 20xx series and newer
                            features.supports_dlss = features.supports_rtx;
                        }

                        // Check for Optimus (hybrid graphics)
                        features.optimus_available = self.check_optimus_available();

                        // NVAPI support check (usually available on all modern NVIDIA cards)
                        features.supports_nvapi = true;
                    }
                }
            }
        }

        Ok(features)
    }

    fn check_optimus_available(&self) -> bool {
        // Check for Intel integrated graphics alongside NVIDIA
        if let Ok(output) = Command::new("lspci").output() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            let has_intel = output_str.contains("Intel") && output_str.contains("VGA");
            let has_nvidia = output_str.contains("NVIDIA");
            return has_intel && has_nvidia;
        }
        false
    }

    pub fn setup_nvidia_optimizations(
        &self,
        nvidia_features: &NvidiaFeatures,
    ) -> Result<NvidiaGameSettings> {
        let mut settings = NvidiaGameSettings::default();

        if !nvidia_features.available {
            return Ok(settings);
        }

        // Enable Prime Render Offload if Optimus is available
        if nvidia_features.optimus_available {
            settings.prime_render_offload = true;
            settings
                .environment_vars
                .insert("__NV_PRIME_RENDER_OFFLOAD".to_string(), "1".to_string());
            settings.environment_vars.insert(
                "__GLX_VENDOR_LIBRARY_NAME".to_string(),
                "nvidia".to_string(),
            );
        }

        // DLSS settings
        if nvidia_features.supports_dlss {
            settings.dlss_enabled = true;
            settings
                .environment_vars
                .insert("DXVK_ENABLE_NVAPI".to_string(), "1".to_string());
            settings.environment_vars.insert(
                "DXVK_NVAPI_ALLOW_OTHER_DRIVERS".to_string(),
                "0".to_string(),
            );
        }

        // RTX settings
        if nvidia_features.supports_rtx {
            settings.rtx_enabled = true;
            settings
                .environment_vars
                .insert("VKD3D_CONFIG".to_string(), "dxr".to_string());
        }

        // General NVIDIA optimizations
        settings
            .environment_vars
            .insert("__GL_SHADER_DISK_CACHE".to_string(), "1".to_string());
        settings.environment_vars.insert(
            "__GL_SHADER_DISK_CACHE_PATH".to_string(),
            "/tmp/nvidia_shader_cache".to_string(),
        );
        settings
            .environment_vars
            .insert("NVIDIA_DRIVER_CAPABILITIES".to_string(), "all".to_string());

        Ok(settings)
    }

    pub fn setup_gamescope(&self, config: &GameScopeConfig) -> Result<Vec<String>> {
        let mut args = vec!["gamescope".to_string()];

        // Resolution settings
        args.push("-w".to_string());
        args.push(config.width.to_string());
        args.push("-h".to_string());
        args.push(config.height.to_string());

        // Refresh rate
        if let Some(rate) = config.refresh_rate {
            args.push("-r".to_string());
            args.push(rate.to_string());
        }

        // Display mode
        if config.fullscreen {
            args.push("-f".to_string());
        } else if config.borderless {
            args.push("-b".to_string());
        }

        // Adaptive sync (FreeSync/G-Sync)
        if config.adaptive_sync {
            args.push("--adaptive-sync".to_string());
        }

        // HDR
        if config.hdr {
            args.push("--hdr-enabled".to_string());
        }

        // Force grab cursor
        if config.force_grab_cursor {
            args.push("--force-grab-cursor".to_string());
        }

        // Steam integration
        if config.steam_integration {
            args.push("--steam".to_string());
        }

        // Upscaling settings
        match config.upscaling {
            GameScopeUpscaling::FSR => {
                args.push("--fsr-upscaling".to_string());
            }
            GameScopeUpscaling::NIS => {
                args.push("--nis-upscaling".to_string());
            }
            GameScopeUpscaling::None => {}
        }

        // Scaling filter
        match config.scaling_filter {
            GameScopeFilter::Nearest => {
                args.push("--filter".to_string());
                args.push("nearest".to_string());
            }
            GameScopeFilter::Linear => {
                args.push("--filter".to_string());
                args.push("linear".to_string());
            }
            GameScopeFilter::FSR => {
                args.push("--filter".to_string());
                args.push("fsr".to_string());
            }
        }

        // Add separator for game command
        args.push("--".to_string());

        Ok(args)
    }

    pub fn setup_mangohud_nvidia(
        &self,
        nvidia_features: &NvidiaFeatures,
    ) -> Result<HashMap<String, String>> {
        let mut env_vars = HashMap::new();

        // Basic MangoHud configuration
        env_vars.insert("MANGOHUD".to_string(), "1".to_string());

        let mut config_items = vec![
            "fps",
            "frametime=0",
            "cpu_temp",
            "cpu_power",
            "cpu_mhz",
            "ram",
            "vram",
        ];

        // Add NVIDIA-specific monitoring if available
        if nvidia_features.available {
            config_items.extend_from_slice(&[
                "gpu_temp",
                "gpu_power",
                "gpu_core_clock",
                "gpu_mem_clock",
                "gpu_load_change",
                "gpu_load_value=60,90",
            ]);
        }

        let config_string = config_items.join(",");
        env_vars.insert("MANGOHUD_CONFIG".to_string(), config_string);

        // NVIDIA-specific environment variables
        if nvidia_features.available {
            env_vars.insert("MANGOHUD_DLSSG".to_string(), "1".to_string());
        }

        Ok(env_vars)
    }

    pub fn get_gamemode_command(&self) -> Vec<String> {
        vec!["gamemoderun".to_string()]
    }
}
