# 核心模块 -- WiFi-DensePose 边缘智能

> 每个 ESP32 节点运行的基础模块。这些模块处理手势检测、信号质量监控、异常检测、区域占用、生命体征跟踪、入侵分类和模型打包。

所有七个模块都编译为 `wasm32-unknown-unknown` 并在第 2 层 DSP 完成后（ADR-040）在 ESP32-S3 上的 WASM3 解释器中运行。它们共享一个兼容 `no_std` 的通用设计：一个带有 `const fn new()` 的结构体，一个 `process_frame`（或 `on_timer`）入口点，以及零堆分配。

## 概述

| 模块 | 文件 | 功能 | 计算预算 |
|------|------|------|----------|
| 手势分类器 | `gesture.rs` | 使用 DTW 模板匹配从 CSI 相位序列识别手势 | ~2,400 f32 ops/帧（60x40 成本矩阵） |
| 相干性监控器 | `coherence.rs` | 通过子载波间的相量相干性测量信号质量 | ~100 三角运算/帧（32 个子载波） |
| 异常检测器 | `adversarial.rs` | 标记物理上不可能的信号：相位跳变、平线、能量尖峰 | ~130 f32 ops/帧 |
| 入侵检测器 | `intrusion.rs` | 通过相位速度和幅度扰动检测未授权进入 | ~130 f32 ops/帧 |
| 占用检测器 | `occupancy.rs` | 将感应区域划分为空间区域并报告哪些区域被占用 | ~100 f32 ops/帧 |
| 生命体征趋势分析器 | `vital_trend.rs` | 在 1 分钟和 5 分钟窗口内监控呼吸/心率以进行临床警报 | ~20 f32 ops/定时器 tick |
| RVF 容器 | `rvf.rs` | 二进制容器格式，打包 WASM 模块及其清单和签名 | 仅构建器（std），无每帧成本 |

## 模块

---

### 手势分类器 (`gesture.rs`)

**功能**：从 WiFi CSI 相位序列识别预定义的手势。它使用动态时间规整（DTW）将相位增量的滑动窗口与 4 个内置模板（挥手、推送、拉动、滑动）进行比较。

**工作原理**：每个传入帧提供子载波相位。检测器计算与前一帧的相位增量并将其推入 60 样本环形缓冲区。当累积足够样本时，它在观察窗口的尾部和每个模板之间运行约束 DTW（Sakoe-Chiba 带宽为 5）。如果最佳归一化距离低于阈值（2.5），则发出相应的手势 ID。40 帧冷却期防止重复检测。

#### API

| 项目 | 类型 | 描述 |
|------|------|------|
| `GestureDetector` | struct | 主状态持有者。包含环形缓冲区、模板和冷却计时器。 |
| `GestureDetector::new()` | `const fn` | 创建带有 4 个内置模板的检测器。 |
| `GestureDetector::process_frame(&mut self, phases: &[f32]) -> Option<u8>` | method | 输入一帧相位数据。匹配时返回 `Some(gesture_id)`。 |
| `MAX_TEMPLATE_LEN` | const (40) | 手势模板中的最大样本数。 |
| `MAX_WINDOW_LEN` | const (60) | 最大观察窗口长度。 |
| `NUM_TEMPLATES` | const (4) | 内置模板数量。 |
| `DTW_THRESHOLD` | const (2.5) | 匹配的归一化 DTW 距离阈值。 |
| `BAND_WIDTH` | const (5) | Sakoe-Chiba 带宽（限制时间规整）。 |

#### 配置

| 参数 | 默认值 | 范围 | 描述 |
|-------|--------|------|------|
| `DTW_THRESHOLD` | 2.5 | 0.5 -- 10.0 | 越低 = 匹配越严格，假阳性越少但可能错过柔和手势 |
| `BAND_WIDTH` | 5 | 1 -- 20 | Sakoe-Chiba 带宽。越宽 = 时间规整越灵活但计算量越大 |
| 冷却帧 | 40 | 10 -- 200 | 下次检测前等待的帧数。在 20 Hz 时，40 帧 = 2 秒 |

