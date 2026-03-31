# 安全与防护模块 -- WiFi-DensePose 边缘智能

> 使用 WiFi 信道状态信息 (CSI) 进行周界监控和威胁检测。
> 可穿透墙壁，在完全黑暗环境中工作，无需可见摄像头。
> 每个模块在 $8 的 ESP32-S3 芯片上以 20 Hz 的帧率运行。
> 所有模块均兼容 `no_std`，并编译为 WASM 以通过 ADR-040 Tier 3 热加载。

## 概述

| 模块 | 文件 | 功能描述 | 事件 ID | 预算 |
|------|------|----------|---------|------|
| 入侵检测 | `intrusion.rs` | 相位/幅度异常入侵报警，支持布防/撤防 | 200-203 | S (<5 ms) |
| 周界突破 | `sec_perimeter_breach.rs` | 多区域周界穿越检测，支持接近/离开方向识别 | 210-213 | S (<5 ms) |
| 武器检测 | `sec_weapon_detect.rs` | 通过 RF 反射率比率检测隐藏金属物体 | 220-222 | S (<5 ms) |
| 尾随检测 | `sec_tailgating.rs` | 双峰值运动包络检测未授权跟随 | 230-232 | L (<2 ms) |
| 徘徊检测 | `sec_loitering.rs` | 长时间静止存在检测，使用 4 状态机 | 240-242 | L (<2 ms) |
| 恐慌运动 | `sec_panic_motion.rs` | 检测异常运动、挣扎和逃跑模式 | 250-252 | S (<5 ms) |

预算说明：**S** = 标准 (<5 ms/帧)，**L** = 轻量 (<2 ms/帧)。

## 共享设计模式

所有安全模块遵循以下约定：

- **`const fn new()`**：零分配构造函数，无堆内存，适用于 ESP32 上的 `static mut`。
- **`process_frame(...) -> &[(i32, f32)]`**：通过静态缓冲区返回事件元组 `(event_id, value)`（在单线程 WASM 中安全）。
- **校准阶段**：前 N 帧（通常 100-200 帧，20 Hz 下为 5-10 秒）学习环境基线。校准期间无事件。
- **去抖动**：连续帧计数器防止单帧噪声触发警报。
- **冷却**：发出事件后，冷却窗口抑制重复发射（40-100 帧 = 2-5 秒）。
- **滞后**：去抖动计数器使用 `saturating_sub(1)` 进行逐渐衰减而非硬重置，减少边界信号的抖动。

---

## 模块

### 入侵检测 (`intrusion.rs`)

**功能**：监控先前为空的空间，当有人进入时触发警报。工作原理类似传统运动警报——系统在布防前环境必须稳定。

**工作原理**：在校准期间（200 帧），检测器学习每个子载波的幅度均值和方差。校准后，它等待环境安静（100 个连续低扰动帧）后布防。布防后，它计算相位速度（帧间突然相位跳变）和幅度偏差（幅度偏离基线超过 3 个标准差）的综合扰动分数。如果扰动超过 0.8 持续 3+ 连续帧，触发警报。

#### 状态机

```
Calibrating --> Monitoring --> Armed --> Alert
                   ^                      |
                   |        (quiet for     |
                   |         50 frames)    |
                   +---- Armed <----------+
```

- **Calibrating**：累积 200 帧基线幅度统计。
- **Monitoring**：等待 100 个连续安静帧后布防。
- **Armed**：主动检测。连续 3+ 高扰动帧触发警报。
- **Alert**：主动警报。50 个连续安静帧后返回 Armed 状态。100 帧冷却防止重复触发。

#### API

| 项 | 类型 | 描述 |
|----|------|------|
| `IntrusionDetector::new()` | `const fn` | 创建处于 Calibrating 状态的检测器 |
| `process_frame(phases, amplitudes)` | `fn` | 处理一帧 CSI 数据，返回事件 |
| `state()` | `fn -> DetectorState` | 当前状态 (Calibrating/Monitoring/Armed/Alert) |
| `total_alerts()` | `fn -> u32` | 累计警报次数 |

