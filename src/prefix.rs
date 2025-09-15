use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefixManager {
    pub prefixes_dir: PathBuf,
    pub templates_dir: PathBuf,
    pub dry_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WinePrefix {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub wine_version: String,
    pub arch: String,            // win32, win64
    pub windows_version: String, // win10, win7, etc.
    pub created: DateTime<Utc>,
    pub last_used: DateTime<Utc>,
    pub size_mb: Option<u64>,
    pub template_used: Option<String>,
    pub games: Vec<String>, // Game IDs using this prefix
    pub dll_overrides: HashMap<String, String>,
    pub registry_tweaks: HashMap<String, String>,
    pub installed_packages: Vec<String>, // winetricks packages
    pub graphics_layers: Vec<String>,    // DXVK, VKD3D, etc.
    pub health_status: PrefixHealth,
    pub auto_managed: bool, // If true, GhostForge manages this prefix automatically
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PrefixHealth {
    Healthy,
    NeedsRepair,
    Corrupted,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefixTemplate {
    pub name: String,
    pub description: String,
    pub target_games: Vec<String>,
    pub wine_version: String,
    pub arch: String,
    pub windows_version: String,
    pub dll_overrides: HashMap<String, String>,
    pub registry_tweaks: HashMap<String, String>,
    pub winetricks_packages: Vec<String>,
    pub graphics_layers: Vec<String>,
    pub pre_setup_commands: Vec<String>,
    pub post_setup_commands: Vec<String>,
}

impl PrefixManager {
    pub fn new(base_dir: PathBuf) -> Result<Self> {
        let prefixes_dir = base_dir.join("prefixes");
        let templates_dir = base_dir.join("templates");

        fs::create_dir_all(&prefixes_dir)?;
        fs::create_dir_all(&templates_dir)?;

        let manager = Self {
            prefixes_dir,
            templates_dir,
            dry_run: true,
        };

        // Create built-in templates
        manager.create_builtin_templates()?;

        Ok(manager)
    }

    pub fn set_dry_run(&mut self, dry_run: bool) {
        self.dry_run = dry_run;
    }

    /// Create built-in templates for popular games
    fn create_builtin_templates(&self) -> Result<()> {
        let templates = vec![
            // Battle.net template
            PrefixTemplate {
                name: "battlenet".to_string(),
                description: "Optimized for Battle.net games (WoW, Diablo, Overwatch)".to_string(),
                target_games: vec![
                    "world_of_warcraft".to_string(),
                    "diablo_iv".to_string(),
                    "overwatch_2".to_string(),
                ],
                wine_version: "wine-staging".to_string(),
                arch: "win64".to_string(),
                windows_version: "win10".to_string(),
                dll_overrides: {
                    let mut overrides = HashMap::new();
                    overrides.insert("d3d9".to_string(), "native".to_string());
                    overrides.insert("d3d11".to_string(), "native".to_string());
                    overrides.insert("dxgi".to_string(), "native".to_string());
                    overrides
                },
                registry_tweaks: {
                    let mut tweaks = HashMap::new();
                    tweaks.insert(
                        "HKEY_CURRENT_USER\\Software\\Wine\\Direct3D\\VideoMemorySize".to_string(),
                        "8192".to_string(),
                    );
                    tweaks.insert(
                        "HKEY_CURRENT_USER\\Software\\Wine\\DirectSound\\HardwareAcceleration"
                            .to_string(),
                        "Full".to_string(),
                    );
                    tweaks
                },
                winetricks_packages: vec![
                    "corefonts".to_string(),
                    "vcrun2019".to_string(),
                    "dotnet48".to_string(),
                ],
                graphics_layers: vec!["dxvk".to_string()],
                pre_setup_commands: vec![],
                post_setup_commands: vec!["echo 'Battle.net prefix setup complete'".to_string()],
            },
            // Gaming template (generic)
            PrefixTemplate {
                name: "gaming".to_string(),
                description: "General gaming prefix with performance optimizations".to_string(),
                target_games: vec!["generic".to_string()],
                wine_version: "wine-staging".to_string(),
                arch: "win64".to_string(),
                windows_version: "win10".to_string(),
                dll_overrides: {
                    let mut overrides = HashMap::new();
                    overrides.insert("d3d9".to_string(), "native".to_string());
                    overrides.insert("d3d11".to_string(), "native".to_string());
                    overrides.insert("d3d12".to_string(), "native".to_string());
                    overrides.insert("dxgi".to_string(), "native".to_string());
                    overrides
                },
                registry_tweaks: {
                    let mut tweaks = HashMap::new();
                    tweaks.insert(
                        "HKEY_CURRENT_USER\\Software\\Wine\\Direct3D\\VideoMemorySize".to_string(),
                        "4096".to_string(),
                    );
                    tweaks.insert(
                        "HKEY_CURRENT_USER\\Software\\Wine\\Direct3D\\OffscreenRenderingMode"
                            .to_string(),
                        "fbo".to_string(),
                    );
                    tweaks
                },
                winetricks_packages: vec![
                    "corefonts".to_string(),
                    "vcrun2019".to_string(),
                    "d3dcompiler_47".to_string(),
                ],
                graphics_layers: vec!["dxvk".to_string(), "vkd3d".to_string()],
                pre_setup_commands: vec![],
                post_setup_commands: vec![],
            },
            // Legacy template for older games
            PrefixTemplate {
                name: "legacy".to_string(),
                description: "For older games requiring legacy compatibility".to_string(),
                target_games: vec!["legacy".to_string()],
                wine_version: "wine".to_string(),
                arch: "win32".to_string(),
                windows_version: "winxp".to_string(),
                dll_overrides: HashMap::new(),
                registry_tweaks: HashMap::new(),
                winetricks_packages: vec![
                    "corefonts".to_string(),
                    "vcrun2008".to_string(),
                    "d3dx9".to_string(),
                ],
                graphics_layers: vec![], // Use Wine's built-in D3D
                pre_setup_commands: vec![],
                post_setup_commands: vec![],
            },
        ];

        for template in templates {
            let template_file = self.templates_dir.join(format!("{}.json", template.name));
            if !template_file.exists() {
                let json = serde_json::to_string_pretty(&template)?;
                fs::write(&template_file, json)?;
            }
        }

        Ok(())
    }

    pub fn list_templates(&self) -> Result<Vec<PrefixTemplate>> {
        let mut templates = Vec::new();

        for entry in fs::read_dir(&self.templates_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let content = fs::read_to_string(&path)?;
                let template: PrefixTemplate = serde_json::from_str(&content)?;
                templates.push(template);
            }
        }

        Ok(templates)
    }

    pub fn create_prefix_from_template(
        &self,
        template_name: &str,
        prefix_name: &str,
    ) -> Result<WinePrefix> {
        let templates = self.list_templates()?;
        let template = templates
            .iter()
            .find(|t| t.name == template_name)
            .ok_or_else(|| anyhow::anyhow!("Template '{}' not found", template_name))?;

        let prefix_id = Uuid::new_v4().to_string();
        let prefix_path = self.prefixes_dir.join(&prefix_id);

        let mut prefix = WinePrefix {
            id: prefix_id.clone(),
            name: prefix_name.to_string(),
            path: prefix_path.clone(),
            wine_version: template.wine_version.clone(),
            arch: template.arch.clone(),
            windows_version: template.windows_version.clone(),
            created: Utc::now(),
            last_used: Utc::now(),
            size_mb: None,
            template_used: Some(template_name.to_string()),
            games: Vec::new(),
            dll_overrides: template.dll_overrides.clone(),
            registry_tweaks: template.registry_tweaks.clone(),
            installed_packages: template.winetricks_packages.clone(),
            graphics_layers: template.graphics_layers.clone(),
            health_status: PrefixHealth::Unknown,
            auto_managed: true,
        };

        if self.dry_run {
            println!(
                "🔄 [DRY RUN] Would create prefix from template '{}':",
                template_name
            );
            println!("  Name: {}", prefix_name);
            println!("  Path: {}", prefix_path.display());
            println!("  Wine Version: {}", template.wine_version);
            println!("  Architecture: {}", template.arch);
            println!("  Windows Version: {}", template.windows_version);
            println!("  DLL Overrides: {:?}", template.dll_overrides);
            println!("  Packages: {:?}", template.winetricks_packages);
            return Ok(prefix);
        }

        println!(
            "🍷 Creating Wine prefix from '{}' template...",
            template_name
        );

        // Create the prefix directory
        fs::create_dir_all(&prefix_path)?;

        // Initialize the prefix
        self.initialize_prefix(&prefix)?;

        // Apply template configuration
        self.apply_template_configuration(&prefix, template)?;

        // Save prefix metadata
        self.save_prefix_metadata(&prefix)?;

        // Check initial health
        prefix.health_status = self.check_prefix_health(&prefix)?;

        println!("✅ Prefix '{}' created successfully", prefix_name);

        Ok(prefix)
    }

    fn initialize_prefix(&self, prefix: &WinePrefix) -> Result<()> {
        println!("⏳ Initializing Wine prefix...");

        let wine_cmd = if prefix.wine_version == "system" {
            "wine".to_string()
        } else {
            // In a real implementation, you'd locate the specific Wine version
            "wine".to_string()
        };

        let mut cmd = Command::new(&wine_cmd);
        cmd.env("WINEPREFIX", &prefix.path);
        cmd.env("WINEARCH", &prefix.arch);
        cmd.args(&["wineboot", "--init"]);

        let output = cmd.output()?;
        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "Failed to initialize prefix: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        Ok(())
    }

    fn apply_template_configuration(
        &self,
        prefix: &WinePrefix,
        template: &PrefixTemplate,
    ) -> Result<()> {
        println!("⚙️ Applying template configuration...");

        // Set Windows version
        self.set_windows_version(prefix, &template.windows_version)?;

        // Apply DLL overrides
        for (dll, mode) in &template.dll_overrides {
            self.set_dll_override(prefix, dll, mode)?;
        }

        // Apply registry tweaks
        for (key, value) in &template.registry_tweaks {
            self.set_registry_value(prefix, key, value)?;
        }

        // Install winetricks packages (would integrate with our winetricks module)
        for package in &template.winetricks_packages {
            println!("📦 Would install winetricks package: {}", package);
            // self.install_winetricks_package(prefix, package)?;
        }

        // Apply graphics layers (would integrate with our graphics module)
        for layer in &template.graphics_layers {
            println!("🎨 Would apply graphics layer: {}", layer);
            // self.apply_graphics_layer(prefix, layer)?;
        }

        Ok(())
    }

    fn set_windows_version(&self, prefix: &WinePrefix, version: &str) -> Result<()> {
        Command::new("wine")
            .env("WINEPREFIX", &prefix.path)
            .args(&[
                "reg",
                "add",
                "HKEY_CURRENT_USER\\Software\\Wine",
                "/v",
                "Version",
                "/d",
                version,
                "/f",
            ])
            .output()?;

        println!("  ✅ Set Windows version to {}", version);
        Ok(())
    }

    fn set_dll_override(&self, prefix: &WinePrefix, dll: &str, mode: &str) -> Result<()> {
        Command::new("wine")
            .env("WINEPREFIX", &prefix.path)
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
        Ok(())
    }

    fn set_registry_value(&self, prefix: &WinePrefix, key: &str, value: &str) -> Result<()> {
        // Parse the key to extract the registry path and value name
        if let Some(last_slash) = key.rfind('\\') {
            let (reg_path, value_name) = key.split_at(last_slash);
            let value_name = &value_name[1..]; // Remove leading backslash

            Command::new("wine")
                .env("WINEPREFIX", &prefix.path)
                .args(&["reg", "add", reg_path, "/v", value_name, "/d", value, "/f"])
                .output()?;

            println!("  ✅ Set registry {}/{} = {}", reg_path, value_name, value);
        }

        Ok(())
    }

    pub fn clone_prefix(&self, source_id: &str, new_name: &str) -> Result<WinePrefix> {
        let source_prefix = self.get_prefix(source_id)?;
        let new_id = Uuid::new_v4().to_string();
        let new_path = self.prefixes_dir.join(&new_id);

        if self.dry_run {
            println!("🔄 [DRY RUN] Would clone prefix:");
            println!(
                "  From: {} ({})",
                source_prefix.name,
                source_prefix.path.display()
            );
            println!("  To: {} ({})", new_name, new_path.display());
            return Ok(source_prefix); // Return source for demo
        }

        println!(
            "📋 Cloning prefix '{}' to '{}'...",
            source_prefix.name, new_name
        );

        // Copy the entire prefix directory
        self.copy_directory(&source_prefix.path, &new_path)?;

        let mut new_prefix = source_prefix.clone();
        new_prefix.id = new_id;
        new_prefix.name = new_name.to_string();
        new_prefix.path = new_path;
        new_prefix.created = Utc::now();
        new_prefix.last_used = Utc::now();
        new_prefix.games = Vec::new(); // Don't copy game associations

        // Save metadata for the new prefix
        self.save_prefix_metadata(&new_prefix)?;

        println!("✅ Prefix cloned successfully");

        Ok(new_prefix)
    }

    fn copy_directory(&self, src: &Path, dst: &Path) -> Result<()> {
        // Use system cp for efficiency
        let output = Command::new("cp")
            .args(&["-r", src.to_str().unwrap(), dst.to_str().unwrap()])
            .output()?;

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "Failed to copy directory: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        Ok(())
    }

    pub fn list_prefixes(&self) -> Result<Vec<WinePrefix>> {
        let mut prefixes = Vec::new();

        if !self.prefixes_dir.exists() {
            return Ok(prefixes);
        }

        for entry in fs::read_dir(&self.prefixes_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                let metadata_file = path.join("ghostforge.json");
                if metadata_file.exists() {
                    let content = fs::read_to_string(&metadata_file)?;
                    let prefix: WinePrefix = serde_json::from_str(&content)?;
                    prefixes.push(prefix);
                }
            }
        }

        Ok(prefixes)
    }

    pub fn get_prefix(&self, id: &str) -> Result<WinePrefix> {
        let prefix_dir = self.prefixes_dir.join(id);
        let metadata_file = prefix_dir.join("ghostforge.json");

        if !metadata_file.exists() {
            return Err(anyhow::anyhow!("Prefix '{}' not found", id));
        }

        let content = fs::read_to_string(&metadata_file)?;
        let prefix: WinePrefix = serde_json::from_str(&content)?;

        Ok(prefix)
    }

    fn save_prefix_metadata(&self, prefix: &WinePrefix) -> Result<()> {
        let metadata_file = prefix.path.join("ghostforge.json");
        let json = serde_json::to_string_pretty(prefix)?;
        fs::write(&metadata_file, json)?;

        Ok(())
    }

    pub fn check_prefix_health(&self, prefix: &WinePrefix) -> Result<PrefixHealth> {
        // Check if essential Wine files exist
        let essential_files = vec!["system.reg", "user.reg", "drive_c/windows/system32"];

        for file in essential_files {
            let file_path = prefix.path.join(file);
            if !file_path.exists() {
                return Ok(PrefixHealth::Corrupted);
            }
        }

        // Check if Wine can run in the prefix
        let output = Command::new("wine")
            .env("WINEPREFIX", &prefix.path)
            .args(&["--version"])
            .output();

        match output {
            Ok(output) if output.status.success() => Ok(PrefixHealth::Healthy),
            _ => Ok(PrefixHealth::NeedsRepair),
        }
    }

    pub fn repair_prefix(&self, prefix: &WinePrefix) -> Result<()> {
        if self.dry_run {
            println!("🔄 [DRY RUN] Would repair prefix: {}", prefix.name);
            println!("  • Run wineboot --init");
            println!("  • Reinstall essential packages");
            println!("  • Reapply DLL overrides");
            return Ok(());
        }

        println!("🔧 Repairing prefix '{}'...", prefix.name);

        // Reinitialize the prefix
        Command::new("wine")
            .env("WINEPREFIX", &prefix.path)
            .args(&["wineboot", "--init"])
            .output()?;

        // Reapply DLL overrides
        for (dll, mode) in &prefix.dll_overrides {
            self.set_dll_override(prefix, dll, mode)?;
        }

        // Reapply registry tweaks
        for (key, value) in &prefix.registry_tweaks {
            self.set_registry_value(prefix, key, value)?;
        }

        println!("✅ Prefix repair completed");

        Ok(())
    }

    pub fn delete_prefix(&self, id: &str) -> Result<()> {
        let prefix = self.get_prefix(id)?;

        if self.dry_run {
            println!("🔄 [DRY RUN] Would delete prefix: {}", prefix.name);
            println!("  Path: {}", prefix.path.display());
            return Ok(());
        }

        println!("🗑️ Deleting prefix '{}'...", prefix.name);

        if prefix.path.exists() {
            fs::remove_dir_all(&prefix.path)?;
            println!("✅ Prefix deleted successfully");
        }

        Ok(())
    }

    pub fn get_prefix_size(&self, prefix: &WinePrefix) -> Result<u64> {
        if !prefix.path.exists() {
            return Ok(0);
        }

        let output = Command::new("du")
            .args(&["-sb", prefix.path.to_str().unwrap()])
            .output()?;

        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            if let Some(size_str) = output_str.split_whitespace().next() {
                return Ok(size_str.parse().unwrap_or(0));
            }
        }

        Ok(0)
    }
}
