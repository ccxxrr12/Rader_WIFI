# 工业与专业模块 -- WiFi-DensePose 边缘智能

> 使用 WiFi CSI 信号进行工人安全和合规监控。可穿透灰尘、烟雾、货架和墙壁，在摄像头失效的环境中工作。专为仓库、工厂、洁净室、农场和建筑工地设计。

**ADR-041 类别 5 | 事件 ID 500--599 |  crate `wifi-densepose-wasm-edge`**

## 安全警告

这些模块是**辅助监控工具**。它们不能替代：

- 认证安全系统（SIL 评级控制器、安全 PLC）
- 气体检测器、氧气监测器或 LEL 传感器
- OSHA 要求的个人防护装备
- 物理屏障、护栏或互锁装置
- 训练有素的安全人员或救援队

始终与认证的主要安全系统一起部署。WiFi CSI 传感易受环境变化（新金属物体、湿度、温度）影响，可能导致漏报。定期校准并根据实际情况验证。

---

## 概述

| 模块 | 文件 | 功能描述 | 事件 ID | 预算 |
|---|---|---|---|---|
| 叉车接近 | `ind_forklift_proximity.rs` | 当行人靠近移动的叉车/AGV 时发出警告 | 500--502 | S (<5 ms) |
| 受限空间 | `ind_confined_space.rs` | 监控储罐、人孔、容器中的工人生命体征 | 510--514 | L (<2 ms) |
| 洁净室 | `ind_clean_room.rs` | 人员计数和湍流运动检测，符合 ISO 14644 | 520--523 | L (<2 ms) |
|  livestock 监控 | `ind_livestock_monitor.rs` | 圈舍、畜棚、围栏中的动物健康监控 | 530--533 | L (<2 ms) |
| 结构振动 | `ind_structural_vibration.rs` | 地震、共振和结构漂移检测 | 540--543 | H (<10 ms) |

---

## 模块

### 叉车接近警告 (`ind_forklift_proximity.rs`)

**功能**：当人员过于靠近移动的叉车、AGV 或移动机器人时发出警告，即使在盲角和穿过货架的情况下也能工作。

**工作原理**：该模块使用三个 CSI 特征区分叉车特征和人类特征：

1. **幅度比率**：大型金属物体（叉车）相对于空仓库基线在所有子载波上产生 2--5 倍的幅度增加。
2. **低频相位主导**：叉车移动缓慢（<0.3 Hz 相位调制），而行走的人类移动较快（0.5--2 Hz）。该模块计算低频能量与总相位能量的比率。
3. **电机振动**：电动叉车内产生升高的、均匀的子载波方差（>0.08 阈值）。

当所有三个条件连续 4 帧（去抖动）满足时，模块声明车辆存在。如果人类特征（主机报告的存在 + 运动能量 >0.15）同时发生，则发出接近警告，距离类别由幅度比率推导。

#### API

```rust
pub struct ForkliftProximityDetector { /* ... */ }

impl ForkliftProximityDetector {
    /// 创建新的检测器。需要 100 帧校准（20 Hz 下约 5 秒）。
    pub const fn new() -> Self;

    /// 处理一帧 CSI 数据。返回事件作为 (event_id, value) 对。
    pub fn process_frame(
        &mut self,
        phases: &[f32],       // 每个子载波的相位值
        amplitudes: &[f32],   // 每个子载波的幅度值
        variance: &[f32],     // 每个子载波的方差值
        motion_energy: f32,   // 主机报告的运动能量
        presence: i32,        // 主机报告的存在标志 (0/1)
        n_persons: i32,       // 主机报告的人数
    ) -> &[(i32, f32)];

    /// 当前是否检测到车辆。
    pub fn is_vehicle_present(&self) -> bool;

    /// 当前幅度比率（车辆接近的代理）。
    pub fn amplitude_ratio(&self) -> f32;
}
```

#### 发出的事件