#### 发出的事件

| 事件 ID | 常量 | 触发时机 |
|---------|------|----------|
| 200 | `EVENT_INTRUSION_ALERT` | 检测到入侵（值为扰动分数） |
| 201 | `EVENT_INTRUSION_ZONE` | 最高扰动区域索引 |
| 202 | `EVENT_INTRUSION_ARMED` | 系统转换到 Armed 状态 |
| 203 | `EVENT_INTRUSION_DISARMED` | 系统撤防（当前未使用 - 预留） |

#### 配置

| 参数 | 默认值 | 范围 | 描述 |
|------|--------|------|------|
| `INTRUSION_VELOCITY_THRESH` | 1.5 | 0.5-3.0 | 相位速度阈值 (rad/帧) |
| `AMPLITUDE_CHANGE_THRESH` | 3.0 | 2.0-5.0 | 幅度偏差的标准差倍数 |
| `ARM_FRAMES` | 100 | 40-200 | 布防前所需的安静帧数 (20 Hz 下 5 秒) |
| `DETECT_DEBOUNCE` | 3 | 2-10 | 触发警报前的连续扰动帧数 |
| `ALERT_COOLDOWN` | 100 | 20-200 | 重新警报之间的帧数 (20 Hz 下 5 秒) |
| `BASELINE_FRAMES` | 200 | 100-500 | 校准帧数 (20 Hz 下 10 秒) |

---

### 周界突破检测 (`sec_perimeter_breach.rs`)

**功能**：将监控区域划分为 4 个区域（映射到子载波组），检测跨越区域边界的移动。使用能量梯度趋势将运动方向分类为接近或离开。

**工作原理**：子载波被平均分为 4 组，每组代表一个空间区域。每帧计算每个区域的指标：
1. **相位梯度**：区域子载波范围内当前帧与前一帧的平均绝对相位差。
2. **方差比率**：当前区域方差除以校准基线方差。

当相位梯度超过 0.6 rad/子载波且方差比率超过基线的 2.5 倍时，标记为突破。方向通过 8 帧能量历史缓冲区的线性回归斜率确定——正斜率 = 接近，负斜率 = 离开。

#### 状态机

没有显式的状态机枚举。而是通过每个区域的计数器跟踪：
- `disturb_run`：连续突破帧数（区域安静时重置为 0）。
- `approach_run` / `departure_run`：连续帧的正/负能量趋势（去抖动为 3 帧）。
- 四个独立的冷却计时器，用于突破、接近、离开和转换事件。

无卡住状态可能：所有计数器要么在安静输入时重置，要么通过 `saturating_add` 有界。

#### API

| 项 | 类型 | 描述 |
|----|------|------|
| `PerimeterBreachDetector::new()` | `const fn` | 创建未校准的检测器 |
| `process_frame(phases, amplitudes, variance, motion_energy)` | `fn` | 处理一帧，返回最多 4 个事件 |
| `is_calibrated()` | `fn -> bool` | 基线校准是否完成 |
| `frame_count()` | `fn -> u32` | 处理的总帧数 |

#### 发出的事件

| 事件 ID | 常量 | 触发时机 |
|---------|------|----------|
| 210 | `EVENT_PERIMETER_BREACH` | 任何区域的显著扰动（值 = 能量分数） |
| 211 | `EVENT_APPROACH_DETECTED` | 突破区域的能量趋势上升（值 = 区域索引） |
| 212 | `EVENT_DEPARTURE_DETECTED` | 区域的能量趋势下降（值 = 区域索引） |
| 213 | `EVENT_ZONE_TRANSITION` | 移动从一个区域转移到另一个区域（值 = `from*10 + to`） |

#### 配置

