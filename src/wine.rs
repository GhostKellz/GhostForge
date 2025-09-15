use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use which::which;
use futures_util::StreamExt;
use regex::Regex;
use reqwest;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::io::Write;
use tar::Archive;
use flate2::read::GzDecoder;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WineVersion {
    pub name: String,
    pub version: String,
    pub path: PathBuf,
    pub wine_type: WineType,
    pub arch: Vec<String>, // ["win32", "win64"]
    pub installed: bool,
    pub system: bool,
    pub download_url: Option<String>,
    pub checksum: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WineType {
    Wine,
    WineStaging,
    Proton,
    ProtonGE,
    ProtonTKG,
    Lutris,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WinePrefix {
    pub path: PathBuf,
    pub name: String,
    pub wine_version: String,
    pub arch: String, // win32 or win64
    pub windows_version: String,
    pub dll_overrides: HashMap<String, String>,
    pub registry_keys: HashMap<String, String>,
    pub installed_packages: Vec<String>, // winetricks verbs
    pub env_vars: HashMap<String, String>,
}

pub struct WineManager {
    wine_dir: PathBuf,
    config_dir: PathBuf,
}

impl WineManager {
    pub fn new(wine_dir: PathBuf, config_dir: PathBuf) -> Self {
        Self {
            wine_dir,
            config_dir,
        }
    }

    pub async fn list_installed(&self) -> Result<Vec<WineVersion>> {
        let mut versions = Vec::new();

        // Check system Wine
        if let Ok(wine_path) = which("wine") {
            if let Ok(version) = self.get_wine_version(&wine_path) {
                versions.push(WineVersion {
                    name: "System Wine".to_string(),
                    version,
                    path: wine_path.clone(),
                    wine_type: WineType::Wine,
                    arch: self.get_wine_arch(&wine_path)?,
                    installed: true,
                    system: true,
                    download_url: None,
                    checksum: None,
                });
            }
        }

        // Check Proton versions in Steam
        let steam_proton_dirs = vec![
            dirs::home_dir().unwrap().join(".local/share/Steam/steamapps/common"),
            dirs::home_dir().unwrap().join(".steam/steam/steamapps/common"),
            PathBuf::from("/usr/share/steam/compatibilitytools.d"),
        ];

        for dir in steam_proton_dirs {
            if dir.exists() {
                for entry in fs::read_dir(&dir)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.is_dir() {
                        let name = path.file_name().unwrap().to_str().unwrap();
                        if name.starts_with("Proton") || name.contains("proton") {
                            if let Ok(version) = self.detect_proton_version(&path) {
                                versions.push(version);
                            }
                        }
                    }
                }
            }
        }

        // Check custom Wine/Proton in GhostForge directory
        if self.wine_dir.exists() {
            for entry in fs::read_dir(&self.wine_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    if let Ok(version) = self.detect_wine_version(&path) {
                        versions.push(version);
                    }
                }
            }
        }

        Ok(versions)
    }

    pub async fn list_available(&self) -> Result<Vec<WineVersion>> {
        let mut versions = Vec::new();

        // Fetch GE-Proton releases
        let ge_proton = self.fetch_ge_proton_releases().await?;
        versions.extend(ge_proton);

        // Fetch Wine builds from WineHQ
        let wine_builds = self.fetch_wine_builds().await?;
        versions.extend(wine_builds);

        // Fetch Lutris Wine builds
        let lutris_builds = self.fetch_lutris_wine().await?;
        versions.extend(lutris_builds);

        Ok(versions)
    }

    async fn fetch_ge_proton_releases(&self) -> Result<Vec<WineVersion>> {
        let client = reqwest::Client::new();
        let response = client
            .get("https://api.github.com/repos/GloriousEggroll/proton-ge-custom/releases")
            .header("User-Agent", "GhostForge")
            .send()
            .await?;

        let releases: Vec<serde_json::Value> = response.json().await?;
        let mut versions = Vec::new();

        for release in releases.iter().take(10) {
            if let Some(tag) = release["tag_name"].as_str() {
                if let Some(assets) = release["assets"].as_array() {
                    for asset in assets {
                        if let Some(name) = asset["name"].as_str() {
                            if name.ends_with(".tar.gz") && !name.contains("sha512sum") {
                                versions.push(WineVersion {
                                    name: format!("GE-Proton {}", tag),
                                    version: tag.to_string(),
                                    path: self.wine_dir.join(format!("GE-Proton-{}", tag)),
                                    wine_type: WineType::ProtonGE,
                                    arch: vec!["win64".to_string()],
                                    installed: false,
                                    system: false,
                                    download_url: asset["browser_download_url"].as_str().map(String::from),
                                    checksum: None,
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

    async fn fetch_wine_builds(&self) -> Result<Vec<WineVersion>> {
        // Placeholder for WineHQ builds
        Ok(vec![
            WineVersion {
                name: "Wine Staging 9.18".to_string(),
                version: "9.18".to_string(),
                path: self.wine_dir.join("wine-staging-9.18"),
                wine_type: WineType::WineStaging,
                arch: vec!["win32".to_string(), "win64".to_string()],
                installed: false,
                system: false,
                download_url: Some("https://dl.winehq.org/wine-builds/".to_string()),
                checksum: None,
            },
        ])
    }

    async fn fetch_lutris_wine(&self) -> Result<Vec<WineVersion>> {
        let client = reqwest::Client::new();
        let response = client
            .get("https://api.github.com/repos/lutris/wine/releases")
            .header("User-Agent", "GhostForge")
            .send()
            .await?;

        let releases: Vec<serde_json::Value> = response.json().await?;
        let mut versions = Vec::new();

        for release in releases.iter().take(5) {
            if let Some(tag) = release["tag_name"].as_str() {
                versions.push(WineVersion {
                    name: format!("Lutris Wine {}", tag),
                    version: tag.to_string(),
                    path: self.wine_dir.join(format!("lutris-wine-{}", tag)),
                    wine_type: WineType::Lutris,
                    arch: vec!["win32".to_string(), "win64".to_string()],
                    installed: false,
                    system: false,
                    download_url: Some(format!(
                        "https://github.com/lutris/wine/releases/download/{}/wine-lutris-{}-x86_64.tar.xz",
                        tag, tag
                    )),
                    checksum: None,
                });
            }
        }

        Ok(versions)
    }

    pub async fn install_wine_version(&self, version: &WineVersion) -> Result<()> {
        if version.installed {
            return Err(anyhow::anyhow!("Version already installed"));
        }

        if let Some(url) = &version.download_url {
            println!("Downloading {} from {}", version.name, url);

            fs::create_dir_all(&self.wine_dir)?;

            let client = reqwest::Client::new();
            let response = client.get(url).send().await?;
            let total_size = response
                .content_length()
                .ok_or_else(|| anyhow::anyhow!("Failed to get content length"))?;

            let pb = ProgressBar::new(total_size);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                    .unwrap()
                    .progress_chars("#>-"),
            );

            let tmp_file = self.wine_dir.join(format!("{}.tmp", version.name));
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

            // Extract the archive
            println!("Extracting {}...", version.name);
            self.extract_archive(&tmp_file, &version.path)?;

            // Clean up
            fs::remove_file(&tmp_file)?;

            println!("✅ {} installed successfully", version.name);
        } else {
            return Err(anyhow::anyhow!("No download URL available"));
        }

        Ok(())
    }

    fn extract_archive(&self, archive_path: &Path, destination: &Path) -> Result<()> {
        fs::create_dir_all(destination)?;

        if archive_path.to_str().unwrap().ends_with(".tar.gz") {
            let tar_gz = fs::File::open(archive_path)?;
            let tar = GzDecoder::new(tar_gz);
            let mut archive = Archive::new(tar);
            archive.unpack(destination)?;
        } else if archive_path.to_str().unwrap().ends_with(".tar.xz") {
            // Use system tar for xz files
            Command::new("tar")
                .args(&["-xf", archive_path.to_str().unwrap(), "-C", destination.to_str().unwrap()])
                .status()?;
        }

        Ok(())
    }

    pub fn create_prefix(&self, prefix: &WinePrefix, wine_version: &WineVersion) -> Result<()> {
        fs::create_dir_all(&prefix.path)?;

        let wine_bin = self.get_wine_binary(&wine_version)?;

        // Set environment variables
        let mut cmd = Command::new(&wine_bin);
        cmd.env("WINEPREFIX", &prefix.path);
        cmd.env("WINEARCH", &prefix.arch);

        // Initialize the prefix
        cmd.arg("wineboot");
        cmd.arg("--init");

        let status = cmd.status()?;
        if !status.success() {
            return Err(anyhow::anyhow!("Failed to create Wine prefix"));
        }

        // Set Windows version
        self.set_windows_version(prefix, &prefix.windows_version)?;

        // Apply DLL overrides
        for (dll, mode) in &prefix.dll_overrides {
            self.set_dll_override(prefix, dll, mode)?;
        }

        Ok(())
    }

    pub fn set_windows_version(&self, prefix: &WinePrefix, version: &str) -> Result<()> {
        let _wine_bin = which("winecfg").unwrap_or_else(|_| PathBuf::from("winecfg"));

        Command::new("wine")
            .env("WINEPREFIX", &prefix.path)
            .arg("reg")
            .arg("add")
            .arg("HKEY_CURRENT_USER\\Software\\Wine")
            .arg("/v")
            .arg("Version")
            .arg("/d")
            .arg(version)
            .arg("/f")
            .status()?;

        Ok(())
    }

    pub fn set_dll_override(&self, prefix: &WinePrefix, dll: &str, mode: &str) -> Result<()> {
        Command::new("wine")
            .env("WINEPREFIX", &prefix.path)
            .arg("reg")
            .arg("add")
            .arg("HKEY_CURRENT_USER\\Software\\Wine\\DllOverrides")
            .arg("/v")
            .arg(dll)
            .arg("/d")
            .arg(mode)
            .arg("/f")
            .status()?;

        Ok(())
    }

    pub fn run_winetricks(&self, prefix: &WinePrefix, verbs: Vec<String>) -> Result<()> {
        let winetricks = which("winetricks")?;

        let mut cmd = Command::new(winetricks);
        cmd.env("WINEPREFIX", &prefix.path);

        for verb in verbs {
            cmd.arg(&verb);
        }

        let status = cmd.status()?;
        if !status.success() {
            return Err(anyhow::anyhow!("Winetricks command failed"));
        }

        Ok(())
    }

    fn get_wine_version(&self, wine_path: &Path) -> Result<String> {
        let output = Command::new(wine_path)
            .arg("--version")
            .output()?;

        let version_str = String::from_utf8_lossy(&output.stdout);
        let re = Regex::new(r"wine-?(\d+\.\d+(?:\.\d+)?)")?;

        if let Some(captures) = re.captures(&version_str) {
            Ok(captures[1].to_string())
        } else {
            Ok("unknown".to_string())
        }
    }

    fn get_wine_arch(&self, wine_path: &Path) -> Result<Vec<String>> {
        let mut archs = Vec::new();

        // Check for wine64
        let wine64_path = wine_path.with_file_name("wine64");
        if wine64_path.exists() {
            archs.push("win64".to_string());
        }

        // Wine32 is usually the default
        archs.push("win32".to_string());

        Ok(archs)
    }

    fn detect_proton_version(&self, path: &Path) -> Result<WineVersion> {
        let name = path.file_name().unwrap().to_str().unwrap();
        let version_file = path.join("version");

        let version = if version_file.exists() {
            fs::read_to_string(&version_file)?.trim().to_string()
        } else {
            name.replace("Proton ", "").replace("Proton-", "")
        };

        let wine_type = if name.contains("GE") {
            WineType::ProtonGE
        } else if name.contains("TKG") {
            WineType::ProtonTKG
        } else {
            WineType::Proton
        };

        Ok(WineVersion {
            name: name.to_string(),
            version,
            path: path.to_path_buf(),
            wine_type,
            arch: vec!["win64".to_string()],
            installed: true,
            system: false,
            download_url: None,
            checksum: None,
        })
    }

    fn detect_wine_version(&self, path: &Path) -> Result<WineVersion> {
        let name = path.file_name().unwrap().to_str().unwrap();
        let wine_bin = path.join("bin/wine");

        if wine_bin.exists() {
            let version = self.get_wine_version(&wine_bin)?;
            let wine_type = if name.contains("staging") {
                WineType::WineStaging
            } else if name.contains("lutris") {
                WineType::Lutris
            } else {
                WineType::Wine
            };

            Ok(WineVersion {
                name: name.to_string(),
                version,
                path: path.to_path_buf(),
                wine_type,
                arch: self.get_wine_arch(&wine_bin)?,
                installed: true,
                system: false,
                download_url: None,
                checksum: None,
            })
        } else {
            Err(anyhow::anyhow!("Not a valid Wine installation"))
        }
    }

    fn get_wine_binary(&self, version: &WineVersion) -> Result<PathBuf> {
        match version.wine_type {
            WineType::Proton | WineType::ProtonGE | WineType::ProtonTKG => {
                Ok(version.path.join("proton"))
            }
            _ => {
                if version.system {
                    Ok(version.path.clone())
                } else {
                    Ok(version.path.join("bin/wine"))
                }
            }
        }
    }

    pub fn remove_wine_version(&self, version: &WineVersion) -> Result<()> {
        if version.system {
            return Err(anyhow::anyhow!("Cannot remove system Wine"));
        }

        if version.path.exists() {
            fs::remove_dir_all(&version.path)?;
            println!("Removed {}", version.name);
        }

        Ok(())
    }
}