| 事件 ID | 常量 | 值 | 含义 |
|---|---|---|---|
| 500 | `EVENT_PROXIMITY_WARNING` | 距离类别：0.0 = 临界，1.0 = 警告，2.0 = 注意 | 人员危险接近车辆 |
| 501 | `EVENT_VEHICLE_DETECTED` | 幅度比率（浮点数） | 叉车/AGV 进入传感器区域 |
| 502 | `EVENT_HUMAN_NEAR_VEHICLE` | 运动能量（浮点数） | 检测到人类在车辆区域（在转换时触发一次） |

#### 状态机

```
                  +-----------+
                  |           |
        +-------->| No Vehicle|<---------+
        |         |           |          |
        |         +-----+-----+          |
        |               |               |
        |   amp_ratio > 2.5 AND         |
        |   low_freq_dominant AND        | debounce drops
        |   vibration > 0.08            | below threshold
        |   (4 frames debounce)          |
        |               |               |
        |         +-----v-----+          |
        |         |           |----------+
        +---------|  Vehicle  |
                  |  Present  |
                  +-----+-----+
                        |
          human present |  (presence + motion > 0.15)
          + debounce    |
                  +-----v-----+
                  | Proximity |----> EVENT 500 (cooldown 40 frames)
                  |  Warning  |----> EVENT 502 (once on transition)
                  +-----------+
```

#### 配置

| 参数 | 默认值 | 范围 | 安全影响 |
|---|---|---|---|
| `FORKLIFT_AMP_RATIO` | 2.5 | 1.5--5.0 | 较低 = 更敏感，更多误报 |
| `HUMAN_MOTION_THRESH` | 0.15 | 0.05--0.5 | 较低 = 捕获缓慢移动的工人 |
| `VEHICLE_DEBOUNCE` | 4 帧 | 2--10 | 较高 = 较少误报，响应较慢 |
| `PROXIMITY_DEBOUNCE` | 2 帧 | 1--5 | 较高 = 较少误报，响应较慢 |
| `ALERT_COOLDOWN` | 40 帧 (2 秒) | 10--200 | 较低 = 更频繁的警告 |
| `DIST_CRITICAL` | amp ratio > 4.0 | -- | 非常接近 |
| `DIST_WARNING` | amp ratio > 3.0 | -- | 接近 |

#### 使用示例

```rust
use wifi_densepose_wasm_edge::ind_forklift_proximity::ForkliftProximityDetector;

let mut detector = ForkliftProximityDetector::new();

// 校准阶段：输入 100 帧空仓库数据
for _ in 0..100 {
    detector.process_frame(&phases, &amps, &variance, 0.0, 0, 0);
}

// 正常操作
let events = detector.process_frame(&phases, &amps, &variance, 0.5, 1, 1);
for &(event_id, value) in events {
    match event_id {
        500 => {
            let category = match value as i32 {
                0 => "临界 -- 立即停止叉车",
                1 => "警告 -- 减速",
                _ => "注意 -- 保持警惕",
            };
            trigger_alarm(category);
        }
        501 => log("检测到车辆，幅度比率：{}", value),
        502 => log("人类进入车辆区域"),
        _ => {}
    }
}
```

#### 教程：设置仓库接近警报

1. **传感器放置**：每个过道安装一个 ESP32 WiFi 传感器，高度为货架高度（1.5--2 米）。每个传感器覆盖约一个过道宽度（3--4 米）和 10--15 米的过道长度。

2. **校准**：在安静时期（无叉车、无工人）开机。模块在前 100 帧（20 Hz 下 5 秒）自动校准。基线幅度代表空过道。

3. **阈值调优**：如果手推车或托盘车导致误报，将 `FORKLIFT_AMP_RATIO` 从 2.5 增加到 3.0。如果漏检叉车，减少到 2.0。

4. **集成**：将 `EVENT_PROXIMITY_WARNING` (500) 连接到警告灯（琥珀色表示注意/警告，红色表示临界）和声音警报。连接到设施 SCADA 系统进行记录。

5. **验证**：当叉车运行时走过过道。验证所有三个距离类别在适当的范围内触发。

---

### 受限空间监控 (`ind_confined_space.rs`)

**功能**：监控储罐、人孔、容器或任何封闭空间内的工人。确认他们在呼吸，并在他们停止移动或呼吸时发出警报。

