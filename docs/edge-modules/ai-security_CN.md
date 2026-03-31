# AI 安全模块 -- WiFi-DensePose 边缘智能

> 篡改检测和行为异常分析，保护传感系统免受操纵。这些模块检测重放攻击、信号注入、干扰和异常行为模式 -- 全部在设备上运行，无云依赖。

## 概述

| 模块 | 文件 | 功能 | 事件 ID | 预算 |
|------|------|------|---------|------|
| 信号盾牌 | `ais_prompt_shield.rs` | 检测对 CSI 数据的重放、注入和干扰攻击 | 820-823 | S (<5 ms) |
| 行为分析器 | `ais_behavioral_profiler.rs` | 学习正常行为并检测异常偏差 | 825-828 | S (<5 ms) |

---

## 信号盾牌 (`ais_prompt_shield.rs`)

**功能**：检测 WiFi 传感系统上的三种攻击类型：

1. **重放攻击**：攻击者录制合法 CSI 帧并回放，欺骗传感器看到"正常"场景，而实际上攻击者在房间内。
2. **信号注入**：攻击者传输强 WiFi 信号以压制合法 CSI，在许多子载波上创建幅度峰值。
3. **干扰**：攻击者用噪声淹没 WiFi 信道，将信噪比降至可用水平以下。

**工作原理**：

- **重放检测**：每个帧的特征（平均相位、平均幅度、幅度方差）被量化并使用 FNV-1a 哈希。哈希存储在 64 条目环形缓冲区中。如果新帧的哈希与任何最近的哈希匹配，则标记为重放。
- **注入检测**：如果超过 25% 的子载波显示比前一帧 >10 倍的幅度跳跃，则标记为注入。
- **干扰检测**：模块在前 100 帧内校准基线 SNR（信号 / sqrt(方差)）。如果当前 SNR 连续 5+ 帧低于基线的 10%，则标记为干扰。

#### 公共 API

```rust
use wifi_densepose_wasm_edge::ais_prompt_shield::PromptShield;

let mut shield = PromptShield::new();                     // const fn, zero-alloc
let events = shield.process_frame(&phases, &amplitudes);  // 每帧分析
let calibrated = shield.is_calibrated();                  // 100 帧后为 true
let frames = shield.frame_count();                        // 处理的总帧数
```

#### 事件

| 事件 ID | 常量 | 值 | 频率 |
|---------|------|-----|------|
| 820 | `EVENT_REPLAY_ATTACK` | 1.0 (检测到) | 检测时（冷却：40 帧） |
| 821 | `EVENT_INJECTION_DETECTED` | 有峰值的子载波分数 [0.25, 1.0] | 检测时（冷却：40 帧） |
| 822 | `EVENT_JAMMING_DETECTED` | SNR 下降（dB）(10 * log10(baseline/current)) | 检测时（冷却：40 帧） |
| 823 | `EVENT_SIGNAL_INTEGRITY` | 综合完整性分数 [0.0, 1.0] | 每 20 帧 |

#### 配置常量

| 常量 | 值 | 用途 |
|------|-----|------|
| `MAX_SC` | 32 | 处理的最大子载波数 |
| `HASH_RING` | 64 | 重放检测哈希环形缓冲区大小 |
| `INJECTION_FACTOR` | 10.0 | 幅度跳跃阈值（前一帧的 10 倍） |
| `INJECTION_FRAC` | 0.25 | 有峰值的子载波最小分数 |
| `JAMMING_SNR_FRAC` | 0.10 | SNR 必须降至基线的 10% 以下 |
| `JAMMING_CONSEC` | 5 | 需要的连续低 SNR 帧数 |
| `BASELINE_FRAMES` | 100 | 校准周期长度 |
| `COOLDOWN` | 40 | 重复警报之间的帧数（20 Hz 时为 2 秒） |

#### 信号完整性分数

综合分数（事件 823）每 20 帧发出一次，范围从 0.0（受损）到 1.0（干净）：

| 因素 | 分数减少 | 条件 |
|------|---------|------|
| 检测到重放 | -0.4 | 帧哈希匹配环形缓冲区 |
| 检测到注入 | 最多 -0.3 | 与注入分数成比例 |
| SNR 下降 | 最多 -0.3 | 与 SNR 低于基线的下降成比例 |

#### FNV-1a 哈希详情

哈希函数在哈希前将三个帧统计量量化为整数精度：

