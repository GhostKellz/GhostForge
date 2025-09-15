use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;
use sysinfo::System;
use which::which;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os: String,
    pub kernel: String,
    pub desktop: Option<String>,
    pub gpu: Vec<GpuInfo>,
    pub cpu: CpuInfo,
    pub memory: MemoryInfo,
    pub vulkan: VulkanInfo,
    pub wine_support: WineSupport,
    pub gaming_tools: GamingTools,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuInfo {
    pub vendor: GpuVendor,
    pub name: String,
    pub driver: Option<String>,
    pub vram: Option<u64>,
    pub vulkan_support: bool,
    pub dxvk_support: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GpuVendor {
    Nvidia,
    AMD,
    Intel,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuInfo {
    pub brand: String,
    pub cores: usize,
    pub threads: usize,
    pub frequency: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryInfo {
    pub total: u64,
    pub available: u64,
    pub swap_total: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VulkanInfo {
    pub available: bool,
    pub driver_version: Option<String>,
    pub api_version: Option<String>,
    pub devices: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WineSupport {
    pub installed: bool,
    pub version: Option<String>,
    pub architecture: Vec<String>,
    pub multilib_support: bool,
    pub prefix_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GamingTools {
    pub dxvk: bool,
    pub vkd3d: bool,
    pub mangohud: bool,
    pub gamemode: bool,
    pub gamescope: bool,
    pub winetricks: bool,
    pub protontricks: bool,
}

pub struct SystemDetector;

impl SystemDetector {
    pub fn get_system_info() -> Result<SystemInfo> {
        let mut system = System::new_all();
        system.refresh_all();

        Ok(SystemInfo {
            os: Self::get_os_info(),
            kernel: Self::get_kernel_version(),
            desktop: Self::get_desktop_environment(),
            gpu: Self::detect_gpu()?,
            cpu: Self::get_cpu_info(&system),
            memory: Self::get_memory_info(&system),
            vulkan: Self::get_vulkan_info()?,
            wine_support: Self::get_wine_support()?,
            gaming_tools: Self::detect_gaming_tools()?,
        })
    }

    fn get_os_info() -> String {
        if let Ok(output) = Command::new("lsb_release").args(&["-d", "-s"]).output() {
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        } else if let Ok(contents) = std::fs::read_to_string("/etc/os-release") {
            // Parse NAME or PRETTY_NAME from os-release
            for line in contents.lines() {
                if line.starts_with("PRETTY_NAME=") {
                    return line.split('=').nth(1)
                        .unwrap_or("")
                        .trim_matches('"')
                        .to_string();
                }
            }
            "Linux".to_string()
        } else {
            "Linux".to_string()
        }
    }

    fn get_kernel_version() -> String {
        if let Ok(output) = Command::new("uname").arg("-r").output() {
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        } else {
            "Unknown".to_string()
        }
    }

    fn get_desktop_environment() -> Option<String> {
        // Check common desktop environment variables
        let env_vars = vec![
            "XDG_CURRENT_DESKTOP",
            "DESKTOP_SESSION",
            "GDMSESSION",
        ];

        for var in env_vars {
            if let Ok(desktop) = std::env::var(var) {
                if !desktop.is_empty() {
                    return Some(desktop);
                }
            }
        }

        None
    }

    fn detect_gpu() -> Result<Vec<GpuInfo>> {
        let mut gpus = Vec::new();

        // Use lspci to detect GPUs
        if let Ok(output) = Command::new("lspci").args(&["-nn"]).output() {
            let output_str = String::from_utf8_lossy(&output.stdout);

            for line in output_str.lines() {
                if line.to_lowercase().contains("vga compatible controller") ||
                   line.to_lowercase().contains("3d controller") ||
                   line.to_lowercase().contains("display controller") {

                    let gpu = Self::parse_gpu_line(line)?;
                    if gpu.name != "Unknown" {
                        gpus.push(gpu);
                    }
                }
            }
        }

        // If no GPUs found via lspci, try alternate methods
        if gpus.is_empty() {
            // Check /proc/cpuinfo for integrated graphics info
            if let Ok(contents) = std::fs::read_to_string("/proc/cpuinfo") {
                if contents.contains("Intel") {
                    gpus.push(GpuInfo {
                        vendor: GpuVendor::Intel,
                        name: "Intel Integrated Graphics".to_string(),
                        driver: None,
                        vram: None,
                        vulkan_support: false,
                        dxvk_support: false,
                    });
                }
            }
        }

        // Enhance GPU info with driver and capabilities
        for gpu in &mut gpus {
            gpu.driver = Self::detect_gpu_driver(&gpu.vendor);
            gpu.vulkan_support = Self::check_vulkan_support(&gpu.vendor);
            gpu.dxvk_support = gpu.vulkan_support; // DXVK requires Vulkan
        }

        Ok(gpus)
    }

    fn parse_gpu_line(line: &str) -> Result<GpuInfo> {
        let vendor = if line.to_lowercase().contains("nvidia") {
            GpuVendor::Nvidia
        } else if line.to_lowercase().contains("amd") || line.to_lowercase().contains("ati") {
            GpuVendor::AMD
        } else if line.to_lowercase().contains("intel") {
            GpuVendor::Intel
        } else {
            GpuVendor::Unknown
        };

        // Extract GPU name (everything after the colon)
        let name = if let Some(colon_pos) = line.find(':') {
            let after_colon = &line[colon_pos + 1..];
            if let Some(bracket_pos) = after_colon.find('[') {
                after_colon[..bracket_pos].trim().to_string()
            } else {
                after_colon.trim().to_string()
            }
        } else {
            "Unknown GPU".to_string()
        };

        Ok(GpuInfo {
            vendor,
            name,
            driver: None,
            vram: None,
            vulkan_support: false,
            dxvk_support: false,
        })
    }

    fn detect_gpu_driver(vendor: &GpuVendor) -> Option<String> {
        match vendor {
            GpuVendor::Nvidia => {
                if Command::new("nvidia-smi").output().is_ok() {
                    if let Ok(output) = Command::new("nvidia-smi").args(&["--query-gpu=driver_version", "--format=csv,noheader"]).output() {
                        return Some(format!("NVIDIA {}", String::from_utf8_lossy(&output.stdout).trim()));
                    }
                    Some("NVIDIA".to_string())
                } else {
                    Some("nouveau".to_string())
                }
            },
            GpuVendor::AMD => {
                // Check for AMDGPU vs radeon driver
                if std::fs::read_to_string("/proc/modules").unwrap_or_default().contains("amdgpu") {
                    Some("amdgpu".to_string())
                } else {
                    Some("radeon".to_string())
                }
            },
            GpuVendor::Intel => {
                Some("i915".to_string())
            },
            _ => None,
        }
    }

    fn check_vulkan_support(vendor: &GpuVendor) -> bool {
        // Check if vulkaninfo is available and works
        if let Ok(output) = Command::new("vulkaninfo").args(&["--summary"]).output() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            return output_str.contains("Vulkan Instance Version");
        }

        // Fallback: assume modern GPUs support Vulkan
        match vendor {
            GpuVendor::Nvidia | GpuVendor::AMD => true,
            GpuVendor::Intel => true, // Modern Intel GPUs
            _ => false,
        }
    }

    fn get_cpu_info(system: &System) -> CpuInfo {
        let cpus = system.cpus();
        let cpu = cpus.first().unwrap();

        CpuInfo {
            brand: cpu.brand().to_string(),
            cores: system.physical_core_count().unwrap_or(cpus.len()),
            threads: cpus.len(),
            frequency: cpu.frequency(),
        }
    }

    fn get_memory_info(system: &System) -> MemoryInfo {
        MemoryInfo {
            total: system.total_memory(),
            available: system.available_memory(),
            swap_total: system.total_swap(),
        }
    }

    fn get_vulkan_info() -> Result<VulkanInfo> {
        let mut vulkan_info = VulkanInfo {
            available: false,
            driver_version: None,
            api_version: None,
            devices: Vec::new(),
        };

        if let Ok(output) = Command::new("vulkaninfo").args(&["--summary"]).output() {
            let output_str = String::from_utf8_lossy(&output.stdout);

            vulkan_info.available = true;

            // Parse API version
            for line in output_str.lines() {
                if line.contains("Vulkan Instance Version:") {
                    vulkan_info.api_version = Some(
                        line.split(':').nth(1).unwrap_or("").trim().to_string()
                    );
                }
                if line.contains("deviceName") {
                    vulkan_info.devices.push(
                        line.split('=').nth(1).unwrap_or("").trim().to_string()
                    );
                }
            }
        }

        Ok(vulkan_info)
    }

    fn get_wine_support() -> Result<WineSupport> {
        let wine_installed = which("wine").is_ok();

        let version = if wine_installed {
            Command::new("wine")
                .arg("--version")
                .output()
                .ok()
                .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            None
        };

        let architecture = if wine_installed {
            let mut archs = Vec::new();
            if which("wine").is_ok() {
                archs.push("win32".to_string());
            }
            if which("wine64").is_ok() {
                archs.push("win64".to_string());
            }
            archs
        } else {
            Vec::new()
        };

        let multilib_support = architecture.len() > 1;

        let prefix_path = std::env::var("WINEPREFIX")
            .ok()
            .map(PathBuf::from)
            .or_else(|| Some(dirs::home_dir()?.join(".wine")));

        Ok(WineSupport {
            installed: wine_installed,
            version,
            architecture,
            multilib_support,
            prefix_path,
        })
    }

    fn detect_gaming_tools() -> Result<GamingTools> {
        Ok(GamingTools {
            dxvk: Self::check_command_exists("dxvk") || Self::check_vulkan_layer("VK_LAYER_VALVE_steam_overlay"),
            vkd3d: Self::check_command_exists("vkd3d-proton") || std::path::Path::new("/usr/lib/vkd3d-proton").exists(),
            mangohud: Self::check_command_exists("mangohud") || Self::check_vulkan_layer("VK_LAYER_MANGOHUD_overlay"),
            gamemode: Self::check_command_exists("gamemoderun"),
            gamescope: Self::check_command_exists("gamescope"),
            winetricks: Self::check_command_exists("winetricks"),
            protontricks: Self::check_command_exists("protontricks"),
        })
    }

    fn check_command_exists(cmd: &str) -> bool {
        which(cmd).is_ok()
    }

    fn check_vulkan_layer(layer_name: &str) -> bool {
        if let Ok(output) = Command::new("vulkaninfo").args(&["--summary"]).output() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            return output_str.contains(layer_name);
        }
        false
    }

    pub fn optimize_system_for_gaming() -> Result<()> {
        println!("🎮 Applying gaming optimizations...");

        // Set CPU governor to performance
        if Self::check_command_exists("cpupower") {
            println!("  • Setting CPU governor to performance");
            Command::new("pkexec")
                .args(&["cpupower", "frequency-set", "-g", "performance"])
                .status()?;
        }

        // Enable GameMode if available
        if Self::check_command_exists("gamemode") {
            println!("  • GameMode is available");
        }

        // Check for compositor and suggest disabling for gaming
        if let Ok(desktop) = std::env::var("XDG_CURRENT_DESKTOP") {
            match desktop.to_lowercase().as_str() {
                "kde" | "plasma" => {
                    println!("  • Consider disabling compositing in KDE for better gaming performance");
                }
                "gnome" => {
                    println!("  • GNOME detected - consider using gamescope for better performance");
                }
                _ => {}
            }
        }

        println!("✅ System optimization suggestions applied");
        Ok(())
    }

    pub fn check_battlenet_compatibility() -> Result<String> {
        let mut report = String::new();
        report.push_str("📊 Battle.net Compatibility Report:\n");

        // Check Wine
        if let Ok(wine_path) = which("wine") {
            if let Ok(output) = Command::new(&wine_path).arg("--version").output() {
                let version = String::from_utf8_lossy(&output.stdout);
                report.push_str(&format!("  ✅ Wine: {}\n", version.trim()));
            }
        } else {
            report.push_str("  ❌ Wine not installed\n");
        }

        // Check DXVK
        if Self::check_vulkan_layer("VK_LAYER_VALVE_steam_overlay") {
            report.push_str("  ✅ DXVK: Available\n");
        } else {
            report.push_str("  ⚠️  DXVK: Not detected\n");
        }

        // Check Vulkan
        if Command::new("vulkaninfo").output().is_ok() {
            report.push_str("  ✅ Vulkan: Supported\n");
        } else {
            report.push_str("  ❌ Vulkan: Not available\n");
        }

        // Check for common dependencies
        let deps = vec![
            ("corefonts", "Core Windows fonts"),
            ("vcrun2019", "Visual C++ 2019 Redistributable"),
            ("dotnet48", ".NET Framework 4.8"),
        ];

        for (verb, desc) in deps {
            report.push_str(&format!("  ℹ️  Recommend installing: {} ({})\n", verb, desc));
        }

        Ok(report)
    }

    pub fn generate_wine_prefix_recommendation(game_name: &str) -> String {
        match game_name.to_lowercase().as_str() {
            name if name.contains("world of warcraft") || name.contains("wow") => {
                "Recommended: win10, DXVK enabled, vcrun2019, corefonts".to_string()
            }
            name if name.contains("diablo") => {
                "Recommended: win10, DXVK enabled, vcrun2019, dotnet48".to_string()
            }
            name if name.contains("overwatch") => {
                "Recommended: win10, DXVK enabled, vcrun2019, careful with anti-cheat".to_string()
            }
            name if name.contains("hearthstone") => {
                "Recommended: win10, vcrun2019, corefonts, dotnet48".to_string()
            }
            _ => {
                "Recommended: win10, DXVK enabled, vcrun2019, corefonts".to_string()
            }
        }
    }
}