**合规性**：设计用于支持 OSHA 29 CFR 1910.146 受限空间进入要求。该模块提供连续的生命证明监控，以补充（而非替代）所需的安全人员。

**工作原理**：使用去抖动的存在检测来跟踪进入/退出转换。当工人在内部时，模块持续监控两个生命指标：

1. **呼吸**：主机报告的呼吸 BPM 必须保持在 4.0 BPM 以上。如果 300 帧（20 Hz 下 15 秒）未检测到呼吸，发出撤离警报。
2. **运动**：主机报告的运动能量必须保持在 0.02 以上。如果 1200 帧（60 秒）未检测到运动，发出不动警报。

模块在 `Empty`、`Present`、`BreathingCeased` 和 `Immobile` 状态之间转换。当呼吸或运动恢复时，状态恢复到 `Present`。

#### API

```rust
pub enum WorkerState {
    Empty,           // 空间内无工人
    Present,         // 工人存在，生命体征正常
    BreathingCeased, // 未检测到呼吸（危险）
    Immobile,        // 未检测到运动（危险）
}

pub struct ConfinedSpaceMonitor { /* ... */ }

impl ConfinedSpaceMonitor {
    pub const fn new() -> Self;

    /// 处理一帧。
    pub fn process_frame(
        &mut self,
        presence: i32,       // 主机报告的存在 (0/1)
        breathing_bpm: f32,  // 主机报告的呼吸率
        motion_energy: f32,  // 主机报告的运动能量
        variance: f32,       // 平均 CSI 方差
    ) -> &[(i32, f32)];

    /// 当前工人状态。
    pub fn state(&self) -> WorkerState;

    /// 工人是否在空间内。
    pub fn is_worker_inside(&self) -> bool;

    /// 自上次确认呼吸以来的秒数。
    pub fn seconds_since_breathing(&self) -> f32;

    /// 自上次检测到运动以来的秒数。
    pub fn seconds_since_motion(&self) -> f32;
}
```

#### 发出的事件

| 事件 ID | 常量 | 值 | 含义 |
|---|---|---|---|
| 510 | `EVENT_WORKER_ENTRY` | 1.0 | 工人进入受限空间 |
| 511 | `EVENT_WORKER_EXIT` | 1.0 | 工人离开受限空间 |
| 512 | `EVENT_BREATHING_OK` | BPM (浮点数) | 定期呼吸确认（约每 5 秒） |
| 513 | `EVENT_EXTRACTION_ALERT` | 自上次呼吸以来的秒数 | >15 秒无呼吸 -- 启动救援 |
| 514 | `EVENT_IMMOBILE_ALERT` | 无运动的秒数 | >60 秒无运动 -- 检查工人 |

#### 状态机

```
            +---------+
            |  Empty  |<----------+
            +----+----+           |
                 |                |
     presence    |                | absence (10 frames)
     (10 frames) |                |
                 v                |
            +---------+           |
    +------>| Present |-----------+
    |       +----+----+
    |            |          |
    |  breathing | no       | no motion
    |  resumes   | breathing| (1200 frames)
    |            | (300     |
    |            |  frames) |
    |       +----v------+   |
    +-------|Breathing  |   |
    |       | Ceased    |   |
    |       +-----------+   |
    |                       |
    |       +-----------+   |
    +-------| Immobile  |<--+
            +-----------+
              motion resumes -> Present
```

#### 配置

| 参数 | 默认值 | 范围 | 安全影响 |
|---|---|---|---|
| `BREATHING_CEASE_FRAMES` | 300 (15 秒) | 100--600 | 较低 = 更快警报，更多误报 |
| `IMMOBILE_FRAMES` | 1200 (60 秒) | 400--3600 | 较低 = 捕获较慢的倒下 |
| `MIN_BREATHING_BPM` | 4.0 | 2.0--8.0 | 较低 = 对缓慢呼吸更宽容 |
| `MIN_MOTION_ENERGY` | 0.02 | 0.005--0.1 | 较低 = 捕获微妙的运动 |
| `ENTRY_EXIT_DEBOUNCE` | 10 帧 | 5--30 | 较高 = 较少的误进入/退出 |
| `MIN_PRESENCE_VAR` | 0.005 | 0.001--0.05 | 空空间的噪声抑制 |

