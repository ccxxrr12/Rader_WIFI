# 异国情调与研究模块 -- WiFi-DensePose 边缘智能

> 推动 WiFi 信号检测边界的实验性传感应用。从非接触式睡眠分期到手语识别，这些模块探索 RF 传感的新用途。有些是高度实验性的 -- 标记了它们的成熟度级别。

## 成熟度级别

- **已验证**：基于已发表的研究，结果已验证
- **实验性**：工作实现，需要真实世界验证
- **研究**：概念验证，探索性

## 概述

| 模块 | 文件 | 功能 | 事件 ID | 成熟度 |
|------|------|------|---------|--------|
| 睡眠阶段分类 | `exo_dream_stage.rs` | 从呼吸 + 微运动中分类睡眠阶段 | 600-603 | 实验性 |
| 情绪检测 | `exo_emotion_detect.rs` | 从生理代理估计唤醒/压力 | 610-613 | 研究 |
| 手语识别 | `exo_gesture_language.rs` | 基于 DTW 的手/臂 CSI 模式字母识别 | 620-623 | 研究 |
| 音乐指挥跟踪 | `exo_music_conductor.rs` | 从指挥动作中提取 tempo、节拍、力度 | 630-634 | 研究 |
| 植物生长检测 | `exo_plant_growth.rs` | 检测植物生长漂移和昼夜叶片运动 | 640-643 | 研究 |
| 幽灵猎人（异常） | `exo_ghost_hunter.rs` | 分类空房间中无法解释的扰动 | 650-653 | 实验性 |
| 降雨检测 | `exo_rain_detect.rs` | 从宽带结构振动检测降雨 | 660-662 | 实验性 |
| 呼吸同步 | `exo_breathing_sync.rs` | 检测多人之间的相位锁定呼吸 | 670-673 | 研究 |
| 时间晶体检测 | `exo_time_crystal.rs` | 检测周期倍增和时间协调 | 680-682 | 研究 |
| 双曲空间嵌入 | `exo_hyperbolic_space.rs` | 具有层次结构的 Poincare 球位置分类 | 685-687 | 研究 |

## 架构

所有模块共享这些设计约束：

- **`no_std`** -- 无堆分配，在 ESP32-S3 上的 WASM3 解释器上运行
- **`const fn new()`** -- 所有状态都是栈分配且可 const 构造的
- **静态事件缓冲区** -- 事件通过静态数组返回 `&[(i32, f32)]`（每帧最多 3-5 个事件）
- **预算感知** -- 每个模块声明其每帧时间预算 (L/S/H)
- **帧率** -- 所有模块假设主机第 2 层 DSP 的 20 Hz CSI 帧率

来自 `vendor_common.rs` 的共享工具：
- `CircularBuffer<N>` -- 固定大小的环形缓冲区，O(1) 推送和索引访问
- `Ema` -- 具有可配置 alpha 的指数移动平均
- `WelfordStats` -- 在线均值/方差计算（Welford 算法）

---

## 模块

### 睡眠阶段分类 (`exo_dream_stage.rs`)

**功能**：从呼吸模式、心率变异性和微运动中分类睡眠阶段（清醒、NREM 浅睡、NREM 深睡、REM）-- 无需接触人。

**成熟度**：实验性

**研究基础**：基于 WiFi 的非接触式睡眠监测已在同行评审研究中得到证明。参见 [1] 关于使用呼吸模式和身体运动的基于 RF 的睡眠分期。

#### 工作原理

该模块使用具有滞后的四特征状态机：

1. **呼吸规律性** -- 64 样本呼吸 BPM 窗口的变异系数 (CV)。低 CV (<0.08) 表示深睡；高 CV (>0.20) 表示 REM 或清醒。

2. **运动能量** -- 来自主机第 2 层的 EMA 平滑运动。低于 0.15 = 睡眠状；高于 0.5 = 清醒。

3. **心率变异性 (HRV)** -- 最近 HR BPM 值的方差。高 HRV (>8.0) 与 REM 相关；非常低 HRV (<2.0) 与深睡相关。

4. **相位微运动** -- 相位信号的高通能量（连续差异）。捕获 REM 期间的肌肉张力中断。

阶段转换需要候选阶段的 10 个连续帧（滞后），防止抖动分类。

#### 睡眠阶段

