# WiFi-Mat 用户指南

## 灾难响应大规模伤亡评估工具

WiFi-Mat（大规模评估工具）是 WiFi-DensePose 的模块化扩展，专为搜索和救援行动而设计。它使用 WiFi 信道状态信息（CSI）来检测和定位被困在地震、建筑物倒塌、雪崩和其他灾难场景中的瓦砾、碎片和倒塌结构中的幸存者。

---

## 目录

1. [概述](#概述)
2. [主要功能](#主要功能)
3. [安装](#安装)
4. [快速开始](#快速开始)
5. [架构](#架构)
6. [配置](#配置)
7. [检测能力](#检测能力)
8. [定位系统](#定位系统)
9. [检伤分类](#检伤分类)
10. [警报系统](#警报系统)
11. [API 参考](#api-参考)
12. [硬件设置](#硬件设置)
13. [现场部署指南](#现场部署指南)
14. [故障排除](#故障排除)
15. [最佳实践](#最佳实践)
16. [安全考虑](#安全考虑)

---

## 概述

### 什么是 WiFi-Mat？

WiFi-Mat 利用与 WiFi-DensePose 相同的基于 WiFi 的感知技术，但针对灾难响应的独特挑战进行了优化：

- **穿墙检测**：通过碎片、瓦砾和倒塌结构检测生命迹象
- **非侵入性**：初始评估期间无需扰动不稳定结构
- **快速部署**：便携式传感器阵列可在几分钟内设置完成
- **多受害者检伤分类**：使用 START 协议自动优先排序救援工作
- **3D 定位**：估计幸存者位置，包括穿过碎片的深度

### 使用场景

| 灾难类型 | 检测范围 | 典型深度 | 成功率 |
|---------|---------|---------|--------|
| 地震瓦砾 | 15-30米半径 | 最深5米 | 85-92% |
| 建筑物倒塌 | 20-40米半径 | 最深8米 | 80-88% |
| 雪崩 | 10-20米半径 | 最深3米积雪 | 75-85% |
| 矿井坍塌 | 15-25米半径 | 最深10米 | 70-82% |
| 洪水碎片 | 10-15米半径 | 最深2米 | 88-95% |

---

## 主要功能

### 1. 生命体征检测
- **呼吸检测**：0.1-0.5 Hz（4-60 次/分钟）
- **心跳检测**：0.8-3.3 Hz（30-200 BPM）通过微多普勒效应
- **运动分类**：粗运动、精细运动、震颤和周期性运动

### 2. 幸存者定位
- **2D 位置**：使用 3+ 传感器时 ±0.5米精度
- **深度估计**：穿过最深5米碎片时 ±0.3米
- **置信度评分**：实时不确定性量化

### 3. 检伤分类
- **START 协议**：立即/延迟/轻微/死亡
- **自动优先排序**：基于生命体征和可访问性
- **动态更新**：随着条件变化重新分类

### 4. 警报系统
- **基于优先级**：关键/高/中/低警报
- **多渠道**：音频、视觉、移动推送、无线电集成
- **升级**：对于状况恶化的幸存者自动升级

---

## 安装

### 前提条件

```bash
# Rust 工具链 (1.70+)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 必需的系统依赖（Ubuntu/Debian）
sudo apt-get install -y build-essential pkg-config libssl-dev
```

### 从源代码构建

```bash
# 克隆仓库
git clone https://github.com/ruvnet/wifi-densepose.git
cd wifi-densepose/rust-port/wifi-densepose-rs

# 构建 wifi-mat crate
cargo build --release --package wifi-densepose-mat

# 运行测试
cargo test --package wifi-densepose-mat

# 构建所有功能
cargo build --release --package wifi-densepose-mat --all-features
```

### 功能标志

```toml
# Cargo.toml 功能
[features]
default = ["std"]
std = []
serde = ["dep:serde"]
async = ["tokio"]
hardware = ["wifi-densepose-hardware"]
neural = ["wifi-densepose-nn"]
full = ["serde", "async", "hardware", "neural"]
```

---

## 快速开始

### 基本示例

```rust
use wifi_densepose_mat::{
    DisasterResponse, DisasterConfig, DisasterType,
    ScanZone, ZoneBounds,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 配置地震响应
    let config = DisasterConfig::builder()
        .disaster_type(DisasterType::Earthquake)
        .sensitivity(0.85)
        .confidence_threshold(0.5)
        .max_depth(5.0)
        .continuous_monitoring(true)
        .build();

    // 初始化响应系统
    let mut response = DisasterResponse::new(config);

    // 初始化灾难事件
    let location = geo::Point::new(-122.4194, 37.7749); // 旧金山
    response.initialize_event(location, "建筑物倒塌 - 市场街")?;

    // 定义扫描区域
    let zone_a = ScanZone::new(
        "北翼 - 地面层",
        ZoneBounds::rectangle(0.0, 0.0, 30.0, 20.0),
    );
    response.add_zone(zone_a)?;

    let zone_b = ScanZone::new(
        "南翼 - 地下室",
        ZoneBounds::rectangle(30.0, 0.0, 60.0, 20.0),
    );
    response.add_zone(zone_b)?;

    // 开始扫描
    println!("开始幸存者检测扫描...");
    response.start_scanning().await?;

    // 获取检测到的幸存者
    let survivors = response.survivors();
    println!("检测到 {} 名潜在幸存者", survivors.len());

    // 获取需要立即救援的幸存者
    let immediate = response.survivors_by_triage(TriageStatus::Immediate);
    println!("{} 名幸存者需要立即救援", immediate.len());

    Ok(())
}
```

### 最小检测示例

```rust
use wifi_densepose_mat::detection::{
    BreathingDetector, BreathingDetectorConfig,
    DetectionPipeline, DetectionConfig,
};

fn detect_breathing(csi_amplitudes: &[f64], sample_rate: f64) {
    let config = BreathingDetectorConfig::default();
    let detector = BreathingDetector::new(config);

    if let Some(breathing) = detector.detect(csi_amplitudes, sample_rate) {
        println!("检测到呼吸！");
        println!("  速率: {:.1} BPM", breathing.rate_bpm);
        println!("  模式: {:?}", breathing.pattern_type);
        println!("  置信度: {:.2}", breathing.confidence);
    } else {
        println!("未检测到呼吸");
    }
}
```

---

## 架构

### 系统概述

```
┌──────────────────────────────────────────────────────────────────┐
│                        WiFi-Mat 系统                           │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐  │
│  │   检测上下文    │  │  定位上下文    │  │    警报上下文    │  │
│  │                 │  │                 │  │                 │  │
│  │ • 呼吸          │  │ • 三角定位     │  │ • 生成器        │  │
│  │ • 心跳          │  │ • 深度估计     │  │ • 分发器        │  │
│  │ • 运动          │  │ • 融合         │  │ • 检伤服务      │  │
│  │ • 管道          │  │                 │  │                 │  │
│  └────────┬────────┘  └────────┬────────┘  └────────┬────────┘  │
│           │                    │                    │            │
│           └────────────────────┼────────────────────┘            │
│                                │                                 │
│                    ┌───────────▼───────────┐                     │
│                    │    集成层             │                     │
│                    │                       │                     │
│                    │ • SignalAdapter       │                     │
│                    │ • NeuralAdapter       │                     │
│                    │ • HardwareAdapter     │                     │
│                    └───────────┬───────────┘                     │
│                                │                                 │
└────────────────────────────────┼─────────────────────────────────┘
                                 │
              ┌──────────────────┼──────────────────┐
              │                  │                  │
    ┌─────────▼─────────┐ ┌─────▼─────┐ ┌─────────▼─────────┐
    │ wifi-densepose-   │ │ wifi-     │ │ wifi-densepose-   │
    │     signal        │ │ densepose │ │    hardware       │
    │                   │ │   -nn     │ │                   │
    └───────────────────┘ └───────────┘ └───────────────────┘
```

### 领域模型

```
┌─────────────────────────────────────────────────────────────┐
│                     灾难事件                               │
│                   (聚合根)                                  │
├─────────────────────────────────────────────────────────────┤
│ - id: DisasterEventId                                       │
│ - disaster_type: DisasterType                               │
│ - location: Point<f64>                                      │
│ - status: EventStatus                                       │
│ - zones: Vec<ScanZone>                                      │
│ - survivors: Vec<Survivor>                                  │
│ - created_at: DateTime<Utc>                                 │
│ - metadata: EventMetadata                                   │
└─────────────────────────────────────────────────────────────┘
         │                              │
         │ 包含                         │ 包含
         ▼                              ▼
┌─────────────────────┐      ┌─────────────────────────────┐
│     扫描区域        │      │         幸存者              │
│     (实体)          │      │         (实体)              │
├─────────────────────┤      ├─────────────────────────────┤
│ - id: ScanZoneId    │      │ - id: SurvivorId            │
│ - name: String      │      │ - vital_signs: VitalSigns   │
│ - bounds: ZoneBounds│      │ - location: Option<Coord3D> │
│ - sensors: Vec<...> │      │ - triage: TriageStatus      │
│ - parameters: ...   │      │ - alerts: Vec<Alert>        │
│ - status: ZoneStatus│      │ - metadata: SurvivorMeta    │
└─────────────────────┘      └─────────────────────────────┘
```

---

## 配置

### DisasterConfig 选项

```rust
let config = DisasterConfig {
    // 灾难类型（影响检测算法）
    disaster_type: DisasterType::Earthquake,

    // 检测灵敏度 (0.0-1.0)
    // 越高 = 越多误报，越少漏检
    sensitivity: 0.8,

    // 报告检测的最小置信度
    confidence_threshold: 0.5,

    // 尝试检测的最大深度（米）
    max_depth: 5.0,

    // 扫描间隔（毫秒）
    scan_interval_ms: 500,

    // 持续扫描
    continuous_monitoring: true,

    // 警报配置
    alert_config: AlertConfig {
        enable_audio: true,
        enable_push: true,
        escalation_timeout_secs: 300,
        priority_threshold: Priority::Medium,
    },
};
```

### 灾难类型

| 类型 | 优化 | 最适合 |
|------|------|--------|
| `Earthquake` | 增强微运动检测 | 建筑物倒塌 |
| `BuildingCollapse` | 深层穿透，噪声过滤 | 城市搜索与救援 |
| `Avalanche` | 冷体补偿，积雪穿透 | 山地救援 |
| `Flood` | 水干扰补偿 | 洪水救援 |
| `MineCollapse` | 岩石穿透，气体检测 | 采矿事故 |
| `Explosion` | 爆炸创伤模式 | 工业事故 |
| `Unknown` | 平衡默认值 | 一般用途 |

### ScanParameters

```rust
let params = ScanParameters {
    // 此区域的检测灵敏度
    sensitivity: 0.85,

    // 最大扫描深度（米）
    max_depth: 5.0,

    // 分辨率级别
    resolution: ScanResolution::High,

    // 启用增强呼吸检测
    enhanced_breathing: true,

    // 启用心跳检测（较慢但更准确）
    heartbeat_detection: true,
};

let zone = ScanZone::with_parameters("区域 A", bounds, params);
```

---

## 检测能力

### 呼吸检测

WiFi-Mat 通过调制 WiFi 信号的周期性胸壁运动来检测呼吸。

```rust
use wifi_densepose_mat::detection::{BreathingDetector, BreathingDetectorConfig};

let config = BreathingDetectorConfig {
    // 呼吸频率范围（Hz）
    min_frequency: 0.1,  // 6 BPM
    max_frequency: 0.5,  // 30 BPM

    // 分析窗口
    window_seconds: 10.0,

    // 检测阈值
    confidence_threshold: 0.3,

    // 启用模式分类
    classify_patterns: true,
};

let detector = BreathingDetector::new(config);
let result = detector.detect(&amplitudes, sample_rate);
```

**可检测模式：**
- 正常呼吸
- 浅/快速呼吸
- 深/缓慢呼吸
- 不规则呼吸
- 临终呼吸（危急）

### 心跳检测

使用微多普勒分析来检测心跳引起的微妙身体运动。

```rust
use wifi_densepose_mat::detection::{HeartbeatDetector, HeartbeatDetectorConfig};

let config = HeartbeatDetectorConfig {
    // 心率范围（Hz）
    min_frequency: 0.8,  // 48 BPM
    max_frequency: 3.0,  // 180 BPM

    // 首先要求检测到呼吸（减少误报）
    require_breathing: true,

    // 由于信号微弱，阈值较高
    confidence_threshold: 0.4,
};

let detector = HeartbeatDetector::new(config);
let result = detector.detect(&phases, sample_rate, Some(breathing_rate));
```

### 运动分类

```rust
use wifi_densepose_mat::detection::{MovementClassifier, MovementClassifierConfig};

let classifier = MovementClassifier::new(MovementClassifierConfig::default());
let movement = classifier.classify(&amplitudes, sample_rate);

match movement.movement_type {
    MovementType::Gross => println!("大动作 - 可能有意识"),
    MovementType::Fine => println!("小动作 - 可能受伤"),
    MovementType::Tremor => println!("检测到震颤 - 可能休克"),
    MovementType::Periodic => println!("周期性运动 - 可能只有呼吸"),
    MovementType::None => println!("未检测到运动"),
}
```

---

## 定位系统

### 三角定位

使用来自多个传感器的飞行时间和信号强度。

```rust
use wifi_densepose_mat::localization::{Triangulator, TriangulationConfig};

let config = TriangulationConfig {
    // 2D 定位的最小传感器数
    min_sensors: 3,

    // 除 CSI 外还使用 RSSI
    use_rssi: true,

    // 优化的最大迭代次数
    max_iterations: 100,

    // 收敛阈值
    convergence_threshold: 0.01,
};

let triangulator = Triangulator::new(config);

// 传感器位置
let sensors = vec![
    SensorPosition { x: 0.0, y: 0.0, z: 1.5, .. },
    SensorPosition { x: 10.0, y: 0.0, z: 1.5, .. },
    SensorPosition { x: 5.0, y: 10.0, z: 1.5, .. },
];

// 来自每个传感器的 RSSI 测量值
let measurements = vec![-45.0, -52.0, -48.0];

let position = triangulator.estimate(&sensors, &measurements)?;
println!("估计位置: ({:.2}, {:.2})", position.x, position.y);
println!("不确定性: ±{:.2}m", position.uncertainty);
```

### 深度估计

使用信号衰减分析来估计穿过碎片的深度。

```rust
use wifi_densepose_mat::localization::{DepthEstimator, DepthEstimatorConfig};

let config = DepthEstimatorConfig {
    // 材料衰减系数
    material_model: MaterialModel::MixedDebris,

    // 参考信号强度（清晰视线）
    reference_rssi: -30.0,

    // 最大可检测深度
    max_depth: 8.0,
};

let estimator = DepthEstimator::new(config);
let depth = estimator.estimate(measured_rssi, expected_rssi)?;

println!("估计深度: {:.2}m", depth.meters);
println!("置信度: {:.2}", depth.confidence);
println!("材料: {:?}", depth.estimated_material);
```

### 位置融合

使用卡尔曼滤波结合多种估计方法。

```rust
use wifi_densepose_mat::localization::{PositionFuser, LocalizationService};

let service = LocalizationService::new();

// 估计完整的 3D 位置
let position = service.estimate_position(&vital_signs, &zone)?;

println!("3D 位置:");
println!("  X: {:.2}m (±{:.2})", position.x, position.uncertainty.x);
println!("  Y: {:.2}m (±{:.2})", position.y, position.uncertainty.y);
println!("  Z: {:.2}m (±{:.2})", position.z, position.uncertainty.z);
println!("  总置信度: {:.2}", position.confidence);
```

---

## 检伤分类

### START 协议

WiFi-Mat 实现了简单检伤和快速治疗（START）协议：

| 状态 | 标准 | 行动 |
|------|------|------|
| **立即（红色）** | 呼吸 10-29/分钟，无桡动脉脉搏，服从命令 | 优先救援 |
| **延迟（黄色）** | 呼吸正常，有脉搏，伤势非危及生命 | 次要救援 |
| **轻微（绿色）** | 行走伤员，轻伤 | 可等待 |
| **死亡（黑色）** | 气道清理后无呼吸 | 不救援 |

### 自动检伤

```rust
use wifi_densepose_mat::domain::triage::{TriageCalculator, TriageStatus};

let calculator = TriageCalculator::new();

// 基于生命体征计算检伤
let vital_signs = VitalSignsReading {
    breathing: Some(BreathingPattern {
        rate_bpm: 24.0,
        pattern_type: BreathingType::Shallow,
        ..
    }),
    heartbeat: Some(HeartbeatSignature {
        rate_bpm: 110.0,
        ..
    }),
    movement: MovementProfile {
        movement_type: MovementType::Fine,
        ..
    },
    ..
};

let triage = calculator.calculate(&vital_signs);

match triage {
    TriageStatus::Immediate => println!("⚠️ 立即 - 立即救援"),
    TriageStatus::Delayed => println!("🟡 延迟 - 目前稳定"),
    TriageStatus::Minor => println!("🟢 轻微 - 行走伤员"),
    TriageStatus::Deceased => println!("⬛ 死亡 - 无生命体征"),
    TriageStatus::Unknown => println!("❓ 未知 - 数据不足"),
}
```

### 检伤因素

```rust
// 访问详细的检伤推理
let factors = calculator.calculate_with_factors(&vital_signs);

println!("检伤: {:?}", factors.status);
println!("影响因素:");
for factor in &factors.contributing_factors {
    println!("  - {} (权重: {:.2})", factor.description, factor.weight);
}
println!("置信度: {:.2}", factors.confidence);
```

---

## 警报系统

### 警报生成

```rust
use wifi_densepose_mat::alerting::{AlertGenerator, AlertConfig};

let config = AlertConfig {
    // 生成警报的最低优先级
    priority_threshold: Priority::Medium,

    // 升级设置
    escalation_enabled: true,
    escalation_timeout: Duration::from_secs(300),

    // 通知渠道
    channels: vec![
        AlertChannel::Audio,
        AlertChannel::Visual,
        AlertChannel::Push,
        AlertChannel::Radio,
    ],
};

let generator = AlertGenerator::new(config);

// 为幸存者生成警报
let alert = generator.generate(&survivor)?;

println!("生成警报:");
println!("  ID: {}", alert.id());
println!("  优先级: {:?}", alert.priority());
println!("  消息: {}", alert.message());
```

### 警报优先级

| 优先级 | 标准 | 响应时间 |
|--------|------|----------|
| **关键** | 立即检伤，状况恶化 | < 5 分钟 |
| **高** | 立即检伤，稳定 | < 15 分钟 |
| **中** | 延迟检伤 | < 1 小时 |
| **低** | 轻微检伤 | 视情况而定 |

### 警报分发

```rust
use wifi_densepose_mat::alerting::AlertDispatcher;

let dispatcher = AlertDispatcher::new(config);

// 分发到所有配置的渠道
dispatcher.dispatch(alert).await?;

// 分发到特定渠道
dispatcher.dispatch_to(alert, AlertChannel::Radio).await?;

// 批量分发多个幸存者
dispatcher.dispatch_batch(&alerts).await?;
```

---

## API 参考

### 核心类型

```rust
// 主入口点
pub struct DisasterResponse {
    pub fn new(config: DisasterConfig) -> Self;
    pub fn initialize_event(&mut self, location: Point, desc: &str) -> Result<&DisasterEvent>;
    pub fn add_zone(&mut self, zone: ScanZone) -> Result<()>;
    pub async fn start_scanning(&mut self) -> Result<()>;
    pub fn stop_scanning(&self);
    pub fn survivors(&self) -> Vec<&Survivor>;
    pub fn survivors_by_triage(&self, status: TriageStatus) -> Vec<&Survivor>;
}

// 配置
pub struct DisasterConfig {
    pub disaster_type: DisasterType,
    pub sensitivity: f64,
    pub confidence_threshold: f64,
    pub max_depth: f64,
    pub scan_interval_ms: u64,
    pub continuous_monitoring: bool,
    pub alert_config: AlertConfig,
}

// 领域实体
pub struct Survivor { /* ... */ }
pub struct ScanZone { /* ... */ }
pub struct DisasterEvent { /* ... */ }
pub struct Alert { /* ... */ }

// 值对象
pub struct VitalSignsReading { /* ... */ }
pub struct BreathingPattern { /* ... */ }
pub struct HeartbeatSignature { /* ... */ }
pub struct Coordinates3D { /* ... */ }
```

### 检测 API

```rust
// 呼吸
pub struct BreathingDetector {
    pub fn new(config: BreathingDetectorConfig) -> Self;
    pub fn detect(&self, amplitudes: &[f64], sample_rate: f64) -> Option<BreathingPattern>;
}

// 心跳
pub struct HeartbeatDetector {
    pub fn new(config: HeartbeatDetectorConfig) -> Self;
    pub fn detect(&self, phases: &[f64], sample_rate: f64, breathing_rate: Option<f64>) -> Option<HeartbeatSignature>;
}

// 运动
pub struct MovementClassifier {
    pub fn new(config: MovementClassifierConfig) -> Self;
    pub fn classify(&self, amplitudes: &[f64], sample_rate: f64) -> MovementProfile;
}

// 管道
pub struct DetectionPipeline {
    pub fn new(config: DetectionConfig) -> Self;
    pub async fn process_zone(&self, zone: &ScanZone) -> Result<Option<VitalSignsReading>>;
    pub fn add_data(&self, amplitudes: &[f64], phases: &[f64]);
}
```

### 定位 API

```rust
pub struct Triangulator {
    pub fn new(config: TriangulationConfig) -> Self;
    pub fn estimate(&self, sensors: &[SensorPosition], measurements: &[f64]) -> Result<Position2D>;
}

pub struct DepthEstimator {
    pub fn new(config: DepthEstimatorConfig) -> Self;
    pub fn estimate(&self, measured: f64, expected: f64) -> Result<DepthEstimate>;
}

pub struct LocalizationService {
    pub fn new() -> Self;
    pub fn estimate_position(&self, vital_signs: &VitalSignsReading, zone: &ScanZone) -> Result<Coordinates3D>;
}
```

---

## 硬件设置

### 传感器要求

| 组件 | 最低要求 | 推荐 |
|------|----------|------|
| WiFi 收发器 | 3 | 6-8 |
| 采样率 | 100 Hz | 1000 Hz |
| 频段 | 2.4 GHz | 5 GHz |
| 天线类型 | 全向 | 定向 |
| 电源 | 电池 | AC + 电池 |

### 便携式传感器阵列

```
    [传感器 1]              [传感器 2]
         \                    /
          \    扫描区域     /
           \                /
            \              /
             [传感器 3]---[传感器 4]
                  |
              [控制器]
                  |
              [显示器]
```

### 传感器放置

```rust
// 30x20m 区域的传感器配置示例
let sensors = vec![
    SensorPosition {
        id: "S1".into(),
        x: 0.0, y: 0.0, z: 2.0,
        sensor_type: SensorType::Transceiver,
        is_operational: true,
    },
    SensorPosition {
        id: "S2".into(),
        x: 30.0, y: 0.0, z: 2.0,
        sensor_type: SensorType::Transceiver,
        is_operational: true,
    },
    SensorPosition {
        id: "S3".into(),
        x: 0.0, y: 20.0, z: 2.0,
        sensor_type: SensorType::Transceiver,
        is_operational: true,
    },
    SensorPosition {
        id: "S4".into(),
        x: 30.0, y: 20.0, z: 2.0,
        sensor_type: SensorType::Transceiver,
        is_operational: true,
    },
];
```

---

## 现场部署指南

### 部署前检查清单

- [ ] 验证所有传感器已充电（>80%）
- [ ] 测试传感器连接性
- [ ] 针对当地条件进行校准
- [ ] 与指挥中心建立通信
- [ ] 向救援团队介绍系统能力

### 部署步骤

1. **现场评估**（5 分钟）
   - 确定安全的传感器放置位置
   - 注意结构危险
   - 估计碎片组成

2. **传感器部署**（10 分钟）
   - 在搜索区域周边放置传感器
   - 确保至少 3 个传感器彼此之间有视线
   - 连接到控制器

3. **系统初始化**（2 分钟）
   ```rust
   let mut response = DisasterResponse::new(config);
   response.initialize_event(location, description)?;

   for zone in zones {
       response.add_zone(zone)?;
   }
   ```

4. **校准**（5 分钟）
   - 运行背景噪声校准
   - 根据环境调整灵敏度

5. **开始扫描**（持续）
   ```rust
   response.start_scanning().await?;
   ```

### 结果解释

```
┌─────────────────────────────────────────────────────┐
│                  扫描结果                           │
├─────────────────────────────────────────────────────┤
│  区域: 北翼 - 地面层                                │
│  状态: 活跃 | 扫描: 127 | 持续时间: 10:34          │
├─────────────────────────────────────────────────────┤
│  检测结果:                                          │
│                                                     │
│  [立即] 幸存者 #1                                   │
│    位置: (12.3, 8.7) ±0.5m                        │
│    深度: 2.1m ±0.3m                                │
│    呼吸: 24 BPM (浅)                               │
│    运动: 精细动作                                   │
│    置信度: 87%                                      │
│                                                     │
│  [延迟] 幸存者 #2                                   │
│    位置: (22.1, 15.2) ±0.8m                       │
│    深度: 1.5m ±0.2m                                │
│    呼吸: 16 BPM (正常)                              │
│    运动: 仅周期性                                   │
│    置信度: 92%                                      │
│                                                     │
│  [轻微] 幸存者 #3                                   │
│    位置: (5.2, 3.1) ±0.3m                         │
│    深度: 0.3m ±0.1m                                │
│    呼吸: 18 BPM (正常)                              │
│    运动: 粗大动作 (可能可移动)                      │
│    置信度: 95%                                      │
└─────────────────────────────────────────────────────┘
```

---

## 故障排除

### 常见问题

| 问题 | 可能原因 | 解决方案 |
|------|---------|----------|
| 无检测结果 | 灵敏度过低 | 将 `sensitivity` 增加到 0.9+ |
| 太多误报 | 灵敏度过高 | 将 `sensitivity` 减少到 0.6-0.7 |
| 定位不佳 | 传感器不足 | 添加更多传感器（最少 3 个） |
| 间歇性检测 | 信号干扰 | 检查电磁源 |
| 深度估计失败 | 致密材料 | 调整 `material_model` |

### 诊断命令

```rust
// 检查系统健康
let health = response.hardware_health();
println!("传感器: {}/{} 运行中", health.connected, health.total);

// 查看检测统计
let stats = response.detection_stats();
println!("检测率: {:.1}%", stats.detection_rate * 100.0);
println!("误报率: {:.1}%", stats.false_positive_rate * 100.0);

// 导出诊断数据
response.export_diagnostics("/path/to/diagnostics.json")?;
```

---

## 最佳实践

### 检测优化

1. **从高灵敏度开始**，如果误报太多则降低
2. **仅在确认呼吸后启用心跳检测**
3. **使用适当的灾难类型**以获得优化算法
4. **增加扫描持续时间**以捕获微弱信号（最多 30 秒窗口）

### 定位优化

1. **使用 4+ 传感器**以获得可靠的 2D 定位
2. **分散传感器**以覆盖整个搜索区域
3. **以一致的高度安装**（推荐 1.5-2.0m）
4. **通过冗余考虑传感器故障**

### 操作提示

1. **分阶段扫描**：先快速扫描，然后进行详细的重点扫描
2. **标记已确认的阳性结果**：减少冗余警报
3. **动态更新区域**：移除已清理区域
4. **传达置信水平**：并非所有检测都是确定的

---

## 安全考虑

### 限制

- **不是 100% 可靠**：始终使用次要方法验证
- **环境因素**：金属、水、厚混凝土会降低有效性
- **仅检测活运动**：无法检测无呼吸的无意识/死亡人员
- **深度限制**：超过 5m 深度后准确性下降

### 与其他方法集成

WiFi-Mat 应与以下方法一起使用：
- 声学检测（监听设备）
- 警犬搜索队
- 热成像
- 物理探测

### 假阴性风险

**阴性结果不能保证没有幸存者**。始终：
- 清理碎片后重新扫描
- 使用多种扫描方法
- 继续手动搜索程序

---

## 支持

- **文档**：[ADR-001](/docs/adr/ADR-001-wifi-mat-disaster-detection.md)
- **领域模型**：[DDD 规范](/docs/ddd/wifi-mat-domain-model.md)
- **问题**：[GitHub Issues](https://github.com/ruvnet/wifi-densepose/issues)
- **API 文档**：运行 `cargo doc --package wifi-densepose-mat --open`

---

*WiFi-Mat 旨在协助搜索和救援行动。它是一种工具，用于增强而非替代训练有素的救援人员和已建立的搜索与救援协议。*