#### 使用示例

```rust
use wifi_densepose_wasm_edge::ind_confined_space::{
    ConfinedSpaceMonitor, WorkerState,
    EVENT_EXTRACTION_ALERT, EVENT_IMMOBILE_ALERT,
};

let mut monitor = ConfinedSpaceMonitor::new();

// 处理每个 CSI 帧
let events = monitor.process_frame(presence, breathing_bpm, motion_energy, variance);

for &(event_id, value) in events {
    match event_id {
        513 => {  // EXTRACTION_ALERT
            activate_rescue_alarm();
            notify_safety_attendant(value);  // 自上次呼吸以来的秒数
        }
        514 => {  // IMMOBILE_ALERT
            notify_safety_attendant(value);  // 无运动的秒数
        }
        _ => {}
    }
}

// 查询状态用于仪表板显示
match monitor.state() {
    WorkerState::Empty => display_green("空间为空"),
    WorkerState::Present => display_green("工人正常"),
    WorkerState::BreathingCeased => display_red("无呼吸"),
    WorkerState::Immobile => display_amber("工人不动"),
}
```

---

### 洁净室监控 (`ind_clean_room.rs`)

**功能**：跟踪洁净室中的人员计数和移动模式，以执行 ISO 14644 占用限制并检测可能扰乱层流的湍流运动。

**工作原理**：使用主机报告的人数和去抖动的违规检测。湍流运动（能量 >0.6 的快速移动）被标记，因为它会扰乱保持颗粒计数低的层流。该模块维护运行的合规百分比用于审计报告。

#### API

```rust
pub struct CleanRoomMonitor { /* ... */ }

impl CleanRoomMonitor {
    /// 创建默认最大占用为 4 的监控器。
    pub const fn new() -> Self;

    /// 创建具有自定义最大占用的监控器。
    pub const fn with_max_occupancy(max: u8) -> Self;

    /// 处理一帧。
    pub fn process_frame(
        &mut self,
        n_persons: i32,      // 主机报告的人数
        presence: i32,       // 主机报告的存在 (0/1)
        motion_energy: f32,  // 主机报告的运动能量
    ) -> &[(i32, f32)];

    /// 当前占用计数。
    pub fn current_count(&self) -> u8;

    /// 允许的最大占用。
    pub fn max_occupancy(&self) -> u8;

    /// 当前是否违规。
    pub fn is_in_violation(&self) -> bool;

    /// 合规百分比 (0--100)。
    pub fn compliance_percent(&self) -> f32;

    /// 违规事件总数。
    pub fn total_violations(&self) -> u32;
}
```

#### 发出的事件

| 事件 ID | 常量 | 值 | 含义 |
|---|---|---|---|
| 520 | `EVENT_OCCUPANCY_COUNT` | 人数 (浮点数) | 占用发生变化 |
| 521 | `EVENT_OCCUPANCY_VIOLATION` | 当前计数 (浮点数) | 计数超过最大允许值 |
| 522 | `EVENT_TURBULENT_MOTION` | 运动能量 (浮点数) | 检测到快速移动（气流风险） |
| 523 | `EVENT_COMPLIANCE_REPORT` | 合规 % (0--100) | 定期合规摘要（约 30 秒） |

#### 状态机

```
    +------------------+
    |  Monitoring      |
    |  (count <= max)  |
    +--------+---------+
             |  count > max
             |  (10 frames debounce)
    +--------v---------+
    |  Violation       |----> EVENT 521 (cooldown 200 frames)
    |  (count > max)   |
    +--------+---------+
             |  count <= max
             |
    +--------v---------+
    |  Monitoring      |
    +------------------+

    并行:
    motion_energy > 0.6 (3 frames) ----> EVENT 522 (cooldown 100 frames)
    Every 600 frames (~30 s) ----------> EVENT 523 (compliance %)
```

#### 配置