#### 发出的事件

| 事件 ID | 常量 | 触发时机 |
|---------|------|----------|
| 1 | `event_types::GESTURE_DETECTED` | 手势模板匹配。值 = 手势 ID（1=挥手, 2=推送, 3=拉动, 4=滑动）。 |

#### 使用示例

```rust
use wifi_densepose_wasm_edge::gesture::GestureDetector;

let mut detector = GestureDetector::new();

// 从 CSI 数据输入帧（通常为 20 Hz）。
let phases: Vec<f32> = get_csi_phases(); // 你的相位数据
if let Some(gesture_id) = detector.process_frame(&phases) {
    println!("Detected gesture {}", gesture_id);
    // 1 = 挥手, 2 = 推送, 3 = 拉动, 4 = 滑动
}
```

#### 教程：添加自定义手势模板

1. **收集参考数据**：通过将 CSI 帧输入检测器并记录环形缓冲区中的增量值，记录手势的相位增量序列。

2. **归一化模板**：缩放相位增量值，使其大致在 -1.0 到 1.0 之间。这确保了不同信号强度下一致的 DTW 距离。

3. **编辑模板数组**：在 `gesture.rs` 中，将 `NUM_TEMPLATES` 增加 1，并在 `GestureDetector::new()` 内的 `templates` 数组中添加新条目：
   ```rust
   GestureTemplate {
       values: {
           let mut v = [0.0f32; MAX_TEMPLATE_LEN];
           v[0] = 0.2; v[1] = 0.6; // ... 你的值
           v
       },
       len: 8,  // 有效样本数
       id: 5,   // 唯一手势 ID
   },
   ```

4. **调整阈值**：直接通过 `dtw_distance()` 运行测试数据，查看模板与实际观察值之间的距离。如果你的手势在距离高于 2.5 时一致匹配，调整 `DTW_THRESHOLD`。

5. **测试**：添加一个单元测试，将模板值作为相位输入，并验证 `process_frame` 返回你的新手势 ID。

---

### 相干性监控器 (`coherence.rs`)

**功能**：测量 WiFi 信号在子载波上的相位相干性。高相干性意味着信号稳定且感应准确。低相干性意味着多径干扰或环境变化正在降低信号质量。

**工作原理**：对于每一帧，它计算每个子载波的帧间相位增量，将每个增量转换为单位相量（cos + j*sin），并对它们进行平均。这个平均相量的幅度是原始相干性（0 = 随机，1 = 完全对齐）。这个原始值通过指数移动平均（alpha = 0.1）进行平滑。滞后门将结果分类为接受（>0.7）、警告（0.4--0.7）或拒绝（<0.4）。

#### API

| 项目 | 类型 | 描述 |
|------|------|------|
| `CoherenceMonitor` | struct | 跟踪相量和、EMA 分数和门状态。 |
| `CoherenceMonitor::new()` | `const fn` | 创建初始相干性为 1.0（接受）的监控器。 |
| `process_frame(&mut self, phases: &[f32]) -> f32` | method | 输入一帧相位数据。返回 EMA 平滑的相干性 [0, 1]。 |
| `gate_state(&self) -> GateState` | method | 当前门分类（接受、警告、拒绝）。 |
| `mean_phasor_angle(&self) -> f32` | method | 主相位漂移方向（弧度）。 |
| `coherence_score(&self) -> f32` | method | 当前 EMA 平滑的相干性分数。 |
| `GateState` | enum | `Accept`, `Warn`, `Reject` -- 信号质量分类。 |

#### 配置

| 参数 | 默认值 | 范围 | 描述 |
|-------|--------|------|------|
| `ALPHA` | 0.1 | 0.01 -- 0.5 | EMA 平滑因子。越低 = 响应越慢，越稳定。越高 = 响应越快，越嘈杂 |
| `HIGH_THRESHOLD` | 0.7 | 0.5 -- 0.95 | 相干性高于此值 = 接受 |
| `LOW_THRESHOLD` | 0.4 | 0.1 -- 0.6 | 相干性低于此值 = 拒绝 |
| `MAX_SC` | 32 | 1 -- 64 | 跟踪的最大子载波数（编译时） |

