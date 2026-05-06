//! MAT Pipeline — CSI 数据 → 灾难分诊系统桥接
//!
//! 将 sensing-server 的 CSI 帧流接入 wifi-densepose-mat 分诊系统。
//! 每收到一帧 CSI 数据：提取生命体征 → 更新/创建伤员 → 计算 START 分诊 → 推送 UI。
//!
//! # 竞赛使用
//! ```rust
//! let mut pipeline = MatPipeline::new(MatConfig::competition());
//! pipeline.process_frame(&frame, node_id, rssi, freq_mhz);
//! let updates = pipeline.get_updates();
//! // 通过 WebSocket 推送到 triage dashboard
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// MAT 管道配置
#[derive(Debug, Clone)]
pub struct MatConfig {
    /// 3 个 C5 节点的物理位置 (node_id → (x, y, z) 单位:米)
    pub node_positions: HashMap<u8, (f64, f64, f64)>,
    /// 是否启用 DensePose 3D 骨架 (默认关闭)
    pub enable_densepose: bool,
    /// 最小信号质量阈值 (0.0-1.0, 低于此值的数据丢弃)
    pub min_signal_quality: f64,
    /// 伤员追踪丢失超时 (秒)
    pub survivor_timeout_secs: f64,
}

impl Default for MatConfig {
    fn default() -> Self {
        Self {
            node_positions: HashMap::new(),
            enable_densepose: false,
            min_signal_quality: 0.1,
            survivor_timeout_secs: 30.0,
        }
    }
}

impl MatConfig {
    /// 竞赛默认配置: 3 节点等边三角形, 间距 2 米
    pub fn competition() -> Self {
        let mut positions = HashMap::new();
        // 节点布局 (等边三角形, 中心在原点)
        positions.insert(1, ( 0.00,  1.15, 1.0)); // 节点1: 上方
        positions.insert(2, (-1.00, -0.58, 1.0)); // 节点2: 左下
        positions.insert(3, ( 1.00, -0.58, 1.0)); // 节点3: 右下
        Self {
            node_positions: positions,
            enable_densepose: false,
            min_signal_quality: 0.1,
            survivor_timeout_secs: 30.0,
        }
    }
}

/// CSI 帧元数据 (从 UDP 包解析)
#[derive(Debug, Clone)]
pub struct CsiFrameMeta {
    pub node_id: u8,
    pub n_subcarriers: u8,
    pub freq_mhz: u16,
    pub rssi: i8,
    pub noise_floor: i8,
    pub sequence: u32,
}

/// 提取出的生命体征
#[derive(Debug, Clone, Default)]
pub struct ExtractedVitals {
    pub breathing_rate: Option<f32>,  // 次/分钟
    pub heart_rate: Option<f32>,      // BPM
    pub motion_score: f32,            // 0-1 运动强度
    pub signal_quality: f32,          // 0-1 信号质量
    pub amplitude_mean: f64,          // 平均振幅
    pub phase_variance: f64,          // 相位方差
}