| 阶段 | 代码 | 条件 |
|------|------|------|
| 清醒 | 0 | 无存在，高运动，或中等运动 + 不规则呼吸 |
| NREM 浅睡 | 1 | 低运动，中等呼吸规律性，默认睡眠状态 |
| NREM 深睡 | 2 | 极低运动，非常规律的呼吸 (CV < 0.08)，低 HRV (< 2.0) |
| REM | 3 | 极低运动，高 HRV (> 8.0)，微运动高于阈值 |

#### 事件

| 事件 | ID | 值 | 频率 |
|------|-----|-----|------|
| `SLEEP_STAGE` | 600 | 0-3 (清醒/浅睡/深睡/REM) | 每帧（预热后） |
| `SLEEP_QUALITY` | 601 | 睡眠效率 [0, 100] | 每 20 帧 |
| `REM_EPISODE` | 602 | 当前/最后 REM 发作长度（帧） | REM 活跃或刚结束时 |
| `DEEP_SLEEP_RATIO` | 603 | 深睡/总睡眠比率 [0, 1] | 每 20 帧 |

#### 质量指标

- **效率** = (睡眠帧 / 总帧) * 100
- **深睡比率** = 深睡帧 / 睡眠帧
- **REM 比率** = REM 帧 / 睡眠帧

#### 配置常量

| 参数 | 默认值 | 描述 |
|------|--------|------|
| `BREATH_HIST_LEN` | 64 | 呼吸 BPM 历史的滚动窗口 |
| `HR_HIST_LEN` | 64 | 心率历史的滚动窗口 |
| `PHASE_BUF_LEN` | 128 | 微运动检测的相位缓冲区 |
| `MOTION_ALPHA` | 0.1 | 运动 EMA 平滑因子 |
| `MIN_WARMUP` | 40 | 分类开始前的最小帧数 |
| `STAGE_HYSTERESIS` | 10 | 阶段转换所需的连续帧数 |

#### API

```rust
let mut detector = DreamStageDetector::new();
let events = detector.process_frame(
    breathing_bpm,   // f32: 来自第 2 层 DSP
    heart_rate_bpm,  // f32: 来自第 2 层 DSP
    motion_energy,   // f32: 来自第 2 层 DSP
    phase,           // f32: 代表性子载波相位
    variance,        // f32: 代表性子载波方差
    presence,        // i32: 1 如果检测到人，否则 0
);
// events: &[(i32, f32)] -- 事件 ID + 值对

let stage = detector.stage();          // SleepStage 枚举
let eff = detector.efficiency();       // f32 [0, 100]
let deep = detector.deep_ratio();      // f32 [0, 1]
let rem = detector.rem_ratio();        // f32 [0, 1]
```

#### 教程：设置非接触式睡眠跟踪

1. **放置**：安装 WiFi 发射器和接收器，使视线在胸部高度穿过床。将 ESP32 节点放置在距离床 1-3 米处。

2. **校准**：在期望有效阶段分类之前，让系统运行 40+ 帧（20 Hz 时为 2 秒），人在床上。

3. **解释结果**：监控 `SLEEP_STAGE` 事件。健康的睡眠周期通过浅睡 -> 深睡 -> 浅睡 -> REM 进展，以 ~90 分钟周期重复。`SLEEP_QUALITY` 事件 (601) 给出整体效率百分比 -- 85% 以上被认为是良好的。

4. **限制**：该模块要求第 2 层 DSP 提供有效的 `breathing_bpm` 和 `heart_rate_bpm`。如果人离 WiFi 路径太远或在厚墙后面，这些生命体征可能无法检测到。

---

### 情绪检测 (`exo_emotion_detect.rs`)

**功能**：从 WiFi CSI 估计连续唤醒水平和离散压力/平静/激动状态，无需摄像头或麦克风。使用生理代理：呼吸率、心率、坐立不安和相位方差。

**成熟度**：研究

**限制**：此模块不直接检测情绪。它检测生理唤醒 -- 心率升高、呼吸加快和坐立不安。这些与压力和焦虑相关，但也可能由运动、咖啡因或兴奋引起。该模块无法区分积极和消极唤醒。它是探索通过 RF 进行情感传感可行性的研究工具，不是临床仪器。

#### 工作原理

唤醒水平是四个归一化特征的加权和：