#### 发出的事件

| 事件 ID | 常量 | 触发时机 |
|---------|------|----------|
| 2 | `event_types::COHERENCE_SCORE` | 每 20 帧发出当前相干性分数（来自 `lib.rs` 中的组合管道）。 |

#### 使用示例

```rust
use wifi_densepose_wasm_edge::coherence::{CoherenceMonitor, GateState};

let mut monitor = CoherenceMonitor::new();

let phases: Vec<f32> = get_csi_phases();
let score = monitor.process_frame(&phases);

match monitor.gate_state() {
    GateState::Accept => { /* 完全准确 */ }
    GateState::Warn   => { /* 预测可能降级 */ }
    GateState::Reject => { /* 感应不可靠，重新校准 */ }
}
```

---

### 异常检测器 (`adversarial.rs`)

**功能**：检测物理上不可能或可疑的 CSI 信号，这些信号可能表明传感器故障、RF 干扰、重放攻击或环境干扰。它对每一帧运行三个独立的检查。

**工作原理**：在前 100 帧期间，它累积基线（每个子载波的平均幅度和平均总能量）。校准后，它检查每一帧的三种异常类型：

1. **相位跳变**：如果超过 50% 的子载波显示大于 2.5 弧度的相位不连续性，则发生了非物理事件。
2. **幅度平线**：如果子载波间的幅度方差接近零（低于 0.001）而平均值非零，则传感器可能卡住。
3. **能量尖峰**：如果总信号能量超过基线的 50 倍，则外部源可能正在注入功率。

20 帧冷却期防止事件泛滥。

#### API

| 项目 | 类型 | 描述 |
|------|------|------|
| `AnomalyDetector` | struct | 跟踪基线、先前相位、冷却和异常计数。 |
| `AnomalyDetector::new()` | `const fn` | 创建未校准的检测器。 |
| `process_frame(&mut self, phases: &[f32], amplitudes: &[f32]) -> bool` | method | 如果在此帧检测到异常，则返回 `true`。 |
| `total_anomalies(&self) -> u32` | method | 检测到的异常的生命周期计数。 |

#### 配置

| 参数 | 默认值 | 范围 | 描述 |
|-------|--------|------|------|
| `PHASE_JUMP_THRESHOLD` | 2.5 rad | 1.0 -- pi | 每个子载波标记的相位跳变 |
| `MIN_AMPLITUDE_VARIANCE` | 0.001 | 0.0001 -- 0.1 | 低于此值 = 平线 |
| `MAX_ENERGY_RATIO` | 50.0 | 5.0 -- 500.0 | 能量尖峰阈值与基线的比率 |
| `BASELINE_FRAMES` | 100 | 50 -- 500 | 校准基线的帧数 |
| `ANOMALY_COOLDOWN` | 20 | 5 -- 100 | 异常报告之间的帧数 |

#### 发出的事件

| 事件 ID | 常量 | 触发时机 |
|---------|------|----------|
| 3 | `event_types::ANOMALY_DETECTED` | 当任何异常检查触发时（冷却后）。 |

#### 使用示例

```rust
use wifi_densepose_wasm_edge::adversarial::AnomalyDetector;

let mut detector = AnomalyDetector::new();

// 前 100 帧校准基线（始终返回 false）。
for _ in 0..100 {
    detector.process_frame(&phases, &amplitudes);
}

// 现在报告异常。
if detector.process_frame(&phases, &amplitudes) {
    log!("Signal anomaly detected! Total: {}", detector.total_anomalies());
}
```

---

### 入侵检测器 (`intrusion.rs`)

**功能**：检测未授权进入监控区域。它专为安全应用设计，偏向于低假阴性率（宁愿误报也不愿错过真正的入侵）。

**工作原理**：检测器经历四个状态：

1. **校准**（200 帧）：学习每个子载波的基线幅度均值和方差。
2. **监控**：等待环境安静（100 连续帧低扰动）后再布防。
3. **布防**：主动监视。计算组合相位速度（60% 权重）和幅度偏差（40% 权重）的扰动分数。如果扰动连续 3 帧超过 0.8，则触发警报。
4. **警报**：检测到入侵。一旦扰动连续 50 帧低于 0.3，返回布防状态。