| 参数 | 默认值 | 范围 | 描述 |
|------|--------|------|------|
| `BASELINE_FRAMES` | 100 | 60-200 | 校准帧数 (20 Hz 下 5 秒) |
| `BREACH_GRADIENT_THRESH` | 0.6 | 0.3-1.5 | 突破的相位梯度 (rad/子载波) |
| `VARIANCE_RATIO_THRESH` | 2.5 | 1.5-5.0 | 扰动的基线方差比率 |
| `DIRECTION_DEBOUNCE` | 3 | 2-8 | 方向确认的连续趋势帧数 |
| `COOLDOWN` | 40 | 20-100 | 相同类型事件之间的帧数 (20 Hz 下 2 秒) |
| `HISTORY_LEN` | 8 | 4-16 | 趋势估计的能量历史缓冲区 |
| `MAX_ZONES` | 4 | 2-4 | 周界区域数量 |

#### 使用示例

```rust
use wifi_densepose_wasm_edge::sec_perimeter_breach::*;

let mut detector = PerimeterBreachDetector::new();

// 输入 CSI 帧（相位、幅度、方差数组、运动能量标量）
let events = detector.process_frame(&phases, &amplitudes, &variance, motion_energy);

for &(event_id, value) in events {
    match event_id {
        EVENT_PERIMETER_BREACH => {
            // value = 能量分数（越高越严重）
            log!("检测到突破，能量={:.2}", value);
        }
        EVENT_APPROACH_DETECTED => {
            // value = 区域索引 (0-3)
            log!("区域 {} 有接近", value as u32);
        }
        EVENT_ZONE_TRANSITION => {
            // value 编码为 from*10 + to
            let from = (value as u32) / 10;
            let to = (value as u32) % 10;
            log!("从区域 {} 移动到区域 {}", from, to);
        }
        _ => {}
    }
}
```

#### 教程：设置 4 区域周界系统

1. **传感器放置**：将 ESP32-S3 安装在监控边界的中心（例如，仓库入口、属性线）。WiFi AP 应位于对面，使传感链路穿过所有 4 个区域。

2. **区域映射**：子载波平均分配到 4 个区域。对于 32 个子载波：
   - 区域 0：子载波 0-7（最靠近 ESP32）
   - 区域 1：子载波 8-15
   - 区域 2：子载波 16-23
   - 区域 3：子载波 24-31（最靠近 AP）

3. **校准**：在监控区域无人的情况下开启系统。等待 5 秒（100 帧）完成校准。`is_calibrated()` 返回 `true`。

4. **警报集成**：将事件转发到安全系统：
   - `EVENT_PERIMETER_BREACH` (210) -> 触发警笛/摄像头录制
   - `EVENT_APPROACH_DETECTED` (211) -> 预警报：有人接近
   - `EVENT_ZONE_TRANSITION` (213) -> 跟踪通过区域的移动方向

5. **调优**：如果在多风或高流量环境中出现误报，增加 `BREACH_GRADIENT_THRESH` 和 `VARIANCE_RATIO_THRESH`。如果漏检，减少这些值。

---

### 隐藏金属物体检测 (`sec_weapon_detect.rs`)

**功能**：检测通过传感区域的人携带的隐藏金属物体（刀具、枪支、工具）。金属的 RF 反射率显著高于人体组织，产生特征性的幅度方差与相位方差比率。

**工作原理**：在校准期间（空房间 100 帧），检测器使用在线方差累积计算每个子载波的基线幅度和相位方差。校准后，运行中的 Welford 统计实时跟踪幅度和相位方差。计算所有子载波上运行幅度方差与运行相位方差的比率。金属产生高比率（幅度因镜面反射而剧烈波动，而相位变化小于漫射组织）。

应用两个阈值：
- **金属异常**（比率 > 4.0，去抖动 4 帧）：一般金属物体检测。
- **武器警报**（比率 > 8.0，去抖动 6 帧）：高反射率警报，用于较大金属块。

检测需要 `presence >= 1` 和 `motion_energy >= 0.5` 以避免环境噪声引起的误报。

**重要**：此模块为研究级和实验性。它需要每个环境的校准，不应作为唯一的安全措施。

#### API

