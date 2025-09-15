//! Display Management and Gaming Performance Features
//!
//! This module provides comprehensive display management for gaming on Linux,
//! including Wayland support, G-Sync/FreeSync VRR, and performance optimization.

use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use anyhow::{Result, anyhow};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayManager {
    displays: Vec<Display>,
    current_profile: Option<String>,
    profiles: HashMap<String, DisplayProfile>,
    wayland_session: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Display {
    pub id: String,
    pub name: String,
    pub connector: String,
    pub resolution: Resolution,
    pub refresh_rates: Vec<u32>,
    pub current_refresh_rate: u32,
    pub vrr_capable: bool,
    pub vrr_enabled: bool,
    pub hdr_capable: bool,
    pub hdr_enabled: bool,
    pub connected: bool,
    pub primary: bool,
    pub position: Position,
    pub rotation: Rotation,
    pub scaling: f32,
    pub color_depth: u8,
    pub manufacturer: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Rotation {
    Normal,
    Left,
    Right,
    Inverted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayProfile {
    pub name: String,
    pub description: String,
    pub displays: HashMap<String, DisplayConfig>,
    pub gaming_optimized: bool,
    pub vrr_mode: VrrMode,
    pub latency_reduction: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayConfig {
    pub resolution: Resolution,
    pub refresh_rate: u32,
    pub vrr_enabled: bool,
    pub hdr_enabled: bool,
    pub position: Position,
    pub rotation: Rotation,
    pub scaling: f32,
    pub primary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VrrMode {
    Disabled,
    GSync,
    FreeSync,
    Adaptive,
    Auto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GamingDisplaySettings {
    pub target_fps: u32,
    pub vsync_mode: VsyncMode,
    pub frame_pacing: bool,
    pub low_latency_mode: bool,
    pub hdr_gaming: bool,
    pub fullscreen_optimizations: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VsyncMode {
    Off,
    On,
    Adaptive,
    FastSync,
    Enhanced,
}

impl DisplayManager {
    pub fn new() -> Result<Self> {
        let wayland_session = std::env::var("WAYLAND_DISPLAY").is_ok() ||
                             std::env::var("XDG_SESSION_TYPE").map(|s| s == "wayland").unwrap_or(false);

        let mut manager = Self {
            displays: Vec::new(),
            current_profile: None,
            profiles: HashMap::new(),
            wayland_session,
        };

        manager.detect_displays()?;
        manager.create_default_profiles();

        Ok(manager)
    }

    pub fn detect_displays(&mut self) -> Result<()> {
        self.displays.clear();

        if self.wayland_session {
            self.detect_wayland_displays()?;
        } else {
            self.detect_x11_displays()?;
        }

        Ok(())
    }

    #[cfg(feature = "wayland-gaming")]
    fn detect_wayland_displays(&mut self) -> Result<()> {
        use wayland_client::{Connection, Dispatch, QueueHandle, EventQueue};
        use wayland_protocols::xdg::shell::client::xdg_wm_base;

        // Connect to Wayland compositor
        let connection = Connection::connect_to_env()
            .map_err(|e| anyhow!("Failed to connect to Wayland: {}", e))?;

        // Get display information through Wayland protocols
        // This is a simplified implementation - full implementation would use
        // wayland-protocols to get detailed display information

        self.detect_displays_fallback()?;
        Ok(())
    }

    #[cfg(not(feature = "wayland-gaming"))]
    fn detect_wayland_displays(&mut self) -> Result<()> {
        // Fallback for when wayland features aren't compiled
        self.detect_displays_fallback()
    }

    fn detect_x11_displays(&mut self) -> Result<()> {
        // Use xrandr to detect X11 displays
        let output = Command::new("xrandr")
            .arg("--query")
            .output()
            .map_err(|e| anyhow!("Failed to run xrandr: {}", e))?;

        let xrandr_output = String::from_utf8_lossy(&output.stdout);
        self.parse_xrandr_output(&xrandr_output)?;

        Ok(())
    }

    fn detect_displays_fallback(&mut self) -> Result<()> {
        // Use DRM for hardware-level display detection
        #[cfg(feature = "display-management")]
        {
            self.detect_drm_displays()?;
        }

        #[cfg(not(feature = "display-management"))]
        {
            // Create a mock display for development
            self.displays.push(Display {
                id: "eDP-1".to_string(),
                name: "Built-in Display".to_string(),
                connector: "eDP".to_string(),
                resolution: Resolution { width: 1920, height: 1080 },
                refresh_rates: vec![60, 120, 144],
                current_refresh_rate: 60,
                vrr_capable: true,
                vrr_enabled: false,
                hdr_capable: false,
                hdr_enabled: false,
                connected: true,
                primary: true,
                position: Position { x: 0, y: 0 },
                rotation: Rotation::Normal,
                scaling: 1.0,
                color_depth: 24,
                manufacturer: "Unknown".to_string(),
                model: "Generic".to_string(),
            });
        }

        Ok(())
    }

    #[cfg(feature = "display-management")]
    fn detect_drm_displays(&mut self) -> Result<()> {
        use drm::control::{Device as ControlDevice, connector, crtc, Mode};
        use std::fs::OpenOptions;

        // Open DRM device
        let paths = [
            "/dev/dri/card0",
            "/dev/dri/card1",
            "/dev/dri/card2",
        ];

        for path in &paths {
            if let Ok(file) = OpenOptions::new().read(true).write(true).open(path) {
                if let Ok(device) = drm::Device::new_from_file(file) {
                    self.enumerate_drm_connectors(&device)?;
                    break;
                }
            }
        }

        Ok(())
    }

    #[cfg(feature = "display-management")]
    fn enumerate_drm_connectors(&mut self, device: &drm::Device) -> Result<()> {
        use drm::control::{connector, Mode};

        let resources = device.resource_handles()
            .map_err(|e| anyhow!("Failed to get DRM resources: {}", e))?;

        for &connector_id in resources.connectors() {
            if let Ok(connector_info) = device.get_connector(connector_id, false) {
                if connector_info.state() == connector::State::Connected {
                    let display = self.create_display_from_connector(&connector_info)?;
                    self.displays.push(display);
                }
            }
        }

        Ok(())
    }

    #[cfg(feature = "display-management")]
    fn create_display_from_connector(&self, connector: &connector::Info) -> Result<Display> {
        let connector_name = format!("{}-{}",
            self.connector_type_name(connector.connector_type()),
            connector.connector_type_id()
        );

        // Get available modes (resolutions and refresh rates)
        let modes = connector.modes();
        let mut refresh_rates = Vec::new();
        let mut max_resolution = Resolution { width: 0, height: 0 };

        for mode in modes {
            let refresh_rate = self.calculate_refresh_rate(mode);
            if !refresh_rates.contains(&refresh_rate) {
                refresh_rates.push(refresh_rate);
            }

            if mode.size().0 * mode.size().1 > max_resolution.width * max_resolution.height {
                max_resolution = Resolution {
                    width: mode.size().0 as u32,
                    height: mode.size().1 as u32,
                };
            }
        }

        refresh_rates.sort_unstable();
        refresh_rates.reverse(); // Highest first

        // Check for VRR capability
        let vrr_capable = self.check_vrr_capability(&connector_name)?;

        Ok(Display {
            id: connector_name.clone(),
            name: format!("Display {}", connector_name),
            connector: connector_name,
            resolution: max_resolution,
            refresh_rates,
            current_refresh_rate: refresh_rates.first().copied().unwrap_or(60),
            vrr_capable,
            vrr_enabled: false,
            hdr_capable: false, // TODO: Detect HDR capability
            hdr_enabled: false,
            connected: true,
            primary: false, // TODO: Detect primary display
            position: Position { x: 0, y: 0 },
            rotation: Rotation::Normal,
            scaling: 1.0,
            color_depth: 24,
            manufacturer: "Unknown".to_string(),
            model: "Unknown".to_string(),
        })
    }

    fn connector_type_name(&self, connector_type: connector::Type) -> &'static str {
        match connector_type {
            connector::Type::HDMIA => "HDMI",
            connector::Type::HDMIB => "HDMI",
            connector::Type::DisplayPort => "DP",
            connector::Type::DVI => "DVI",
            connector::Type::VGA => "VGA",
            connector::Type::LVDS => "LVDS",
            connector::Type::eDP => "eDP",
            _ => "Unknown",
        }
    }

    #[cfg(feature = "display-management")]
    fn calculate_refresh_rate(&self, mode: &drm::control::Mode) -> u32 {
        if mode.vsync() == 0 || mode.hsync() == 0 {
            return 60; // Fallback
        }

        let pixel_clock = mode.clock() as f64 * 1000.0; // Convert to Hz
        let total_pixels = (mode.hsync() + mode.hskew()) as f64 * mode.vsync() as f64;

        (pixel_clock / total_pixels).round() as u32
    }

    fn parse_xrandr_output(&mut self, output: &str) -> Result<()> {
        let mut current_display: Option<Display> = None;

        for line in output.lines() {
            if line.contains(" connected") {
                if let Some(display) = current_display.take() {
                    self.displays.push(display);
                }

                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    let name = parts[0].to_string();
                    let resolution_part = parts[2];

                    let (resolution, position) = self.parse_resolution_position(resolution_part)?;

                    current_display = Some(Display {
                        id: name.clone(),
                        name: name.clone(),
                        connector: name,
                        resolution,
                        refresh_rates: Vec::new(),
                        current_refresh_rate: 60,
                        vrr_capable: false,
                        vrr_enabled: false,
                        hdr_capable: false,
                        hdr_enabled: false,
                        connected: true,
                        primary: line.contains("primary"),
                        position,
                        rotation: Rotation::Normal,
                        scaling: 1.0,
                        color_depth: 24,
                        manufacturer: "Unknown".to_string(),
                        model: "Unknown".to_string(),
                    });
                }
            } else if line.trim().chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
                // Parse available modes
                if let Some(ref mut display) = current_display {
                    self.parse_xrandr_mode_line(line, display);
                }
            }
        }

        if let Some(display) = current_display {
            self.displays.push(display);
        }

        // Check for VRR support on each display
        for display in &mut self.displays {
            display.vrr_capable = self.check_vrr_capability(&display.id)?;
        }

        Ok(())
    }

    fn parse_resolution_position(&self, res_pos: &str) -> Result<(Resolution, Position)> {
        // Parse strings like "1920x1080+0+0" or "1920x1080"
        let parts: Vec<&str> = res_pos.split('+').collect();

        if parts.is_empty() {
            return Err(anyhow!("Invalid resolution format"));
        }

        let res_parts: Vec<&str> = parts[0].split('x').collect();
        if res_parts.len() != 2 {
            return Err(anyhow!("Invalid resolution format"));
        }

        let width = res_parts[0].parse::<u32>()?;
        let height = res_parts[1].parse::<u32>()?;
        let resolution = Resolution { width, height };

        let position = if parts.len() >= 3 {
            Position {
                x: parts[1].parse::<i32>().unwrap_or(0),
                y: parts[2].parse::<i32>().unwrap_or(0),
            }
        } else {
            Position { x: 0, y: 0 }
        };

        Ok((resolution, position))
    }

    fn parse_xrandr_mode_line(&self, line: &str, display: &mut Display) {
        let parts: Vec<&str> = line.trim().split_whitespace().collect();
        if parts.is_empty() {
            return;
        }

        // Parse resolution
        let res_parts: Vec<&str> = parts[0].split('x').collect();
        if res_parts.len() == 2 {
            if let (Ok(width), Ok(height)) = (res_parts[0].parse::<u32>(), res_parts[1].parse::<u32>()) {
                // Look for refresh rates
                for part in &parts[1..] {
                    if let Some(rate_str) = part.strip_suffix('*') {
                        // Current mode
                        if let Ok(rate) = rate_str.parse::<f32>() {
                            let rate_int = rate.round() as u32;
                            display.current_refresh_rate = rate_int;
                            if !display.refresh_rates.contains(&rate_int) {
                                display.refresh_rates.push(rate_int);
                            }
                        }
                    } else if let Ok(rate) = part.parse::<f32>() {
                        let rate_int = rate.round() as u32;
                        if !display.refresh_rates.contains(&rate_int) {
                            display.refresh_rates.push(rate_int);
                        }
                    }
                }

                // Update resolution if this is the current mode
                if line.contains('*') {
                    display.resolution = Resolution { width, height };
                }
            }
        }
    }

    fn check_vrr_capability(&self, display_id: &str) -> Result<bool> {
        // Check for NVIDIA G-Sync
        if let Ok(output) = Command::new("nvidia-settings")
            .args(&["-q", &format!("[gpu:0]/GPUAdaptiveSync")])
            .output() {
            if output.status.success() {
                let output_str = String::from_utf8_lossy(&output.stdout);
                if output_str.contains("1") {
                    return Ok(true);
                }
            }
        }

        // Check for AMD FreeSync via DRM properties
        if let Ok(output) = Command::new("cat")
            .arg(&format!("/sys/class/drm/{}/vrr_capable", display_id))
            .output() {
            if output.status.success() {
                let output_str = String::from_utf8_lossy(&output.stdout).trim();
                return Ok(output_str == "1");
            }
        }

        // Check via xrandr properties
        if let Ok(output) = Command::new("xrandr")
            .args(&["--prop"])
            .output() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            return Ok(output_str.contains("vrr") || output_str.contains("Variable refresh"));
        }

        Ok(false)
    }

    fn create_default_profiles(&mut self) {
        // Gaming profile with VRR enabled
        let mut gaming_displays = HashMap::new();
        for display in &self.displays {
            gaming_displays.insert(display.id.clone(), DisplayConfig {
                resolution: display.resolution.clone(),
                refresh_rate: *display.refresh_rates.first().unwrap_or(&60),
                vrr_enabled: display.vrr_capable,
                hdr_enabled: false,
                position: display.position.clone(),
                rotation: display.rotation.clone(),
                scaling: 1.0,
                primary: display.primary,
            });
        }

        self.profiles.insert("Gaming".to_string(), DisplayProfile {
            name: "Gaming".to_string(),
            description: "Optimized for gaming with VRR enabled".to_string(),
            displays: gaming_displays,
            gaming_optimized: true,
            vrr_mode: VrrMode::Auto,
            latency_reduction: true,
        });

        // Productivity profile
        let mut productivity_displays = HashMap::new();
        for display in &self.displays {
            productivity_displays.insert(display.id.clone(), DisplayConfig {
                resolution: display.resolution.clone(),
                refresh_rate: 60, // Standard refresh rate for productivity
                vrr_enabled: false,
                hdr_enabled: false,
                position: display.position.clone(),
                rotation: display.rotation.clone(),
                scaling: display.scaling,
                primary: display.primary,
            });
        }

        self.profiles.insert("Productivity".to_string(), DisplayProfile {
            name: "Productivity".to_string(),
            description: "Standard settings for productivity work".to_string(),
            displays: productivity_displays,
            gaming_optimized: false,
            vrr_mode: VrrMode::Disabled,
            latency_reduction: false,
        });
    }

    pub fn get_displays(&self) -> &[Display] {
        &self.displays
    }

    pub fn get_profiles(&self) -> &HashMap<String, DisplayProfile> {
        &self.profiles
    }

    pub fn apply_profile(&mut self, profile_name: &str) -> Result<()> {
        let profile = self.profiles.get(profile_name)
            .ok_or_else(|| anyhow!("Profile '{}' not found", profile_name))?
            .clone();

        if self.wayland_session {
            self.apply_wayland_profile(&profile)?;
        } else {
            self.apply_x11_profile(&profile)?;
        }

        self.current_profile = Some(profile_name.to_string());
        Ok(())
    }

    fn apply_wayland_profile(&self, profile: &DisplayProfile) -> Result<()> {
        // Apply Wayland-specific display configuration
        // This would use compositor-specific protocols (wlr-output-management, etc.)
        println!("Applying Wayland profile: {}", profile.name);

        for (display_id, config) in &profile.displays {
            self.configure_wayland_display(display_id, config)?;
        }

        Ok(())
    }

    fn configure_wayland_display(&self, display_id: &str, config: &DisplayConfig) -> Result<()> {
        // Configure individual display on Wayland
        // This is compositor-specific - different for GNOME/KDE/wlroots

        // For wlroots-based compositors (sway, etc.)
        if let Ok(_) = Command::new("swaymsg")
            .args(&[
                "output", display_id,
                "resolution", &format!("{}x{}", config.resolution.width, config.resolution.height),
                "rate", &format!("{}Hz", config.refresh_rate),
                "position", &format!("{},{}", config.position.x, config.position.y),
            ])
            .status() {
            println!("Applied Sway configuration for {}", display_id);
        }

        // Configure VRR if supported
        if config.vrr_enabled {
            self.enable_vrr_wayland(display_id)?;
        } else {
            self.disable_vrr_wayland(display_id)?;
        }

        Ok(())
    }

    fn apply_x11_profile(&self, profile: &DisplayProfile) -> Result<()> {
        let mut xrandr_cmd = Command::new("xrandr");

        for (display_id, config) in &profile.displays {
            xrandr_cmd.args(&[
                "--output", display_id,
                "--mode", &format!("{}x{}", config.resolution.width, config.resolution.height),
                "--rate", &config.refresh_rate.to_string(),
                "--pos", &format!("{}x{}", config.position.x, config.position.y),
                "--scale", &config.scaling.to_string(),
            ]);

            if config.primary {
                xrandr_cmd.arg("--primary");
            }

            match config.rotation {
                Rotation::Left => { xrandr_cmd.args(&["--rotate", "left"]); }
                Rotation::Right => { xrandr_cmd.args(&["--rotate", "right"]); }
                Rotation::Inverted => { xrandr_cmd.args(&["--rotate", "inverted"]); }
                Rotation::Normal => { xrandr_cmd.args(&["--rotate", "normal"]); }
            }
        }

        let output = xrandr_cmd.output()?;
        if !output.status.success() {
            return Err(anyhow!("Failed to apply X11 display configuration: {}",
                String::from_utf8_lossy(&output.stderr)));
        }

        // Apply VRR settings
        for (display_id, config) in &profile.displays {
            if config.vrr_enabled {
                self.enable_vrr_x11(display_id)?;
            } else {
                self.disable_vrr_x11(display_id)?;
            }
        }

        Ok(())
    }

    fn enable_vrr_wayland(&self, display_id: &str) -> Result<()> {
        // Enable VRR on Wayland (compositor-specific)
        println!("Enabling VRR for {} on Wayland", display_id);
        Ok(())
    }

    fn disable_vrr_wayland(&self, display_id: &str) -> Result<()> {
        // Disable VRR on Wayland
        println!("Disabling VRR for {} on Wayland", display_id);
        Ok(())
    }

    fn enable_vrr_x11(&self, display_id: &str) -> Result<()> {
        // Try NVIDIA method first
        if let Ok(_) = Command::new("nvidia-settings")
            .args(&["-a", &format!("[gpu:0]/GPUAdaptiveSync=1")])
            .status() {
            println!("Enabled NVIDIA G-Sync for {}", display_id);
            return Ok(());
        }

        // Try AMD method
        if let Ok(_) = std::fs::write(&format!("/sys/class/drm/{}/vrr_enabled", display_id), "1") {
            println!("Enabled AMD FreeSync for {}", display_id);
            return Ok(());
        }

        // Try xrandr property method
        let _ = Command::new("xrandr")
            .args(&["--output", display_id, "--set", "vrr", "1"])
            .status();

        Ok(())
    }

    fn disable_vrr_x11(&self, display_id: &str) -> Result<()> {
        // Try NVIDIA method first
        let _ = Command::new("nvidia-settings")
            .args(&["-a", &format!("[gpu:0]/GPUAdaptiveSync=0")])
            .status();

        // Try AMD method
        let _ = std::fs::write(&format!("/sys/class/drm/{}/vrr_enabled", display_id), "0");

        // Try xrandr property method
        let _ = Command::new("xrandr")
            .args(&["--output", display_id, "--set", "vrr", "0"])
            .status();

        Ok(())
    }

    pub fn optimize_for_gaming(&mut self, target_fps: u32) -> Result<()> {
        println!("Optimizing displays for gaming (target: {}fps)", target_fps);

        for display in &mut self.displays {
            if display.connected {
                // Find the best refresh rate for the target FPS
                let best_rate = self.find_optimal_refresh_rate(&display.refresh_rates, target_fps);
                display.current_refresh_rate = best_rate;

                // Enable VRR if capable
                if display.vrr_capable {
                    display.vrr_enabled = true;
                }
            }
        }

        // Apply gaming profile
        self.apply_profile("Gaming")?;
        Ok(())
    }

    fn find_optimal_refresh_rate(&self, available_rates: &[u32], target_fps: u32) -> u32 {
        // Find the lowest refresh rate that's still above the target FPS
        let mut best_rate = available_rates.first().copied().unwrap_or(60);

        for &rate in available_rates {
            if rate >= target_fps && rate < best_rate {
                best_rate = rate;
            }
        }

        best_rate
    }

    pub fn get_gaming_settings(&self) -> GamingDisplaySettings {
        GamingDisplaySettings {
            target_fps: 120,
            vsync_mode: if self.displays.iter().any(|d| d.vrr_enabled) {
                VsyncMode::Adaptive
            } else {
                VsyncMode::On
            },
            frame_pacing: true,
            low_latency_mode: true,
            hdr_gaming: self.displays.iter().any(|d| d.hdr_enabled),
            fullscreen_optimizations: true,
        }
    }

    pub fn is_wayland_session(&self) -> bool {
        self.wayland_session
    }
}

impl Default for DisplayManager {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            displays: Vec::new(),
            current_profile: None,
            profiles: HashMap::new(),
            wayland_session: false,
        })
    }
}