#### API

| 项目 | 类型 | 描述 |
|------|------|------|
| `IntrusionDetector` | struct | 带有基线、防抖和冷却的状态机。 |
| `IntrusionDetector::new()` | `const fn` | 创建处于校准状态的检测器。 |
| `process_frame(&mut self, phases: &[f32], amplitudes: &[f32]) -> &[(i32, f32)]` | method | 返回事件切片（每帧最多 4 个）。 |
| `state(&self) -> DetectorState` | method | 当前状态机状态。 |
| `total_alerts(&self) -> u32` | method | 生命周期警报计数。 |
| `DetectorState` | enum | `Calibrating`, `Monitoring`, `Armed`, `Alert`。 |

#### 配置

| 参数 | 默认值 | 范围 | 描述 |
|-------|--------|------|------|
| `INTRUSION_VELOCITY_THRESH` | 1.5 rad/frame | 0.5 -- 3.0 | 算作快速移动的相位速度 |
| `AMPLITUDE_CHANGE_THRESH` | 3.0 sigma | 1.0 -- 10.0 | 标准偏差中的幅度偏差 |
| `ARM_FRAMES` | 100 | 20 -- 500 | 布防所需的安静帧数（20 Hz 时：5 秒） |
| `DETECT_DEBOUNCE` | 3 | 1 -- 10 | 警报前的连续检测帧数 |
| `ALERT_COOLDOWN` | 100 | 20 -- 500 | 警报之间的帧数 |
| `BASELINE_FRAMES` | 200 | 100 -- 1000 | 校准窗口 |

#### 发出的事件

| 事件 ID | 常量 | 触发时机 |
|---------|------|----------|
| 200 | `EVENT_INTRUSION_ALERT` | 检测到入侵。值 = 扰动分数。 |
| 201 | `EVENT_INTRUSION_ZONE` | 识别哪个子载波区域扰动最大。 |
| 202 | `EVENT_INTRUSION_ARMED` | 检测器在安静期后布防。 |
| 203 | `EVENT_INTRUSION_DISARMED` | 检测器撤防（当前未发出）。 |

#### 使用示例

```rust
use wifi_densepose_wasm_edge::intrusion::{IntrusionDetector, DetectorState};

let mut detector = IntrusionDetector::new();

// 校准并布防（输入安静帧）。
for _ in 0..300 {
    detector.process_frame(&quiet_phases, &quiet_amps);
}
assert_eq!(detector.state(), DetectorState::Armed);

// 现在处理实时数据。
let events = detector.process_frame(&live_phases, &live_amps);
for &(event_type, value) in events {
    if event_type == 200 {
        trigger_alarm(value);
    }
}
```

---

### 占用检测器 (`occupancy.rs`)

**功能**：将感应区域划分为空间区域（基于子载波分组）并确定哪些区域当前被人占用。适用于智能建筑应用，如 HVAC 控制和照明自动化。

**工作原理**：子载波被分为 4 个一组，每组代表一个空间区域（最多 8 个区域）。对于每个区域，检测器计算该组内幅度值的方差。在校准（200 帧）期间，它学习基线方差。校准后，它计算与基线的偏差，应用 EMA 平滑（alpha=0.15），并使用滞后阈值将每个区域分类为占用或空。事件包括每个区域的占用情况（每 10 帧发出）和区域转换（状态变化时立即发出）。

#### API

| 项目 | 类型 | 描述 |
|------|------|------|
| `OccupancyDetector` | struct | 每个区域的状态、校准累加器、帧计数器。 |
| `OccupancyDetector::new()` | `const fn` | 创建未校准的检测器。 |
| `process_frame(&mut self, phases: &[f32], amplitudes: &[f32]) -> &[(i32, f32)]` | method | 返回事件（每帧最多 12 个）。 |
| `occupied_count(&self) -> u8` | method | 当前占用的区域数量。 |
| `is_zone_occupied(&self, zone_id: usize) -> bool` | method | 检查特定区域。 |

