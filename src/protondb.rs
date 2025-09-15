use anyhow::Result;
use reqwest;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ProtonDB API Response Structures
#[derive(Debug, Deserialize)]
struct ProtonDBSummaryResponse {
    name: Option<String>,
    confidence: String,
    score: f32,
    tier: ProtonDBTier,
    total: i32,
    recent_reports: Option<u32>,
    trending: Option<String>,
    #[serde(rename = "bestReportedTier")]
    bestReportedTier: Option<ProtonDBTier>,
}

#[derive(Debug, Deserialize)]
struct ProtonDBReportsResponse {
    reports: Vec<ProtonDBReport>,
}

#[derive(Debug, Deserialize)]
struct ProtonDBReport {
    tier: ProtonDBTier,
    #[serde(rename = "protonVersion")]
    protonVersion: Option<String>,
    timestamp: String,
    notes: Option<String>,
    specs: Option<ProtonDBSpecs>,
    rating: Option<u8>,
}

#[derive(Debug, Deserialize)]
struct ProtonDBSpecs {
    gpu: Option<String>,
    gpu_driver: Option<String>,
    cpu: Option<String>,
    kernel: Option<String>,
    ram: Option<String>,
    os: Option<String>,
}

// Steam API structures
#[derive(Debug, Deserialize)]
struct SteamAppListResponse {
    applist: SteamAppList,
}

#[derive(Debug, Deserialize)]
struct SteamAppList {
    apps: Vec<SteamApp>,
}

#[derive(Debug, Deserialize)]
struct SteamApp {
    appid: u32,
    name: String,
}

