#[cfg(feature = "container-bolt")]
use crate::bolt_integration::{BoltGameManager, BoltSystemMetrics, ContainerStatus, GameContainer};
#[cfg(feature = "gui")]
use anyhow::Result;
#[cfg(feature = "gui")]
use eframe::egui;
#[cfg(feature = "gui")]
use poll_promise::Promise;
#[cfg(feature = "gui")]
// Remove unused import
#[cfg(feature = "gui")]
use std::sync::Arc;
#[cfg(feature = "gui")]
use std::time::{Duration, Instant};

// Mock types when Bolt is not available
#[cfg(all(feature = "gui", not(feature = "container-bolt")))]
mod mock_bolt {
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Serialize};
    use std::sync::Arc;

    #[derive(Debug, Clone)]
    pub struct BoltGameManager;

    impl BoltGameManager {
        pub fn default() -> Self {
            Self
        }
        pub fn get_containers(&self) -> Vec<GameContainer> {
            vec![]
        }
        pub fn get_cached_metrics(&self) -> Option<BoltSystemMetrics> {
            None
        }
        pub async fn launch_game(
            &self,
            _id: &str,
            _game: &crate::game::Game,
        ) -> anyhow::Result<String> {
            Err(anyhow::anyhow!("Bolt not enabled"))
        }
        pub async fn stop_game(&self, _id: &str) -> anyhow::Result<()> {
            Err(anyhow::anyhow!("Bolt not enabled"))
        }
    }

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
}

#[cfg(all(feature = "gui", not(feature = "container-bolt")))]
use mock_bolt::{BoltGameManager, BoltSystemMetrics, ContainerStatus, GameContainer};

#[cfg(feature = "gui")]
use crate::display::{DisplayManager, GamingDisplaySettings, VsyncMode};
#[cfg(feature = "gui")]
use crate::vrr_monitor::VrrMonitor;

#[cfg(feature = "gui")]
pub struct GhostForgeApp {
    current_tab: Tab,
    system_info: Option<crate::utils::SystemInfo>,
    games: Vec<crate::game::Game>,
    wine_versions: Vec<crate::wine::WineVersion>,
    launchers: Vec<crate::launcher::Launcher>,
    #[allow(dead_code)]
    protondb_client: crate::protondb::ProtonDBClient,
    show_about: bool,
    search_query: String,
    protondb_games: Vec<crate::protondb::ProtonDBGame>,
    // Async state management
    loading_system_info: bool,
    loading_games: bool,
    loading_wine: bool,
    error_message: Option<String>,
    // Bolt integration
    bolt_manager: Arc<BoltGameManager>,
    game_containers: Vec<GameContainer>,
    bolt_metrics: Option<BoltSystemMetrics>,
    // Async operations
    container_refresh_promise: Option<Promise<Result<Vec<GameContainer>, String>>>,
    metrics_promise: Option<Promise<Result<BoltSystemMetrics, String>>>,
    last_refresh: Instant,
    // UI state
    selected_game: Option<String>,
    view_mode: ViewMode,
    show_container_details: bool,
    container_logs: String,
    // Display management
    display_manager: DisplayManager,
    vrr_monitor: VrrMonitor,
    gaming_display_settings: GamingDisplaySettings,
    show_display_settings: bool,
}

#[cfg(feature = "gui")]
#[derive(Debug, Clone, Copy, PartialEq)]
enum Tab {
    Dashboard,
    Games,
    ProtonDB,
    Wine,
    Graphics,
    Containers, // New Bolt container management tab
    Display,    // Display and VRR management
    Settings,
}

#[cfg(feature = "gui")]
#[derive(Debug, Clone, Copy, PartialEq)]
enum ViewMode {
    Grid,
    List,
    Details,
}

#[cfg(feature = "gui")]
impl Default for GhostForgeApp {
    fn default() -> Self {
        let mut app = Self {
            current_tab: Tab::Dashboard,
            system_info: None,
            games: Vec::new(),
            wine_versions: Vec::new(),
            launchers: Vec::new(),
            protondb_client: crate::protondb::ProtonDBClient::new(),
            show_about: false,
            search_query: String::new(),
            protondb_games: Vec::new(),
            loading_system_info: false,
            loading_games: false,
            loading_wine: false,
            error_message: None,
            // Bolt integration
            bolt_manager: Arc::new(BoltGameManager::default()),
            game_containers: Vec::new(),
            bolt_metrics: None,
            // Async operations
            container_refresh_promise: None,
            metrics_promise: None,
            last_refresh: Instant::now(),
            // UI state
            selected_game: None,
            view_mode: ViewMode::Grid,
            show_container_details: false,
            container_logs: String::new(),
            // Display management
            display_manager: DisplayManager::default(),
            vrr_monitor: VrrMonitor::default(),
            gaming_display_settings: GamingDisplaySettings {
                target_fps: 120,
                vsync_mode: VsyncMode::Adaptive,
                frame_pacing: true,
                low_latency_mode: true,
                hdr_gaming: false,
                fullscreen_optimizations: true,
            },
            show_display_settings: false,
        };

        // Load initial data
        app.load_system_info();
        app.refresh_games();
        app.load_wine_versions();

        app
    }
}