| 项 | 类型 | 描述 |
|----|------|------|
| `WeaponDetector::new()` | `const fn` | 创建未校准的检测器 |
| `process_frame(phases, amplitudes, variance, motion_energy, presence)` | `fn` | 处理一帧，返回最多 3 个事件 |
| `is_calibrated()` | `fn -> bool` | 基线校准是否完成 |
| `frame_count()` | `fn -> u32` | 处理的总帧数 |

#### 发出的事件

| 事件 ID | 常量 | 触发时机 |
|---------|------|----------|
| 220 | `EVENT_METAL_ANOMALY` | 检测到金属物体特征（值 = 幅度/相位比率） |
| 221 | `EVENT_WEAPON_ALERT` | 高反射率金属特征（值 = 幅度/相位比率） |
| 222 | `EVENT_CALIBRATION_NEEDED` | 基线漂移超过阈值（值 = 最大漂移比率） |

#### 配置

| 参数 | 默认值 | 范围 | 描述 |
|------|--------|------|------|
| `BASELINE_FRAMES` | 100 | 60-200 | 校准帧数（空房间，20 Hz 下 5 秒） |
| `METAL_RATIO_THRESH` | 4.0 | 2.0-8.0 | 金属检测的幅度/相位方差比率 |
| `WEAPON_RATIO_THRESH` | 8.0 | 5.0-15.0 | 武器级警报的比率 |
| `MIN_MOTION_ENERGY` | 0.5 | 0.2-2.0 | 视为有效检测的最小运动 |
| `METAL_DEBOUNCE` | 4 | 2-10 | 金属异常的连续帧数 |
| `WEAPON_DEBOUNCE` | 6 | 3-12 | 武器警报的连续帧数 |
| `COOLDOWN` | 60 | 20-120 | 事件之间的帧数（20 Hz 下 3 秒） |
| `RECALIB_DRIFT_THRESH` | 3.0 | 2.0-5.0 | 触发重新校准警报的漂移比率 |

#### 使用示例

```rust
use wifi_densepose_wasm_edge::sec_weapon_detect::*;

let mut detector = WeaponDetector::new();

// 在空房间校准（100 帧）
for _ in 0..100 {
    detector.process_frame(&phases, &amplitudes, &variance, 0.0, 0);
}
assert!(detector.is_calibrated());

// 正常操作：人走过
let events = detector.process_frame(&phases, &amplitudes, &variance, motion_energy, presence);

for &(event_id, value) in events {
    match event_id {
        EVENT_METAL_ANOMALY => {
            log!("检测到金属，比率={:.1}", value);
        }
        EVENT_WEAPON_ALERT => {
            log!("武器警报，比率={:.1}", value);
            // 触发安全响应
        }
        EVENT_CALIBRATION_NEEDED => {
            log!("环境变化，建议重新校准");
        }
        _ => {}
    }
}
```

---

### 尾随检测 (`sec_tailgating.rs`)

**功能**：检测门口的尾随行为——两人或多人快速连续通过。单次授权通行产生一个平滑的能量峰值；紧随其后的尾随者在可配置窗口（默认 3 秒）内产生第二个峰值。

**工作原理**：检测器通过 3 状态机使用运动能量峰值的时间聚类：

1. **Idle**：等待运动能量超过自适应阈值。
2. **InPeak**：跟踪活动峰值。记录峰值最大能量和持续时间。当能量降至峰值最大值的 30% 以下时，峰值结束。噪声尖峰（持续时间短于 3 帧的峰值）被丢弃。
3. **Watching**：峰值结束，监控尾随窗口（60 帧 = 3 秒）内的另一个峰值。如果另一个峰值到达，它会转换回 InPeak。当窗口过期时，它评估：1 个峰值 = 单次通行，2+ 个峰值 = 尾随。

阈值通过方差的指数移动平均适应环境噪声。

#### 状态机

```
Idle ----[energy > threshold]----> InPeak
                                      |
                          [energy < 30% of peak max]
                                      |
             [peak too short]         v
Idle <------------------------- InPeak end
                                      |
                          [peak valid (>= 3 frames)]
                                      v
                                  Watching
                                   /    \
              [new peak starts]   /      \  [window expires]
                                 v        v
                              InPeak    Evaluate
                                        /     \
                               [1 peak]        [2+ peaks]
                                  |                |
                          SINGLE_PASSAGE    TAILGATE_DETECTED
                                  |           + MULTI_PASSAGE
                                  v                v
                                Idle             Idle
```