| 参数 | 默认值 | 范围 | 安全影响 |
|---|---|---|---|
| `DEFAULT_MAX_OCCUPANCY` | 4 | 1--255 | 符合 ISO 14644 房间等级 |
| `TURBULENT_MOTION_THRESH` | 0.6 | 0.3--0.9 | 较低 = 更严格的移动控制 |
| `VIOLATION_DEBOUNCE` | 10 帧 | 3--20 | 较高 = 容忍短暂的超员 |
| `VIOLATION_COOLDOWN` | 200 帧 (10 秒) | 40--600 | 警报重复间隔 |
| `COMPLIANCE_REPORT_INTERVAL` | 600 帧 (30 秒) | 200--6000 | 审计报告频率 |

#### 使用示例

```rust
use wifi_densepose_wasm_edge::ind_clean_room::{
    CleanRoomMonitor, EVENT_OCCUPANCY_VIOLATION, EVENT_COMPLIANCE_REPORT,
};

// ISO 5 级洁净室：最多 3 人
let mut monitor = CleanRoomMonitor::with_max_occupancy(3);

let events = monitor.process_frame(n_persons, presence, motion_energy);
for &(event_id, value) in events {
    match event_id {
        521 => alert_cleanroom_supervisor(value as u8),
        522 => alert_turbulent_motion(),
        523 => log_compliance_audit(value),
        _ => {}
    }
}

// 仪表板
println!("占用: {}/{}", monitor.current_count(), monitor.max_occupancy());
println!("合规: {:.1}%", monitor.compliance_percent());
```

---

###  livestock 监控 (`ind_livestock_monitor.rs`)

**功能**：监控圈舍、畜棚和围栏中的动物存在和健康。检测异常静止（可能生病）、呼吸困难和逃逸事件。

**工作原理**：使用去抖动的进入/退出检测跟踪存在。根据物种特定的正常范围监控呼吸率。检测长时间静止（>5 分钟）作为疾病迹象，以及确认存在后的突然缺席作为逃逸事件。

物种特定的呼吸范围：

| 物种 | 正常 BPM | 呼吸困难：低于 | 呼吸困难：高于 |
|---|---|---|---|
| 牛 | 12--30 | 8.4 (0.7x 最小值) | 39.0 (1.3x 最大值) |
| 羊 | 12--20 | 8.4 (0.7x 最小值) | 26.0 (1.3x 最大值) |
| 家禽 | 15--30 | 10.5 (0.7x 最小值) | 39.0 (1.3x 最大值) |
| 自定义 | 可配置 | 0.7x 最小值 | 1.3x 最大值 |

#### API

```rust
pub enum Species {
    Cattle,
    Sheep,
    Poultry,
    Custom { min_bpm: f32, max_bpm: f32 },
}

pub struct LivestockMonitor { /* ... */ }

impl LivestockMonitor {
    /// 使用默认物种（牛）创建。
    pub const fn new() -> Self;

    /// 使用特定物种创建。
    pub const fn with_species(species: Species) -> Self;

    /// 处理一帧。
    pub fn process_frame(
        &mut self,
        presence: i32,       // 主机报告的存在 (0/1)
        breathing_bpm: f32,  // 主机报告的呼吸率
        motion_energy: f32,  // 主机报告的运动能量
        variance: f32,       // 平均 CSI 方差（未使用，预留）
    ) -> &[(i32, f32)];

    /// 当前是否检测到动物。
    pub fn is_animal_present(&self) -> bool;

    /// 配置的物种。
    pub fn species(&self) -> Species;

    /// 静止的分钟数。
    pub fn stillness_minutes(&self) -> f32;

    /// 最后观察到的呼吸 BPM。
    pub fn last_breathing_bpm(&self) -> f32;
}
```

#### 发出的事件

| 事件 ID | 常量 | 值 | 含义 |
|---|---|---|---|
| 530 | `EVENT_ANIMAL_PRESENT` | BPM (浮点数) | 定期存在报告（约 10 秒） |
| 531 | `EVENT_ABNORMAL_STILLNESS` | 静止分钟数 (浮点数) | >5 分钟无运动 |
| 532 | `EVENT_LABORED_BREATHING` | BPM (浮点数) | 呼吸超出正常范围 |
| 533 | `EVENT_ESCAPE_ALERT` | 逃逸前存在的分钟数 (浮点数) | 确认存在后动物突然缺席 |