#[cfg(feature = "gui")]
impl eframe::App for GhostForgeApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Apply Material Ocean Blue theme
        ctx.set_visuals(self.get_ocean_blue_theme());
        // Top menu bar with modern styling
        egui::TopBottomPanel::top("top_panel")
            .min_height(40.0)
            .show(ctx, |ui| {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.add_space(16.0);

                    // App title with icon
                    ui.heading("🎮 GhostForge");
                    ui.add_space(20.0);

                    egui::menu::bar(ui, |ui| {
                        ui.menu_button("File", |ui| {
                            if ui.button("⚙️ Settings").clicked() {
                                self.current_tab = Tab::Settings;
                            }
                            ui.separator();
                            if ui.button("❌ Exit").clicked() {
                                std::process::exit(0);
                            }
                        });

                        ui.menu_button("Help", |ui| {
                            if ui.button("ℹ️ About").clicked() {
                                self.show_about = true;
                            }
                            if ui.button("📝 Documentation").clicked() {
                                // TODO: Open documentation
                            }
                        });

                        // Add some spacing before status indicators
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add_space(16.0);
                            // System status indicators
                            if let Some(_info) = &self.system_info {
                                ui.colored_label(egui::Color32::from_rgb(76, 175, 80), "🟢 Ready");
                            } else {
                                ui.colored_label(
                                    egui::Color32::from_rgb(255, 193, 7),
                                    "🟡 Loading...",
                                );
                            }
                        });
                    });
                });
                ui.add_space(4.0);
            });

        // Side panel for navigation with enhanced styling
        egui::SidePanel::left("side_panel")
            .min_width(220.0)
            .max_width(280.0)
            .resizable(false)
            .show(ctx, |ui| {
                ui.add_space(8.0);

                // App logo/title section
                ui.vertical_centered(|ui| {
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new("🎮 GhostForge")
                            .size(18.0)
                            .color(egui::Color32::from_rgb(100, 181, 246)),
                    );
                    ui.small("Gaming Manager v0.1.0");
                    ui.add_space(8.0);
                });

                ui.separator();
                ui.add_space(8.0);

                // Main navigation with enhanced styling
                ui.label(
                    egui::RichText::new("Navigation")
                        .size(14.0)
                        .color(egui::Color32::from_rgb(176, 190, 210)),
                );
                ui.add_space(4.0);

                // Custom navigation buttons with better styling
                let nav_items = [
                    (Tab::Dashboard, "📊", "Dashboard"),
                    (Tab::Games, "🎯", "Games"),
                    (Tab::Containers, "📦", "Containers"),
                    (Tab::Display, "🖼️", "Display/VRR"),
                    (Tab::ProtonDB, "🌐", "ProtonDB"),
                    (Tab::Wine, "🍷", "Wine/Proton"),
                    (Tab::Graphics, "🖥️", "Graphics"),
                    (Tab::Settings, "⚙️", "Settings"),
                ];

                for (tab, icon, name) in nav_items {
                    let is_selected = self.current_tab == tab;
                    let button_color = if is_selected {
                        egui::Color32::from_rgb(100, 181, 246)
                    } else {
                        egui::Color32::from_rgb(176, 190, 210)
                    };

                    ui.horizontal(|ui| {
                        ui.add_space(8.0);
                        if ui
                            .add_sized(
                                [180.0, 32.0],
                                egui::Button::new(
                                    egui::RichText::new(format!("{} {}", icon, name))
                                        .color(button_color)
                                        .size(13.0),
                                )
                                .fill(if is_selected {
                                    egui::Color32::from_rgb(52, 69, 94)
                                } else {
                                    egui::Color32::TRANSPARENT
                                })
                                .rounding(egui::Rounding::same(6.0)),
                            )
                            .clicked()
                        {
                            self.current_tab = tab;
                        }
                    });
                    ui.add_space(2.0);
                }

                ui.add_space(16.0);
                ui.separator();
                ui.add_space(8.0);

                // Game categories (like Lutris sidebar)
                ui.label(
                    egui::RichText::new("Categories")
                        .size(14.0)
                        .color(egui::Color32::from_rgb(176, 190, 210)),
                );
                ui.add_space(4.0);

                ui.indent("categories", |ui| {
                    let categories = [
                        ("📦", "All Games"),
                        ("✅", "Installed"),
                        ("▶️", "Running"),
                        ("🕓", "Recently Played"),
                        ("♥️", "Favorites"),
                    ];

                    for (icon, name) in categories {
                        ui.horizontal(|ui| {
                            ui.add_space(4.0);
                            let _ = ui.selectable_label(
                                false,
                                egui::RichText::new(format!("{} {}", icon, name))
                                    .size(12.0)
                                    .color(egui::Color32::from_rgb(176, 190, 210)),
                            );
                        });
                        ui.add_space(1.0);
                    }
                });

                ui.separator();

                // Launchers (like Lutris services)
                ui.label("Launchers:");
                ui.indent("launchers", |ui| {
                    for launcher in &self.launchers {
                        let icon = match launcher.launcher_type {
                            crate::launcher::LauncherType::Steam => "🔵",
                            crate::launcher::LauncherType::BattleNet => "🔷",
                            crate::launcher::LauncherType::Epic => "⚫",
                            crate::launcher::LauncherType::GOG => "🟣",
                            _ => "📦",
                        };
                        ui.selectable_label(false, format!("{} {}", icon, launcher.name));
                    }
                    if self.launchers.is_empty() {
                        ui.small("No launchers detected");
                    }
                });

                ui.separator();

                // System status
                ui.label("System Status:");
                if let Some(info) = &self.system_info {
                    ui.small(format!(
                        "🟢 Wine: {}",
                        info.wine_support
                            .version
                            .as_ref()
                            .unwrap_or(&"Available".to_string())
                    ));
                    ui.small(format!(
                        "🟢 Vulkan: {}",
                        if info.vulkan.available {
                            "Ready"
                        } else {
                            "Not available"
                        }
                    ));
                    ui.small(format!(
                        "🟢 DXVK: {}",
                        if info.gaming_tools.dxvk {
                            "Ready"
                        } else {
                            "Not installed"
                        }
                    ));
                } else {
                    ui.small("🔄 Loading system info...");
                }
            });

        // Auto-refresh containers and metrics every 5 seconds
        if self.last_refresh.elapsed() > Duration::from_secs(5) {
            self.refresh_containers_async(ctx);
            self.refresh_metrics_async(ctx);
            self.last_refresh = Instant::now();
        }

        // Main content area
        egui::CentralPanel::default().show(ctx, |ui| {
            // Show error notifications at the top
            let mut clear_error = false;
            if let Some(error) = &self.error_message {
                ui.horizontal(|ui| {
                    ui.colored_label(egui::Color32::RED, "⚠️ Error:");
                    ui.label(error);
                    if ui.small_button("❌").clicked() {
                        clear_error = true;
                    }
                });
                ui.separator();
            }
            if clear_error {
                self.error_message = None;
            }

            match self.current_tab {
                Tab::Dashboard => self.show_dashboard(ui),
                Tab::Games => self.show_games(ui),
                Tab::Containers => self.show_containers(ui),
                Tab::Display => self.show_display(ui),
                Tab::ProtonDB => self.show_protondb(ui),
                Tab::Wine => self.show_wine(ui),
                Tab::Graphics => self.show_graphics(ui),
                Tab::Settings => self.show_settings(ui),
            }
        });

        // About dialog
        if self.show_about {
            egui::Window::new("About GhostForge")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label("🎮 GhostForge v0.1.0");
                    ui.separator();
                    ui.label("A modern Linux gaming platform replacing Lutris");
                    ui.label("with better Wine/Proton management.");
                    ui.separator();
                    ui.label("Built with Rust and egui");
                    ui.horizontal(|ui| {
                        if ui.button("Close").clicked() {
                            self.show_about = false;
                        }
                    });
                });
        }
    }
}