| 特征 | 权重 | 来源 | 分数 = 0 | 分数 = 1 |
|------|------|------|----------|----------|
| 呼吸率 | 0.30 | 主机第 2 层 | 6-10 BPM（平静） | >= 20 BPM（压力） |
| 心率 | 0.20 | 主机第 2 层 | <= 70 BPM（基线） | 100+ BPM（升高） |
| 坐立不安能量 | 0.30 | 运动连续差异 | 无坐立不安 | 持续坐立不安 |
| 相位方差 | 0.20 | 子载波方差 | 稳定信号 | 剧烈身体运动 |

压力指数使用不同权重 (0.4/0.3/0.2/0.1)，强调呼吸和心率而非坐立不安。

#### 事件

| 事件 | ID | 值 | 频率 |
|------|-----|-----|------|
| `AROUSAL_LEVEL` | 610 | 连续唤醒 [0, 1] | 每帧 |
| `STRESS_INDEX` | 611 | 压力指数 [0, 1] | 每帧 |
| `CALM_DETECTED` | 612 | 1.0 当检测到平静状态 | 条件满足时 |
| `AGITATION_DETECTED` | 613 | 1.0 当检测到激动 | 条件满足时 |

#### 离散状态检测

- **平静**：唤醒 < 0.25 AND 运动 < 0.08 AND 呼吸 6-10 BPM AND 呼吸 CV < 0.08
- **激动**：唤醒 > 0.75 AND (运动 > 0.6 OR 坐立不安 > 0.15 OR 呼吸 CV > 0.25)

#### API

```rust
let mut detector = EmotionDetector::new();
let events = detector.process_frame(
    breathing_bpm,   // f32
    heart_rate_bpm,  // f32
    motion_energy,   // f32
    phase,           // f32（当前实现未使用）
    variance,        // f32
);

let arousal = detector.arousal();      // f32 [0, 1]
let stress = detector.stress_index();  // f32 [0, 1]
let calm = detector.is_calm();         // bool
let agitated = detector.is_agitated(); // bool
```

---

### 手语识别 (`exo_gesture_language.rs`)

**功能**：使用 WiFi CSI 相位和幅度模式将手/臂运动分类为手语字母组。在紧凑的 6D 特征序列上使用 DTW（动态时间规整）模板匹配。

**成熟度**：研究

**限制**：通过 WiFi 识别完整的 26 字母 ASL 字母表极具挑战性。此模块提供概念验证框架。真实世界的准确性在很大程度上取决于：(a) 模板质量和多样性，(b) 环境稳定性，(c) 人与人之间的变异。预期概念验证准确性，而非生产 ASL 翻译。

#### 工作原理

1. **特征提取**：每帧计算 6 个特征：平均相位、相位散布、平均幅度、幅度散布、运动能量、方差。这些累积在手势窗口（最多 32 帧）中。

2. **手势分割**：活动手势由暂停（低运动 15+ 帧）界定。检测到暂停时，累积的手势窗口与模板匹配。

3. **DTW 匹配**：每个模板是参考特征序列。使用 Sakoe-Chiba 带（宽度=4）的多变量 DTW 计算对齐距离。接受阈值以下（0.5）的最佳匹配。

4. **词边界**：延长暂停（15+ 低运动帧）发出词边界事件。

#### 事件

| 事件 | ID | 值 | 频率 |
|------|-----|-----|------|
| `LETTER_RECOGNIZED` | 620 | 字母索引 (0=A, ..., 25=Z) | 暂停后匹配时 |
| `LETTER_CONFIDENCE` | 621 | 逆 DTW 距离 [0, 1] | 与识别的字母一起 |
| `WORD_BOUNDARY` | 622 | 1.0 | 延长暂停后 |
| `GESTURE_REJECTED` | 623 | 1.0 | 手势不匹配时 |

#### API

```rust
let mut detector = GestureLanguageDetector::new();

// 加载模板（识别工作前必需）
detector.load_synthetic_templates();  // 用于测试的 26 个斜坡模式模板
// 或加载自定义模板：
detector.set_template(0, &features_for_letter_a);  // 0 = 'A'

let events = detector.process_frame(
    &phases,         // &[f32]: 每个子载波的相位
    &amplitudes,     // &[f32]: 每个子载波的幅度
    variance,        // f32
    motion_energy,   // f32
    presence,        // i32
);
```