#### API

| 项 | 类型 | 描述 |
|----|------|------|
| `TailgateDetector::new()` | `const fn` | 创建检测器 |
| `process_frame(motion_energy, presence, n_persons, variance)` | `fn` | 处理一帧，返回最多 3 个事件 |
| `frame_count()` | `fn -> u32` | 处理的总帧数 |
| `tailgate_count()` | `fn -> u32` | 检测到的尾随事件总数 |
| `single_passages()` | `fn -> u32` | 记录的单次通行总数 |

#### 发出的事件

| 事件 ID | 常量 | 触发时机 |
|---------|------|----------|
| 230 | `EVENT_TAILGATE_DETECTED` | 窗口内有两个或更多峰值（值 = 峰值计数） |
| 231 | `EVENT_SINGLE_PASSAGE` | 单个峰值后跟安静窗口（值 = 峰值能量） |
| 232 | `EVENT_MULTI_PASSAGE` | 窗口内有三个或更多峰值（值 = 峰值计数） |

#### 配置

| 参数 | 默认值 | 范围 | 描述 |
|------|--------|------|------|
| `ENERGY_PEAK_THRESH` | 2.0 | 1.0-5.0 | 峰值开始的运动能量阈值 |
| `ENERGY_VALLEY_FRAC` | 0.3 | 0.1-0.5 | 峰值结束的峰值最大值分数 |
| `TAILGATE_WINDOW` | 60 | 20-120 | 尾随的最大峰间间隔（20 Hz 下 3 秒） |
| `MIN_PEAK_ENERGY` | 1.5 | 0.5-3.0 | 有效通行的最小峰值能量 |
| `COOLDOWN` | 100 | 40-200 | 事件之间的帧数（20 Hz 下 5 秒） |
| `MIN_PEAK_FRAMES` | 3 | 2-10 | 过滤噪声尖峰的最小峰值持续时间 |
| `MAX_PEAKS` | 8 | 4-16 | 一个窗口中跟踪的最大峰值数 |

#### 使用示例

```rust
use wifi_densepose_wasm_edge::sec_tailgating::*;

let mut detector = TailgateDetector::new();

// 处理来自主机的帧
let events = detector.process_frame(motion_energy, presence, n_persons, variance_mean);

for &(event_id, value) in events {
    match event_id {
        EVENT_TAILGATE_DETECTED => {
            log!("尾随：{} 人快速连续通过", value as u32);
            // 锁门/警报安全
        }
        EVENT_SINGLE_PASSAGE => {
            log!("正常通行，能量={:.2}", value);
        }
        EVENT_MULTI_PASSAGE => {
            log!("多人通行：{} 人", value as u32);
        }
        _ => {}
    }
}
```

---

### 徘徊检测 (`sec_loitering.rs`)

**功能**：检测监控区域内的长时间静止存在。区分正常通过的人和长时间站立的人（徘徊）。默认停留阈值为 5 分钟。

**工作原理**：使用 4 状态机跟踪存在持续时间和运动水平。只有静止帧（运动能量低于 0.5）计入停留阈值——积极通过的人不会累积徘徊时间。退出冷却（30 秒）防止短暂信号丢失或遮挡导致的误报"徘徊结束"事件。

#### 状态机

```
Absent --[presence + no post_end cooldown]--> Entering
                                                  |
                                   [60 frames with presence]
                                                  |
            [absence before 60]                   v
Absent <------------------------------ Entering confirmed
                                                  |
                                                  v
                                              Present
                                             /       \
                          [6000 stationary   /         \ [absent > 300
                            frames]         /           \  frames]
                                           v             v
                                      Loitering       Absent
                                       /     \
                    [presence continues]       [absent >= 600 frames]
                              |                        |
                     LOITERING_ONGOING          LOITERING_END
                     (every 600 frames)                |
                              |                        v
                              v                     Absent
                          Loitering              (post_end_cd = 200)
```

