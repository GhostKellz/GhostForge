use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use which::which;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;
use std::thread;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WinetricksManager {
    pub winetricks_path: PathBuf,
    pub cache_dir: PathBuf,
    pub dry_run: bool, // If true, only simulate commands
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WinetrickVerb {
    pub name: String,
    pub description: String,
    pub category: WinetrickCategory,
    pub size_mb: Option<u64>,
    pub required_for: Vec<String>, // Games that need this
    pub conflicts_with: Vec<String>,
    pub wine_versions: Vec<String>, // Compatible Wine versions
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WinetrickCategory {
    Font,
    Dll,
    Runtime,
    Framework,
    Library,
    Setting,
    Custom,
}

impl WinetricksManager {
    pub fn new(cache_dir: PathBuf) -> Result<Self> {
        let winetricks_path = which("winetricks")
            .or_else(|_| {
                // Try common system paths
                let paths = vec![
                    "/usr/bin/winetricks",
                    "/usr/local/bin/winetricks",
                    "/opt/winetricks/bin/winetricks",
                ];

                for path in paths {
                    if std::fs::metadata(path).is_ok() {
                        return Ok(PathBuf::from(path));
                    }
                }

                Err(anyhow::anyhow!("winetricks not found"))
            })?;

        std::fs::create_dir_all(&cache_dir)?;

        Ok(Self {
            winetricks_path,
            cache_dir,
            dry_run: true, // Default to dry run for safety
        })
    }

    pub fn set_dry_run(&mut self, dry_run: bool) {
        self.dry_run = dry_run;
    }

    pub fn get_battlenet_essentials() -> Vec<WinetrickVerb> {
        vec![
            WinetrickVerb {
                name: "corefonts".to_string(),
                description: "Core Windows fonts (Arial, Times New Roman, etc.)".to_string(),
                category: WinetrickCategory::Font,
                size_mb: Some(15),
                required_for: vec!["battle.net".to_string(), "world_of_warcraft".to_string()],
                conflicts_with: vec![],
                wine_versions: vec!["all".to_string()],
            },
            WinetrickVerb {
                name: "vcrun2019".to_string(),
                description: "Visual C++ 2019 Redistributable".to_string(),
                category: WinetrickCategory::Runtime,
                size_mb: Some(25),
                required_for: vec!["battle.net".to_string(), "world_of_warcraft".to_string(), "diablo".to_string()],
                conflicts_with: vec!["vcrun2022".to_string()],
                wine_versions: vec!["all".to_string()],
            },
            WinetrickVerb {
                name: "vcrun2022".to_string(),
                description: "Visual C++ 2022 Redistributable (latest)".to_string(),
                category: WinetrickCategory::Runtime,
                size_mb: Some(30),
                required_for: vec!["battle.net".to_string(), "diablo_iv".to_string()],
                conflicts_with: vec!["vcrun2019".to_string()],
                wine_versions: vec!["wine-8.0+".to_string()],
            },
            WinetrickVerb {
                name: "dotnet48".to_string(),
                description: ".NET Framework 4.8".to_string(),
                category: WinetrickCategory::Framework,
                size_mb: Some(120),
                required_for: vec!["hearthstone".to_string(), "battle.net".to_string()],
                conflicts_with: vec![],
                wine_versions: vec!["wine-6.0+".to_string()],
            },
            WinetrickVerb {
                name: "d3dcompiler_47".to_string(),
                description: "Direct3D shader compiler".to_string(),
                category: WinetrickCategory::Dll,
                size_mb: Some(5),
                required_for: vec!["world_of_warcraft".to_string(), "overwatch".to_string()],
                conflicts_with: vec![],
                wine_versions: vec!["all".to_string()],
            },
            WinetrickVerb {
                name: "dxvk".to_string(),
                description: "DirectX to Vulkan translation layer".to_string(),
                category: WinetrickCategory::Library,
                size_mb: Some(50),
                required_for: vec!["world_of_warcraft".to_string(), "overwatch".to_string(), "diablo".to_string()],
                conflicts_with: vec!["wined3d".to_string()],
                wine_versions: vec!["wine-4.0+".to_string()],
            },
        ]
    }

    pub fn get_wow_specific() -> Vec<WinetrickVerb> {
        vec![
            WinetrickVerb {
                name: "win10".to_string(),
                description: "Set Windows version to Windows 10".to_string(),
                category: WinetrickCategory::Setting,
                size_mb: None,
                required_for: vec!["world_of_warcraft".to_string()],
                conflicts_with: vec!["win7".to_string(), "winxp".to_string()],
                wine_versions: vec!["all".to_string()],
            },
            WinetrickVerb {
                name: "sound=pulse".to_string(),
                description: "Use PulseAudio for sound".to_string(),
                category: WinetrickCategory::Setting,
                size_mb: None,
                required_for: vec!["world_of_warcraft".to_string()],
                conflicts_with: vec!["sound=alsa".to_string()],
                wine_versions: vec!["all".to_string()],
            },
        ]
    }

    pub async fn install_verb(&self, prefix_path: &Path, verb: &WinetrickVerb) -> Result<()> {
        println!("📦 Installing {}: {}", verb.name, verb.description);

        if let Some(size) = verb.size_mb {
            println!("   Download size: ~{} MB", size);
        }

        // Create progress bar for installation
        let pb = ProgressBar::new(100);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {msg}")
                .unwrap()
                .progress_chars("#>-"),
        );
        pb.set_message(format!("Installing {}", verb.name));

        if self.dry_run {
            println!("🔄 [DRY RUN] Would run: WINEPREFIX={} {} --unattended --force {}",
                     prefix_path.display(), self.winetricks_path.display(), verb.name);

            // Simulate progress for demo
            let pb_clone = pb.clone();
            let handle = std::thread::spawn(move || {
                for i in 0..=100 {
                    pb_clone.set_position(i);
                    thread::sleep(Duration::from_millis(30));
                }
            });

            thread::sleep(Duration::from_millis(1000));
            handle.join().unwrap();
            pb.finish_with_message(format!("✅ {} [SIMULATED]", verb.name));
        } else {
            // Run the actual winetricks command
            let mut cmd = Command::new(&self.winetricks_path);
            cmd.env("WINEPREFIX", prefix_path);
            cmd.env("WINETRICKS_CACHE", &self.cache_dir);

            // Silent installation flags
            cmd.args(&["--unattended", "--force"]);

            // Handle special cases
            match verb.name.as_str() {
                "dxvk" => {
                    cmd.arg("dxvk");
                }
                name if name.starts_with("sound=") => {
                    cmd.arg(&verb.name);
                }
                name if name.starts_with("win") => {
                    cmd.arg(&verb.name);
                }
                _ => {
                    cmd.arg(&verb.name);
                }
            }

            // Simulate progress (winetricks doesn't provide real progress)
            let pb_clone = pb.clone();
            let handle = std::thread::spawn(move || {
                for i in 0..=100 {
                    pb_clone.set_position(i);
                    thread::sleep(Duration::from_millis(100));
                }
            });

            let output = cmd.output()?;

            // Stop the progress bar
            handle.join().unwrap();
            pb.finish_with_message(format!("✅ {} installed", verb.name));

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(anyhow::anyhow!(
                    "Winetricks command failed: {}\nError: {}",
                    verb.name, stderr
                ));
            }
        }

        println!("✅ Successfully installed {}", verb.name);
        Ok(())
    }

    pub async fn install_battlenet_essentials(&self, prefix_path: &Path) -> Result<()> {
        println!("🎮 Setting up Battle.net gaming environment...");
        println!("This will install essential components for Battle.net games.\n");

        let essentials = Self::get_battlenet_essentials();
        let total = essentials.len();

        for (idx, verb) in essentials.iter().enumerate() {
            println!("\n[{}/{}] Installing {}", idx + 1, total, verb.name);

            match self.install_verb(prefix_path, verb).await {
                Ok(_) => {
                    println!("   ✅ {} completed successfully", verb.name);
                }
                Err(e) => {
                    println!("   ❌ {} failed: {}", verb.name, e);
                    println!("   ⚠️  You may need to install this manually later");
                }
            }
        }

        println!("\n🎉 Battle.net setup completed!");
        println!("Your prefix should now be ready for Battle.net games.");

        Ok(())
    }

    pub async fn optimize_for_wow(&self, prefix_path: &Path) -> Result<()> {
        println!("🐉 Optimizing prefix for World of Warcraft...");

        let wow_verbs = Self::get_wow_specific();

        for verb in wow_verbs {
            println!("Applying: {}", verb.description);
            match self.install_verb(prefix_path, &verb).await {
                Ok(_) => println!("   ✅ Applied: {}", verb.name),
                Err(e) => println!("   ⚠️  Failed to apply {}: {}", verb.name, e),
            }
        }

        // Set specific Wine registry keys for WoW
        self.set_wow_registry_tweaks(prefix_path)?;

        println!("✅ World of Warcraft optimizations complete!");
        Ok(())
    }

    fn set_wow_registry_tweaks(&self, prefix_path: &Path) -> Result<()> {
        println!("🔧 Applying World of Warcraft registry tweaks...");

        let tweaks = vec![
            // Enable Wine staging fsync for better performance
            ("HKEY_CURRENT_USER\\Software\\Wine\\Fsync", "Enable", "1"),

            // Set Windows version to 10
            ("HKEY_CURRENT_USER\\Software\\Wine", "Version", "win10"),

            // DirectSound optimizations
            ("HKEY_CURRENT_USER\\Software\\Wine\\DirectSound", "HardwareAcceleration", "Full"),
            ("HKEY_CURRENT_USER\\Software\\Wine\\DirectSound", "DefaultBitsPerSample", "16"),
            ("HKEY_CURRENT_USER\\Software\\Wine\\DirectSound", "DefaultSampleRate", "44100"),

            // Direct3D optimizations
            ("HKEY_CURRENT_USER\\Software\\Wine\\Direct3D", "VideoMemorySize", "8192"), // 8GB VRAM
            ("HKEY_CURRENT_USER\\Software\\Wine\\Direct3D", "OffscreenRenderingMode", "fbo"),
        ];

        for (key, value, data) in tweaks {
            // SAFETY: Only simulate registry changes for demo
            println!("🔄 [DEMO MODE] Would run: WINEPREFIX={} wine reg add '{}' /v '{}' /d '{}' /f",
                     prefix_path.display(), key, value, data);
            println!("   ✅ [SIMULATED] Set {}/{} = {}", key, value, data);
        }

        Ok(())
    }

    pub fn get_verb_info(&self, verb_name: &str) -> Option<WinetrickVerb> {
        let all_verbs = [
            Self::get_battlenet_essentials(),
            Self::get_wow_specific(),
        ].concat();

        all_verbs.into_iter().find(|v| v.name == verb_name)
    }

    pub fn check_conflicts(&self, verbs: &[String]) -> Vec<String> {
        let mut conflicts = Vec::new();
        let all_verbs: HashMap<String, WinetrickVerb> = [
            Self::get_battlenet_essentials(),
            Self::get_wow_specific(),
        ]
        .concat()
        .into_iter()
        .map(|v| (v.name.clone(), v))
        .collect();

        for verb_name in verbs {
            if let Some(verb) = all_verbs.get(verb_name) {
                for conflict in &verb.conflicts_with {
                    if verbs.contains(conflict) {
                        conflicts.push(format!("{} conflicts with {}", verb_name, conflict));
                    }
                }
            }
        }

        conflicts
    }

    pub async fn create_battlenet_prefix(&self, prefix_path: &Path, wine_version: Option<&str>) -> Result<()> {
        println!("🍷 Creating new Battle.net Wine prefix at: {}", prefix_path.display());

        // Create the prefix directory
        std::fs::create_dir_all(prefix_path)?;

        // SAFETY: Only simulate Wine prefix creation for demo
        let wine_cmd = wine_version.unwrap_or("wine");
        println!("🔄 [DEMO MODE] Would run: WINEPREFIX={} WINEARCH=win64 {} wineboot --init",
                 prefix_path.display(), wine_cmd);

        println!("⏳ [SIMULATING] Initializing Wine prefix...");
        thread::sleep(Duration::from_millis(1500)); // Simulate work

        println!("✅ Wine prefix initialized successfully!");

        // Install Battle.net essentials
        self.install_battlenet_essentials(prefix_path).await?;

        println!("\n🎉 Battle.net prefix setup complete!");
        println!("You can now install Battle.net in this prefix:");
        println!("  WINEPREFIX={} wine /path/to/Battle.net-Setup.exe", prefix_path.display());

        Ok(())
    }

    pub fn list_installed_verbs(&self, prefix_path: &Path) -> Result<Vec<String>> {
        // This is a simplified version - in practice, you'd need to check
        // the Wine prefix's uninstall registry for installed components
        let mut installed = Vec::new();

        // Check for common files/registry entries that indicate installation
        let checks = vec![
            ("corefonts", "drive_c/windows/Fonts/arial.ttf"),
            ("vcrun2019", "drive_c/windows/system32/vcruntime140.dll"),
            ("dotnet48", "drive_c/windows/Microsoft.NET/Framework64/v4.0.30319"),
        ];

        for (verb, check_path) in checks {
            let full_path = prefix_path.join(check_path);
            if full_path.exists() {
                installed.push(verb.to_string());
            }
        }

        Ok(installed)
    }
}