#### 状态机

```
    +---------+
    |  Empty  |<---------+
    +----+----+          |
         |               |
   presence              | absence >= 20 frames
   (10 frames)           | (after >= 200 frames presence
         v               |  -> EVENT 533 escape alert)
    +---------+          |
    | Present |----------+
    +----+----+
         |
   no motion (6000 frames = 5 min) -> EVENT 531 (once)
   breathing outside range (20 frames) -> EVENT 532 (repeating)
```

#### 配置

| 参数 | 默认值 | 范围 | 安全影响 |
|---|---|---|---|
| `STILLNESS_FRAMES` | 6000 (5 分钟) | 1200--12000 | 较低 = 更早的疾病检测 |
| `MIN_PRESENCE_FOR_ESCAPE` | 200 (10 秒) | 60--600 | 逃逸计数前的最小存在时间 |
| `ESCAPE_ABSENCE_FRAMES` | 20 (1 秒) | 10--100 | 容忍短暂缺席 |
| `LABORED_DEBOUNCE` | 20 帧 (1 秒) | 5--60 | 较低 = 更快的呼吸警报 |
| `MIN_MOTION_ACTIVE` | 0.03 | 0.01--0.1 | 对微妙运动的敏感度 |

#### 使用示例

```rust
use wifi_densepose_wasm_edge::ind_livestock_monitor::{
    LivestockMonitor, Species, EVENT_ESCAPE_ALERT, EVENT_LABORED_BREATHING,
};

// 奶牛场：监控奶牛
let mut monitor = LivestockMonitor::with_species(Species::Cattle);

let events = monitor.process_frame(presence, breathing_bpm, motion_energy, variance);
for &(event_id, value) in events {
    match event_id {
        532 => alert_veterinarian(value),  // 呼吸困难 BPM
        533 => alert_farm_security(value), // 逃逸：丢失前存在的分钟数
        531 => log_health_concern(value),  // 静止的分钟数
        _ => {}
    }
}
```

---

### 结构振动监控 (`ind_structural_vibration.rs`)

**功能**：使用 CSI 相位稳定性检测建筑物振动、地震活动和结构应力。仅在监控空间无人时运行（人类运动掩盖结构信号）。

**工作原理**：当无人存在时，WiFi CSI 相位高度稳定（噪声底 ~0.02 rad）。该模块检测三种类型的结构事件：

1. **地震**：宽带能量增加（>60% 的子载波受影响，RMS >0.15 rad）。表示地震、重型车辆经过或建筑活动。
2. **机械共振**：通过平均相位时间序列的自相关检测窄带峰值。峰均值比 >3.0 且 RMS 高于噪声底 2 倍表示周期性机械振动（HVAC、泵、旋转设备）。
3. **结构漂移**：>50% 的子载波上超过 30 秒的缓慢单调相位变化。表示材料应力、地基沉降或热膨胀。

#### API

```rust
pub struct StructuralVibrationMonitor { /* ... */ }

impl StructuralVibrationMonitor {
    /// 创建新的监控器。空房间时需要 100 帧校准。
    pub const fn new() -> Self;

    /// 处理一帧 CSI 数据。
    pub fn process_frame(
        &mut self,
        phases: &[f32],       // 每个子载波的相位值
        amplitudes: &[f32],   // 每个子载波的幅度值
        variance: &[f32],     // 每个子载波的方差值
        presence: i32,        // 0 = 空（分析），1 = 有人（跳过）
    ) -> &[(i32, f32)];

    /// 当前 RMS 振动水平。
    pub fn rms_vibration(&self) -> f32;

    /// 是否建立基线。
    pub fn is_calibrated(&self) -> bool;
}
```

#### 发出的事件

| 事件 ID | 常量 | 值 | 含义 |
|---|---|---|---|
| 540 | `EVENT_SEISMIC_DETECTED` | RMS 振动水平 (rad) | 宽带地震活动 |
| 541 | `EVENT_MECHANICAL_RESONANCE` | 主导频率 (Hz) | 窄带机械振动 |
| 542 | `EVENT_STRUCTURAL_DRIFT` | 漂移率 (rad/s) | 缓慢结构变形 |
| 543 | `EVENT_VIBRATION_SPECTRUM` | RMS 水平 (rad) | 周期性频谱报告（约 5 秒） |