#### API

| 项 | 类型 | 描述 |
|----|------|------|
| `LoiteringDetector::new()` | `const fn` | 创建处于 Absent 状态的检测器 |
| `process_frame(presence, motion_energy)` | `fn` | 处理一帧，返回最多 2 个事件 |
| `state()` | `fn -> LoiterState` | 当前状态 (Absent/Entering/Present/Loitering) |
| `frame_count()` | `fn -> u32` | 处理的总帧数 |
| `loiter_count()` | `fn -> u32` | 徘徊事件总数 |
| `dwell_frames()` | `fn -> u32` | 当前累积的静止停留帧数 |

#### 发出的事件

| 事件 ID | 常量 | 触发时机 |
|---------|------|----------|
| 240 | `EVENT_LOITERING_START` | 超过停留阈值（值 = 停留时间，单位：秒） |
| 241 | `EVENT_LOITERING_ONGOING` | 徘徊期间的定期报告（值 = 总停留秒数） |
| 242 | `EVENT_LOITERING_END` | 徘徊者在退出冷却后离开（值 = 总停留秒数） |

#### 配置

| 参数 | 默认值 | 范围 | 描述 |
|------|--------|------|------|
| `ENTER_CONFIRM_FRAMES` | 60 | 20-120 | 存在确认（20 Hz 下 3 秒） |
| `DWELL_THRESHOLD` | 6000 | 1200-12000 | 徘徊的静止帧数（20 Hz 下 5 分钟） |
| `EXIT_COOLDOWN` | 600 | 200-1200 | 结束徘徊前的缺席帧数（20 Hz 下 30 秒） |
| `STATIONARY_MOTION_THRESH` | 0.5 | 0.2-1.5 | 人静止的运动能量阈值 |
| `ONGOING_REPORT_INTERVAL` | 600 | 200-1200 | 持续报告之间的帧数（20 Hz 下 30 秒） |
| `POST_END_COOLDOWN` | 200 | 100-600 | 结束后重新检测前的冷却（20 Hz 下 10 秒） |

#### 使用示例

```rust
use wifi_densepose_wasm_edge::sec_loitering::*;

let mut detector = LoiteringDetector::new();

let events = detector.process_frame(presence, motion_energy);

for &(event_id, value) in events {
    match event_id {
        EVENT_LOITERING_START => {
            log!("{}秒后开始徘徊", value);
            // 警报安全
        }
        EVENT_LOITERING_ONGOING => {
            log!("仍在徘徊，总计{}秒", value);
        }
        EVENT_LOITERING_END => {
            log!("徘徊者离开，总计{}秒", value);
        }
        _ => {}
    }
}

// 以编程方式检查状态
if detector.state() == LoiterState::Loitering {
    // 持续监控动作
}
```

---

### 恐慌/异常运动检测 (`sec_panic_motion.rs`)

**功能**：检测三类与遇险相关的运动：
1. **恐慌**：不稳定、高加速度运动，方向快速随机变化（例如，有人挣扎、被攻击）。
2. **挣扎**：高加速度，中等能量，有一些方向变化（例如，身体冲突、试图挣脱）。
3. **逃跑**：持续高能量，低熵——向一个方向奔跑。

**工作原理**：维护 100 帧（5 秒）的运动能量和方差值的循环缓冲区。每帧计算窗口级统计：

- **平均加速度**：窗口内运动能量变化率的平均绝对值。高加速度 = 不稳定、不可预测的运动。
- **熵代理**：方向反转的帧分数（能量从增加变为减少或反之）。高熵 = 混乱运动。
- **高加速度分数**：超过 `JERK_THRESH` 的帧间加速度分数。确保高平均值不是来自单个尖峰。

检测逻辑：
- **恐慌** = `mean_jerk > 2.0` 且 `entropy > 0.35` 且 `high_jerk_frac > 0.3`
- **挣扎** = `mean_jerk > 1.5` 且 `energy in [1.0, 5.0)` 且 `entropy > 0.175` 且非恐慌
- **逃跑** = `mean_energy > 5.0` 且 `mean_jerk > 0.05` 且 `entropy < 0.25` 且非恐慌