#[derive(Debug, Clone)]
pub struct ProtonDBClient {
    pub base_url: String,
    #[allow(dead_code)]
    pub client: reqwest::Client,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtonDBGame {
    pub appid: u32,
    pub name: String,
    pub confidence: String, // "pending", "low", "medium", "high"
    pub score: f32,
    pub tier: ProtonDBTier,
    pub total_reports: u32,
    pub recent_reports: u32,
    pub trending: Option<String>,
    pub best_reported_tier: Option<ProtonDBTier>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProtonDBTier {
    Platinum, // Works perfectly out of the box
    Gold,     // Works perfectly after tweaks
    Silver,   // Works with minor issues
    Bronze,   // Runs, but crashes often or has major issues
    Borked,   // Doesn't work
    Pending,  // Not enough reports
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameReport {
    pub appid: u32,
    pub tier: ProtonDBTier,
    pub proton_version: String,
    pub timestamp: String,
    pub notes: Option<String>,
    pub specs: Option<SystemSpecs>,
    pub score: u8, // 1-10 rating
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSpecs {
    pub gpu: Option<String>,
    pub gpu_driver: Option<String>,
    pub os: Option<String>,
    pub kernel: Option<String>,
    pub ram: Option<u32>,
    pub cpu: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtonDBSummary {
    pub appid: u32,
    pub tier: ProtonDBTier,
    pub confidence: String,
    pub score: f32,
    pub total: u32,
    pub tiers: HashMap<String, u32>, // tier -> count
}

impl ProtonDBClient {
    pub fn new() -> Self {
        Self {
            base_url: "https://www.protondb.com/api/v1".to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// Get game compatibility rating and reports
    pub async fn get_game_summary(&self, steam_appid: u32) -> Result<Option<ProtonDBSummary>> {
        let url = format!("{}/reports/summaries/{}.json", self.base_url, steam_appid);

        let response = self
            .client
            .get(&url)
            .header("User-Agent", "GhostForge/1.0")
            .send()
            .await?;

        if response.status() == 404 {
            return Ok(None);
        }

        let summary: ProtonDBSummary = response.json().await?;
        Ok(Some(summary))
    }

    /// Get detailed reports for a game
    pub async fn get_game_reports(
        &self,
        steam_appid: u32,
        limit: Option<u32>,
    ) -> Result<Vec<GameReport>> {
        let limit = limit.unwrap_or(20);
        let url = format!(
            "{}/reports/summaries/{}.json?limit={}",
            self.base_url, steam_appid, limit
        );

        let response = self
            .client
            .get(&url)
            .header("User-Agent", "GhostForge/1.0")
            .send()
            .await?;

        if response.status() == 404 {
            return Ok(Vec::new());
        }

        // ProtonDB API returns reports in a different format
        // This is a simplified version - the actual API structure may vary
        let reports: Vec<GameReport> = response.json().await?;
        Ok(reports)
    }

    /// Search for games by name
    pub async fn search_games(&self, query: &str) -> Result<Vec<ProtonDBGame>> {
        // ProtonDB doesn't have a direct search API, but we can use Steam's
        // For now, this is a placeholder implementation
        let url = format!("{}/aggregate/summaries.json", self.base_url);

        let response = self
            .client
            .get(&url)
            .header("User-Agent", "GhostForge/1.0")
            .send()
            .await?;

        if response.status() != 200 {
            return Ok(Vec::new());
        }

        // Filter games by name (simplified)
        let games: Vec<ProtonDBGame> = response.json().await?;
        let filtered = games
            .into_iter()
            .filter(|game| game.name.to_lowercase().contains(&query.to_lowercase()))
            .collect();

        Ok(filtered)
    }

    /// Get trending games (works best/most reported)
    pub async fn get_trending_games(&self, limit: Option<u32>) -> Result<Vec<ProtonDBGame>> {
        let limit = limit.unwrap_or(50);
        let url = format!("{}/aggregate/summaries.json", self.base_url);

        let response = self
            .client
            .get(&url)
            .header("User-Agent", "GhostForge/1.0")
            .send()
            .await?;

        let mut games: Vec<ProtonDBGame> = response.json().await?;

        // Sort by tier and recent reports
        games.sort_by(|a, b| {
            // First by tier (Platinum > Gold > Silver...)
            let tier_order_a = match a.tier {
                ProtonDBTier::Platinum => 5,
                ProtonDBTier::Gold => 4,
                ProtonDBTier::Silver => 3,
                ProtonDBTier::Bronze => 2,
                ProtonDBTier::Borked => 1,
                ProtonDBTier::Pending => 0,
            };
            let tier_order_b = match b.tier {
                ProtonDBTier::Platinum => 5,
                ProtonDBTier::Gold => 4,
                ProtonDBTier::Silver => 3,
                ProtonDBTier::Bronze => 2,
                ProtonDBTier::Borked => 1,
                ProtonDBTier::Pending => 0,
            };

            // Then by number of recent reports
            tier_order_b
                .cmp(&tier_order_a)
                .then_with(|| b.recent_reports.cmp(&a.recent_reports))
        });

        games.truncate(limit as usize);
        Ok(games)
    }

    /// Get compatibility tips for a specific game
    pub fn get_compatibility_tips(&self, appid: u32, tier: &ProtonDBTier) -> Vec<String> {
        let mut tips = Vec::new();

        match tier {
            ProtonDBTier::Platinum => {
                tips.push("🟢 Game works perfectly! No tweaks needed.".to_string());
                tips.push("Recommended: Latest Proton or Proton GE".to_string());
            }
            ProtonDBTier::Gold => {
                tips.push("🟡 Game works great with minor tweaks".to_string());
                tips.push("Try: Proton GE or latest Proton Experimental".to_string());
                tips.push("May need: DXVK, specific launch options".to_string());
            }
            ProtonDBTier::Silver => {
                tips.push("🟠 Playable with some issues".to_string());
                tips.push("Try: Proton GE with DXVK".to_string());
                tips.push("Common fixes: Different Proton version, launch options".to_string());
            }
            ProtonDBTier::Bronze => {
                tips.push("🔴 Major issues, playable with effort".to_string());
                tips.push("Try: Multiple Proton versions, community fixes".to_string());
                tips.push("May need: Specific Wine version, extensive tweaking".to_string());
            }
            ProtonDBTier::Borked => {
                tips.push("💀 Currently not working".to_string());
                tips.push("Check: Recent community reports for potential fixes".to_string());
                tips.push("Consider: Alternative solutions or waiting for updates".to_string());
            }
            ProtonDBTier::Pending => {
                tips.push("❓ Not enough data available".to_string());
                tips.push("Try: Latest Proton GE as starting point".to_string());
                tips.push("Help: Submit a report after testing!".to_string());
            }
        }

        // Game-specific tips (hardcoded for now, could be expanded)
        match appid {
            // World of Warcraft (hypothetical Steam app)
            // Battle.net games don't have Steam AppIDs, but this shows the concept
            12345 if appid == 12345 => {
                tips.push("🐉 WoW-specific: Enable DXVK, use Proton GE".to_string());
                tips.push("Launch via Battle.net launcher, not directly".to_string());
            }
            _ => {}
        }

        tips
    }

    /// Format tier for display
    pub fn format_tier(tier: &ProtonDBTier) -> (&'static str, &'static str) {
        match tier {
            ProtonDBTier::Platinum => ("🟢 Platinum", "Works perfectly"),
            ProtonDBTier::Gold => ("🟡 Gold", "Works after tweaks"),
            ProtonDBTier::Silver => ("🟠 Silver", "Runs with minor issues"),
            ProtonDBTier::Bronze => ("🔴 Bronze", "Major issues"),
            ProtonDBTier::Borked => ("💀 Borked", "Doesn't work"),
            ProtonDBTier::Pending => ("❓ Pending", "Not enough reports"),
        }
    }

    /// Get recommended Proton version for a game
    pub async fn get_recommended_proton(&self, steam_appid: u32) -> Result<Option<String>> {
        let summary = self.get_game_summary(steam_appid).await?;

        if let Some(summary) = summary {
            let recommendation = match summary.tier {
                ProtonDBTier::Platinum | ProtonDBTier::Gold => {
                    Some("Proton GE (Latest)".to_string())
                }
                ProtonDBTier::Silver => Some("Try Proton GE or Proton Experimental".to_string()),
                ProtonDBTier::Bronze => {
                    Some("Multiple versions - check community reports".to_string())
                }
                ProtonDBTier::Borked => {
                    Some("Not currently working - try latest Proton GE".to_string())
                }
                ProtonDBTier::Pending => Some("Unknown - start with Proton GE".to_string()),
            };

            return Ok(recommendation);
        }

        Ok(None)
    }

    /// Generate a compatibility report for GhostForge UI
    pub async fn generate_compatibility_report(
        &self,
        steam_appid: u32,
        game_name: &str,
    ) -> Result<GameCompatibilityReport> {
        let summary = self.get_game_summary(steam_appid).await?;

        let report = if let Some(summary) = summary {
            let (tier_display, tier_description) = Self::format_tier(&summary.tier);
            let tips = self.get_compatibility_tips(steam_appid, &summary.tier);
            let recommended_proton = self
                .get_recommended_proton(steam_appid)
                .await?
                .unwrap_or("Proton GE".to_string());

            GameCompatibilityReport {
                appid: steam_appid,
                game_name: game_name.to_string(),
                protondb_available: true,
                tier: summary.tier,
                tier_display: tier_display.to_string(),
                tier_description: tier_description.to_string(),
                confidence: summary.confidence,
                total_reports: summary.total,
                score: summary.score,
                recommended_proton,
                compatibility_tips: tips,
                last_updated: chrono::Utc::now(),
            }
        } else {
            // No ProtonDB data available
            GameCompatibilityReport {
                appid: steam_appid,
                game_name: game_name.to_string(),
                protondb_available: false,
                tier: ProtonDBTier::Pending,
                tier_display: "❓ Unknown".to_string(),
                tier_description: "No ProtonDB data".to_string(),
                confidence: "none".to_string(),
                total_reports: 0,
                score: 0.0,
                recommended_proton: "Proton GE (Latest)".to_string(),
                compatibility_tips: vec![
                    "No ProtonDB data available for this game".to_string(),
                    "Try starting with Proton GE".to_string(),
                    "Submit a report to help the community!".to_string(),
                ],
                last_updated: chrono::Utc::now(),
            }
        };

        Ok(report)
    }

    /// Get Steam app ID from game name
    pub async fn get_steam_appid(&self, game_name: &str) -> Result<Option<u32>> {
        let steam_url = "https://api.steampowered.com/ISteamApps/GetAppList/v2/";

        let response = self.client.get(steam_url).send().await?;

        if response.status() != 200 {
            return Ok(None);
        }

        let steam_response: SteamAppListResponse = response.json().await?;

        // Find exact match first, then partial match
        for app in &steam_response.applist.apps {
            if app.name.to_lowercase() == game_name.to_lowercase() {
                return Ok(Some(app.appid));
            }
        }

        // Try partial match
        for app in &steam_response.applist.apps {
            if app.name.to_lowercase().contains(&game_name.to_lowercase()) {
                return Ok(Some(app.appid));
            }
        }

        Ok(None)
    }

    /// Get comprehensive compatibility report for a game
    pub async fn get_compatibility_info(
        &self,
        steam_appid: u32,
    ) -> Result<GameCompatibilityReport> {
        let summary = self
            .get_game_summary(steam_appid)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Game not found in ProtonDB"))?;

        let reports = self.get_game_reports(steam_appid, Some(10)).await?;

        let mut wine_versions = HashMap::new();
        let mut common_issues = Vec::new();
        let mut working_configs = Vec::new();

        for report in &reports {
            // Count Wine/Proton versions
            *wine_versions
                .entry(report.proton_version.clone())
                .or_insert(0) += 1;

            // Collect issues and working configs
            if let Some(notes) = &report.notes {
                if report.tier == ProtonDBTier::Platinum || report.tier == ProtonDBTier::Gold {
                    working_configs.push(notes.clone());
                } else if report.tier == ProtonDBTier::Bronze || report.tier == ProtonDBTier::Borked
                {
                    common_issues.push(notes.clone());
                }
            }
        }

        // Find most recommended Proton version
        let recommended_proton = wine_versions
            .iter()
            .max_by_key(|(_, count)| *count)
            .map(|(version, _)| version.clone())
            .unwrap_or_else(|| "GE-Proton Latest".to_string());

        Ok(GameCompatibilityReport {
            appid: steam_appid,
            game_name: format!("Game {}", steam_appid), // Will be filled from Steam API later
            protondb_available: true,
            tier: summary.tier.clone(),
            tier_display: format!("{:?}", summary.tier),
            tier_description: self.get_tier_description(&summary.tier),
            confidence: summary.confidence,
            total_reports: summary.total,
            score: summary.score,
            recommended_proton,
            compatibility_tips: self.suggest_winetricks(&summary.tier),
            last_updated: chrono::Utc::now(),
        })
    }

    fn get_tier_description(&self, tier: &ProtonDBTier) -> String {
        match tier {
            ProtonDBTier::Platinum => "Works perfectly out of the box".to_string(),
            ProtonDBTier::Gold => "Works perfectly after tweaks".to_string(),
            ProtonDBTier::Silver => "Works with minor issues".to_string(),
            ProtonDBTier::Bronze => "Runs, but crashes often or has major issues".to_string(),
            ProtonDBTier::Borked => "Doesn't work".to_string(),
            ProtonDBTier::Pending => "Not enough reports".to_string(),
        }
    }

    fn suggest_winetricks(&self, tier: &ProtonDBTier) -> Vec<String> {
        match tier {
            ProtonDBTier::Platinum => vec![],
            ProtonDBTier::Gold | ProtonDBTier::Silver => {
                vec!["vcrun2019".to_string(), "corefonts".to_string()]
            }
            ProtonDBTier::Bronze => {
                vec![
                    "vcrun2019".to_string(),
                    "corefonts".to_string(),
                    "dotnet48".to_string(),
                    "mfc140".to_string(),
                ]
            }
            _ => vec![
                "vcrun2019".to_string(),
                "corefonts".to_string(),
                "dotnet48".to_string(),
                "d3dcompiler_47".to_string(),
            ],
        }
    }

    fn suggest_launch_options(&self, tier: &ProtonDBTier) -> Vec<String> {
        match tier {
            ProtonDBTier::Platinum => vec![],
            ProtonDBTier::Gold => vec!["PROTON_USE_WINED3D=1".to_string()],
            ProtonDBTier::Silver | ProtonDBTier::Bronze => vec![
                "PROTON_USE_WINED3D=1".to_string(),
                "PROTON_NO_ESYNC=1".to_string(),
            ],
            _ => vec![
                "PROTON_USE_WINED3D=1".to_string(),
                "PROTON_NO_ESYNC=1".to_string(),
                "PROTON_NO_FSYNC=1".to_string(),
            ],
        }
    }

    /// Cache ProtonDB data locally
    pub async fn cache_game_data(
        &self,
        steam_appid: u32,
        cache_dir: &std::path::Path,
    ) -> Result<()> {
        let summary = self.get_game_summary(steam_appid).await?;
        if let Some(game_data) = summary {
            let cache_file = cache_dir.join(format!("{}.json", steam_appid));
            std::fs::create_dir_all(cache_dir)?;
            let json = serde_json::to_string_pretty(&game_data)?;
            std::fs::write(cache_file, json)?;
        }
        Ok(())
    }

    /// Load cached ProtonDB data
    pub fn load_cached_data(
        &self,
        steam_appid: u32,
        cache_dir: &std::path::Path,
    ) -> Result<Option<ProtonDBGame>> {
        let cache_file = cache_dir.join(format!("{}.json", steam_appid));
        if cache_file.exists() {
            let json = std::fs::read_to_string(cache_file)?;
            let game_data: ProtonDBGame = serde_json::from_str(&json)?;
            Ok(Some(game_data))
        } else {
            Ok(None)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameCompatibilityReport {
    pub appid: u32,
    pub game_name: String,
    pub protondb_available: bool,
    pub tier: ProtonDBTier,
    pub tier_display: String,
    pub tier_description: String,
    pub confidence: String,
    pub total_reports: u32,
    pub score: f32,
    pub recommended_proton: String,
    pub compatibility_tips: Vec<String>,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

// Helper functions for integration with GhostForge
impl GameCompatibilityReport {
    pub fn should_recommend_dxvk(&self) -> bool {
        match self.tier {
            ProtonDBTier::Platinum | ProtonDBTier::Gold | ProtonDBTier::Silver => true,
            _ => false,
        }
    }

    pub fn get_likelihood_of_success(&self) -> f32 {
        match self.tier {
            ProtonDBTier::Platinum => 0.95,
            ProtonDBTier::Gold => 0.85,
            ProtonDBTier::Silver => 0.70,
            ProtonDBTier::Bronze => 0.40,
            ProtonDBTier::Borked => 0.10,
            ProtonDBTier::Pending => 0.50, // Unknown
        }
    }

    pub fn get_setup_complexity(&self) -> &'static str {
        match self.tier {
            ProtonDBTier::Platinum => "None - works out of the box",
            ProtonDBTier::Gold => "Easy - minor tweaks needed",
            ProtonDBTier::Silver => "Moderate - some configuration required",
            ProtonDBTier::Bronze => "Hard - extensive tweaking needed",
            ProtonDBTier::Borked => "Not working - advanced users only",
            ProtonDBTier::Pending => "Unknown - experimental",
        }
    }
}