#### 配置

| 参数 | 默认值 | 范围 | 描述 |
|-------|--------|------|------|
| `MAX_ZONES` | 8 | 1 -- 16 | 最大空间区域数量 |
| `ZONE_THRESHOLD` | 0.02 | 0.005 -- 0.5 | 分数高于此值 = 占用。滞后退出为 0.5x |
| `ALPHA` | 0.15 | 0.05 -- 0.5 | 区域分数的 EMA 平滑因子 |
| `BASELINE_FRAMES` | 200 | 100 -- 1000 | 校准窗口长度 |

#### 发出的事件

| 事件 ID | 常量 | 触发时机 |
|---------|------|----------|
| 300 | `EVENT_ZONE_OCCUPIED` | 每 10 帧为每个占用区域发出。值 = `zone_id + 置信度`。 |
| 301 | `EVENT_ZONE_COUNT` | 每 10 帧发出。值 = 总占用区域数。 |
| 302 | `EVENT_ZONE_TRANSITION` | 区域状态变化时立即发出。值 = `zone_id + 0.5`（进入）或 `zone_id + 0.0`（离开）。 |

#### 使用示例

```rust
use wifi_densepose_wasm_edge::occupancy::OccupancyDetector;

let mut detector = OccupancyDetector::new();

// 用空房间数据校准。
for _ in 0..200 {
    detector.process_frame(&empty_phases, &empty_amps);
}

// 实时监控。
let events = detector.process_frame(&live_phases, &live_amps);
println!("Occupied zones: {}", detector.occupied_count());
println!("Zone 0 occupied: {}", detector.is_zone_occupied(0));
```

---

### 生命体征趋势分析器 (`vital_trend.rs`)

**功能**：随时间监控呼吸率和心率，并对临床显著状况发出警报。它跟踪 1 分钟和 5 分钟趋势，并检测呼吸暂停、呼吸过缓、呼吸过速、心动过缓和心动过速。

**工作原理**：以 1 Hz 的频率调用，输入当前生命体征读数（来自第 2 层 DSP）。它将每个读数推入 300 样本环形缓冲区（5 分钟历史）。每次调用检查：

- **呼吸暂停**：呼吸 BPM 低于 1.0 持续 20+ 秒。
- **呼吸过缓**：持续呼吸低于 12 BPM（5+ 连续样本）。
- **呼吸过速**：持续呼吸高于 25 BPM（5+ 连续样本）。
- **心动过缓**：持续心率低于 50 BPM（5+ 连续样本）。
- **心动过速**：持续心率高于 120 BPM（5+ 连续样本）。

每 60 秒，它发出呼吸和心率的 1 分钟平均值。

#### API

| 项目 | 类型 | 描述 |
|------|------|------|
| `VitalTrendAnalyzer` | struct | 两个环形缓冲区（呼吸、心率）、防抖计数器、呼吸暂停计数器。 |
| `VitalTrendAnalyzer::new()` | `const fn` | 创建带有空历史的分析器。 |
| `on_timer(&mut self, breathing_bpm: f32, heartrate_bpm: f32) -> &[(i32, f32)]` | method | 以 1 Hz 调用。返回临床警报（最多 8 个）。 |
| `breathing_avg_1m(&self) -> f32` | method | 1 分钟呼吸率平均值。 |
| `breathing_trend_5m(&self) -> f32` | method | 5 分钟呼吸趋势（正值 = 增加）。 |

#### 配置

| 参数 | 默认值 | 范围 | 描述 |
|-------|--------|------|------|
| `BRADYPNEA_THRESH` | 12.0 BPM | 8 -- 15 | 低于此值 = 危险的缓慢呼吸 |
| `TACHYPNEA_THRESH` | 25.0 BPM | 20 -- 35 | 高于此值 = 危险的快速呼吸 |
| `BRADYCARDIA_THRESH` | 50.0 BPM | 40 -- 60 | 低于此值 = 危险的缓慢心率 |
| `TACHYCARDIA_THRESH` | 120.0 BPM | 100 -- 150 | 高于此值 = 危险的快速心率 |
| `APNEA_SECONDS` | 20 | 10 -- 60 | 接近零呼吸前的警报秒数 |
| `ALERT_DEBOUNCE` | 5 | 2 -- 15 | 警报前的连续异常样本数 |