#### API

| 项 | 类型 | 描述 |
|----|------|------|
| `PanicMotionDetector::new()` | `const fn` | 创建检测器 |
| `process_frame(motion_energy, variance_mean, phase_mean, presence)` | `fn` | 处理一帧，返回最多 3 个事件 |
| `frame_count()` | `fn -> u32` | 处理的总帧数 |
| `panic_count()` | `fn -> u32` | 检测到的恐慌事件总数 |

#### 发出的事件

| 事件 ID | 常量 | 触发时机 |
|---------|------|----------|
| 250 | `EVENT_PANIC_DETECTED` | 不稳定高加速度 + 高熵运动（值 = 严重程度 0-10） |
| 251 | `EVENT_STRUGGLE_PATTERN` | 中等能量下的高加速度（值 = 平均加速度） |
| 252 | `EVENT_FLEEING_DETECTED` | 持续高能量定向运动（值 = 平均能量） |

#### 配置

| 参数 | 默认值 | 范围 | 描述 |
|------|--------|------|------|
| `WINDOW` | 100 | 40-200 | 分析窗口大小（20 Hz 下 5 秒） |
| `JERK_THRESH` | 2.0 | 1.0-4.0 | 恐慌的每帧加速度阈值 |
| `ENTROPY_THRESH` | 0.35 | 0.2-0.6 | 方向反转率阈值 |
| `MIN_MOTION` | 1.0 | 0.3-2.0 | 最小运动能量（忽略空闲） |
| `TRIGGER_FRAC` | 0.3 | 0.2-0.5 | 超过阈值的窗口帧分数 |
| `COOLDOWN` | 100 | 40-200 | 事件之间的帧数（20 Hz 下 5 秒） |
| `FLEE_ENERGY_THRESH` | 5.0 | 3.0-10.0 | 逃跑检测的最小能量 |
| `FLEE_JERK_THRESH` | 0.05 | 0.01-0.5 | 逃跑的最小加速度（高于噪声底） |
| `FLEE_MAX_ENTROPY` | 0.25 | 0.1-0.4 | 逃跑的最大熵（定向运动） |
| `STRUGGLE_JERK_THRESH` | 1.5 | 0.8-3.0 | 挣扎模式的最小平均加速度 |

#### 使用示例

```rust
use wifi_densepose_wasm_edge::sec_panic_motion::*;

let mut detector = PanicMotionDetector::new();

let events = detector.process_frame(motion_energy, variance_mean, phase_mean, presence);

for &(event_id, value) in events {
    match event_id {
        EVENT_PANIC_DETECTED => {
            log!("恐慌：严重程度={:.1}", value);
            // 立即安全调度
        }
        EVENT_STRUGGLE_PATTERN => {
            log!("检测到挣扎，加速度={:.2}", value);
            // 调查
        }
        EVENT_FLEEING_DETECTED => {
            log!("有人逃跑，能量={:.1}", value);
            // 通过周界模块跟踪方向
        }
        _ => {}
    }
}
```

---

## 事件 ID 注册表（安全范围 200-299）

| 范围 | 模块 | 事件 |
|------|------|------|
| 200-203 | `intrusion.rs` | INTRUSION_ALERT, INTRUSION_ZONE, INTRUSION_ARMED, INTRUSION_DISARMED |
| 210-213 | `sec_perimeter_breach.rs` | PERIMETER_BREACH, APPROACH_DETECTED, DEPARTURE_DETECTED, ZONE_TRANSITION |
| 220-222 | `sec_weapon_detect.rs` | METAL_ANOMALY, WEAPON_ALERT, CALIBRATION_NEEDED |
| 230-232 | `sec_tailgating.rs` | TAILGATE_DETECTED, SINGLE_PASSAGE, MULTI_PASSAGE |
| 240-242 | `sec_loitering.rs` | LOITERING_START, LOITERING_ONGOING, LOITERING_END |
| 250-252 | `sec_panic_motion.rs` | PANIC_DETECTED, STRUGGLE_PATTERN, FLEEING_DETECTED |
| 253-299 | | 预留用于未来安全模块 |