```
hash = FNV_OFFSET (2166136261)
for each of [mean_phase*100, mean_amp*100, amp_variance*100]:
    for each byte in value.to_le_bytes():
        hash ^= byte
        hash = hash.wrapping_mul(FNV_PRIME)   // FNV_PRIME = 16777619
```

这意味着两个帧必须具有几乎相同的统计配置文件（在 1% 量化范围内）才能触发重放警报。

#### 示例：检测重放攻击

```
校准（帧 1-100）：
  具有变化相位的正常 CSI -> 建立基线 SNR
  校准期间不发出警报

帧 150：正常操作
  phases = [0.31, 0.28, ...], amps = [1.02, 0.98, ...]
  hash = 0xA7F3B21C -> 存储在环形缓冲区
  无警报

帧 200：攻击者完全重放帧 150
  phases = [0.31, 0.28, ...], amps = [1.02, 0.98, ...]
  hash = 0xA7F3B21C -> 在环形缓冲区中找到匹配！
  -> EVENT_REPLAY_ATTACK = 1.0
  -> EVENT_SIGNAL_INTEGRITY = 0.6（减少 0.4）
```

#### 示例：检测信号注入

```
帧 300：正常幅度
  amps = [1.0, 1.1, 0.9, 1.0, ...]

帧 301：攻击者注入强信号
  amps = [15.0, 12.0, 14.0, 13.0, ...] （所有子载波 >10 倍跳跃）
  injection_fraction = 1.0（100% 的子载波出现峰值）
  -> EVENT_INJECTION_DETECTED = 1.0
  -> EVENT_SIGNAL_INTEGRITY = 0.4
```

---

## 行为分析器 (`ais_behavioral_profiler.rs`)

**功能**：随着时间的推移学习什么是"正常"行为，然后检测异常偏差。它使用在线统计（Welford 算法）构建 6 维行为配置文件，并在新观察显著偏离学习基线时标记。

**工作原理**：每 200 帧，模块从观察窗口计算 6D 特征向量。在学习阶段（前 1000 帧），它为每个维度训练 Welford 累加器。成熟后，它计算每个维度的 Z 分数和组合 RMS Z 分数。如果组合分数超过 3.0，则报告异常。

#### 6 个行为维度

| # | 维度 | 描述 | 典型范围 |
|---|------|------|----------|
| 0 | 存在率 | 有存在的帧的分数 | [0, 1] |
| 1 | 平均运动 | 窗口中的平均运动能量 | [0, ~5] |
| 2 | 平均人数 | 平均人数 | [0, ~4] |
| 3 | 活动方差 | 运动能量的方差 | [0, ~10] |
| 4 | 转换率 | 每帧的存在状态变化 | [0, 0.5] |
| 5 | 停留时间 | 连续存在运行长度的平均值 | [0, 200] |

#### 公共 API

```rust
use wifi_densepose_wasm_edge::ais_behavioral_profiler::BehavioralProfiler;

let mut bp = BehavioralProfiler::new();                   // const fn
let events = bp.process_frame(present, motion, n_persons); // 每帧
let mature = bp.is_mature();                               // 学习后为 true
let anomalies = bp.total_anomalies();                      // 累积计数
let mean = bp.dim_mean(0);                                 // 维度 0 的均值
let var = bp.dim_variance(1);                              // 维度 1 的方差
```

#### 事件

| 事件 ID | 常量 | 值 | 频率 |
|---------|------|-----|------|
| 825 | `EVENT_BEHAVIOR_ANOMALY` | 组合 Z 分数（RMS, > 3.0） | 检测时（冷却：100 帧） |
| 826 | `EVENT_PROFILE_DEVIATION` | 最偏离的维度索引 (0-5) | 与异常配对 |
| 827 | `EVENT_NOVEL_PATTERN` | Z > 2.0 的维度计数 | 当 3+ 维度偏离时 |
| 828 | `EVENT_PROFILE_MATURITY` | 传感器启动以来的天数 | 成熟时 + 定期 |

#### 配置常量

| 常量 | 值 | 用途 |
|------|-----|------|
| `N_DIM` | 6 | 行为维度 |
| `LEARNING_FRAMES` | 1000 | 分析器成熟前的帧数 |
| `ANOMALY_Z` | 3.0 | 异常的组合 Z 分数阈值 |
| `NOVEL_Z` | 2.0 | 新颖性的每维度 Z 分数阈值 |
| `NOVEL_MIN` | 3 | NOVEL_PATTERN 的最小偏离维度数 |
| `OBS_WIN` | 200 | 观察窗口大小（帧） |
| `COOLDOWN` | 100 | 重复异常警报之间的帧数 |
| `MATURITY_INTERVAL` | 72000 | 成熟度报告之间的帧数（20 Hz 时为 1 小时） |

