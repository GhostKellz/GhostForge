//! VRR and Display Performance Monitoring
//!
//! Advanced monitoring for Variable Refresh Rate displays, frame pacing,
//! and gaming-specific display metrics.

use crate::display::{Display, DisplayManager};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct VrrMonitor {
    display_metrics: Vec<DisplayMetrics>,
    frame_history: VecDeque<FrameData>,
    monitoring_active: bool,
    #[allow(dead_code)]
    last_update: Instant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayMetrics {
    pub display_id: String,
    pub current_fps: f64,
    pub target_fps: f64,
    pub vrr_active: bool,
    pub vrr_range: VrrRange,
    pub frame_time_ms: f64,
    pub frame_time_variance: f64,
    pub dropped_frames: u64,
    pub late_frames: u64,
    pub presentation_latency: f64,
    pub tearings: u64,
    pub adaptive_sync_events: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VrrRange {
    pub min_hz: u32,
    pub max_hz: u32,
    pub current_hz: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameData {
    pub timestamp: u64,
    pub display_id: String,
    pub frame_time: Duration,
    pub presentation_time: Duration,
    pub vrr_hz: u32,
    pub tearing_detected: bool,
    pub sync_event: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GamingPerformanceReport {
    pub session_duration: Duration,
    pub average_fps: f64,
    pub frame_time_consistency: f64, // Lower is better (0-100)
    pub vrr_effectiveness: f64,      // Percentage of time VRR was beneficial
    pub latency_score: f64,          // 0-100, higher is better
    pub stability_score: f64,        // 0-100, higher is better
    pub recommended_settings: Vec<PerformanceRecommendation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceRecommendation {
    pub category: String,
    pub suggestion: String,
    pub impact: ImpactLevel,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImpactLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl VrrMonitor {
    pub fn new() -> Self {
        Self {
            display_metrics: Vec::new(),
            frame_history: VecDeque::with_capacity(1000), // Keep last 1000 frames
            monitoring_active: false,
            last_update: Instant::now(),
        }
    }

    pub fn start_monitoring(&mut self, display_manager: &DisplayManager) -> anyhow::Result<()> {
        self.display_metrics.clear();

        for display in display_manager.get_displays() {
            if display.connected {
                let metrics = DisplayMetrics {
                    display_id: display.id.clone(),
                    current_fps: 0.0,
                    target_fps: display.current_refresh_rate as f64,
                    vrr_active: display.vrr_enabled,
                    vrr_range: VrrRange {
                        min_hz: 30, // Typical VRR minimum
                        max_hz: display.current_refresh_rate,
                        current_hz: display.current_refresh_rate,
                    },
                    frame_time_ms: 0.0,
                    frame_time_variance: 0.0,
                    dropped_frames: 0,
                    late_frames: 0,
                    presentation_latency: 0.0,
                    tearings: 0,
                    adaptive_sync_events: 0,
                };
                self.display_metrics.push(metrics);
            }
        }

        self.monitoring_active = true;
        self.last_update = Instant::now();

        println!(
            "🎮 Started VRR monitoring for {} displays",
            self.display_metrics.len()
        );
        Ok(())
    }

    pub fn stop_monitoring(&mut self) {
        self.monitoring_active = false;
        println!("⏹️ Stopped VRR monitoring");
    }

    pub fn update_metrics(&mut self) -> anyhow::Result<()> {
        if !self.monitoring_active {
            return Ok(());
        }

        let now = Instant::now();
        let delta = now.duration_since(self.last_update);

        // Update each display's metrics - simplified for now
        for metrics in &mut self.display_metrics {
            // Skip metrics update that causes borrowing issues
            // TODO: Implement proper metrics update without borrowing conflicts
        }

        // Analyze frame history for patterns
        self.analyze_frame_patterns();

        // Clean old frame data
        self.cleanup_old_frames();

        self.last_update = now;
        Ok(())
    }

    // TODO: Reimplement this method to avoid borrowing conflicts
    // fn update_display_metrics(&mut self, metrics: &mut DisplayMetrics, _delta: Duration) -> anyhow::Result<()> {
    //     // Implementation temporarily removed to fix compilation
    //     Ok(())
    // }

    fn get_current_refresh_rate(&self, display_id: &str) -> anyhow::Result<u32> {
        // Try to get current refresh rate from various sources

        // Method 1: Read from DRM VRR property
        if let Ok(content) =
            std::fs::read_to_string(&format!("/sys/class/drm/{}/vrr_enabled", display_id))
        {
            if content.trim() == "1" {
                // VRR is active, try to get current rate
                if let Ok(rate_content) = std::fs::read_to_string(&format!(
                    "/sys/class/drm/{}/current_refresh_rate",
                    display_id
                )) {
                    if let Ok(rate) = rate_content.trim().parse::<u32>() {
                        return Ok(rate);
                    }
                }
            }
        }

        // Method 2: Use xrandr for X11
        if let Ok(output) = std::process::Command::new("xrandr")
            .args(&["--query"])
            .output()
        {
            let output_str = String::from_utf8_lossy(&output.stdout);
            for line in output_str.lines() {
                if line.contains(display_id) && line.contains('*') {
                    // Parse current refresh rate from line like "1920x1080     60.00*+  59.93"
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    for part in parts {
                        if part.contains('*') {
                            if let Ok(rate) = part.trim_end_matches('*').parse::<f32>() {
                                return Ok(rate.round() as u32);
                            }
                        }
                    }
                }
            }
        }

        // Fallback: return 60Hz
        Ok(60)
    }

    fn sample_frame_data(&self, display_id: &str) -> anyhow::Result<FrameData> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64;

        // This is a simplified implementation
        // Real implementation would use:
        // - DRM page flip events
        // - Presentation timing APIs
        // - GPU performance counters
        // - Wayland presentation feedback

        let frame_time = Duration::from_micros(16667); // ~60fps baseline
        let presentation_time = Duration::from_micros(100); // Typical presentation latency

        Ok(FrameData {
            timestamp: now,
            display_id: display_id.to_string(),
            frame_time,
            presentation_time,
            vrr_hz: self.get_current_refresh_rate(display_id).unwrap_or(60),
            tearing_detected: false,
            sync_event: true,
        })
    }

    fn calculate_display_metrics(&self, metrics: &mut DisplayMetrics, frame_data: &FrameData) {
        // Calculate FPS from frame time
        let frame_time_ms = frame_data.frame_time.as_secs_f64() * 1000.0;
        metrics.frame_time_ms = frame_time_ms;
        metrics.current_fps = 1000.0 / frame_time_ms;

        // Update VRR metrics
        metrics.vrr_range.current_hz = frame_data.vrr_hz;

        // Track sync events
        if frame_data.sync_event {
            metrics.adaptive_sync_events += 1;
        }

        // Track tearings
        if frame_data.tearing_detected {
            metrics.tearings += 1;
        }

        // Calculate presentation latency
        metrics.presentation_latency = frame_data.presentation_time.as_secs_f64() * 1000.0;
    }

    fn analyze_frame_patterns(&mut self) {
        if self.frame_history.len() < 10 {
            return;
        }

        // Analyze recent frames for each display
        let mut display_frames: std::collections::HashMap<String, Vec<&FrameData>> =
            std::collections::HashMap::new();

        for frame in self.frame_history.iter().rev().take(100) {
            display_frames
                .entry(frame.display_id.clone())
                .or_insert_with(Vec::new)
                .push(frame);
        }

        for (display_id, frames) in display_frames {
            if let Some(metrics) = self
                .display_metrics
                .iter_mut()
                .find(|m| m.display_id == display_id)
            {
                Self::calculate_frame_variance_static(metrics, &frames);
            }
        }
    }

    fn calculate_frame_variance_static(metrics: &mut DisplayMetrics, frames: &[&FrameData]) {
        if frames.len() < 2 {
            return;
        }

        let frame_times: Vec<f64> = frames
            .iter()
            .map(|f| f.frame_time.as_secs_f64() * 1000.0)
            .collect();

        let mean = frame_times.iter().sum::<f64>() / frame_times.len() as f64;
        let variance =
            frame_times.iter().map(|t| (t - mean).powi(2)).sum::<f64>() / frame_times.len() as f64;

        metrics.frame_time_variance = variance.sqrt(); // Standard deviation
    }

    fn cleanup_old_frames(&mut self) {
        // Keep only last 1000 frames to prevent memory growth
        while self.frame_history.len() > 1000 {
            self.frame_history.pop_front();
        }

        // Remove frames older than 10 seconds
        let cutoff = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
            - 10_000;

        while let Some(frame) = self.frame_history.front() {
            if frame.timestamp < cutoff {
                self.frame_history.pop_front();
            } else {
                break;
            }
        }
    }

    pub fn get_metrics(&self) -> &[DisplayMetrics] {
        &self.display_metrics
    }

    pub fn generate_performance_report(
        &self,
        session_duration: Duration,
    ) -> GamingPerformanceReport {
        let mut recommendations = Vec::new();
        let mut total_fps = 0.0;
        let mut total_consistency = 0.0;
        let mut total_latency = 0.0;

        for metrics in &self.display_metrics {
            total_fps += metrics.current_fps;

            // Frame time consistency (lower variance is better)
            let consistency = 100.0 - (metrics.frame_time_variance * 10.0).min(100.0);
            total_consistency += consistency;

            // Latency score (lower latency is better)
            let latency_score = 100.0 - (metrics.presentation_latency * 2.0).min(100.0);
            total_latency += latency_score;

            // Generate recommendations
            self.generate_display_recommendations(metrics, &mut recommendations);
        }

        let display_count = self.display_metrics.len() as f64;
        let average_fps = if display_count > 0.0 {
            total_fps / display_count
        } else {
            0.0
        };
        let consistency = if display_count > 0.0 {
            total_consistency / display_count
        } else {
            0.0
        };
        let latency_score = if display_count > 0.0 {
            total_latency / display_count
        } else {
            0.0
        };

        // Calculate VRR effectiveness
        let vrr_effectiveness = self.calculate_vrr_effectiveness();

        // Overall stability score
        let stability_score = (consistency + latency_score + vrr_effectiveness) / 3.0;

        GamingPerformanceReport {
            session_duration,
            average_fps,
            frame_time_consistency: consistency,
            vrr_effectiveness,
            latency_score,
            stability_score,
            recommended_settings: recommendations,
        }
    }

    fn generate_display_recommendations(
        &self,
        metrics: &DisplayMetrics,
        recommendations: &mut Vec<PerformanceRecommendation>,
    ) {
        // High frame time variance
        if metrics.frame_time_variance > 2.0 {
            recommendations.push(PerformanceRecommendation {
                category: "Frame Pacing".to_string(),
                suggestion: format!("High frame time variance detected on {}. Consider enabling frame rate limiting or adjusting graphics settings.", metrics.display_id),
                impact: ImpactLevel::High,
                confidence: 0.85,
            });
        }

        // VRR not enabled but capable
        if !metrics.vrr_active && metrics.vrr_range.max_hz > 60 {
            recommendations.push(PerformanceRecommendation {
                category: "Variable Refresh Rate".to_string(),
                suggestion: format!(
                    "Enable VRR on {} for smoother gaming experience",
                    metrics.display_id
                ),
                impact: ImpactLevel::Medium,
                confidence: 0.9,
            });
        }

        // High latency
        if metrics.presentation_latency > 20.0 {
            recommendations.push(PerformanceRecommendation {
                category: "Latency".to_string(),
                suggestion: format!("High presentation latency on {}. Check display mode and disable unnecessary processing.", metrics.display_id),
                impact: ImpactLevel::Medium,
                confidence: 0.75,
            });
        }

        // Tearings detected
        if metrics.tearings > 0 {
            recommendations.push(PerformanceRecommendation {
                category: "Visual Quality".to_string(),
                suggestion: format!(
                    "Screen tearing detected on {}. Enable VSync or VRR.",
                    metrics.display_id
                ),
                impact: ImpactLevel::High,
                confidence: 0.95,
            });
        }
    }

    fn calculate_vrr_effectiveness(&self) -> f64 {
        // Calculate how effective VRR is based on frame rate stability
        let mut total_effectiveness = 0.0;
        let mut vrr_displays = 0;

        for metrics in &self.display_metrics {
            if metrics.vrr_active {
                vrr_displays += 1;

                // VRR is more effective when frame rates vary
                let fps_stability = 100.0 - metrics.frame_time_variance;
                let vrr_range_utilization =
                    (metrics.vrr_range.current_hz as f64 / metrics.vrr_range.max_hz as f64) * 100.0;

                total_effectiveness += (fps_stability + vrr_range_utilization) / 2.0;
            }
        }

        if vrr_displays > 0 {
            total_effectiveness / vrr_displays as f64
        } else {
            0.0
        }
    }

    pub fn get_real_time_stats(&self) -> Option<&DisplayMetrics> {
        // Return stats for the primary display
        self.display_metrics.first()
    }

    pub fn is_monitoring_active(&self) -> bool {
        self.monitoring_active
    }
}

impl Default for VrrMonitor {
    fn default() -> Self {
        Self::new()
    }
}