---

### 音乐指挥跟踪 (`exo_music_conductor.rs`)

**功能**：从 WiFi CSI 运动签名中提取音乐指挥参数： tempo (BPM)、节拍位置（4/4 拍中的 1-4）、动态水平（MIDI 力度 0-127）和特殊手势（截断和延长记号）。

**成熟度**：研究

**研究基础**：通过 WiFi CSI 进行手势跟踪已被证明可用于粗略的手臂运动。指挥跟踪将此扩展到周期性节奏运动分析。

#### 工作原理

1. **Tempo 检测**：128 点运动能量缓冲区在滞后 4-64 处的自相关。主导峰值确定周期，转换为 BPM：`BPM = 60 * 20 / lag`（20 Hz 帧率）。有效范围：30-240 BPM。

2. **节拍位置**：相对于检测到的周期的模块化帧计数器映射到 4/4 拍中的节拍 1-4。

3. **动态水平**：运动能量相对于 EMA 平滑峰值，缩放到 MIDI 力度 [0, 127]。

4. **截断检测**：运动能量的急剧下降（比率 < 0.2 最近峰值），之前有高运动。

5. **延长记号检测**：持续低运动 (< 0.05) 10+ 连续帧。

#### 事件

| 事件 | ID | 值 | 频率 |
|------|-----|-----|------|
| `CONDUCTOR_BPM` | 630 | 检测到的 tempo（BPM） | Tempo 锁定后 |
| `BEAT_POSITION` | 631 | 节拍数 (1-4) | Tempo 锁定后 |
| `DYNAMIC_LEVEL` | 632 | MIDI 力度 [0, 127] | 每帧 |
| `GESTURE_CUTOFF` | 633 | 1.0 | 截断手势时 |
| `GESTURE_FERMATA` | 634 | 1.0 | 延长记号保持期间 |

#### API

```rust
let mut detector = MusicConductorDetector::new();
let events = detector.process_frame(
    phase,           // f32（未使用）
    amplitude,       // f32（未使用）
    motion_energy,   // f32: 来自第 2 层 DSP
    variance,        // f32（未使用）
);

let bpm = detector.tempo_bpm();        // f32
let fermata = detector.is_fermata();   // bool
let cutoff = detector.is_cutoff();     // bool
```

---

### 植物生长检测 (`exo_plant_growth.rs`)

**功能**：从数小时/天的微 CSI 变化中检测植物生长和叶片运动。植物在 CSI 幅度中引起极其缓慢、单调的漂移（生长）和昼夜相位振荡（昼夜叶片运动 -- 睡眠运动）。

**成熟度**：研究

**要求**：房间必须为空 (`presence == 0`)，以将植物规模的扰动与人类运动隔离。此模块设计用于长期监控（数小时到数天）。

#### 工作原理

- **生长率**：通过非常慢的 EWMA（alpha=0.0001，半衰期 ~175 秒）跟踪幅度基线的缓慢漂移。植物生长产生连续 ~0.01 dB/小时的幅度减少，因为新的叶片面积拦截 RF 能量。

- **昼夜相位**：在滚动窗口上跟踪相位 EWMA 的峰谷振荡。睡眠运动（夜间折叠）产生 ~24 小时振荡。

- **枯萎检测**：短期幅度上升超过基线（较少吸收），结合相位方差减少。

- **浇水事件**：突然的幅度下降（更多水 = 更多 RF 吸收），随后恢复。

#### 事件

| 事件 | ID | 值 | 频率 |
|------|-----|-----|------|
| `GROWTH_RATE` | 640 | 幅度漂移率（缩放） | 每 100 空房间帧 |
| `CIRCADIAN_PHASE` | 641 | 振荡幅度 [0, 1] | 检测到振荡时 |
| `WILT_DETECTED` | 642 | 1.0 | 看到枯萎特征时 |
| `WATERING_EVENT` | 643 | 1.0 | 看到浇水特征时 |

#### API

```rust
let mut detector = PlantGrowthDetector::new();
let events = detector.process_frame(
    &amplitudes,  // &[f32]: 每个子载波的幅度（最多 32）
    &phases,      // &[f32]: 每个子载波的相位（最多 32）
    &variance,    // &[f32]: 每个子载波的方差（最多 32）
    presence,     // i32: 0 = 空房间（检测所需）
);

let calibrated = detector.is_calibrated();  // true 在 MIN_EMPTY_FRAMES 后
let empty = detector.empty_frames();        // 空房间数据的帧数
```