#### 状态机

```
    +--------------+
    | Calibrating  |  (100 frames, presence=0 required)
    +------+-------+
           |
    +------v-------+
    |   Idle       |  (presence=1: skip analysis, reset drift)
    | (Occupied)   |
    +------+-------+
           |  presence=0
    +------v-------+
    |  Analyzing   |
    +------+-------+
           |
           +-----> RMS > 0.15 + broadband -------> EVENT 540 (seismic)
           +-----> autocorr peak ratio > 3.0 ----> EVENT 541 (resonance)
           +-----> monotonic drift > 30 s -------> EVENT 542 (drift)
           +-----> every 100 frames -------------> EVENT 543 (spectrum)
```

#### 配置

| 参数 | 默认值 | 范围 | 安全影响 |
|---|---|---|---|
| `SEISMIC_THRESH` | 0.15 rad RMS | 0.05--0.5 | 较低 = 对震动更敏感 |
| `RESONANCE_PEAK_RATIO` | 3.0 | 2.0--5.0 | 较低 = 检测较弱的共振 |
| `DRIFT_RATE_THRESH` | 0.0005 rad/frame | 0.0001--0.005 | 较低 = 检测较慢的漂移 |
| `DRIFT_MIN_FRAMES` | 600 (30 秒) | 200--2400 | 警报前的最小漂移持续时间 |
| `SEISMIC_DEBOUNCE` | 4 帧 | 2--10 | 较高 = 较少的地震误报 |
| `SEISMIC_COOLDOWN` | 200 帧 (10 秒) | 40--600 | 警报重复间隔 |

#### 使用示例

```rust
use wifi_densepose_wasm_edge::ind_structural_vibration::{
    StructuralVibrationMonitor, EVENT_SEISMIC_DETECTED, EVENT_STRUCTURAL_DRIFT,
};

let mut monitor = StructuralVibrationMonitor::new();

// 在无人期间校准
for _ in 0..100 {
    monitor.process_frame(&phases, &amps, &variance, 0);
}
assert!(monitor.is_calibrated());

// 正常操作
let events = monitor.process_frame(&phases, &amps, &variance, presence);
for &(event_id, value) in events {
    match event_id {
        540 => {
            trigger_building_alarm();
            log_seismic_event(value);  // RMS 振动水平
        }
        542 => {
            notify_structural_engineer(value);  // 漂移率 rad/s
        }
        _ => {}
    }
}
```

---

## OSHA 合规说明

### 叉车接近（OSHA 29 CFR 1910.178）

- **标准**：动力工业卡车 -- 操作员必须警告他人。
- **模块支持**：自动接近检测补充喇叭/灯光警告。不能替代操作员培训、安全带或速度限制。
- **所需的额外设备**：物理屏障、地面标记、交通镜、操作员培训计划。

### 受限空间（OSHA 29 CFR 1910.146）

- **标准**：需要许可的受限空间。
- **模块支持**：连续的生命证明监控（呼吸和运动确认）。协助所需的安全人员。
- **所需的额外设备**：
  - 大气监测（O2、H2S、CO、LEL）-- WiFi 模块无法检测气体危害。
  - 进入者和安全人员之间的通信系统。
  - 救援设备（检索系统、安全带、三脚架）。
  - 记录危害和控制措施的进入许可。
- **审计跟踪**：`EVENT_BREATHING_OK` (512) 提供带时间戳的生命证明记录，用于合规文档。

### 洁净室（ISO 14644）

- **标准**：洁净室和相关控制环境。
- **模块支持**：实时占用执行和湍流运动检测，用于颗粒控制。
- **所需的额外设备**：颗粒计数器、压差监测器、HEPA/ULPA 过滤系统。
- **文档**：`EVENT_COMPLIANCE_REPORT` (523) 提供定期合规百分比，用于审计记录。

###  livestock（无直接 OSHA 标准；参见 USDA 动物福利法）