#### 发出的事件

| 事件 ID | 常量 | 触发时机 |
|---------|------|----------|
| 100 | `EVENT_VITAL_TREND` | 保留用于通用趋势事件。 |
| 101 | `EVENT_BRADYPNEA` | 持续缓慢呼吸。值 = 当前 BPM。 |
| 102 | `EVENT_TACHYPNEA` | 持续快速呼吸。值 = 当前 BPM。 |
| 103 | `EVENT_BRADYCARDIA` | 持续缓慢心率。值 = 当前 BPM。 |
| 104 | `EVENT_TACHYCARDIA` | 持续快速心率。值 = 当前 BPM。 |
| 105 | `EVENT_APNEA` | 呼吸停止。值 = 呼吸暂停秒数。 |
| 110 | `EVENT_BREATHING_AVG` | 1 分钟呼吸平均值。每 60 秒发出。 |
| 111 | `EVENT_HEARTRATE_AVG` | 1 分钟心率平均值。每 60 秒发出。 |

#### 使用示例

```rust
use wifi_densepose_wasm_edge::vital_trend::VitalTrendAnalyzer;

let mut analyzer = VitalTrendAnalyzer::new();

// 从 on_timer WASM 导出以 1 Hz 调用。
let events = analyzer.on_timer(breathing_bpm, heartrate_bpm);
for &(event_type, value) in events {
    match event_type {
        105 => alert_apnea(value as u32),
        101 => alert_bradypnea(value),
        104 => alert_tachycardia(value),
        110 => log_breathing_avg(value),
        _ => {}
    }
}

// 查询趋势数据。
let avg = analyzer.breathing_avg_1m();
let trend = analyzer.breathing_trend_5m();
```

---

### RVF 容器 (`rvf.rs`)

**功能**：定义 RVF（RuVector 格式）二进制容器，该容器打包编译后的 WASM 模块及其清单（名称、作者、功能、预算、哈希）和可选的 Ed25519 签名。这是通过 `/api/wasm/upload` 端点上传到 ESP32 节点的文件格式。

**工作原理**：该格式按顺序布局四个部分：

```
[Header: 32 bytes][Manifest: 96 bytes][WASM: N bytes][Signature: 0|64 bytes]
```

头部包含魔数（`RVF\x01`）、格式版本、部分大小和标志。清单描述模块的身份（名称、作者）、资源要求（最大帧时间、内存限制）和功能标志（需要哪些主机 API）。WASM 部分是原始编译的二进制文件。签名部分是可选的（由 `FLAG_HAS_SIGNATURE` 指示），涵盖之前的所有内容。

构建器（仅在 `std` 特性下可用）从 WASM 二进制数据和配置结构体创建 RVF 文件。它自动计算 WASM 有效载荷的 SHA-256 哈希并将其嵌入清单中以进行完整性验证。

#### API

| 项目 | 类型 | 描述 |
|------|------|------|
| `RvfHeader` | `#[repr(C, packed)]` struct | 32 字节头部，包含魔数、版本、部分大小。 |
| `RvfManifest` | `#[repr(C, packed)]` struct | 96 字节清单，包含模块元数据。 |
| `RvfConfig` | struct (std only) | 构建器配置输入。 |
| `build_rvf(wasm_data: &[u8], config: &RvfConfig) -> Vec<u8>` | function (std only) | 构建完整的 RVF 容器。 |
| `patch_signature(rvf: &mut [u8], signature: &[u8; 64])` | function (std only) | 将 Ed25519 签名修补到现有 RVF 中。 |
| `RVF_MAGIC` | const (`0x0146_5652`) | 魔数：`RVF\x01` 作为小端序 u32。 |
| `RVF_FORMAT_VERSION` | const (1) | 当前格式版本。 |
| `RVF_HEADER_SIZE` | const (32) | 头部大小（字节）。 |
| `RVF_MANIFEST_SIZE` | const (96) | 清单大小（字节）。 |
| `RVF_SIGNATURE_LEN` | const (64) | Ed25519 签名长度。 |
| `RVF_HOST_API_V1` | const (1) | 此 crate 支持的主机 API 版本。 |