---

### 幽灵猎人 -- 环境异常检测器 (`exo_ghost_hunter.rs`)

**功能**：当未检测到人类时监控 CSI 是否有任何高于噪声底的扰动。当房间应该为空但检测到 CSI 变化时，发生了无法解释的事情。通过其时间特征对异常进行分类。

**成熟度**：实验性

**实际应用**：尽管名称有趣，此模块有严肃的用途：检测 HVAC 压缩机循环、害虫/动物移动、结构沉降、气体泄漏（改变介电特性）、躲避主要存在检测器的隐藏入侵者，以及电磁干扰。

#### 异常分类

| 类别 | 代码 | 特征 | 典型来源 |
|------|------|------|----------|
| 脉冲性 | 1 | < 5 帧，尖锐瞬态 | 物体掉落，热裂纹 |
| 周期性 | 2 | 重复，可检测自相关峰值 | HVAC，电器，害虫移动 |
| 漂移 | 3 | 30+ 帧同符号幅度增量 | 温度变化，湿度，气体泄漏 |
| 随机 | 4 | 随机，无模式 | EMI，同信道 WiFi 干扰 |

#### 隐藏存在检测

子检测器在相位信号中寻找呼吸特征：通过滞后 5-15 处的自相关（20 Hz 帧率）在 0.2-2.0 Hz 处的周期性振荡。这可以检测躲避主要存在检测器的静止人员。

#### 事件

| 事件 | ID | 值 | 频率 |
|------|-----|-----|------|
| `ANOMALY_DETECTED` | 650 | 能量水平 [0, 1] | 异常活跃时 |
| `ANOMALY_CLASS` | 651 | 1-4（见上表） | 异常检测时 |
| `HIDDEN_PRESENCE` | 652 | 置信度 [0, 1] | 发现呼吸特征时 |
| `ENVIRONMENTAL_DRIFT` | 653 | 漂移幅度 | 检测到持续漂移时 |

#### API

```rust
let mut detector = GhostHunterDetector::new();
let events = detector.process_frame(
    &phases,         // &[f32]
    &amplitudes,     // &[f32]
    &variance,       // &[f32]
    presence,        // i32: 检测必须为 0
    motion_energy,   // f32
);

let class = detector.anomaly_class();                // AnomalyClass 枚举
let hidden = detector.hidden_presence_confidence();   // f32 [0, 1]
let energy = detector.anomaly_energy();               // f32
```

---

### 降雨检测 (`exo_rain_detect.rs`)

**功能**：从雨滴撞击建筑物表面引起的宽带 CSI 相位方差扰动中检测降雨。将强度分类为轻度、中度或重度。

**成熟度**：实验性

**研究基础**：撞击表面的雨滴产生宽带脉冲振动，通过建筑结构传播并调制 CSI 相位。这些通过其宽带性质（所有子载波组同等受影响）、随机时间和小幅度与人类运动区分。

#### 工作原理

1. **需要空房间** (`presence == 0`) 以避免与人类运动混淆。
2. **宽带标准**：计算每组方差比（短期 / 基线）。如果 >= 75% 的组（6/8）方差升高（比率 > 2.5x），则信号是宽带的 -- 与降雨一致。
3. **滞后状态机**：开始需要 10 个连续宽带帧；停止需要 20 个连续安静帧。
4. **强度分类**：基于高于基线的平滑 excess 能量。

#### 事件

| 事件 | ID | 值 | 频率 |
|------|-----|-----|------|
| `RAIN_ONSET` | 660 | 1.0 | 降雨开始时 |
| `RAIN_INTENSITY` | 661 | 1=轻度, 2=中度, 3=重度 | 降雨时 |
| `RAIN_CESSATION` | 662 | 1.0 | 降雨停止时 |

#### 强度阈值

| 级别 | 代码 | 能量范围 |
|------|------|----------|
| 无 | 0 | (未降雨) |
| 轻度 | 1 | energy < 0.3 |
| 中度 | 2 | 0.3 <= energy < 0.7 |
| 重度 | 3 | energy >= 0.7 |

#### API