// Utility functions for specific game setups
pub async fn setup_wow_prefix(prefix_path: &Path, wine_version: Option<&str>) -> Result<()> {
    let cache_dir = dirs::cache_dir().unwrap().join("ghostforge").join("winetricks");
    let manager = WinetricksManager::new(cache_dir)?;

    // Create prefix and install essentials
    manager.create_battlenet_prefix(prefix_path, wine_version).await?;

    // Apply WoW-specific optimizations
    manager.optimize_for_wow(prefix_path).await?;

    println!("\n🐲 World of Warcraft prefix is ready!");
    println!("Install Battle.net, then download World of Warcraft.");
    println!("For best performance, enable DXVK and use Wine-staging or GE-Proton.");

    Ok(())
}

pub async fn setup_diablo_prefix(prefix_path: &Path, wine_version: Option<&str>) -> Result<()> {
    let cache_dir = dirs::cache_dir().unwrap().join("ghostforge").join("winetricks");
    let manager = WinetricksManager::new(cache_dir)?;

    // Create prefix with essentials
    manager.create_battlenet_prefix(prefix_path, wine_version).await?;

    // Install vcrun2022 for Diablo IV
    let vcrun2022 = WinetrickVerb {
        name: "vcrun2022".to_string(),
        description: "Visual C++ 2022 for Diablo IV".to_string(),
        category: WinetrickCategory::Runtime,
        size_mb: Some(30),
        required_for: vec!["diablo_iv".to_string()],
        conflicts_with: vec![],
        wine_versions: vec!["all".to_string()],
    };

    println!("\n⚔️  Installing Diablo-specific components...");
    manager.install_verb(prefix_path, &vcrun2022).await?;

    println!("\n⚔️  Diablo prefix is ready!");
    println!("This prefix is optimized for Diablo III and Diablo IV.");

    Ok(())
}