/// MAT 管道更新事件 (推送到 UI)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriageUpdate {
    pub event_type: String,           // "survivor_update" | "alert" | "assessment"
    pub survivors: Vec<SurvivorSnapshot>,
    pub assessment: Option<MassCasualtySnapshot>,
    pub alerts: Vec<AlertSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurvivorSnapshot {
    pub id: String,
    pub position: Option<PositionSnapshot>,
    pub vitals: VitalsSnapshot,
    pub triage: String,              // "Immediate" | "Delayed" | "Minor" | "Deceased" | "Unknown"
    pub triage_color: String,        // "red" | "yellow" | "green" | "black" | "gray"
    pub is_deteriorating: bool,
    pub tracked_seconds: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionSnapshot {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VitalsSnapshot {
    pub breathing_rate: Option<f32>,
    pub heart_rate: Option<f32>,
    pub motion_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MassCasualtySnapshot {
    pub total: u32,
    pub immediate: u32,
    pub delayed: u32,
    pub minor: u32,
    pub deceased: u32,
    pub unknown: u32,
    pub severity: String,            // "Critical" | "Major" | "Moderate" | "Minimal"
    pub rescuers_needed: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertSnapshot {
    pub time: String,
    pub survivor_id: String,
    pub alert_type: String,
    pub message: String,
    pub priority: u8,
}

// ── 内部状态 ──────────────────────────────────────────────────────────────────

/// 伤员追踪内部状态
#[derive(Debug, Clone)]
struct TrackedSurvivor {
    id: String,
    /// 最近的呼吸率历史 (用于平滑)
    breathing_history: Vec<f32>,
    /// 最近的心率历史
    heart_rate_history: Vec<f32>,
    /// 估计位置 (x, y, z)
    position: (f64, f64, f64),
    position_confidence: f64,
    /// 当前生命体征
    vitals: ExtractedVitals,
    /// 当前分诊
    triage: TriageLevel,
    /// 首次检测时间 (epoch seconds)
    first_seen: f64,
    /// 最后更新时间
    last_updated: f64,
    /// 恶化趋势计数
    deterioration_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TriageLevel {
    Immediate,  // 红色
    Delayed,    // 黄色
    Minor,      // 绿色
    Deceased,   // 黑色
    Unknown,    // 灰色
}

impl TriageLevel {
    fn name(&self) -> &'static str {
        match self {
            TriageLevel::Immediate => "Immediate",
            TriageLevel::Delayed => "Delayed",
            TriageLevel::Minor => "Minor",
            TriageLevel::Deceased => "Deceased",
            TriageLevel::Unknown => "Unknown",
        }
    }

    fn color(&self) -> &'static str {
        match self {
            TriageLevel::Immediate => "red",
            TriageLevel::Delayed => "yellow",
            TriageLevel::Minor => "green",
            TriageLevel::Deceased => "black",
            TriageLevel::Unknown => "gray",
        }
    }

    fn priority(&self) -> u8 {
        match self {
            TriageLevel::Immediate => 1,
            TriageLevel::Delayed => 2,
            TriageLevel::Minor => 3,
            TriageLevel::Deceased => 4,
            TriageLevel::Unknown => 5,
        }
    }
}

/// 从生命体征计算 START 分诊
fn calculate_triage(vitals: &ExtractedVitals) -> TriageLevel {
    // 无有效生命体征 → Unknown
    if vitals.breathing_rate.is_none() && vitals.heart_rate.is_none() {
        if vitals.signal_quality < 0.05 {
            return TriageLevel::Unknown;
        }
        // 有信号但检测不到生命体征 → Deceased
        return TriageLevel::Deceased;
    }

    let br = vitals.breathing_rate.unwrap_or(0.0);
    let hr = vitals.heart_rate.unwrap_or(0.0);

    // START 协议判定:
    // Immediate (红): 呼吸 >30 或 <10, 或心率 >120 或 <40
    if br > 30.0 || (br > 0.0 && br < 10.0) || hr > 120.0 || (hr > 0.0 && hr < 40.0) {
        return TriageLevel::Immediate;
    }

    // Delayed (黄): 呼吸/心率异常但不致命, 或运动强度高(可能受伤)
    if (br >= 10.0 && br <= 12.0) || (br >= 25.0 && br <= 30.0)
        || (hr >= 100.0 && hr <= 120.0) || (hr >= 40.0 && hr <= 50.0)
        || vitals.motion_score > 0.7
    {
        return TriageLevel::Delayed;
    }

    // Minor (绿): 生命体征正常, 有运动能力
    if vitals.motion_score > 0.3 {
        return TriageLevel::Minor;
    }

    // 默认: 稳定但需观察 → Delayed
    TriageLevel::Delayed
}

// ── 主管道 ────────────────────────────────────────────────────────────────────

/// MAT Pipeline: CSI → 分诊 完整数据流
pub struct MatPipeline {
    config: MatConfig,
    survivors: HashMap<String, TrackedSurvivor>,
    alerts: Vec<AlertSnapshot>,
    survivor_counter: u32,
    start_time: f64,
    /// 最新的更新事件
    latest_update: Option<TriageUpdate>,
}

impl MatPipeline {
    /// 创建新管道
    pub fn new(config: MatConfig) -> Self {
        Self {
            config,
            survivors: HashMap::new(),
            alerts: Vec::new(),
            survivor_counter: 0,
            start_time: current_epoch_secs(),
            latest_update: None,
        }
    }

    /// 处理一帧 CSI 数据
    ///
    /// # Arguments
    /// * `amplitudes` - 各子载波的振幅值 (raw f64)
    /// * `phases` - 各子载波的相位值 (raw f64)
    /// * `meta` - 帧元数据 (节点ID, RSSI, 频率等)
    ///
    /// # Returns
    /// * true 如果产生了新的 UI 更新
    pub fn process_frame(
        &mut self,
        amplitudes: &[f64],
        phases: &[f64],
        meta: &CsiFrameMeta,
    ) -> bool {
        if amplitudes.is_empty() {
            return false;
        }

        // 1. 提取振幅特征
        let amp_mean = amplitudes.iter().sum::<f64>() / amplitudes.len() as f64;
        let amp_variance = amplitudes.iter()
            .map(|a| (a - amp_mean).powi(2))
            .sum::<f64>() / amplitudes.len() as f64;

        // 2. 提取相位方差 (运动检测)
        let phase_mean = phases.iter().sum::<f64>() / phases.len() as f64;
        let phase_variance = phases.iter()
            .map(|p| {
                let diff = (*p - phase_mean + std::f64::consts::PI)
                    .rem_euclid(2.0 * std::f64::consts::PI) - std::f64::consts::PI;
                diff * diff
            })
            .sum::<f64>() / phases.len() as f64;

        // 3. 信号质量评估 (基于 RSSI)
        let signal_quality = {
            let rssi_norm = ((meta.rssi as f64 + 90.0) / 60.0).clamp(0.0, 1.0); // -90..-30 → 0..1
            let var_norm = (amp_variance / 10.0).clamp(0.0, 1.0);
            (rssi_norm * 0.6 + var_norm * 0.4).clamp(0.0, 1.0)
        };

        if signal_quality < self.config.min_signal_quality {
            return false;
        }

        // 4. 运动强度评分
        let motion_score = (phase_variance / 0.5).clamp(0.0, 1.0);

        // 5. 简易呼吸率估计 (从相位低频分量)
        let breathing_rate = if amplitudes.len() >= 10 {
            let sample = &phases[..amplitudes.len().min(128)];
            estimate_breathing_rate(sample, meta.freq_mhz as f64)
        } else {
            None
        };

        // 6. 简易心率估计 (从振幅微变)
        let heart_rate = if amplitudes.len() >= 20 {
            let sample = &amplitudes[..amplitudes.len().min(128)];
            estimate_heart_rate(sample)
        } else {
            None
        };

        let vitals = ExtractedVitals {
            breathing_rate,
            heart_rate,
            motion_score: motion_score as f32,
            signal_quality: signal_quality as f32,
            amplitude_mean: amp_mean,
            phase_variance,
        };

        // 7. 伤员匹配/创建 (基于节点+位置的简易聚类)
        let survivor_id = self.match_or_create_survivor(meta, motion_score);
        let survivor = self.survivors.get_mut(&survivor_id).unwrap();

        // 8. 更新生命体征历史
        if let Some(br) = vitals.breathing_rate {
            survivor.breathing_history.push(br);
            if survivor.breathing_history.len() > 30 {
                survivor.breathing_history.remove(0);
            }
        }
        if let Some(hr) = vitals.heart_rate {
            survivor.heart_rate_history.push(hr);
            if survivor.heart_rate_history.len() > 30 {
                survivor.heart_rate_history.remove(0);
            }
        }

        // 9. 平滑生命体征
        let smoothed_vitals = ExtractedVitals {
            breathing_rate: smooth(&survivor.breathing_history),
            heart_rate: smooth(&survivor.heart_rate_history),
            ..vitals
        };

        // 10. 计算 START 分诊
        let triage = calculate_triage(&smoothed_vitals);
        let prev_triage = survivor.triage;
        survivor.triage = triage;
        survivor.vitals = smoothed_vitals;
        survivor.last_updated = current_epoch_secs();

        // 11. 恶化检测
        if triage.priority() < prev_triage.priority() {
            survivor.deterioration_count += 1;
            // 连续 3 帧恶化 → 告警
            if survivor.deterioration_count >= 3 {
                survivor.deterioration_count = 0;
                self.alerts.push(AlertSnapshot {
                    time: chrono_now(),
                    survivor_id: survivor_id.clone(),
                    alert_type: "TRIAGE_DETERIORATION".to_string(),
                    message: format!(
                        "伤员 {} 分诊等级恶化: {} → {}",
                        &survivor_id[..8],
                        prev_triage.name(),
                        triage.name()
                    ),
                    priority: triage.priority(),
                });
            }
        } else {
            survivor.deterioration_count = 0;
        }

        // 12. 更新位置 (简易三角测量)
        if let Some(node_pos) = self.config.node_positions.get(&meta.node_id) {
            let rssi_distance = estimate_distance(meta.rssi);
            survivor.position = (
                node_pos.0 + rssi_distance * 0.5,
                node_pos.1 + rssi_distance * 0.3,
                0.5, // 假设伤员在地面
            );
            survivor.position_confidence = signal_quality;
        }

        // 13. 生成 UI 更新
        let survivors = self.build_survivor_snapshots();
        let assessment = self.build_assessment();
        let alerts = self.alerts.clone();

        self.latest_update = Some(TriageUpdate {
            event_type: "survivor_update".to_string(),
            survivors,
            assessment: Some(assessment),
            alerts,
        });

        true
    }

    /// 获取最新的 UI 更新
    pub fn get_updates(&self) -> Option<&TriageUpdate> {
        self.latest_update.as_ref()
    }

    /// 获取 JSON 序列化的更新
    pub fn get_updates_json(&self) -> Option<String> {
        self.latest_update.as_ref()
            .and_then(|u| serde_json::to_string(u).ok())
    }

    /// 伤员匹配: 基于节点 ID 和运动特征的简易聚类
    fn match_or_create_survivor(&mut self, meta: &CsiFrameMeta, motion: f64) -> String {
        let now = current_epoch_secs();

        // 清理超时的伤员 (超过 timeout 秒无更新)
        self.survivors.retain(|_, s| {
            (now - s.last_updated) < self.config.survivor_timeout_secs
        });

        // 尝试匹配现有伤员 (如果同一节点最近检测到高运动 → 可能是同一人)
        for (id, s) in &self.survivors {
            if s.last_updated > now - 3.0 && motion > 0.3 {
                return id.clone();
            }
        }

        // 新建伤员
        self.survivor_counter += 1;
        let id = format!("SURV-{:04x}", self.survivor_counter);
        self.survivors.insert(id.clone(), TrackedSurvivor {
            id: id.clone(),
            breathing_history: Vec::new(),
            heart_rate_history: Vec::new(),
            position: (0.0, 0.0, 0.0),
            position_confidence: 0.0,
            vitals: ExtractedVitals::default(),
            triage: TriageLevel::Unknown,
            first_seen: now,
            last_updated: now,
            deterioration_count: 0,
        });

        id
    }

    fn build_survivor_snapshots(&self) -> Vec<SurvivorSnapshot> {
        self.survivors.iter().map(|(id, s)| {
            SurvivorSnapshot {
                id: id.clone(),
                position: Some(PositionSnapshot {
                    x: s.position.0,
                    y: s.position.1,
                    z: s.position.2,
                    confidence: s.position_confidence,
                }),
                vitals: VitalsSnapshot {
                    breathing_rate: s.vitals.breathing_rate,
                    heart_rate: s.vitals.heart_rate,
                    motion_score: s.vitals.motion_score,
                },
                triage: s.triage.name().to_string(),
                triage_color: s.triage.color().to_string(),
                is_deteriorating: s.deterioration_count > 0,
                tracked_seconds: s.last_updated - s.first_seen,
            }
        }).collect()
    }

    fn build_assessment(&self) -> MassCasualtySnapshot {
        let mut immediate = 0u32;
        let mut delayed = 0u32;
        let mut minor = 0u32;
        let mut deceased = 0u32;
        let mut unknown = 0u32;

        for s in self.survivors.values() {
            match s.triage {
                TriageLevel::Immediate => immediate += 1,
                TriageLevel::Delayed => delayed += 1,
                TriageLevel::Minor => minor += 1,
                TriageLevel::Deceased => deceased += 1,
                TriageLevel::Unknown => unknown += 1,
            }
        }

        let total = immediate + delayed + minor + deceased + unknown;
        let rescuers = immediate * 4 + delayed * 2 + minor / 2;

        let severity = if total == 0 {
            "Minimal"
        } else if immediate >= 3 || (immediate + delayed) as f64 / total as f64 > 0.5 {
            "Critical"
        } else if immediate >= 1 {
            "Major"
        } else if delayed >= 1 {
            "Moderate"
        } else {
            "Minimal"
        };

        MassCasualtySnapshot {
            total,
            immediate,
            delayed,
            minor,
            deceased,
            unknown,
            severity: severity.to_string(),
            rescuers_needed: rescuers,
        }
    }
}

// ── 信号处理辅助 ──────────────────────────────────────────────────────────────

/// 简易呼吸率估计 (相位低频分量过零检测)
fn estimate_breathing_rate(phases: &[f64], _freq_mhz: f64) -> Option<f32> {
    if phases.len() < 10 {
        return None;
    }

    // 一阶差分 → 检测过零点
    let mut zero_crossings = 0u32;
    for i in 1..phases.len() {
        if phases[i - 1].signum() != phases[i].signum()
            && (phases[i] - phases[i - 1]).abs() < 1.0
        {
            zero_crossings += 1;
        }
    }

    if zero_crossings == 0 {
        return None;
    }

    // 过零频率 → 呼吸率 (假设采样率为 ~20Hz)
    let breathing_hz = zero_crossings as f64 / (2.0 * phases.len() as f64 / 20.0);
    let breathing_bpm = breathing_hz * 60.0;

    if breathing_bpm >= 6.0 && breathing_bpm <= 40.0 {
        Some(breathing_bpm as f32)
    } else {
        None
    }
}

/// 简易心率估计 (振幅微小波动的频谱峰值)
fn estimate_heart_rate(amplitudes: &[f64]) -> Option<f32> {
    if amplitudes.len() < 20 {
        return None;
    }

    // 去趋势 + 归一化
    let mean = amplitudes.iter().sum::<f64>() / amplitudes.len() as f64;
    let detrended: Vec<f64> = amplitudes.iter().map(|a| a - mean).collect();

    // 寻找峰值间隔 (时域简易估计)
    let mut peak_intervals = Vec::new();
    let mut last_peak = 0usize;
    for i in 2..detrended.len().saturating_sub(2) {
        if detrended[i] > detrended[i - 1] && detrended[i] > detrended[i + 1]
            && detrended[i] > 0.0
        {
            if last_peak > 0 {
                peak_intervals.push(i - last_peak);
            }
            last_peak = i;
        }
    }

    if peak_intervals.is_empty() {
        return None;
    }

    let avg_interval = peak_intervals.iter().sum::<usize>() as f64 / peak_intervals.len() as f64;
    // 假设 ~50Hz 等效采样 (CSI 帧率约 20-50Hz)
    let hr_bpm = 50.0 * 60.0 / avg_interval;

    if hr_bpm >= 40.0 && hr_bpm <= 150.0 {
        Some(hr_bpm as f32)
    } else {
        None
    }
}

/// 距离估计 (基于 RSSI 的对数路径损耗模型)
fn estimate_distance(rssi: i8) -> f64 {
    let rssi = rssi as f64;
    let ref_rssi = -30.0; // 1 米参考 RSSI
    let path_loss_exp = 3.0; // 室内穿墙环境
    let ref_dist = 1.0;

    ref_dist * 10.0_f64.powf((ref_rssi - rssi) / (10.0 * path_loss_exp))
}

/// 滑动窗口平滑
fn smooth(values: &[f32]) -> Option<f32> {
    if values.is_empty() {
        return None;
    }
    if values.len() <= 3 {
        return Some(values.iter().sum::<f32>() / values.len() as f32);
    }
    // 最近 5 个值的加权平均
    let window: Vec<&f32> = values.iter().rev().take(5).collect();
    let weights: [f32; 5] = [0.4, 0.25, 0.15, 0.1, 0.1];
    let sum: f32 = window.iter().enumerate()
        .map(|(i, &v)| v * weights[i.min(4)])
        .sum();
    let weight_sum: f32 = weights.iter().take(window.len()).sum();
    Some(sum / weight_sum)
}

fn current_epoch_secs() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}

fn chrono_now() -> String {
    chrono::Local::now().format("%H:%M:%S").to_string()
}

// ── 测试 ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_triage_immediate_high_respiration() {
        let vitals = ExtractedVitals {
            breathing_rate: Some(35.0),
            heart_rate: Some(80.0),
            ..Default::default()
        };
        assert_eq!(calculate_triage(&vitals), TriageLevel::Immediate);
    }

    #[test]
    fn test_triage_immediate_low_heart_rate() {
        let vitals = ExtractedVitals {
            breathing_rate: Some(15.0),
            heart_rate: Some(35.0),
            ..Default::default()
        };
        assert_eq!(calculate_triage(&vitals), TriageLevel::Immediate);
    }

    #[test]
    fn test_triage_minor_normal() {
        let vitals = ExtractedVitals {
            breathing_rate: Some(16.0),
            heart_rate: Some(70.0),
            motion_score: 0.5,
            ..Default::default()
        };
        assert_eq!(calculate_triage(&vitals), TriageLevel::Minor);
    }

    #[test]
    fn test_pipeline_creates_survivor() {
        let mut pipeline = MatPipeline::new(MatConfig::competition());
        let amps = vec![1.0; 52];
        let phases = vec![0.1; 52];
        let meta = CsiFrameMeta {
            node_id: 1, n_subcarriers: 52,
            freq_mhz: 2437, rssi: -50, noise_floor: -90, sequence: 0,
        };
        assert!(pipeline.process_frame(&amps, &phases, &meta));
        let update = pipeline.get_updates().unwrap();
        assert_eq!(update.survivors.len(), 1);
    }
}