```rust
let mut detector = RainDetector::new();
let events = detector.process_frame(
    &phases,      // &[f32]
    &variance,    // &[f32]
    &amplitudes,  // &[f32]
    presence,     // i32: 必须为 0
);

let raining = detector.is_raining();   // bool
let intensity = detector.intensity();  // RainIntensity 枚举
let energy = detector.energy();        // f32 [0, 1]
```

---

### 呼吸同步 (`exo_breathing_sync.rs`)

**功能**：检测多人呼吸模式同步时。通过子载波组分解提取每人呼吸成分，并计算成对归一化互相关。

**成熟度**：研究

**研究基础**：呼吸同步（人际生理同步）是夫妻、亲子对和亲密社会群体中的已知现象。此模块尝试通过 WiFi CSI 非接触式检测它。

#### 工作原理

1. **每人分解**：有 N 人时，8 个子载波组在人之间分配（例如，2 人 = 每人 4 组）。每人的相位信号通过双 EWMA（DC 去除 + 低通）带通滤波到呼吸频带。

2. **成对相关**：对于每对，在 64 样本缓冲区上计算归一化零滞后互相关：`rho = sum(x_i * x_j) / sqrt(sum(x_i^2) * sum(x_j^2))`

3. **同步状态机**：高相关（|rho| > 0.6）持续 20+ 连续帧声明同步。低相关持续 15+ 帧声明同步丢失。

#### 事件

| 事件 | ID | 值 | 频率 |
|------|-----|-----|------|
| `SYNC_DETECTED` | 670 | 1.0 | 同步开始时 |
| `SYNC_PAIR_COUNT` | 671 | 同步对的数量 | 计数变化时 |
| `GROUP_COHERENCE` | 672 | 平均相干性 [0, 1] | 每 10 帧 |
| `SYNC_LOST` | 673 | 1.0 | 同步丢失时 |

#### 约束

- 最多 4 人（6 对比较）
- 需要 >= 8 个子载波和 >= 2 人
- 分析开始前 64 帧预热

#### API

```rust
let mut detector = BreathingSyncDetector::new();
let events = detector.process_frame(
    &phases,          // &[f32]: 每个子载波的相位
    &variance,        // &[f32]: 每个子载波的方差
    breathing_bpm,    // f32: 主机聚合（内部未使用）
    n_persons,        // i32: 检测到的人数
);

let synced = detector.is_synced();           // bool
let coherence = detector.group_coherence();  // f32 [0, 1]
let persons = detector.active_persons();     // usize
```

---

### 时间晶体检测 (`exo_time_crystal.rs`)

**功能**：检测运动能量中的时间对称性破坏模式 -- 特别是周期倍增。在这种情况下，"时间晶体"是指系统以驱动频率的次谐波振荡。还将独立非谐波周期性成分计数为多人时间协调的"协调指数"。

**成熟度**：研究

**背景**：在凝聚态物理中，离散时间晶体在周期性驱动下表现出周期倍增。此模块将相同的数学标准（滞后 L 和滞后 2L 处的自相关峰值）应用于人类运动模式。以不同节奏行走的两个人产生非谐波比率的独立周期性峰值。

#### 工作原理

1. **自相关**：256 点运动能量缓冲区，滞后 1-128 处的自相关。为性能预线性化（消除内循环中的模运算）。

2. **周期倍增**：搜索峰值，其中滞后 L 处的强自相关伴随着滞后 2L 处的强峰值（+/- 2 帧容差）。

3. **协调指数**：计数其滞后比率不是任何其他峰值的整数倍的峰值（5% 容差内）。这些代表独立的周期性运动。

4. **稳定性跟踪**：在 200 帧窗口上跟踪晶体检测。稳定性分数是检测到晶体的帧的分数，EMA 平滑。

#### 事件

| 事件 | ID | 值 | 频率 |
|------|-----|-----|------|
| `CRYSTAL_DETECTED` | 680 | 周期倍数 (2 = 倍增) | 检测到时 |
| `CRYSTAL_STABILITY` | 681 | 稳定性分数 [0, 1] | 每帧 |
| `COORDINATION_INDEX` | 682 | 非谐波峰值计数 | 当 > 0 时 |

#### API

```rust
let mut detector = TimeCrystalDetector::new();
let events = detector.process_frame(motion_energy);

let detected = detector.is_detected();          // bool
let multiplier = detector.multiplier();          // u8 (0 or 2)
let stability = detector.stability();            // f32 [0, 1]
let coordination = detector.coordination_index(); // u8
```