#[cfg(feature = "gui")]
impl GhostForgeApp {
    fn get_ocean_blue_theme(&self) -> egui::Visuals {
        let mut visuals = egui::Visuals::dark();

        // Material Ocean Blue color palette
        let primary_bg = egui::Color32::from_rgb(18, 25, 38); // Dark ocean blue
        let secondary_bg = egui::Color32::from_rgb(26, 35, 52); // Lighter ocean blue
        let accent_bg = egui::Color32::from_rgb(34, 45, 65); // Medium ocean blue
        let surface_bg = egui::Color32::from_rgb(42, 55, 78); // Surface blue
        let primary_text = egui::Color32::from_rgb(240, 246, 252); // Light text
        let secondary_text = egui::Color32::from_rgb(176, 190, 210); // Secondary text
        let accent_color = egui::Color32::from_rgb(100, 181, 246); // Light blue accent
        let _success_color = egui::Color32::from_rgb(76, 175, 80); // Green
        let _warning_color = egui::Color32::from_rgb(255, 193, 7); // Amber
        let _error_color = egui::Color32::from_rgb(244, 67, 54); // Red

        // Apply colors
        visuals.window_fill = primary_bg;
        visuals.panel_fill = secondary_bg;
        visuals.faint_bg_color = accent_bg;
        visuals.extreme_bg_color = primary_bg;

        visuals.widgets.noninteractive.bg_fill = surface_bg;
        visuals.widgets.noninteractive.fg_stroke.color = primary_text;

        visuals.widgets.inactive.bg_fill = accent_bg;
        visuals.widgets.inactive.fg_stroke.color = secondary_text;
        visuals.widgets.inactive.bg_stroke.color = egui::Color32::from_rgb(60, 75, 95);

        visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(52, 69, 94);
        visuals.widgets.hovered.fg_stroke.color = primary_text;
        visuals.widgets.hovered.bg_stroke.color = accent_color;

        visuals.widgets.active.bg_fill = accent_color;
        visuals.widgets.active.fg_stroke.color = primary_bg;
        visuals.widgets.active.bg_stroke.color = accent_color;

        visuals.widgets.open.bg_fill = surface_bg;
        visuals.widgets.open.fg_stroke.color = primary_text;
        visuals.widgets.open.bg_stroke.color = accent_color;

        // Selection colors
        visuals.selection.bg_fill = accent_color.gamma_multiply(0.3);
        visuals.selection.stroke.color = accent_color;

        // Hyperlink colors
        visuals.hyperlink_color = accent_color;

        // Window styling
        visuals.window_rounding = egui::Rounding::same(8.0);
        visuals.menu_rounding = egui::Rounding::same(6.0);
        visuals.window_shadow = egui::epaint::Shadow {
            offset: egui::Vec2::splat(8.0),
            blur: 16.0,
            spread: 0.0,
            color: egui::Color32::from_black_alpha(80),
        };

        // Separator color
        visuals.widgets.noninteractive.bg_stroke.color = egui::Color32::from_rgb(60, 75, 95);

        visuals
    }
    fn load_system_info(&mut self) {
        if self.loading_system_info {
            return;
        }

        self.loading_system_info = true;
        self.error_message = None;

        // Simulate async loading (disabled for now)
        // let runtime = Arc::clone(&self.runtime);
        // runtime.spawn(async move {
        //     // In a real app, this would load asynchronously
        //     tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        // });

        // For now, load synchronously
        match crate::utils::SystemDetector::get_system_info() {
            Ok(info) => {
                self.system_info = Some(info);
                self.loading_system_info = false;
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to load system info: {}", e));
                self.loading_system_info = false;
            }
        }
    }

    fn refresh_games(&mut self) {
        if self.loading_games {
            return;
        }

        self.loading_games = true;
        self.error_message = None;

        // Load launchers first
        let config_dir = dirs::config_dir().unwrap_or_default().join("ghostforge");
        let launcher_manager = crate::launcher::LauncherManager::new(config_dir);

        match launcher_manager.detect_launchers() {
            Ok(launchers) => {
                self.launchers = launchers;

                // TODO: Load games from each launcher
                // For now, create some mock games
                if !self.launchers.is_empty() {
                    self.games = vec![
                        crate::game::Game {
                            id: "steam_730".to_string(),
                            name: "Counter-Strike 2".to_string(),
                            executable: std::path::PathBuf::from("csgo.exe"),
                            install_path: std::path::PathBuf::from(
                                "/home/user/.steam/steamapps/common/Counter-Strike Global Offensive",
                            ),
                            launcher: Some("Steam".to_string()),
                            launcher_id: Some("730".to_string()),
                            wine_version: None,
                            wine_prefix: None,
                            icon: None,
                            banner: None,
                            launch_arguments: vec![],
                            environment_variables: vec![],
                            pre_launch_script: None,
                            post_launch_script: None,
                            categories: vec!["FPS".to_string()],
                            tags: vec!["multiplayer".to_string()],
                            playtime_minutes: 9000, // 150 hours
                            last_played: Some(chrono::Utc::now()),
                            installed_date: chrono::Utc::now(),
                            favorite: false,
                            hidden: false,
                            notes: None,
                        },
                        crate::game::Game {
                            id: "battlenet_wow".to_string(),
                            name: "World of Warcraft".to_string(),
                            executable: std::path::PathBuf::from("Wow.exe"),
                            install_path: std::path::PathBuf::from(
                                "/home/user/Games/battlenet/drive_c/Program Files (x86)/World of Warcraft",
                            ),
                            launcher: Some("Battle.net".to_string()),
                            launcher_id: Some("wow".to_string()),
                            wine_version: Some("GE-Proton9-2".to_string()),
                            wine_prefix: Some(std::path::PathBuf::from(
                                "/home/user/Games/battlenet",
                            )),
                            icon: None,
                            banner: None,
                            launch_arguments: vec![],
                            environment_variables: vec![],
                            pre_launch_script: None,
                            post_launch_script: None,
                            categories: vec!["MMORPG".to_string()],
                            tags: vec!["rpg".to_string(), "online".to_string()],
                            playtime_minutes: 150000, // 2500 hours
                            last_played: Some(chrono::Utc::now() - chrono::Duration::days(2)),
                            installed_date: chrono::Utc::now() - chrono::Duration::days(365),
                            favorite: true,
                            hidden: false,
                            notes: Some("Favorite MMO".to_string()),
                        },
                    ];
                }
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to detect launchers: {}", e));
            }
        }

        self.loading_games = false;
    }