---

## 测试

```bash
# 运行所有安全模块测试（需要 std 特性）
cd rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge
cargo test --features std -- sec_ intrusion
```

### 测试覆盖摘要

| 模块 | 测试 | 覆盖说明 |
|------|------|----------|
| `intrusion.rs` | 4 | 初始化、校准、布防、入侵检测 |
| `sec_perimeter_breach.rs` | 6 | 初始化、校准、突破、区域转换、接近、安静信号 |
| `sec_weapon_detect.rs` | 6 | 初始化、校准、无存在、金属异常、正常人、漂移重新校准 |
| `sec_tailgating.rs` | 7 | 初始化、单次通行、尾随、宽间距、噪声尖峰、多人通行、低能量 |
| `sec_loitering.rs` | 7 | 初始化、进入、取消、徘徊开始/进行中/结束、短暂缺席、移动的人 |
| `sec_panic_motion.rs` | 7 | 初始化、窗口填充、平静运动、恐慌、无存在、逃跑、挣扎、低运动 |

---

## 部署考虑

### 每个传感器的覆盖区域

每个带有 WiFi AP 链接的 ESP32-S3 覆盖单个传感路径。覆盖区域取决于：
- **距离**：ESP32 与 AP 之间 1-10 米（最佳：室内 3-5 米）。
- **宽度**：第一菲涅尔区宽度——5 GHz 下约 0.5-1.5 米。
- **穿透墙壁**：WiFi CSI 穿透干墙和木材，但通过混凝土/金属时衰减。信号质量在一堵墙后下降。

### 多传感器协调

对于更大的区域，部署多个 ESP32 传感器组成网格：
- 每个传感器独立运行自己的 WASM 模块实例。
- 聚合服务器 (`wifi-densepose-sensing-server`) 收集所有传感器的事件。
- 跨传感器关联（例如，跟踪人员跨区域移动）在服务器端完成，而非设备端。
- 使用周界突破的 `EVENT_ZONE_TRANSITION` (213) 关联相邻传感器之间的移动。

### 误报减少

1. **校准**：始终在预期的操作条件下校准（一天中的时间、HVAC 状态、门位置）。
2. **阈值调优**：从默认值开始，如果出现误报则增加阈值，如果漏检则减少阈值。
3. **去抖动调优**：在高噪声环境（靠近 HVAC 通风口、开窗）中增加去抖动计数器。
4. **多模块关联**：需要 2+ 模块一致同意后才触发高严重性响应。例如：周界突破 + 恐慌运动 = 确认威胁；仅周界突破 = 调查。
5. **时间过滤**：服务器端逻辑可以在营业时间抑制某些事件（例如，白天单次通行是正常的）。

### 与现有安全系统集成

- **事件转发**：事件通过 `csi_emit_event()` 发送到主机固件，后者将它们打包成 UDP 数据包发送到聚合器。
- **REST API**：传感服务器在 `/api/v1/sensing/events` 暴露事件，用于与 SIEM、VMS 或访问控制系统集成。
- **Webhook 支持**：配置服务器将事件负载 POST 到外部端点。
- **MQTT**：对于 IoT 集成，事件可以发布到 MQTT 主题（每个事件类型或每个传感器一个）。

### ESP32-S3 上的资源使用

| 资源 | 预算 | 说明 |
|------|------|------|
| RAM | ~2-4 KB/模块 | 静态缓冲区，无堆分配 |
| CPU | <5 ms/帧 (S 预算) | 远低于 20 Hz 下的 50 ms 帧预算 |
| Flash | ~3-8 KB WASM/模块 | 使用 `opt-level = "s"` 和 LTO 编译 |
| 总计（6 个模块） | ~15-25 KB RAM, ~30 KB Flash | 适合 925 KB 固件，有足够余量 |