---

### 双曲空间嵌入 (`exo_hyperbolic_space.rs`)

**功能**：将 CSI 指纹嵌入 2D Poincare 盘，以利用室内空间的自然层次结构（房间包含区域）。双曲几何在边界附近提供指数级更多的表示能力，非常适合树状结构的位置分类法。

**成熟度**：研究

**研究基础**：双曲嵌入已被证明在层次数据上优于欧几里得嵌入（Nickel & Kiela, 2017）。此模块将该概念应用于室内定位。

#### 工作原理

1. **特征提取**：来自 8 个子载波组平均幅度的 8D 向量。
2. **线性投影**：2x8 矩阵将特征映射到 2D Poincare 盘坐标。
3. **归一化**：如果投影点超过盘边界，缩放到半径 0.95。
4. **最近参考**：计算到 16 个参考点的 Poincare 距离并找到最近的。
5. **层次级别**：靠近中心（半径 < 0.5）的点是房间级；靠近边界的是区域级。

#### Poincare 距离

```
d(x, y) = acosh(1 + 2 * ||x-y||^2 / ((1 - ||x||^2) * (1 - ||y||^2)))
```

此度量尊重双曲几何：边界附近的距离呈指数增长。

#### 默认参考布局

| 索引 | 标签 | 半径 | 描述 |
|------|------|------|------|
| 0-3 | 房间 | 0.3 | 浴室、厨房、客厅、卧室 |
| 4-6 | 区域 0a-c | 0.7 | 浴室子区域 |
| 7-9 | 区域 1a-c | 0.7 | 厨房子区域 |
| 10-12 | 区域 2a-c | 0.7 | 客厅子区域 |
| 13-15 | 区域 3a-c | 0.7 | 卧室子区域 |

#### 事件

| 事件 | ID | 值 | 频率 |
|------|-----|-----|------|
| `HIERARCHY_LEVEL` | 685 | 0 = 房间, 1 = 区域 | 每帧 |
| `HYPERBOLIC_RADIUS` | 686 | 盘半径 [0, 1) | 每帧 |
| `LOCATION_LABEL` | 687 | 最近参考 (0-15) | 每帧 |

#### API

```rust
let mut embedder = HyperbolicEmbedder::new();
let events = embedder.process_frame(&amplitudes);

let label = embedder.label();        // u8 (0-15)
let pos = embedder.position();       // &[f32; 2]

// 自定义校准：
embedder.set_reference(0, [0.2, 0.1]);
embedder.set_projection_row(0, [0.05, 0.03, 0.02, 0.01, -0.01, -0.02, -0.03, -0.04]);
```

---

## 事件 ID 注册表 (600-699)

| 范围 | 模块 | 事件 |
|------|------|------|
| 600-603 | 梦境阶段 | SLEEP_STAGE, SLEEP_QUALITY, REM_EPISODE, DEEP_SLEEP_RATIO |
| 610-613 | 情绪检测 | AROUSAL_LEVEL, STRESS_INDEX, CALM_DETECTED, AGITATION_DETECTED |
| 620-623 | 手势语言 | LETTER_RECOGNIZED, LETTER_CONFIDENCE, WORD_BOUNDARY, GESTURE_REJECTED |
| 630-634 | 音乐指挥 | CONDUCTOR_BPM, BEAT_POSITION, DYNAMIC_LEVEL, GESTURE_CUTOFF, GESTURE_FERMATA |
| 640-643 | 植物生长 | GROWTH_RATE, CIRCADIAN_PHASE, WILT_DETECTED, WATERING_EVENT |
| 650-653 | 幽灵猎人 | ANOMALY_DETECTED, ANOMALY_CLASS, HIDDEN_PRESENCE, ENVIRONMENTAL_DRIFT |
| 660-662 | 降雨检测 | RAIN_ONSET, RAIN_INTENSITY, RAIN_CESSATION |
| 670-673 | 呼吸同步 | SYNC_DETECTED, SYNC_PAIR_COUNT, GROUP_COHERENCE, SYNC_LOST |
| 680-682 | 时间晶体 | CRYSTAL_DETECTED, CRYSTAL_STABILITY, COORDINATION_INDEX |
| 685-687 | 双曲空间 | HIERARCHY_LEVEL, HYPERBOLIC_RADIUS, LOCATION_LABEL |