#### Welford 在线算法

每个维度维护运行统计，无需存储所有过去的值：

```
On each new observation x:
    count += 1
    delta = x - mean
    mean += delta / count
    m2 += delta * (x - mean)

Variance = m2 / count
Z-score  = |x - mean| / sqrt(variance)
```

这在数值上是稳定的，每个维度仅需 12 字节（count + mean + m2）。

#### 示例：检测入侵者的行为特征

```
学习阶段（第 1-2 天）：
  正常模式：1 人，8am-10pm 存在，中等运动
  配置文件成熟 -> EVENT_PROFILE_MATURITY = 0.58（天）

第 3 天，3am：
  观察窗口：presence=1, 高运动, 1 人
  Z 分数：presence_rate=2.8, motion=4.1, persons=0.3,
            variance=3.5, transition=2.2, dwell=1.9
  组合 Z = sqrt(mean(z^2)) = 3.4 > 3.0
  -> EVENT_BEHAVIOR_ANOMALY = 3.4
  -> EVENT_PROFILE_DEVIATION = 1（运动维度最偏离）
  -> EVENT_NOVEL_PATTERN = 3（3 个维度 Z>2.0）
```

---

## 威胁模型

### 这些模块检测的攻击

| 攻击 | 检测模块 | 方法 | 误报率 |
|------|----------|------|--------|
| CSI 帧重放 | 信号盾牌 | FNV-1a 哈希环形匹配 | 低（1% 量化） |
| 信号注入（例如，流氓 AP） | 信号盾牌 | >25% 子载波 >10 倍幅度峰值 | 非常低 |
| 宽带干扰 | 信号盾牌 | SNR 连续 5+ 帧低于基线的 10% | 非常低 |
| 窄带干扰 | 部分 -- 信号盾牌 | 如果 < 25% 子载波受影响可能不触发 | 中等 |
| 行为异常（不寻常时间的入侵者） | 行为分析器 | 跨 6 维度的组合 Z 分数 > 3.0 | 成熟后低 |
| 渐进环境变化 | 行为分析器 | Welford 统计适应，如变化突然可能标记 | 非常低 |

### 这些模块无法检测的攻击

| 攻击 | 为什么 | 推荐缓解措施 |
|------|--------|--------------|
| 带有轻微相位变化的复杂重放 | FNV-1a 使用 1% 量化；小扰动改变哈希 | 添加时间相关性检查（连续帧增量） |
| WiFi 信道上的中间人 | 模块分析 CSI 内容，而非信道认证 | 使用 WPA3 加密 + MAC 过滤 |
| 物理障碍（阻挡视线） | 看起来像人离开，不是攻击 | 与 PIR 传感器交叉引用 |
| 缓慢幅度漂移（渐进注入） | 低于每帧 10 倍阈值 | 添加长期幅度趋势监控 |
| 固件篡改 | 模块在 WASM 沙箱中运行，无法检测主机 compromise | 安全启动 + 签名固件（ADR-032） |

### 部署建议

1. **始终一起运行两个模块**：信号盾牌捕获主动攻击，行为分析器捕获被动异常。
2. **允许完全校准**：信号盾牌需要 100 帧（5 秒）用于 SNR 基线。行为分析器需要 1000 帧（~50 秒）用于可靠的 Z 分数。
3. **与时间逻辑守卫结合** (`tmp_temporal_logic_guard.rs`)：其安全不变量捕获指示传感器操纵的不可能状态组合（例如，"房间空时的跌倒警报"）。
4. **连接到自修复网格** (`aut_self_healing_mesh.rs`)：如果网格中的节点被干扰，网格可以自动围绕受损节点重新配置。

---

## 内存布局

| 模块 | 状态大小（约） | 静态事件缓冲区 |
|------|--------------|---------------|
| 信号盾牌 | ~420 字节（64 哈希 + 32 prev_amps + 校准） | 4 条目 |
| 行为分析器 | ~2.4 KB（200 条目观察窗口 + 6 Welford 统计） | 4 条目 |

两个模块都使用固定大小的数组和静态事件缓冲区。无堆分配。完全 no_std 兼容。