#### 功能标志

| 标志 | 值 | 描述 |
|------|------|------|
| `CAP_READ_PHASE` | `1 << 0` | 模块读取相位数据 |
| `CAP_READ_AMPLITUDE` | `1 << 1` | 模块读取幅度数据 |
| `CAP_READ_VARIANCE` | `1 << 2` | 模块读取方差数据 |
| `CAP_READ_VITALS` | `1 << 3` | 模块读取生命体征数据 |
| `CAP_READ_HISTORY` | `1 << 4` | 模块读取相位历史 |
| `CAP_EMIT_EVENTS` | `1 << 5` | 模块发出事件 |
| `CAP_LOG` | `1 << 6` | 模块使用日志 |
| `CAP_ALL` | `0x7F` | 所有功能 |

#### 使用示例

```rust
use wifi_densepose_wasm_edge::rvf::builder::{build_rvf, RvfConfig, patch_signature};
use wifi_densepose_wasm_edge::rvf::*;

// 读取编译后的 WASM 二进制文件。
let wasm_data = std::fs::read("target/wasm32-unknown-unknown/release/my_module.wasm")?;

// 配置模块。
let config = RvfConfig {
    module_name: "my-gesture-v2".into(),
    author: "team-alpha".into(),
    capabilities: CAP_READ_PHASE | CAP_EMIT_EVENTS,
    max_frame_us: 5000,      // 每帧 5 ms 预算
    max_events_per_sec: 20,
    memory_limit_kb: 64,
    min_subcarriers: 8,
    max_subcarriers: 64,
    ..Default::default()
};

// 构建 RVF 容器。
let rvf = build_rvf(&wasm_data, &config);

// 可选签名并修补。
let signature = sign_with_ed25519(&rvf[..rvf.len() - RVF_SIGNATURE_LEN]);
let mut rvf_mut = rvf;
patch_signature(&mut rvf_mut, &signature);

// 上传到 ESP32。
std::fs::write("my-gesture-v2.rvf", &rvf_mut)?;
```

---

## 测试

### 运行核心模块测试

从 crate 目录：

```bash
cd rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge
cargo test --features std -- gesture coherence adversarial intrusion occupancy vital_trend rvf
```

这将运行名称包含七个模块名称中任何一个的所有测试。需要 `--features std` 标志，因为 RVF 构建器测试需要 `sha2` 和 `std::io`。

### 预期输出

所有测试都应该通过：

```
running 32 tests
test adversarial::tests::test_anomaly_detector_init ... ok
test adversarial::tests::test_calibration_phase ... ok
test adversarial::tests::test_normal_signal_no_anomaly ... ok
test adversarial::tests::test_phase_jump_detection ... ok
test adversarial::tests::test_amplitude_flatline_detection ... ok
test adversarial::tests::test_energy_spike_detection ... ok
test adversarial::tests::test_cooldown_prevents_flood ... ok
test coherence::tests::test_coherence_monitor_init ... ok
test coherence::tests::test_empty_phases_returns_current_score ... ok
test coherence::tests::test_first_frame_returns_one ... ok
test coherence::tests::test_constant_phases_high_coherence ... ok
test coherence::tests::test_incoherent_phases_lower_coherence ... ok
test coherence::tests::test_gate_hysteresis ... ok
test coherence::tests::test_mean_phasor_angle_zero_for_no_drift ... ok
test gesture::tests::test_gesture_detector_init ... ok
test gesture::tests::test_empty_phases_returns_none ... ok
test gesture::tests::test_first_frame_initializes ... ok
test gesture::tests::test_constant_phase_no_gesture_after_cooldown ... ok
test gesture::tests::test_dtw_identical_sequences ... ok
test gesture::tests::test_dtw_different_sequences ... ok
test gesture::tests::test_dtw_empty_input ... ok
test gesture::tests::test_cooldown_prevents_duplicate_detection ... ok
test gesture::tests::test_window_ring_buffer_wraps ... ok
test intrusion::tests::test_intrusion_init ... ok
test intrusion::tests::test_calibration_phase ... ok
test intrusion::tests::test_arm_after_quiet ... ok
test intrusion::tests::test_intrusion_detection ... ok
test occupancy::tests::test_occupancy_detector_init ... ok
test occupancy::tests::test_occupancy_calibration ... ok
test occupancy::tests::test_occupancy_detection ... ok
test vital_trend::tests::test_vital_trend_init ... ok
test vital_trend::tests::test_normal_vitals_no_alerts ... ok
test vital_trend::tests::test_apnea_detection ... ok
test vital_trend::tests::test_tachycardia_detection ... ok
test vital_trend::tests::test_breathing_average ... ok
test rvf::builder::tests::test_build_rvf_roundtrip ... ok
test rvf::builder::tests::test_build_hash_integrity ... ok
```