    fn load_wine_versions(&mut self) {
        if self.loading_wine {
            return;
        }

        self.loading_wine = true;

        let wine_dir = dirs::data_dir()
            .unwrap_or_default()
            .join("ghostforge")
            .join("wine-versions");
        let config_dir = dirs::config_dir().unwrap_or_default().join("ghostforge");
        let manager = crate::wine::WineManager::new(wine_dir, config_dir);

        // let _runtime = Arc::clone(&self.runtime);
        let _manager = manager;

        // This should be done with proper async state management in a real app
        // For now, we'll create mock data

        // For now, create mock data
        self.wine_versions = vec![
            crate::wine::WineVersion {
                name: "System Wine".to_string(),
                version: "9.0".to_string(),
                path: std::path::PathBuf::from("/usr/bin/wine"),
                wine_type: crate::wine::WineType::Wine,
                arch: vec!["win64".to_string()],
                installed: true,
                system: true,
                download_url: None,
                checksum: None,
            },
            crate::wine::WineVersion {
                name: "Proton GE 9-2".to_string(),
                version: "9.2".to_string(),
                path: std::path::PathBuf::from(
                    "/home/user/.local/share/ghostforge/wine-versions/GE-Proton9-2",
                ),
                wine_type: crate::wine::WineType::ProtonGE,
                arch: vec!["win64".to_string()],
                installed: true,
                system: false,
                download_url: Some(
                    "https://github.com/GloriousEggroll/proton-ge-custom/releases/".to_string(),
                ),
                checksum: None,
            },
        ];

        self.loading_wine = false;
    }
    fn show_dashboard(&mut self, ui: &mut egui::Ui) {
        ui.heading("📊 Dashboard");
        ui.separator();

        // Quick stats row (like Lutris overview)
        ui.horizontal(|ui| {
            // Games stats
            ui.group(|ui| {
                ui.vertical_centered(|ui| {
                    ui.strong(format!("{}", self.games.len()));
                    ui.label("Games Installed");
                });
            });

            ui.group(|ui| {
                ui.vertical_centered(|ui| {
                    ui.strong(format!("{}", self.wine_versions.len()));
                    ui.label("Wine Versions");
                });
            });

            ui.group(|ui| {
                ui.vertical_centered(|ui| {
                    ui.strong(format!("{}", self.launchers.len()));
                    ui.label("Launchers");
                });
            });

            // Running games indicator (from Bolt containers)
            ui.group(|ui| {
                ui.vertical_centered(|ui| {
                    let running_count = if let Some(metrics) = &self.bolt_metrics {
                        metrics.running_containers
                    } else {
                        self.game_containers
                            .iter()
                            .filter(|c| matches!(c.status, ContainerStatus::Running))
                            .count()
                    };
                    ui.strong(format!("{}", running_count));
                    ui.label("Running");
                });
            });
        });

        ui.separator();

        // Two-column layout for system info and recent activity
        ui.horizontal_top(|ui| {
            // Left column - System Information
            ui.vertical(|ui| {
                ui.heading("System Information");
                ui.separator();

                if self.loading_system_info {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Loading system information...");
                    });
                } else if let Some(info) = &self.system_info {
                    ui.group(|ui| {
                        ui.vertical(|ui| {
                            ui.label(format!("💻 OS: {}", info.os));
                            ui.label(format!("⚙️ Kernel: {}", info.kernel));
                            ui.label(format!("🖼️ CPU: {}", info.cpu.brand));
                            ui.label(format!(
                                "💾 RAM: {:.1} GB",
                                info.memory.total as f64 / (1024.0 * 1024.0 * 1024.0)
                            ));

                            if !info.gpu.is_empty() {
                                ui.label(format!("🖥️ GPU: {}", info.gpu[0].name));
                                ui.label(format!(
                                    "    Driver: {}",
                                    info.gpu[0]
                                        .driver
                                        .as_ref()
                                        .unwrap_or(&"Unknown".to_string())
                                ));
                            }

                            ui.separator();
                            ui.label(format!(
                                "🍷 Wine: {}",
                                if info.wine_support.installed {
                                    "Installed"
                                } else {
                                    "Not installed"
                                }
                            ));
                            ui.label(format!(
                                "🌋 Vulkan: {}",
                                if info.vulkan.available {
                                    "Available"
                                } else {
                                    "Not available"
                                }
                            ));
                            ui.label(format!(
                                "🖥️ DXVK: {}",
                                if info.gaming_tools.dxvk {
                                    "Installed"
                                } else {
                                    "Not installed"
                                }
                            ));
                            ui.label(format!(
                                "🎮 GameMode: {}",
                                if info.gaming_tools.gamemode {
                                    "Available"
                                } else {
                                    "Not available"
                                }
                            ));
                        });
                    });
                } else {
                    if ui.button("Load System Info").clicked() {
                        self.load_system_info();
                    }
                }
            });

            ui.separator();

            // Right column - Recent Activity and Quick Actions
            ui.vertical(|ui| {
                ui.heading("Quick Actions");
                ui.separator();

                ui.group(|ui| {
                    ui.vertical(|ui| {
                        if ui.button("📥 Install Wine/Proton").clicked() {
                            self.current_tab = Tab::Wine;
                        }
                        if ui.button("🔧 Configure Graphics").clicked() {
                            self.current_tab = Tab::Graphics;
                        }
                        if ui.button("🔄 Sync Game Libraries").clicked() {
                            self.refresh_games();
                        }
                        if ui.button("⚙️ Open Settings").clicked() {
                            self.current_tab = Tab::Settings;
                        }
                    });
                });

                ui.add_space(10.0);

                ui.heading("Recent Activity");
                ui.separator();

                ui.group(|ui| {
                    ui.vertical(|ui| {
                        // Show running containers first
                        let running_containers: Vec<_> = self
                            .game_containers
                            .iter()
                            .filter(|c| matches!(c.status, ContainerStatus::Running))
                            .collect();

                        if !running_containers.is_empty() {
                            ui.label("🟢 Currently Running:");
                            for container in running_containers.iter().take(3) {
                                ui.horizontal(|ui| {
                                    ui.label("📦");
                                    ui.label(&container.name);
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            ui.small("Running");
                                            if ui.small_button("⏹️").clicked() {
                                                let bolt_manager: Arc<BoltGameManager> =
                                                    Arc::clone(&self.bolt_manager);
                                                let container_id = container.id.clone();
                                                let ctx = ui.ctx().clone();

                                                tokio::spawn(async move {
                                                    if let Err(e) =
                                                        bolt_manager.stop_game(&container_id).await
                                                    {
                                                        eprintln!(
                                                            "Failed to stop container: {}",
                                                            e
                                                        );
                                                    }
                                                    ctx.request_repaint();
                                                });
                                            }
                                        },
                                    );
                                });
                            }
                            ui.separator();
                        }

                        // Show recent games
                        if self.games.is_empty() {
                            ui.label("No recent activity");
                            ui.small("Games you play will appear here");
                        } else {
                            ui.label("📜 Recent Games:");
                            for game in self.games.iter().take(3) {
                                ui.horizontal(|ui| {
                                    ui.label("🎮");
                                    ui.label(&game.name);
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if let Some(last_played) = &game.last_played {
                                                let duration = chrono::Utc::now()
                                                    .signed_duration_since(*last_played);
                                                let time_str = if duration.num_days() > 0 {
                                                    format!("{} days ago", duration.num_days())
                                                } else if duration.num_hours() > 0 {
                                                    format!("{} hours ago", duration.num_hours())
                                                } else {
                                                    "Recently".to_string()
                                                };
                                                ui.small(time_str);
                                            } else {
                                                ui.small("Never played");
                                            }
                                        },
                                    );
                                });
                            }
                        }
                    });
                });
            });
        });
    }

    fn show_games(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("🎯 Game Library");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // View options (like Lutris)
                let grid_selected = self.view_mode == ViewMode::Grid;
                let list_selected = self.view_mode == ViewMode::List;

                if ui
                    .selectable_label(grid_selected, "🗺️")
                    .on_hover_text("Grid View")
                    .clicked()
                {
                    self.view_mode = ViewMode::Grid;
                }
                if ui
                    .selectable_label(list_selected, "📊")
                    .on_hover_text("List View")
                    .clicked()
                {
                    self.view_mode = ViewMode::List;
                }
                ui.separator();
                if ui.button("➕ Add Game").clicked() {
                    // TODO: Open add game dialog
                }
                if ui.button("🔄 Refresh").clicked() {
                    self.refresh_games();
                    self.refresh_containers_async(ui.ctx());
                }
            });
        });

        ui.separator();

        // Search and filter bar
        ui.horizontal(|ui| {
            ui.label("Search:");
            ui.text_edit_singleline(&mut self.search_query);

            ui.separator();

            // Filter options
            ui.label("Filter:");
            egui::ComboBox::from_id_salt("game_filter")
                .selected_text("All Games")
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut "all", "all", "All Games");
                    ui.selectable_value(&mut "installed", "installed", "Installed Only");
                    ui.selectable_value(&mut "running", "running", "Currently Running");
                    ui.selectable_value(&mut "favorites", "favorites", "Favorites");
                });
        });

        ui.separator();

        // Game grid/list (inspired by Lutris GameStore)
        egui::ScrollArea::vertical().show(ui, |ui| {
            if self.loading_games {
                ui.centered_and_justified(|ui| {
                    ui.spinner();
                    ui.label("Loading games...");
                });
            } else if self.games.is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.label("🎮 No games found");
                        ui.label("Click 'Add Game' to get started or configure game launchers.");
                        if ui.button("Setup Steam Integration").clicked() {
                            self.current_tab = Tab::Settings;
                        }
                    });
                });
            } else {
                match self.view_mode {
                    ViewMode::Grid => {
                        // Game grid layout (like Lutris grid view)
                        let available_width = ui.available_width();
                        let card_width = 250.0;
                        let cards_per_row =
                            ((available_width / (card_width + 10.0)).floor() as usize).max(1);

                        // Clone games to avoid borrowing issues
                        let games = self.games.clone();
                        for games_chunk in games.chunks(cards_per_row) {
                            ui.horizontal(|ui| {
                                for game in games_chunk {
                                    self.show_game_card(ui, game, card_width);
                                }
                            });
                        }
                    }
                    ViewMode::List => {
                        // List view like Lutris list mode
                        let games = self.games.clone();
                        for game in &games {
                            self.show_game_list_item(ui, game);
                        }
                    }
                    ViewMode::Details => {
                        // Detailed view (not implemented yet)
                        ui.label("Details view not implemented yet");
                    }
                }
            }
        });
    }

    fn show_wine(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("🍷 Wine & Proton Management");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("📥 Install New Version").clicked() {
                    // TODO: Show wine installation dialog
                }
                if ui.button("🔄 Refresh").clicked() {
                    self.load_wine_versions();
                }
            });
        });

        ui.separator();

        if let Some(error) = &self.error_message {
            ui.colored_label(egui::Color32::RED, format!("Error: {}", error));
            ui.separator();
        }

        // Wine versions list
        ui.label(format!(
            "Installed Wine Versions ({}):",
            self.wine_versions.len()
        ));
        egui::ScrollArea::vertical().show(ui, |ui| {
            if self.loading_wine {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Loading Wine versions...");
                });
            } else if self.wine_versions.is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.label("🍷 No Wine versions found");
                        ui.label("Install Wine or Proton to get started.");
                        if ui.button("Install GE-Proton").clicked() {
                            // TODO: Install GE-Proton
                        }
                    });
                });
            } else {
                for version in &self.wine_versions {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            // Wine type icon
                            let icon = match version.wine_type {
                                crate::wine::WineType::Wine => "🍷",
                                crate::wine::WineType::WineStaging => "🍷",
                                crate::wine::WineType::Proton => "⚙️",
                                crate::wine::WineType::ProtonGE => "🔧",
                                crate::wine::WineType::ProtonTKG => "🔧",
                                crate::wine::WineType::Lutris => "🔵",
                                crate::wine::WineType::Custom => "📦",
                            };

                            ui.label(icon);
                            ui.vertical(|ui| {
                                ui.strong(&version.name);
                                ui.small(format!("Version: {}", version.version));
                                ui.small(format!("Architecture: {}", version.arch.join(", ")));
                                if let Some(url) = &version.download_url {
                                    ui.small(format!(
                                        "Source: {}",
                                        if url.contains("github") {
                                            "GitHub"
                                        } else {
                                            "Custom"
                                        }
                                    ));
                                }
                            });

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if !version.system {
                                        if ui.small_button("🗑").on_hover_text("Remove").clicked()
                                        {
                                            // TODO: Remove wine version
                                        }
                                    }

                                    if ui.small_button("📊").on_hover_text("Details").clicked() {
                                        // TODO: Show version details
                                    }

                                    // Status indicator
                                    if version.system {
                                        ui.small("(System)");
                                    } else if version.installed {
                                        ui.colored_label(egui::Color32::GREEN, "(Installed)");
                                    } else {
                                        ui.colored_label(egui::Color32::GRAY, "(Available)");
                                    }
                                },
                            );
                        });
                    });
                }
            }
        });

        ui.separator();

        // Quick install section
        ui.heading("Quick Install");
        ui.horizontal(|ui| {
            if ui.button("🔧 Install Latest GE-Proton").clicked() {
                // TODO: Install latest GE-Proton
            }
            if ui.button("🍷 Install Wine Staging").clicked() {
                // TODO: Install Wine Staging
            }
            if ui.button("🐍 Install Lutris Wine").clicked() {
                // TODO: Install Lutris Wine
            }
        });
    }

    fn show_graphics(&mut self, ui: &mut egui::Ui) {
        ui.heading("🖥️ Graphics Management");
        ui.separator();

        ui.horizontal(|ui| {
            if ui.button("📥 Install DXVK").clicked() {
                // TODO: Install DXVK
            }
            if ui.button("📥 Install VKD3D").clicked() {
                // TODO: Install VKD3D
            }
            if ui.button("🔄 Update All").clicked() {
                // TODO: Update all graphics layers
            }
        });

        ui.separator();

        ui.label("Installed Graphics Layers:");
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label("DXVK 2.4");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("🗑 Remove").clicked() {
                            // TODO: Remove DXVK
                        }
                        ui.label("✅ Active");
                    });
                });
            });

            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label("VKD3D 2.13");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("🗑 Remove").clicked() {
                            // TODO: Remove VKD3D
                        }
                        ui.label("⚪ Available");
                    });
                });
            });
        });
    }

    fn show_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading("⚙️ Settings");
        ui.separator();

        ui.group(|ui| {
            ui.label("General Settings");
            ui.checkbox(&mut true, "Start with system");
            ui.checkbox(&mut false, "Minimize to system tray");
            ui.checkbox(&mut true, "Check for updates automatically");
        });

        ui.separator();

        ui.group(|ui| {
            ui.label("Wine Settings");
            ui.horizontal(|ui| {
                ui.label("Default Wine Version:");
                egui::ComboBox::from_id_salt("wine_version")
                    .selected_text("System Wine")
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut "system", "system", "System Wine");
                        ui.selectable_value(&mut "proton", "proton", "Proton GE 9-2");
                    });
            });
        });

        ui.separator();

        ui.group(|ui| {
            ui.label("Graphics Settings");
            ui.checkbox(&mut true, "Enable DXVK by default");
            ui.checkbox(&mut false, "Enable VKD3D by default");
            ui.checkbox(&mut true, "Use GPU acceleration");
        });
    }

    fn show_protondb(&mut self, ui: &mut egui::Ui) {
        ui.heading("🌐 ProtonDB Game Database");
        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Search games:");
            ui.text_edit_singleline(&mut self.search_query);
            if ui.button("🔍 Search").clicked() {
                // TODO: Implement async ProtonDB search
            }
            if ui.button("🔥 Trending").clicked() {
                // TODO: Load trending games
            }
        });

        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            if self.protondb_games.is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.label("Search for games to see ProtonDB compatibility data");
                });
            } else {
                for game in &self.protondb_games {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            // Game info
                            ui.vertical(|ui| {
                                ui.strong(&game.name);
                                ui.label(format!("App ID: {}", game.appid));
                                ui.label(format!("Reports: {}", game.total_reports));
                            });

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    // Compatibility tier
                                    let (tier_color, tier_text) = match game.tier {
                                        crate::protondb::ProtonDBTier::Platinum => {
                                            (egui::Color32::from_rgb(220, 220, 220), "🟢 Platinum")
                                        }
                                        crate::protondb::ProtonDBTier::Gold => {
                                            (egui::Color32::from_rgb(255, 215, 0), "🟡 Gold")
                                        }
                                        crate::protondb::ProtonDBTier::Silver => {
                                            (egui::Color32::from_rgb(192, 192, 192), "🟠 Silver")
                                        }
                                        crate::protondb::ProtonDBTier::Bronze => {
                                            (egui::Color32::from_rgb(205, 127, 50), "🔴 Bronze")
                                        }
                                        crate::protondb::ProtonDBTier::Borked => {
                                            (egui::Color32::from_rgb(255, 0, 0), "💀 Borked")
                                        }
                                        crate::protondb::ProtonDBTier::Pending => {
                                            (egui::Color32::from_rgb(128, 128, 128), "❓ Pending")
                                        }
                                    };

                                    ui.colored_label(tier_color, tier_text);

                                    if ui.button("📋 Details").clicked() {
                                        // TODO: Show game details
                                    }
                                },
                            );
                        });
                    });
                }
            }
        });
    }

    // Async container management methods
    fn refresh_containers_async(&mut self, ctx: &egui::Context) {
        if self.container_refresh_promise.is_some() {
            return; // Already refreshing
        }

        let bolt_manager: Arc<BoltGameManager> = Arc::clone(&self.bolt_manager);
        let ctx = ctx.clone();

        self.container_refresh_promise =
            Some(Promise::spawn_thread("container_refresh", move || {
                // Use blocking call instead of async for poll-promise
                match bolt_manager.get_containers() {
                    containers => {
                        ctx.request_repaint();
                        Ok(containers)
                    }
                }
            }));
    }

    fn refresh_metrics_async(&mut self, ctx: &egui::Context) {
        if self.metrics_promise.is_some() {
            return; // Already refreshing
        }

        let bolt_manager: Arc<BoltGameManager> = Arc::clone(&self.bolt_manager);
        let ctx = ctx.clone();

        self.metrics_promise = Some(Promise::spawn_thread("metrics_refresh", move || {
            // Use cached metrics for GUI updates
            match bolt_manager.get_cached_metrics() {
                Some(metrics) => {
                    ctx.request_repaint();
                    Ok(metrics)
                }
                None => {
                    ctx.request_repaint();
                    Err("No metrics available".to_string())
                }
            }
        }));
    }

    fn show_game_card(&mut self, ui: &mut egui::Ui, game: &crate::game::Game, card_width: f32) {
        ui.group(|ui| {
            ui.set_min_size(egui::Vec2::new(card_width, 140.0));
            ui.vertical(|ui| {
                // Game icon and info
                ui.horizontal(|ui| {
                    // Larger icon for grid view
                    let icon_size = 80.0;
                    ui.add_sized([icon_size, icon_size], egui::Button::new("🎮"));

                    ui.vertical(|ui| {
                        ui.strong(&game.name);
                        ui.label(format!(
                            "Platform: {}",
                            game.launcher.as_ref().unwrap_or(&"Unknown".to_string())
                        ));
                        ui.small(format!("Playtime: {}h", game.playtime_minutes / 60));
                        if game.favorite {
                            ui.small("♥️ Favorite");
                        }

                        // Show container status if running
                        let is_running = self.game_containers.iter().any(|c| {
                            c.game_id == game.id && matches!(c.status, ContainerStatus::Running)
                        });
                        if is_running {
                            ui.colored_label(egui::Color32::GREEN, "🟢 Running");
                        }
                    });
                });

                ui.separator();

                // Action buttons
                ui.horizontal(|ui| {
                    // Show container status if running
                    let is_running = self.game_containers.iter().any(|c| {
                        c.game_id == game.id && matches!(c.status, ContainerStatus::Running)
                    });

                    let play_button = if game.install_path.exists() {
                        if is_running {
                            ui.button("⏹️ Stop")
                        } else {
                            ui.button("▶ Play")
                        }
                    } else {
                        ui.button("📥 Install")
                    };

                    if play_button.clicked() {
                        self.handle_game_action(game, ui.ctx());
                    }

                    if ui.small_button("⚙").on_hover_text("Settings").clicked() {
                        // TODO: Open game settings
                    }

                    if ui.small_button("ℹ️").on_hover_text("Info").clicked() {
                        // TODO: Show game info
                    }
                });
            });
        });
    }

    fn show_game_list_item(&mut self, ui: &mut egui::Ui, game: &crate::game::Game) {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                // Smaller icon for list view
                ui.add_sized([32.0, 32.0], egui::Button::new("🎮"));

                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.strong(&game.name);
                        if game.favorite {
                            ui.label("♥️");
                        }
                    });
                    ui.small(format!(
                        "{} • {}h played",
                        game.launcher.as_ref().unwrap_or(&"Unknown".to_string()),
                        game.playtime_minutes / 60
                    ));
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Show container status
                    let is_running = self.game_containers.iter().any(|c| {
                        c.game_id == game.id && matches!(c.status, ContainerStatus::Running)
                    });

                    if is_running {
                        ui.colored_label(egui::Color32::GREEN, "🟢 Running");
                        if ui.small_button("⏹️").on_hover_text("Stop").clicked() {
                            self.stop_game_container(&game.id, ui.ctx());
                        }
                    } else if game.install_path.exists() {
                        if ui.button("▶ Play").clicked() {
                            self.handle_game_action(game, ui.ctx());
                        }
                    } else {
                        if ui.button("📥 Install").clicked() {
                            // TODO: Install game
                        }
                    }

                    if ui.small_button("⚙").on_hover_text("Settings").clicked() {
                        // TODO: Open game settings
                    }
                });
            });
        });
    }

    fn handle_game_action(&mut self, game: &crate::game::Game, ctx: &egui::Context) {
        // Check if game is already running
        let is_running = self
            .game_containers
            .iter()
            .any(|c| c.game_id == game.id && matches!(c.status, ContainerStatus::Running));

        if is_running {
            self.stop_game_container(&game.id, ctx);
        } else if game.install_path.exists() {
            // Launch game using Bolt
            let bolt_manager: Arc<BoltGameManager> = Arc::clone(&self.bolt_manager);
            let game_clone = game.clone();
            let ctx = ctx.clone();

            tokio::spawn(async move {
                match bolt_manager.launch_game(&game_clone.id, &game_clone).await {
                    Ok(container_id) => {
                        println!(
                            "🎮 Launched {} in container: {}",
                            game_clone.name, container_id
                        );
                    }
                    Err(e) => {
                        eprintln!("❌ Failed to launch {}: {}", game_clone.name, e);
                    }
                }
                ctx.request_repaint();
            });
        }
    }

    fn stop_game_container(&mut self, game_id: &str, ctx: &egui::Context) {
        if let Some(container) = self
            .game_containers
            .iter()
            .find(|c| c.game_id == game_id && matches!(c.status, ContainerStatus::Running))
        {
            let bolt_manager: Arc<BoltGameManager> = Arc::clone(&self.bolt_manager);
            let container_id = container.id.clone();
            let ctx = ctx.clone();

            tokio::spawn(async move {
                if let Err(e) = bolt_manager.stop_game(&container_id).await {
                    eprintln!("Failed to stop container: {}", e);
                }
                ctx.request_repaint();
            });
        }
    }

    fn show_containers(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("📦 Container Management");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("🔄 Refresh").clicked() {
                    self.refresh_containers_async(ui.ctx());
                }
                if ui.button("📊 Metrics").clicked() {
                    self.refresh_metrics_async(ui.ctx());
                }
                if ui.button("🗑️ Cleanup").clicked() {
                    // TODO: Cleanup stopped containers
                }
            });
        });

        ui.separator();

        // Check for completed async operations
        if let Some(promise) = &self.container_refresh_promise {
            if let Some(result) = promise.ready() {
                match result {
                    Ok(containers) => {
                        self.game_containers = containers.clone();
                    }
                    Err(error) => {
                        self.error_message = Some(error.clone());
                    }
                }
                self.container_refresh_promise = None;
            }
        }

        if let Some(promise) = &self.metrics_promise {
            if let Some(result) = promise.ready() {
                match result {
                    Ok(metrics) => {
                        self.bolt_metrics = Some(metrics.clone());
                    }
                    Err(error) => {
                        self.error_message = Some(error.clone());
                    }
                }
                self.metrics_promise = None;
            }
        }

        // System metrics overview
        if let Some(metrics) = &self.bolt_metrics {
            ui.horizontal(|ui| {
                ui.group(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.strong(format!("{}", metrics.running_containers));
                        ui.label("Running");
                    });
                });

                ui.group(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.strong(format!("{}", metrics.total_containers));
                        ui.label("Total");
                    });
                });

                ui.group(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.strong(format!("{:.1}%", metrics.cpu_usage));
                        ui.label("CPU");
                    });
                });

                ui.group(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.strong(format!("{:.1}%", metrics.memory_usage));
                        ui.label("Memory");
                    });
                });

                ui.group(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.strong(format!("{:.1}%", metrics.gpu_usage));
                        ui.label("GPU");
                    });
                });
            });

            ui.separator();
        }

        // Container list
        if self.container_refresh_promise.is_some() {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Loading containers...");
            });
        } else if self.game_containers.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.vertical_centered(|ui| {
                    ui.label("📦 No game containers found");
                    ui.label("Launch a game to see containers here");
                });
            });
        } else {
            egui::ScrollArea::vertical().show(ui, |ui| {
                for container in &self.game_containers {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            // Container status indicator
                            let (status_color, status_icon) = match &container.status {
                                ContainerStatus::Running => (egui::Color32::GREEN, "🟢"),
                                ContainerStatus::Stopped => (egui::Color32::GRAY, "⭕"),
                                ContainerStatus::Paused => (egui::Color32::YELLOW, "⏸️"),
                                ContainerStatus::Error(_) => (egui::Color32::RED, "❌"),
                                ContainerStatus::Creating => (egui::Color32::BLUE, "🔄"),
                                ContainerStatus::Updating => (egui::Color32::BLUE, "⬆️"),
                            };

                            ui.label(status_icon);

                            ui.vertical(|ui| {
                                ui.strong(&container.name);
                                ui.small(format!("Game: {}", container.game_id));
                                ui.small(format!("Image: {}", container.image));
                                if let Some(wine_version) = &container.wine_version {
                                    ui.small(format!("Wine: {}", wine_version));
                                }
                                ui.small(format!(
                                    "GPU: {}",
                                    if container.gpu_enabled {
                                        "Enabled"
                                    } else {
                                        "Disabled"
                                    }
                                ));
                            });

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    match &container.status {
                                        ContainerStatus::Running => {
                                            if ui.small_button("⏹️").on_hover_text("Stop").clicked()
                                            {
                                                let bolt_manager: Arc<BoltGameManager> =
                                                    Arc::clone(&self.bolt_manager);
                                                let container_id = container.id.clone();
                                                let ctx = ui.ctx().clone();

                                                tokio::spawn(async move {
                                                    if let Err(e) =
                                                        bolt_manager.stop_game(&container_id).await
                                                    {
                                                        eprintln!(
                                                            "Failed to stop container: {}",
                                                            e
                                                        );
                                                    }
                                                    ctx.request_repaint();
                                                });
                                            }
                                        }
                                        ContainerStatus::Stopped => {
                                            if ui
                                                .small_button("▶️")
                                                .on_hover_text("Start")
                                                .clicked()
                                            {
                                                // TODO: Implement container restart
                                            }
                                        }
                                        _ => {}
                                    }

                                    if ui.small_button("📊").on_hover_text("Details").clicked() {
                                        self.selected_game = Some(container.id.clone());
                                        self.show_container_details = true;
                                    }

                                    if ui.small_button("📋").on_hover_text("Logs").clicked() {
                                        // TODO: Show container logs
                                    }

                                    ui.colored_label(
                                        status_color,
                                        format!("{:?}", container.status),
                                    );
                                },
                            );
                        });
                    });
                }
            });
        }

        // Container details popup
        if self.show_container_details {
            if let Some(selected_id) = &self.selected_game {
                if let Some(container) = self.game_containers.iter().find(|c| &c.id == selected_id)
                {
                    egui::Window::new(format!("Container Details: {}", container.name))
                        .collapsible(false)
                        .resizable(true)
                        .show(ui.ctx(), |ui| {
                            ui.label(format!("ID: {}", container.id));
                            ui.label(format!("Game: {}", container.game_id));
                            ui.label(format!("Image: {}", container.image));
                            ui.label(format!(
                                "Created: {}",
                                container.created.format("%Y-%m-%d %H:%M:%S")
                            ));
                            ui.label(format!("Status: {:?}", container.status));
                            ui.label(format!("GPU Enabled: {}", container.gpu_enabled));
                            ui.label(format!(
                                "Performance Profile: {}",
                                container.performance_profile
                            ));

                            if !container.ports.is_empty() {
                                ui.label("Ports:");
                                for port in &container.ports {
                                    ui.label(format!("  - {}", port));
                                }
                            }

                            ui.separator();
                            ui.horizontal(|ui| {
                                if ui.button("Close").clicked() {
                                    self.show_container_details = false;
                                    self.selected_game = None;
                                }
                            });
                        });
                }
            }
        }
    }

    fn show_display(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("🖼️ Display & VRR Management");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("🔄 Refresh Displays").clicked() {
                    if let Err(e) = self.display_manager.detect_displays() {
                        self.error_message = Some(format!("Failed to detect displays: {}", e));
                    }
                }
                if ui.button("⚙️ Gaming Optimize").clicked() {
                    if let Err(e) = self
                        .display_manager
                        .optimize_for_gaming(self.gaming_display_settings.target_fps)
                    {
                        self.error_message = Some(format!("Failed to optimize displays: {}", e));
                    }
                }
                if ui.button("📊 Start VRR Monitor").clicked() {
                    if let Err(e) = self.vrr_monitor.start_monitoring(&self.display_manager) {
                        self.error_message = Some(format!("Failed to start VRR monitoring: {}", e));
                    }
                }
            });
        });

        ui.separator();

        // Session type indicator
        ui.horizontal(|ui| {
            let session_type = if self.display_manager.is_wayland_session() {
                "🐧 Wayland Session"
            } else {
                "🖼️ X11 Session"
            };
            ui.colored_label(egui::Color32::from_rgb(100, 150, 200), session_type);

            if self.vrr_monitor.is_monitoring_active() {
                ui.colored_label(egui::Color32::GREEN, "📊 VRR Monitoring Active");
            }
        });

        ui.separator();

        // Display configuration
        egui::ScrollArea::vertical().show(ui, |ui| {
            for display in self.display_manager.get_displays() {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        // Display info
                        ui.vertical(|ui| {
                            ui.strong(&display.name);
                            ui.label(format!(
                                "{}x{} @ {}Hz",
                                display.resolution.width,
                                display.resolution.height,
                                display.current_refresh_rate
                            ));
                            ui.small(format!("Connector: {}", display.connector));
                        });

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            // VRR Status
                            if display.vrr_capable {
                                let vrr_color = if display.vrr_enabled {
                                    egui::Color32::GREEN
                                } else {
                                    egui::Color32::YELLOW
                                };
                                let vrr_text = if display.vrr_enabled {
                                    "🟢 VRR ON"
                                } else {
                                    "🟡 VRR OFF"
                                };
                                ui.colored_label(vrr_color, vrr_text);
                            } else {
                                ui.colored_label(egui::Color32::GRAY, "❌ No VRR");
                            }

                            // HDR Status
                            if display.hdr_capable {
                                let hdr_color = if display.hdr_enabled {
                                    egui::Color32::GREEN
                                } else {
                                    egui::Color32::YELLOW
                                };
                                let hdr_text = if display.hdr_enabled {
                                    "✨ HDR ON"
                                } else {
                                    "🌙 HDR OFF"
                                };
                                ui.colored_label(hdr_color, hdr_text);
                            }

                            // Primary indicator
                            if display.primary {
                                ui.colored_label(egui::Color32::BLUE, "👑 Primary");
                            }
                        });
                    });

                    // Available refresh rates
                    if !display.refresh_rates.is_empty() {
                        ui.horizontal(|ui| {
                            ui.label("Available rates:");
                            for &rate in &display.refresh_rates {
                                let is_current = rate == display.current_refresh_rate;
                                let rate_color = if is_current {
                                    egui::Color32::from_rgb(100, 200, 100)
                                } else {
                                    egui::Color32::GRAY
                                };
                                ui.colored_label(rate_color, format!("{}Hz", rate));
                            }
                        });
                    }
                });

                ui.add_space(5.0);
            }

            // VRR Performance Metrics
            if self.vrr_monitor.is_monitoring_active() {
                ui.group(|ui| {
                    ui.heading("📊 Real-time Performance");

                    if let Some(metrics) = self.vrr_monitor.get_real_time_stats() {
                        ui.horizontal(|ui| {
                            ui.group(|ui| {
                                ui.vertical_centered(|ui| {
                                    ui.strong(format!("{:.1}", metrics.current_fps));
                                    ui.label("FPS");
                                });
                            });

                            ui.group(|ui| {
                                ui.vertical_centered(|ui| {
                                    ui.strong(format!("{:.1}ms", metrics.frame_time_ms));
                                    ui.label("Frame Time");
                                });
                            });

                            ui.group(|ui| {
                                ui.vertical_centered(|ui| {
                                    ui.strong(format!("{:.1}ms", metrics.presentation_latency));
                                    ui.label("Latency");
                                });
                            });

                            ui.group(|ui| {
                                ui.vertical_centered(|ui| {
                                    ui.strong(format!("{}Hz", metrics.vrr_range.current_hz));
                                    ui.label("VRR Rate");
                                });
                            });
                        });

                        // Frame time variance (jitter)
                        ui.horizontal(|ui| {
                            ui.label("Frame consistency:");
                            let consistency =
                                100.0 - (metrics.frame_time_variance * 10.0).min(100.0);
                            let consistency_color = if consistency > 90.0 {
                                egui::Color32::GREEN
                            } else if consistency > 70.0 {
                                egui::Color32::YELLOW
                            } else {
                                egui::Color32::RED
                            };
                            ui.colored_label(consistency_color, format!("{:.1}%", consistency));
                        });
                    }
                });
            }

            // Display Profiles
            ui.group(|ui| {
                ui.heading("🎮 Display Profiles");

                ui.horizontal(|ui| {
                    // Collect profile names to avoid borrowing issues
                    let profile_names: Vec<_> = self
                        .display_manager
                        .get_profiles()
                        .iter()
                        .map(|(name, profile)| (name.clone(), profile.clone()))
                        .collect();

                    for (profile_name, profile) in profile_names {
                        let profile_color = if profile.gaming_optimized {
                            egui::Color32::from_rgb(100, 200, 100)
                        } else {
                            egui::Color32::from_rgb(100, 150, 200)
                        };

                        if ui.colored_label(profile_color, &profile.name).clicked() {
                            if let Err(e) = self.display_manager.apply_profile(&profile_name) {
                                self.error_message =
                                    Some(format!("Failed to apply profile: {}", e));
                            }
                        }
                    }
                });
            });

            // Gaming Display Settings
            ui.group(|ui| {
                ui.heading("⚙️ Gaming Settings");

                ui.horizontal(|ui| {
                    ui.label("Target FPS:");
                    ui.add(egui::Slider::new(
                        &mut self.gaming_display_settings.target_fps,
                        30..=240,
                    ));
                });

                ui.horizontal(|ui| {
                    ui.label("VSync Mode:");
                    egui::ComboBox::from_label("")
                        .selected_text(format!("{:?}", self.gaming_display_settings.vsync_mode))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.gaming_display_settings.vsync_mode,
                                VsyncMode::Off,
                                "Off",
                            );
                            ui.selectable_value(
                                &mut self.gaming_display_settings.vsync_mode,
                                VsyncMode::On,
                                "On",
                            );
                            ui.selectable_value(
                                &mut self.gaming_display_settings.vsync_mode,
                                VsyncMode::Adaptive,
                                "Adaptive",
                            );
                            ui.selectable_value(
                                &mut self.gaming_display_settings.vsync_mode,
                                VsyncMode::FastSync,
                                "FastSync",
                            );
                            ui.selectable_value(
                                &mut self.gaming_display_settings.vsync_mode,
                                VsyncMode::Enhanced,
                                "Enhanced",
                            );
                        });
                });

                ui.checkbox(
                    &mut self.gaming_display_settings.frame_pacing,
                    "Frame Pacing",
                );
                ui.checkbox(
                    &mut self.gaming_display_settings.low_latency_mode,
                    "Low Latency Mode",
                );
                ui.checkbox(&mut self.gaming_display_settings.hdr_gaming, "HDR Gaming");
                ui.checkbox(
                    &mut self.gaming_display_settings.fullscreen_optimizations,
                    "Fullscreen Optimizations",
                );
            });
        });
    }
}

#[cfg(feature = "gui")]
pub fn run_gui() -> Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([800.0, 600.0])
            .with_icon(
                eframe::icon_data::from_png_bytes(&include_bytes!("../assets/icons/GhostForge-icon-64.png")[..])
                    .unwrap_or_default(),
            ),
        ..Default::default()
    };

    eframe::run_native(
        "GhostForge - Gaming Manager",
        options,
        Box::new(|_cc| {
            // Configure egui fonts and visuals
            Ok(Box::new(GhostForgeApp::default()))
        }),
    )
    .map_err(|e| anyhow::anyhow!("Failed to run GUI: {}", e))
}

#[cfg(not(feature = "gui"))]
pub fn run_gui() -> anyhow::Result<()> {
    Err(anyhow::anyhow!(
        "GUI feature not enabled. Compile with --features gui"
    ))
}