## 代码质量说明

所有 10 个模块都经过以下审查：

- **边缘情况**：除以零在各处都有保护（除法前显式检查，EPSILON 常量）。浮点舍入产生的负方差被钳制为零。空缓冲区返回安全默认值。
- **NaN 保护**：所有计算使用 `libm` 函数（`sqrtf`、`acoshf`、`sinf`），这些函数对有效输入有明确定义。输入在到达数学函数之前经过验证。
- **缓冲区安全**：所有 `CircularBuffer` 访问使用 `get(i)` 方法，该方法对越界索引返回 0.0。固定大小数组防止溢出。
- **范围钳制**：所有表示比率或概率的输出都被钳制到 [0, 1]。MIDI 力度被钳制到 [0, 127]。Poincare 盘坐标被归一化到半径 < 1。
- **测试覆盖**：每个模块有 7-10 个测试，涵盖：构造、预热期、快乐路径检测、边缘情况（无存在、数据不足）、范围验证和重置。

## 研究参考文献

1. Liu, J., et al. "Monitoring Vital Signs and Postures During Sleep Using WiFi Signals." IEEE Internet of Things Journal, 2018. -- 使用 CSI 呼吸模式的基于 WiFi 的睡眠监测。
2. Zhao, M., et al. "Through-Wall Human Pose Estimation Using Radio Signals." CVPR 2018. -- 基于 RF 的姿态估计基础。
3. Wang, H., et al. "RT-Fall: A Real-Time and Contactless Fall Detection System with Commodity WiFi Devices." IEEE Transactions on Mobile Computing, 2017. -- 用于人类活动识别的 WiFi CSI。
4. Li, H., et al. "WiFinger: Talk to Your Smart Devices with Finger Gesture." UbiComp 2016. -- 使用 CSI 的基于 WiFi 的手势识别。
5. Ma, Y., et al. "SignFi: Sign Language Recognition Using WiFi." ACM IMWUT, 2018. -- 用于手语的 WiFi CSI。
6. Nickel, M. & Kiela, D. "Poincare Embeddings for Learning Hierarchical Representations." NeurIPS 2017. -- 双曲嵌入基础。
7. Wang, W., et al. "Understanding and Modeling of WiFi Signal Based Human Activity Recognition." MobiCom 2015. -- 基于 CSI 的活动识别。
8. Adib, F., et al. "Smart Homes that Monitor Breathing and Heart Rate." CHI 2015. -- 通过 RF 信号进行非接触式生命体征监测。

## 贡献新研究模块

### 添加新的异国情调模块

1. **选择事件 ID 范围**：使用 600-699 块中的下一个可用范围。检查 `lib.rs` event_types 中的已分配 ID。

2. **创建源文件**：在 `src/` 中命名为 `exo_<name>.rs`。遵循现有模式：
   - 模块级文档注释，包含算法描述、事件和预算
   - `const fn new()` 构造函数
   - `process_frame()` 通过静态缓冲区返回 `&[(i32, f32)]`
   - 关键状态的公共访问方法
   - `reset()` 方法

3. **在 `lib.rs` 中注册**：在 Category 6 部分添加 `pub mod exo_<name>;`。

4. **注册事件常量**：在 `lib.rs` 的 `event_types` 中添加条目。

5. **更新本文档**：将模块添加到概述表并编写其部分。

6. **测试要求**：
   - 至少：`test_const_new`、`test_warmup_no_events`、一个快乐路径检测测试、`test_reset`
   - 测试边缘情况：空输入、极值、数据不足
   - 验证所有输出值在其记录的范围内
   - 运行：`cargo test --features std -- exo_`（在 wasm-edge crate 目录内）

### 设计约束

- **`no_std`**：无堆分配。使用 `CircularBuffer`、`Ema`、`WelfordStats` 从 `vendor_common`。
- **栈预算**：保持总结构体大小合理。ESP32-S3 WASM3 栈有限。
- **时间预算**：保持在声明的预算内（L < 2ms, S < 5ms, H < 10ms at 20 Hz）。
- **静态事件**：使用 `static mut EVENTS` 数组进行零分配事件返回。
- **输入验证**：始终检查数组长度，优雅处理缺失数据。