use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub timestamp: DateTime<Utc>,
    pub fps: f32,
    pub frame_time_ms: f32,
    pub gpu_usage_percent: f32,
    pub gpu_memory_used_mb: u64,
    pub gpu_memory_total_mb: u64,
    pub gpu_temperature_c: f32,
    pub gpu_power_watts: f32,
    pub cpu_usage_percent: f32,
    pub ram_used_mb: u64,
    pub ram_total_mb: u64,
    pub disk_read_mbps: f32,
    pub disk_write_mbps: f32,
    pub network_download_mbps: f32,
    pub network_upload_mbps: f32,
    pub game_process_memory_mb: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceProfile {
    pub game_id: String,
    pub hardware_signature: String,
    pub average_metrics: PerformanceMetrics,
    pub peak_metrics: PerformanceMetrics,
    pub low_metrics: PerformanceMetrics,
    pub sample_count: u64,
    pub session_duration_minutes: f32,
    pub stability_score: f32,    // 0.0 - 1.0
    pub optimization_score: f32, // 0.0 - 1.0
    pub bottlenecks: Vec<PerformanceBottleneck>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceBottleneck {
    pub component: SystemComponent,
    pub severity: BottleneckSeverity,
    pub description: String,
    pub recommendation: String,
    pub confidence: f32, // 0.0 - 1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemComponent {
    GPU,
    CPU,
    RAM,
    VRAM,
    Storage,
    Network,
    Thermal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BottleneckSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceTarget {
    pub target_fps: f32,
    pub max_frame_time_ms: f32,
    pub target_gpu_usage: f32,
    pub max_gpu_temperature: f32,
    pub quality_preset: QualityPreset,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QualityPreset {
    Performance, // High FPS, lower quality
    Balanced,    // Balance between FPS and quality
    Quality,     // High quality, acceptable FPS
    Ultra,       // Maximum quality, FPS secondary
}

impl Default for PerformanceTarget {
    fn default() -> Self {
        Self {
            target_fps: 60.0,
            max_frame_time_ms: 16.7, // ~60 FPS
            target_gpu_usage: 85.0,
            max_gpu_temperature: 80.0,
            quality_preset: QualityPreset::Balanced,
        }
    }
}

pub struct PerformanceMonitor {
    game_id: String,
    metrics_history: Vec<PerformanceMetrics>,
    current_session_start: Instant,
    metrics_sender: mpsc::UnboundedSender<PerformanceMetrics>,
    metrics_receiver: mpsc::UnboundedReceiver<PerformanceMetrics>,
    overlay_enabled: bool,
    recording_enabled: bool,
    hardware_info: HardwareInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareInfo {
    pub cpu_model: String,
    pub cpu_cores: u32,
    pub cpu_threads: u32,
    pub ram_total_mb: u64,
    pub gpu_model: String,
    pub gpu_vram_mb: u64,
    pub gpu_driver_version: String,
    pub storage_type: StorageType,
    pub signature: String, // Unique hash of hardware config
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageType {
    SSD,
    NVME,
    HDD,
    Unknown,
}

impl PerformanceMonitor {
    pub fn new(game_id: String) -> Result<Self> {
        let (metrics_sender, metrics_receiver) = mpsc::unbounded_channel();
        let hardware_info = Self::detect_hardware()?;

        Ok(Self {
            game_id,
            metrics_history: Vec::new(),
            current_session_start: Instant::now(),
            metrics_sender,
            metrics_receiver,
            overlay_enabled: true,
            recording_enabled: true,
            hardware_info,
        })
    }

    fn detect_hardware() -> Result<HardwareInfo> {
        let mut hw_info = HardwareInfo {
            cpu_model: "Unknown CPU".to_string(),
            cpu_cores: 1,
            cpu_threads: 1,
            ram_total_mb: 8192,
            gpu_model: "Unknown GPU".to_string(),
            gpu_vram_mb: 4096,
            gpu_driver_version: "Unknown".to_string(),
            storage_type: StorageType::Unknown,
            signature: "".to_string(),
        };

        // Detect CPU info
        if let Ok(output) = std::process::Command::new("lscpu").output() {
            let cpu_info = String::from_utf8_lossy(&output.stdout);

            // Parse CPU model
            for line in cpu_info.lines() {
                if line.starts_with("Model name:") {
                    hw_info.cpu_model = line
                        .split(':')
                        .nth(1)
                        .unwrap_or("Unknown")
                        .trim()
                        .to_string();
                }
                if line.starts_with("CPU(s):") {
                    if let Ok(threads) = line.split(':').nth(1).unwrap_or("1").trim().parse::<u32>()
                    {
                        hw_info.cpu_threads = threads;
                    }
                }
                if line.starts_with("Core(s) per socket:") {
                    if let Ok(cores) = line.split(':').nth(1).unwrap_or("1").trim().parse::<u32>() {
                        hw_info.cpu_cores = cores;
                    }
                }
            }
        }

        // Detect RAM
        if let Ok(output) = std::process::Command::new("free").args(&["-m"]).output() {
            let mem_info = String::from_utf8_lossy(&output.stdout);
            for line in mem_info.lines() {
                if line.starts_with("Mem:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() > 1 {
                        if let Ok(total_mb) = parts[1].parse::<u64>() {
                            hw_info.ram_total_mb = total_mb;
                        }
                    }
                }
            }
        }

        // Detect GPU info (NVIDIA)
        if let Ok(output) = std::process::Command::new("nvidia-smi")
            .args(&[
                "--query-gpu=name,driver_version,memory.total",
                "--format=csv,noheader,nounits",
            ])
            .output()
        {
            if output.status.success() {
                let gpu_info = String::from_utf8_lossy(&output.stdout);
                let parts: Vec<&str> = gpu_info.trim().split(", ").collect();
                if parts.len() >= 3 {
                    hw_info.gpu_model = parts[0].trim().to_string();
                    hw_info.gpu_driver_version = parts[1].trim().to_string();
                    if let Ok(vram_mb) = parts[2].trim().parse::<u64>() {
                        hw_info.gpu_vram_mb = vram_mb;
                    }
                }
            }
        }

        // Detect storage type
        if let Ok(output) = std::process::Command::new("lsblk")
            .args(&["-d", "-o", "NAME,ROTA"])
            .output()
        {
            let storage_info = String::from_utf8_lossy(&output.stdout);
            for line in storage_info.lines().skip(1) {
                if line.contains("0") {
                    // Non-rotating = SSD/NVME
                    hw_info.storage_type = StorageType::SSD;
                    break;
                } else if line.contains("1") {
                    // Rotating = HDD
                    hw_info.storage_type = StorageType::HDD;
                }
            }
        }

        // Generate hardware signature
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        hw_info.cpu_model.hash(&mut hasher);
        hw_info.cpu_cores.hash(&mut hasher);
        hw_info.ram_total_mb.hash(&mut hasher);
        hw_info.gpu_model.hash(&mut hasher);
        hw_info.gpu_vram_mb.hash(&mut hasher);
        hw_info.signature = format!("{:x}", hasher.finish());

        Ok(hw_info)
    }

    pub async fn start_monitoring(&mut self) -> Result<()> {
        println!(
            "🔍 Starting performance monitoring for game: {}",
            self.game_id
        );

        // Start metrics collection in background
        let sender = self.metrics_sender.clone();
        let game_id = self.game_id.clone();

        tokio::spawn(async move {
            Self::collect_metrics_loop(sender, game_id).await;
        });

        Ok(())
    }

    async fn collect_metrics_loop(
        sender: mpsc::UnboundedSender<PerformanceMetrics>,
        _game_id: String,
    ) {
        let mut interval = tokio::time::interval(Duration::from_millis(1000)); // 1 Hz
        let mut frame_times = Vec::new();
        let mut last_frame_time = Instant::now();

        loop {
            interval.tick().await;

            // Calculate frame metrics
            let now = Instant::now();
            let frame_time_ms = now.duration_since(last_frame_time).as_millis() as f32;
            frame_times.push(frame_time_ms);

            // Keep only last 60 frames for FPS calculation
            if frame_times.len() > 60 {
                frame_times.remove(0);
            }

            let avg_frame_time = frame_times.iter().sum::<f32>() / frame_times.len() as f32;
            let fps = if avg_frame_time > 0.0 {
                1000.0 / avg_frame_time
            } else {
                0.0
            };

            // Collect system metrics
            let metrics = match Self::collect_system_metrics(fps, avg_frame_time).await {
                Ok(metrics) => metrics,
                Err(e) => {
                    eprintln!("Failed to collect metrics: {}", e);
                    continue;
                }
            };

            // Send metrics
            if sender.send(metrics).is_err() {
                break; // Channel closed, exit loop
            }

            last_frame_time = now;
        }
    }

    async fn collect_system_metrics(fps: f32, frame_time_ms: f32) -> Result<PerformanceMetrics> {
        let mut metrics = PerformanceMetrics {
            timestamp: Utc::now(),
            fps,
            frame_time_ms,
            gpu_usage_percent: 0.0,
            gpu_memory_used_mb: 0,
            gpu_memory_total_mb: 0,
            gpu_temperature_c: 0.0,
            gpu_power_watts: 0.0,
            cpu_usage_percent: 0.0,
            ram_used_mb: 0,
            ram_total_mb: 0,
            disk_read_mbps: 0.0,
            disk_write_mbps: 0.0,
            network_download_mbps: 0.0,
            network_upload_mbps: 0.0,
            game_process_memory_mb: 0,
        };

        // GPU metrics (NVIDIA)
        if let Ok(output) = tokio::process::Command::new("nvidia-smi")
            .args(&[
                "--query-gpu=utilization.gpu,memory.used,memory.total,temperature.gpu,power.draw",
                "--format=csv,noheader,nounits",
            ])
            .output()
            .await
        {
            if output.status.success() {
                let gpu_data = String::from_utf8_lossy(&output.stdout);
                let parts: Vec<&str> = gpu_data.trim().split(", ").collect();
                if parts.len() >= 5 {
                    metrics.gpu_usage_percent = parts[0].parse().unwrap_or(0.0);
                    metrics.gpu_memory_used_mb = parts[1].parse().unwrap_or(0);
                    metrics.gpu_memory_total_mb = parts[2].parse().unwrap_or(0);
                    metrics.gpu_temperature_c = parts[3].parse().unwrap_or(0.0);
                    metrics.gpu_power_watts = parts[4].parse().unwrap_or(0.0);
                }
            }
        }

        // CPU metrics
        if let Ok(output) = tokio::process::Command::new("top")
            .args(&["-bn1", "-p1"])
            .output()
            .await
        {
            let top_output = String::from_utf8_lossy(&output.stdout);
            for line in top_output.lines() {
                if line.starts_with("%Cpu(s):") {
                    // Parse CPU usage from top output
                    if let Some(usage_str) = line.split_whitespace().nth(1) {
                        if let Ok(usage) = usage_str.trim_end_matches("us,").parse::<f32>() {
                            metrics.cpu_usage_percent = usage;
                        }
                    }
                    break;
                }
            }
        }

        // Memory metrics
        if let Ok(output) = tokio::process::Command::new("free")
            .args(&["-m"])
            .output()
            .await
        {
            let mem_info = String::from_utf8_lossy(&output.stdout);
            for line in mem_info.lines() {
                if line.starts_with("Mem:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 3 {
                        metrics.ram_total_mb = parts[1].parse().unwrap_or(0);
                        metrics.ram_used_mb = parts[2].parse().unwrap_or(0);
                    }
                }
            }
        }

        Ok(metrics)
    }

    pub async fn process_metrics(&mut self) -> Result<()> {
        while let Ok(metrics) = self.metrics_receiver.try_recv() {
            self.metrics_history.push(metrics);

            // Keep only last 1000 metrics (about 16 minutes at 1Hz)
            if self.metrics_history.len() > 1000 {
                self.metrics_history.remove(0);
            }

            // Update overlay if enabled
            if self.overlay_enabled {
                self.update_overlay().await?;
            }
        }

        Ok(())
    }

    async fn update_overlay(&self) -> Result<()> {
        if let Some(latest_metrics) = self.metrics_history.last() {
            // This would integrate with the UI overlay system
            println!(
                "FPS: {:.1} | Frame Time: {:.1}ms | GPU: {:.1}% | RAM: {}MB",
                latest_metrics.fps,
                latest_metrics.frame_time_ms,
                latest_metrics.gpu_usage_percent,
                latest_metrics.ram_used_mb
            );
        }
        Ok(())
    }

    pub fn generate_performance_profile(&self) -> Result<PerformanceProfile> {
        if self.metrics_history.is_empty() {
            return Err(anyhow::anyhow!("No metrics collected yet"));
        }

        let sample_count = self.metrics_history.len() as u64;
        let session_duration = self.current_session_start.elapsed().as_secs() as f32 / 60.0;

        // Calculate average metrics
        let mut avg_metrics = PerformanceMetrics {
            timestamp: Utc::now(),
            fps: 0.0,
            frame_time_ms: 0.0,
            gpu_usage_percent: 0.0,
            gpu_memory_used_mb: 0,
            gpu_memory_total_mb: 0,
            gpu_temperature_c: 0.0,
            gpu_power_watts: 0.0,
            cpu_usage_percent: 0.0,
            ram_used_mb: 0,
            ram_total_mb: 0,
            disk_read_mbps: 0.0,
            disk_write_mbps: 0.0,
            network_download_mbps: 0.0,
            network_upload_mbps: 0.0,
            game_process_memory_mb: 0,
        };

        // Sum all metrics
        for metrics in &self.metrics_history {
            avg_metrics.fps += metrics.fps;
            avg_metrics.frame_time_ms += metrics.frame_time_ms;
            avg_metrics.gpu_usage_percent += metrics.gpu_usage_percent;
            avg_metrics.gpu_memory_used_mb += metrics.gpu_memory_used_mb;
            avg_metrics.gpu_temperature_c += metrics.gpu_temperature_c;
            avg_metrics.gpu_power_watts += metrics.gpu_power_watts;
            avg_metrics.cpu_usage_percent += metrics.cpu_usage_percent;
            avg_metrics.ram_used_mb += metrics.ram_used_mb;
        }

        // Calculate averages
        let count = self.metrics_history.len() as f32;
        avg_metrics.fps /= count;
        avg_metrics.frame_time_ms /= count;
        avg_metrics.gpu_usage_percent /= count;
        avg_metrics.gpu_memory_used_mb = (avg_metrics.gpu_memory_used_mb as f32 / count) as u64;
        avg_metrics.gpu_temperature_c /= count;
        avg_metrics.gpu_power_watts /= count;
        avg_metrics.cpu_usage_percent /= count;
        avg_metrics.ram_used_mb = (avg_metrics.ram_used_mb as f32 / count) as u64;

        // Find peak and low metrics
        let peak_metrics = self.find_peak_metrics();
        let low_metrics = self.find_low_metrics();

        // Calculate stability and optimization scores
        let stability_score = self.calculate_stability_score();
        let optimization_score = self.calculate_optimization_score(&avg_metrics);

        // Identify bottlenecks
        let bottlenecks = self.identify_bottlenecks(&avg_metrics, &peak_metrics);

        Ok(PerformanceProfile {
            game_id: self.game_id.clone(),
            hardware_signature: self.hardware_info.signature.clone(),
            average_metrics: avg_metrics,
            peak_metrics,
            low_metrics,
            sample_count,
            session_duration_minutes: session_duration,
            stability_score,
            optimization_score,
            bottlenecks,
        })
    }

    fn find_peak_metrics(&self) -> PerformanceMetrics {
        self.metrics_history
            .iter()
            .max_by(|a, b| {
                a.fps
                    .partial_cmp(&b.fps)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .cloned()
            .unwrap_or_else(|| self.metrics_history[0].clone())
    }

    fn find_low_metrics(&self) -> PerformanceMetrics {
        self.metrics_history
            .iter()
            .min_by(|a, b| {
                a.fps
                    .partial_cmp(&b.fps)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .cloned()
            .unwrap_or_else(|| self.metrics_history[0].clone())
    }

    fn calculate_stability_score(&self) -> f32 {
        if self.metrics_history.len() < 10 {
            return 0.5; // Not enough data
        }

        let fps_values: Vec<f32> = self.metrics_history.iter().map(|m| m.fps).collect();
        let mean_fps = fps_values.iter().sum::<f32>() / fps_values.len() as f32;

        let variance = fps_values
            .iter()
            .map(|fps| (*fps - mean_fps).powi(2))
            .sum::<f32>()
            / fps_values.len() as f32;

        let std_dev = variance.sqrt();
        let coefficient_of_variation = if mean_fps > 0.0 {
            std_dev / mean_fps
        } else {
            1.0
        };

        // Lower coefficient of variation = higher stability
        (1.0 - coefficient_of_variation.min(1.0)).max(0.0)
    }

    fn calculate_optimization_score(&self, avg_metrics: &PerformanceMetrics) -> f32 {
        let mut score = 0.0;
        let mut factors = 0;

        // FPS score (target 60 FPS)
        if avg_metrics.fps > 0.0 {
            score += (avg_metrics.fps / 60.0).min(1.0);
            factors += 1;
        }

        // GPU utilization score (target 80-90%)
        if avg_metrics.gpu_usage_percent > 0.0 {
            let gpu_score = if avg_metrics.gpu_usage_percent <= 90.0 {
                avg_metrics.gpu_usage_percent / 90.0
            } else {
                1.0 - ((avg_metrics.gpu_usage_percent - 90.0) / 10.0).min(1.0)
            };
            score += gpu_score;
            factors += 1;
        }

        // Temperature score (lower is better, target < 80°C)
        if avg_metrics.gpu_temperature_c > 0.0 {
            let temp_score = if avg_metrics.gpu_temperature_c <= 80.0 {
                1.0
            } else {
                (100.0 - avg_metrics.gpu_temperature_c) / 20.0
            }
            .max(0.0);
            score += temp_score;
            factors += 1;
        }

        if factors > 0 {
            score / factors as f32
        } else {
            0.5
        }
    }

    fn identify_bottlenecks(
        &self,
        avg_metrics: &PerformanceMetrics,
        _peak_metrics: &PerformanceMetrics,
    ) -> Vec<PerformanceBottleneck> {
        let mut bottlenecks = Vec::new();

        // GPU bottleneck detection
        if avg_metrics.gpu_usage_percent > 95.0 {
            bottlenecks.push(PerformanceBottleneck {
                component: SystemComponent::GPU,
                severity: BottleneckSeverity::High,
                description: "GPU usage consistently above 95%".to_string(),
                recommendation: "Lower graphics settings or enable DLSS/FSR".to_string(),
                confidence: 0.9,
            });
        }

        // VRAM bottleneck
        if avg_metrics.gpu_memory_total_mb > 0 {
            let vram_usage = (avg_metrics.gpu_memory_used_mb as f32
                / avg_metrics.gpu_memory_total_mb as f32)
                * 100.0;
            if vram_usage > 90.0 {
                bottlenecks.push(PerformanceBottleneck {
                    component: SystemComponent::VRAM,
                    severity: BottleneckSeverity::Critical,
                    description: format!("VRAM usage at {:.1}%", vram_usage),
                    recommendation: "Lower texture quality or resolution".to_string(),
                    confidence: 0.95,
                });
            }
        }

        // CPU bottleneck
        if avg_metrics.cpu_usage_percent > 90.0 {
            bottlenecks.push(PerformanceBottleneck {
                component: SystemComponent::CPU,
                severity: BottleneckSeverity::Medium,
                description: "CPU usage consistently above 90%".to_string(),
                recommendation: "Close background applications or lower CPU-intensive settings"
                    .to_string(),
                confidence: 0.8,
            });
        }

        // Thermal throttling detection
        if avg_metrics.gpu_temperature_c > 85.0 {
            bottlenecks.push(PerformanceBottleneck {
                component: SystemComponent::Thermal,
                severity: BottleneckSeverity::High,
                description: format!("GPU temperature at {:.1}°C", avg_metrics.gpu_temperature_c),
                recommendation: "Improve cooling or lower graphics settings".to_string(),
                confidence: 0.85,
            });
        }

        // Low FPS detection
        if avg_metrics.fps < 30.0 {
            bottlenecks.push(PerformanceBottleneck {
                component: SystemComponent::GPU, // Most likely GPU related for gaming
                severity: BottleneckSeverity::Critical,
                description: format!("Average FPS below 30 ({:.1})", avg_metrics.fps),
                recommendation: "Significantly lower graphics settings or upgrade hardware"
                    .to_string(),
                confidence: 0.7,
            });
        }

        bottlenecks
    }

    pub fn get_optimization_recommendations(
        &self,
        target: &PerformanceTarget,
    ) -> Result<Vec<String>> {
        let profile = self.generate_performance_profile()?;
        let mut recommendations = Vec::new();

        // FPS recommendations
        if profile.average_metrics.fps < target.target_fps {
            let fps_deficit = target.target_fps - profile.average_metrics.fps;
            if fps_deficit > 20.0 {
                recommendations.push("Consider lowering resolution or using DLSS/FSR".to_string());
            } else if fps_deficit > 10.0 {
                recommendations.push("Lower graphics settings to Medium or High".to_string());
            } else {
                recommendations.push("Disable anti-aliasing or reduce shadow quality".to_string());
            }
        }

        // GPU recommendations
        if profile.average_metrics.gpu_usage_percent > 95.0 {
            recommendations.push(
                "GPU is fully utilized - consider upgrading GPU or lowering settings".to_string(),
            );
        } else if profile.average_metrics.gpu_usage_percent < 60.0 {
            recommendations
                .push("GPU is underutilized - you can increase graphics settings".to_string());
        }

        // Temperature recommendations
        if profile.average_metrics.gpu_temperature_c > target.max_gpu_temperature {
            recommendations
                .push("GPU is running hot - improve case airflow or lower power limit".to_string());
        }

        // Memory recommendations
        if profile.average_metrics.gpu_memory_total_mb > 0 {
            let vram_usage = (profile.average_metrics.gpu_memory_used_mb as f32
                / profile.average_metrics.gpu_memory_total_mb as f32)
                * 100.0;
            if vram_usage > 90.0 {
                recommendations.push("VRAM usage is critical - lower texture quality".to_string());
            }
        }

        // Stability recommendations
        if profile.stability_score < 0.7 {
            recommendations.push(
                "Performance is unstable - check for thermal throttling or background processes"
                    .to_string(),
            );
        }

        Ok(recommendations)
    }

    pub fn export_session_data(&self, format: ExportFormat) -> Result<String> {
        match format {
            ExportFormat::JSON => {
                let profile = self.generate_performance_profile()?;
                Ok(serde_json::to_string_pretty(&profile)?)
            }
            ExportFormat::CSV => {
                let mut csv = String::from(
                    "timestamp,fps,frame_time_ms,gpu_usage,gpu_temp,cpu_usage,ram_used\n",
                );
                for metrics in &self.metrics_history {
                    csv.push_str(&format!(
                        "{},{},{},{},{},{},{}\n",
                        metrics.timestamp.format("%Y-%m-%d %H:%M:%S"),
                        metrics.fps,
                        metrics.frame_time_ms,
                        metrics.gpu_usage_percent,
                        metrics.gpu_temperature_c,
                        metrics.cpu_usage_percent,
                        metrics.ram_used_mb
                    ));
                }
                Ok(csv)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum ExportFormat {
    JSON,
    CSV,
}

// Real-time overlay component for in-game display
pub struct PerformanceOverlay {
    enabled: bool,
    position: OverlayPosition,
    elements: Vec<OverlayElement>,
    transparency: f32,
}

#[derive(Debug, Clone)]
pub enum OverlayPosition {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Center,
}

#[derive(Debug, Clone)]
pub struct OverlayElement {
    pub metric_type: MetricType,
    pub display_name: String,
    pub format: String,
    pub color: (u8, u8, u8),
    pub threshold_warning: Option<f32>,
    pub threshold_critical: Option<f32>,
}

#[derive(Debug, Clone)]
pub enum MetricType {
    FPS,
    FrameTime,
    GPUUsage,
    GPUTemp,
    GPUMemory,
    CPUUsage,
    RAMUsage,
}

impl Default for PerformanceOverlay {
    fn default() -> Self {
        Self {
            enabled: true,
            position: OverlayPosition::TopLeft,
            elements: vec![
                OverlayElement {
                    metric_type: MetricType::FPS,
                    display_name: "FPS".to_string(),
                    format: "{:.0}".to_string(),
                    color: (255, 255, 255),
                    threshold_warning: Some(30.0),
                    threshold_critical: Some(20.0),
                },
                OverlayElement {
                    metric_type: MetricType::GPUTemp,
                    display_name: "GPU".to_string(),
                    format: "{:.0}°C".to_string(),
                    color: (255, 255, 255),
                    threshold_warning: Some(80.0),
                    threshold_critical: Some(90.0),
                },
            ],
            transparency: 0.8,
        }
    }
}