- **模块支持**：自动健康监控减少手动检查负担。逃逸检测支持周界安全。
- **所需的额外设备**：兽医监控系统、适当的围栏、温度/湿度传感器。

### 结构振动（OSHA 29 CFR 1926 子部分 P，挖掘）

- **标准**：建筑的结构稳定性要求。
- **模块支持**：无人期间的连续振动监控。地震检测提供早期预警。
- **所需的额外设备**：认证结构检查、关键结构的加速度计、倾斜传感器。

---

## 部署指南

### 仓库覆盖的传感器放置

```
    +---+---+---+---+---+
    | S |   |   |   | S |   S = WiFi 传感器 (ESP32)
    +---+ Aisle 1   +---+   安装在货架高度 (1.5-2 m)
    |   |           |   |   每个过道交叉点一个传感器
    +---+ Aisle 2   +---+
    | S |           | S |   覆盖范围：每个传感器 ~15 m 范围
    +---+---+---+---+---+   对于接近：沿过道每 10 m 一个传感器
```

- 将传感器安装在货架高度（1.5--2 米）以获得最佳的人/叉车分离。
- 放置在过道交叉点以覆盖盲角。
- 每个传感器覆盖约 10--15 米的过道长度。
- 对于关键区域（装卸码头、充电区），使用重叠传感器。

### 受限空间的多传感器设置

```
    Ground Level
    +-----------+
    |  Sensor A | <-- 入口点监控
    +-----+-----+
          |
          | Manhole / Hatch
          |
    +-----v-----+
    |  Sensor B | <-- 空间内部（如果可能）
    +-----------+
```

- 入口点的传感器 A 检测工人进入/退出。
- 受限空间内的传感器 B（如果可以安全安装）提供呼吸和运动监控。
- 如果只有一个传感器可用，安装在入口处面向空间内部。
- WiFi 信号穿透金属墙能力差 -- 大型容器使用多个传感器。

### 与安全 PLC 集成

通过以下方式将 ESP32 事件输出连接到安全 PLC：

1. **UDP**：传感服务器接收 ESP32 CSI 数据并通过 REST API 发出事件。轮询 `/api/v1/events` 获取实时警报。
2. **Modbus TCP**：使用网关将 UDP 事件转换为 Modbus 寄存器，用于直接 PLC 集成。
3. **GPIO**：对于硬连线安全电路，将 ESP32 GPIO 输出连接到 PLC 安全输入。配置 ESP32 固件在特定事件 ID 上断言 GPIO。

### 校准检查表

1. 确保监控空间处于正常的空状态。
2. 打开传感器并等待校准完成：
   - 叉车接近：100 帧（5 秒）
   - 结构振动：100 帧（5 秒）
   - 受限空间：无需校准（使用主机存在）
   - 洁净室：无需校准（使用主机人数）
   -  livestock：无需校准（使用主机存在）
3. 通过走过空间并确认存在检测进行验证。
4. 对于叉车接近，驾驶叉车通过并验证车辆检测和在适当距离的接近警告。
5. 记录校准日期、传感器位置和固件版本。

---

## 事件 ID 注册表（类别 5）

| 范围 | 模块 | 事件 |
|---|---|---|
| 500--502 | 叉车接近 | `PROXIMITY_WARNING`, `VEHICLE_DETECTED`, `HUMAN_NEAR_VEHICLE` |
| 510--514 | 受限空间 | `WORKER_ENTRY`, `WORKER_EXIT`, `BREATHING_OK`, `EXTRACTION_ALERT`, `IMMOBILE_ALERT` |
| 520--523 | 洁净室 | `OCCUPANCY_COUNT`, `OCCUPANCY_VIOLATION`, `TURBULENT_MOTION`, `COMPLIANCE_REPORT` |
| 530--533 |  livestock 监控 | `ANIMAL_PRESENT`, `ABNORMAL_STILLNESS`, `LABORED_BREATHING`, `ESCAPE_ALERT` |
| 540--543 | 结构振动 | `SEISMIC_DETECTED`, `MECHANICAL_RESONANCE`, `STRUCTURAL_DRIFT`, `VIBRATION_SPECTRUM` |

总计：5 个模块中的 20 种事件类型。