### 测试覆盖说明

| 模块 | 测试 | 覆盖范围 |
|------|------|----------|
| `gesture.rs` | 8 | 初始化、空输入、首帧、恒定输入、DTW 相同/不同/空、环形缓冲区环绕、冷却 |
| `coherence.rs` | 7 | 初始化、空输入、首帧、恒定相位、非相干相位、门滞后、相量角度 |
| `adversarial.rs` | 7 | 初始化、校准、正常信号、相位跳变、平线、能量尖峰、冷却 |
| `intrusion.rs` | 4 | 初始化、校准、布防、入侵检测 |
| `occupancy.rs` | 3 | 初始化、校准、区域检测 |
| `vital_trend.rs` | 5 | 初始化、正常生命体征、呼吸暂停、心动过速、呼吸平均值 |
| `rvf.rs` | 2 | 构建往返、哈希完整性 |

## 常见模式

所有七个核心模块共享这些设计模式：

### 1. 可常量构造的状态

每个模块的主结构体都可以通过 `const fn new()` 创建，这意味着它可以放置在 `static` 变量中，无需运行时初始化。这对于没有分配器的 WASM 模块至关重要。

```rust
static mut STATE: MyModule = MyModule::new();
```

### 2. 校准后检测的生命周期

需要基线的模块（`adversarial`、`intrusion`、`occupancy`）遵循相同的模式：累积 N 帧的统计数据，计算均值/方差，然后切换到检测模式。校准帧数始终是编译时常量。

### 3. 用于历史的环形缓冲区

`gesture`（相位增量）和 `vital_trend`（BPM 读数）都使用固定大小的环形缓冲区和模索引算术。模式是：

```rust
self.values[self.idx] = new_value;
self.idx = (self.idx + 1) % MAX_SIZE;
if self.len < MAX_SIZE { self.len += 1; }
```

### 4. 静态事件缓冲区

每帧返回多个事件的模块（`intrusion`、`occupancy`、`vital_trend`）使用 `static mut` 数组作为返回缓冲区，以避免堆分配。这在单线程 WASM 中是安全的，但需要 `unsafe` 块。模式是：

```rust
static mut EVENTS: [(i32, f32); N] = [(0, 0.0); N];
let mut n_events = 0;
// ... 填充 EVENTS[n_events] ...
unsafe { &EVENTS[..n_events] }
```

### 5. 冷却/防抖

每个检测模块使用冷却计数器来防止事件泛滥。触发事件后，计数器设置为常量值并每帧递减。计数器为正时不发出新事件。

### 6. EMA 平滑

跟踪连续分数的模块（`coherence`、`occupancy`）使用指数移动平均平滑：`smoothed = alpha * raw + (1 - alpha) * smoothed`。alpha 常量控制响应性与稳定性。

### 7. 滞后阈值

为防止检测边界处的振荡，模块使用不同的阈值进入和退出状态。例如，相干性监控器需要分数高于 0.7 才能进入接受状态，但只需低于 0.4 就会下降到拒绝状态。