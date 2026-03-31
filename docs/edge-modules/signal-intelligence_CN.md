# 信号智能模块 -- WiFi-DensePose 边缘智能

> 直接在 ESP32 芯片上运行的实时 WiFi 信号分析和增强。这些模块清理、压缩并从原始 WiFi 通道数据中提取特征，使高级模块（健康、安全等）获得更好的输入。

## 概述

| 模块 | 文件 | 功能 | 事件 ID | 预算 |
|------|------|------|---------|------|
| 闪存注意力 | `sig_flash_attention.rs` | 将处理聚焦于最具信息性的子载波组 | 700-702 | S (<5ms) |
| 相干门 | `sig_coherence_gate.rs` | 使用相位相干性过滤掉噪声/损坏的 CSI 帧 | 710-712 | L (<2ms) |
| 时间压缩 | `sig_temporal_compress.rs` | 在 3 层压缩环形缓冲区中存储 CSI 历史 | 705-707 | S (<5ms) |
| 稀疏恢复 | `sig_sparse_recovery.rs` | 使用 ISTA 稀疏优化恢复丢失的子载波 | 715-717 | H (<10ms) |
| 最小割人员匹配 | `sig_mincut_person_match.rs` | 使用二分匹配在帧间维护稳定的人员 ID | 720-722 | H (<10ms) |
| 最优传输 | `sig_optimal_transport.rs` | 通过切片 Wasserstein 距离检测细微运动 | 725-727 | S (<5ms) |

## 信号处理的位置

信号智能模块在原始 CSI 数据和应用级模块之间形成处理管道：

```
  来自 WiFi 芯片组的原始 CSI（0-2 层固件 DSP）
       |
       v
  +---------------------+     +---------------------+
  | 相干门               | --> | 稀疏恢复              |
  | 拒绝噪声帧，         |     | 通过 ISTA 填充丢失的    |
  | 门控质量级别         |     | 子载波                |
  +---------------------+     +---------------------+
       |                              |
       v                              v
  +---------------------+     +---------------------+
  | 闪存注意力            |     | 时间压缩              |
  | 聚焦于信息丰富的       |     | 存储 CSI 历史         |
  | 子载波组             |     | 3 个质量层级          |
  +---------------------+     +---------------------+
       |                              |
       v                              v
  +---------------------+     +---------------------+
  | 最小割人员匹配        |     | 最优传输              |
  | 跨帧跟踪人员 ID       |     | 通过分布检测细微运动    |
  +---------------------+     +---------------------+
       |                              |
       v                              v
  应用模块：健康、安全、智能建筑等
```

**相干门**在管道顶部充当质量过滤器。通过门的帧进入**稀疏恢复**模块（如果检测到子载波丢失），然后进入下游分析。**闪存注意力**识别哪些空间区域承载最多信号，而**时间压缩**维护高效的滚动历史。**最小割人员匹配**和**最优传输**提取应用模块消费的高级特征（人员身份和运动）。

## 共享工具 (`vendor_common.rs`)

所有信号智能模块共享来自 `vendor_common.rs` 的这些工具：

| 工具 | 用途 |
|------|------|
| `CircularBuffer<N>` | 固定大小的环形缓冲区，用于相位历史，栈分配 |
| `Ema` | 具有可配置 alpha 的指数移动平均 |
| `WelfordStats` | O(1) 内存中的在线均值/方差/标准差 |
| `dot_product`, `l2_norm`, `cosine_similarity` | 固定大小的向量数学 |
| `dtw_distance`, `dtw_distance_banded` | 用于手势/模式匹配的动态时间规整 |
| `FixedPriorityQueue<CAP>` | 无堆分配的 Top-K 选择 |

---

## 模块

### 闪存注意力 (`sig_flash_attention.rs`)

**功能**：将处理聚焦于携带最有用信息的 WiFi 通道 -- 忽略噪声。将 32 个子载波分为 8 组，并计算显示信号活动集中位置的注意力权重。

**算法**：对 8 个子载波组进行平铺注意力 (Q*K/sqrt(d))，使用 softmax 归一化和香农熵跟踪。

1. 计算组均值：Q = 每组当前相位，K = 每组先前相位，V = 每组幅度
2. 为每组评分：`score[g] = Q[g] * K[g] / sqrt(8)`
3. Softmax 归一化（数值稳定：exp 前减去最大值）
4. 通过 EMA 平滑跟踪熵 H = -sum(p * ln(p))

低熵意味着活动集中在一个空间区域（菲涅尔区域）；高熵意味着活动均匀分布。

#### 公共 API

```rust
pub struct FlashAttention { /* ... */ }

impl FlashAttention {
    pub const fn new() -> Self;
    pub fn process_frame(&mut self, phases: &[f32], amplitudes: &[f32]) -> &[(i32, f32)];
    pub fn weights() -> &[f32; 8];       // 当前每组的注意力权重
    pub fn entropy() -> f32;             // EMA 平滑的熵 [0, ln(8)]
    pub fn peak_group() -> usize;        // 权重最高的组索引
    pub fn centroid() -> f32;            // 跨组的注意力加权中心 [0, 7]
    pub fn frame_count() -> u32;
    pub fn reset(&mut self);
}
```

#### 事件

| ID | 名称 | 值 | 含义 |
|----|------|-----|------|
| 700 | `ATTENTION_PEAK_SC` | 组索引 (0-7) | 哪个子载波组具有最强的注意力权重 |
| 701 | `ATTENTION_SPREAD` | 熵 (0 到 ~2.08) | 注意力的分散程度（低 = 聚焦，高 = 均匀） |
| 702 | `SPATIAL_FOCUS_ZONE` | 中心 (0.0-7.0) | 跨组注意力的加权中心 |

#### 配置

| 常量 | 值 | 用途 |
|------|-----|------|
| `N_GROUPS` | 8 | 子载波组（瓦片）数量 |
| `MAX_SC` | 32 | 处理的最大子载波数 |
| `ENTROPY_ALPHA` | 0.15 | 熵的 EMA 平滑因子 |

#### 教程：理解注意力权重

8 个注意力权重总和为 1.0。当人站在房间的特定区域时，WiFi 信号在其子载波组（菲涅尔区与该区域相交）中变化最大。

- **所有权重接近 0.125 (= 1/8)**：均匀注意力。无局部活动 -- 要么是空房间，要么是影响所有子载波的全身运动。
- **一个权重接近 1.0，其他接近 0.0**：高度聚焦。活动集中在一个空间区域。`peak_group` 索引告诉您是哪个区域。
- **两个相邻组升高**：活动在两个空间区域之间的边界，或人在它们之间移动。
- **熵低于 1.0**：强烈的空间聚焦。适合区域级定位。
- **熵高于 1.8**：几乎均匀。难以定位活动。

`centroid` 值 (0.0 到 7.0) 给出加权平均位置。随时间跟踪质心揭示整个房间的运动方向。

---

### 相干门 (`sig_coherence_gate.rs`)

**功能**：决定每个传入的 CSI 帧是否足够可信以用于感测，或应被丢弃。使用跨子载波的相位变化的统计一致性来测量信号质量。

**算法**：每个子载波的相位增量形成单位相量 (cos + i*sin)。平均相量的幅度是相干分数 [0,1]。Welford 在线统计跟踪 Z 分数计算的均值/方差。滞后状态机防止状态之间的快速振荡。

状态转换：
- Accept -> PredictOnly: 5 个连续帧低于 LOW_THRESHOLD (0.40)
- PredictOnly -> Reject: 单个帧低于阈值
- Reject/PredictOnly -> Accept: 10 个连续帧高于 HIGH_THRESHOLD (0.75)
- Any -> Recalibrate: 运行方差超过初始快照的 4 倍

#### 公共 API

```rust
pub struct CoherenceGate { /* ... */ }

impl CoherenceGate {
    pub const fn new() -> Self;
    pub fn process_frame(&mut self, phases: &[f32]) -> &[(i32, f32)];
    pub fn gate() -> GateDecision;       // Accept/PredictOnly/Reject/Recalibrate
    pub fn coherence() -> f32;           // 最后相干分数 [0, 1]
    pub fn zscore() -> f32;              // 最后相干的 Z 分数
    pub fn variance() -> f32;            // 相干的运行方差
    pub fn frame_count() -> u32;
    pub fn reset(&mut self);
}

pub enum GateDecision { Accept, PredictOnly, Reject, Recalibrate }
```

#### 事件

| ID | 名称 | 值 | 含义 |
|----|------|-----|------|
| 710 | `GATE_DECISION` | 2/1/0/-1 | Accept(2), PredictOnly(1), Reject(0), Recalibrate(-1) |
| 711 | `COHERENCE_SCORE` | [0.0, 1.0] | 相位相量相干幅度 |
| 712 | `RECALIBRATE_NEEDED` | 方差 | 环境发生显著变化 -- 重新训练基线 |

#### 配置

| 常量 | 值 | 用途 |
|------|-----|------|
| `HIGH_THRESHOLD` | 0.75 | 相干高于此值 = 高质量 |
| `LOW_THRESHOLD` | 0.40 | 相干低于此值 = 低质量 |
| `DEGRADE_COUNT` | 5 | 降级前的连续坏帧数 |
| `RECOVER_COUNT` | 10 | 恢复前的连续好帧数 |
| `VARIANCE_DRIFT_MULT` | 4.0 | 触发重新校准的方差倍数 |

#### 教程：使用相干门

相干门保护下游模块免受处理垃圾数据的影响。在实践中：

1. **Accept** (value=2)：帧是干净的。将其用于所有感测任务（生命体征、存在、手势）。
2. **PredictOnly** (value=1)：帧质量边缘。使用先前帧的缓存预测；不更新模型。
3. **Reject** (value=0)：帧太嘈杂。完全跳过。不要馈送到任何学习模块。
4. **Recalibrate** (value=-1)：环境发生根本性变化（家具移动、新 AP、门打开）。重置基线并重新学习。

低相干的常见原因：
- 微波炉运行（2.4 GHz 干扰）
- 多人向不同方向行走（相位抵消）
- 硬件故障（间歇性天线接触）

---

### 时间压缩 (`sig_temporal_compress.rs`)

**功能**：以压缩形式维护最多 512 个 CSI 快照的滚动历史。最近的数据以高精度存储；旧数据被逐渐压缩以节省内存，同时保留长期趋势。

**算法**：三层量化，在年龄边界自动降级。

| 层级 | 年龄范围 | 位 | 量化级别 | 最大误差 |
|------|----------|------|----------|----------|
| Hot | 0-63（最新） | 8 位 | 256 | <0.5% |
| Warm | 64-255 | 5 位 | 32 | <3% |
| Cold | 256-511 | 3 位 | 8 | <15% |

在 20 Hz 时，缓冲区存储大约：
- Hot：3.2 秒的高保真数据
- Warm：9.6 秒的中等保真数据
- Cold：12.8 秒的低保真数据
- 总计：~25.6 秒，或在较低帧速率下更长

每个快照存储 8 个相位 + 8 个幅度值（组均值），加上比例因子和层级标签。

#### 公共 API

```rust
pub struct TemporalCompressor { /* ... */ }

impl TemporalCompressor {
    pub const fn new() -> Self;
    pub fn push_frame(&mut self, phases: &[f32], amps: &[f32], ts_ms: u32) -> &[(i32, f32)];
    pub fn on_timer() -> &[(i32, f32)];
    pub fn get_snapshot(age: usize) -> Option<[f32; 16]>;  // 解压后的 8 相位 + 8 幅度
    pub fn compression_ratio() -> f32;
    pub fn frame_rate() -> f32;
    pub fn total_written() -> u32;
    pub fn occupied() -> usize;
}
```

#### 事件

| ID | 名称 | 值 | 含义 |
|----|------|-----|------|
| 705 | `COMPRESSION_RATIO` | 比率 (>1.0) | 原始字节 / 压缩字节 |
| 706 | `TIER_TRANSITION` | 层级 (1 或 2) | 快照被降级到 Warm(1) 或 Cold(2) |
| 707 | `HISTORY_DEPTH_HOURS` | 小时 | 缓冲区覆盖的挂钟时间 |

#### 配置

| 常量 | 值 | 用途 |
|------|-----|------|
| `CAP` | 512 | 总快照容量 |
| `HOT_END` | 64 | 前 N 个快照，8 位精度 |
| `WARM_END` | 256 | 快照 64-255，5 位精度 |
| `RATE_ALPHA` | 0.05 | 帧率估计的 EMA alpha |

---

### 稀疏恢复 (`sig_sparse_recovery.rs`)

**功能**：当 WiFi 硬件丢失一些子载波测量值（由于深度衰落、固件故障或多径零点导致的空值/零）时，该模块使用数学优化重建缺失值。

**算法**：迭代收缩阈值算法 (ISTA) -- 一种 L1 最小化稀疏恢复方法。

```
x_{k+1} = soft_threshold(x_k + step * A^T * (b - A*x_k), lambda)
```

其中：
- `A` 是三对角相关模型（对角线 + 直接邻居，96 个 f32 而非完整的 32x32=1024）
- `b` 是观察到的（非空）子载波值
- `soft_threshold(x, t) = sign(x) * max(|x| - t, 0)` 促进稀疏性
- 每帧最多 10 次迭代

相关模型通过 EMA 混合产品从有效帧在线学习。

#### 公共 API

```rust
pub struct SparseRecovery { /* ... */ }

impl SparseRecovery {
    pub const fn new() -> Self;
    pub fn process_frame(&mut self, amplitudes: &mut [f32]) -> &[(i32, f32)];
    pub fn dropout_rate() -> f32;           // 空子载波的分数
    pub fn last_residual_norm() -> f32;     // 上次恢复的 L2 残差
    pub fn last_recovered_count() -> u32;   // 恢复了多少个子载波
    pub fn is_initialized() -> bool;        // 相关模型是否准备就绪
}
```

注意：`process_frame` 就地修改 `amplitudes` -- 空子载波被覆盖为恢复值。

#### 事件

| ID | 名称 | 值 | 含义 |
|----|------|-----|------|
| 715 | `RECOVERY_COMPLETE` | 计数 | 恢复的子载波数量 |
| 716 | `RECOVERY_ERROR` | L2 范数 | 恢复的残差误差 |
| 717 | `DROPOUT_RATE` | 分数 [0,1] | 空子载波的分数（每 20 帧发出） |

#### 配置

| 常量 | 值 | 用途 |
|------|-----|------|
| `NULL_THRESHOLD` | 0.001 | 幅度低于此值 = 丢失 |
| `MIN_DROPOUT_RATE` | 0.10 | 触发恢复的最小丢失分数 |
| `MAX_ITERATIONS` | 10 | 每帧 ISTA 迭代上限 |
| `STEP_SIZE` | 0.05 | 梯度下降学习率 |
| `LAMBDA` | 0.01 | L1 稀疏性惩罚权重 |
| `CORR_ALPHA` | 0.05 | 相关模型更新的 EMA alpha |

#### 教程：恢复何时启动

1. 该模块需要至少 10 个完全有效的帧来初始化相关模型 (`is_initialized() == true`)。
2. 仅当丢失超过 10% 时（例如，32 个子载波中有 4+ 个为空）才触发恢复。
3. 低于 10% 时，空值太稀疏，不值得恢复开销。
4. 三对角相关模型利用相邻 WiFi 子载波高度相关的事实。子载波 15 处的空值可以从子载波 14 和 16 估计。
5. 监控 `RECOVERY_ERROR` -- 残差上升表明相关模型过时且环境已改变。

---

### 最小割人员匹配 (`sig_mincut_person_match.rs`)

**功能**：为感应区域中的最多 4 人维护稳定的身份标签。当人们移动时，他们的 WiFi 签名改变位置 -- 该模块跟踪哪些签名属于哪些人跨连续帧。

**算法**：灵感来自 `ruvector-mincut` (DynamicPersonMatcher)。每帧：

1. **特征提取**：对于每个检测到的人，从其空间区域提取前 8 个子载波方差（降序排序）。这产生一个 8D 签名向量。
2. **成本矩阵**：计算所有当前特征与所有存储签名之间的 L2 距离。
3. **贪婪分配**：选择最小成本（检测，槽）对，将两者标记为已使用，重复。如简化的匈牙利算法，最多 4 人时最优。
4. **签名更新**：通过 EMA (alpha=0.15) 将新特征混合到存储的签名中。
5. **超时**：100 帧 absence 后释放槽。

#### 公共 API

```rust
pub struct PersonMatcher { /* ... */ }

impl PersonMatcher {
    pub const fn new() -> Self;
    pub fn process_frame(&mut self, amplitudes: &[f32], variances: &[f32], n_persons: usize) -> &[(i32, f32)];
    pub fn active_persons() -> u8;
    pub fn total_swaps() -> u32;
    pub fn is_person_stable(slot: usize) -> bool;
    pub fn person_signature(slot: usize) -> Option<&[f32; 8]>;
}
```

#### 事件

| ID | 名称 | 值 | 含义 |
|----|------|-----|------|
| 720 | `PERSON_ID_ASSIGNED` | person_id + confidence*0.01 | 分配了哪个槽（整数部分）和匹配置信度（小数部分） |
| 721 | `PERSON_ID_SWAP` | prev*16 + curr | 检测到身份交换（编码的先前和当前槽索引） |
| 722 | `MATCH_CONFIDENCE` | [0.0, 1.0] | 所有检测到的人的平均匹配置信度（每 10 帧发出） |

#### 配置

| 常量 | 值 | 用途 |
|------|-----|------|
| `MAX_PERSONS` | 4 | 最大同时人员跟踪数 |
| `FEAT_DIM` | 8 | 签名向量维度 |
| `SIG_ALPHA` | 0.15 | 签名更新的 EMA 混合因子 |
| `MAX_MATCH_DISTANCE` | 5.0 | 有效匹配的 L2 距离阈值 |
| `STABLE_FRAMES` | 10 | 轨道被视为稳定之前的帧数 |
| `ABSENT_TIMEOUT` | 100 | 槽释放前的 absence 帧数 (~5s at 20Hz) |

---

### 最优传输 (`sig_optimal_transport.rs`)

**功能**：检测传统基于方差的检测器错过的细微运动。计算 WiFi 信号分布的整体形状在帧之间变化了多少，即使总功率保持不变。

**算法**：切片 Wasserstein 距离 -- 全 Wasserstein（地球移动者）距离的计算效率高的近似值。

1. 生成 4 个固定随机投影方向（确定性 LCG PRNG，编译时 const 计算）
2. 将当前和先前的幅度向量投影到每个方向
3. 对投影值排序（使用 Ciura 间隙的 Shell 排序，O(n^1.3)）
4. 计算排序投影之间的 1D Wasserstein-1 距离（仅平均绝对差）
5. 跨所有 4 个投影平均
6. 通过 EMA 平滑并与阈值比较

**细微运动检测**：当 Wasserstein 距离升高（分布形状改变）但方差稳定（总功率不变）时，某物移动而未产生明显干扰 -- 例如，缓慢的手部运动、呼吸或门缓慢关闭。

#### 公共 API

```rust
pub struct OptimalTransportDetector { /* ... */ }

impl OptimalTransportDetector {
    pub const fn new() -> Self;
    pub fn process_frame(&mut self, amplitudes: &[f32]) -> &[(i32, f32)];
    pub fn distance() -> f32;            // EMA 平滑的 Wasserstein 距离
    pub fn variance_smoothed() -> f32;   // EMA 平滑的方差
    pub fn frame_count() -> u32;
}
```

#### 事件

| ID | 名称 | 值 | 含义 |
|----|------|-----|------|
| 725 | `WASSERSTEIN_DISTANCE` | 距离 | 平滑的切片 Wasserstein 距离（每 5 帧发出） |
| 726 | `DISTRIBUTION_SHIFT` | 距离 | 检测到大型分布变化（去抖，3 个连续帧 > 0.25） |
| 727 | `SUBTLE_MOTION` | 距离 | 尽管方差稳定但检测到运动（5 个连续帧，距离 > 0.10 且方差变化 < 15%） |

#### 配置

| 常量 | 值 | 用途 |
|------|-----|------|
| `N_PROJ` | 4 | 随机投影方向数 |
| `ALPHA` | 0.15 | 距离平滑的 EMA alpha |
| `VAR_ALPHA` | 0.1 | 方差平滑的 EMA alpha |
| `WASS_SHIFT` | 0.25 | 分布偏移事件的 Wasserstein 阈值 |
| `WASS_SUBTLE` | 0.10 | 细微运动的 Wasserstein 阈值 |
| `VAR_STABLE` | 0.15 | "稳定"分类的最大相对方差变化 |
| `SHIFT_DEB` | 3 | 分布偏移的去抖计数 |
| `SUBTLE_DEB` | 5 | 细微运动的去抖计数 |

#### 教程：解释 Wasserstein 距离

Wasserstein 距离测量将一个分布转换为另一个分布的"成本"。与仅测量扩散的基于方差的指标不同，它捕获形状、位置和模式结构的变化。

**典型值**：
- 0.00-0.05：无运动。静态环境。
- 0.05-0.15：呼吸、轻微身体摇摆、环境漂移。
- 0.15-0.30：行走、手臂运动、正常活动。
- 0.30+：大运动、多人移动或突然环境变化。

**为什么"细微运动"重要**：一个人坐着不动并缓慢举手几乎不会改变总信号方差，但 Wasserstein 距离增加，因为信号强度的空间分布发生了变化。这对于以下情况至关重要：
- 跌倒检测（跌倒前的摇摆）
- 手势识别（微运动）
- 入侵者检测（有人试图偷偷移动）

---

## 性能预算

| 模块 | 预算层级 | 典型延迟 | 栈内存 | 关键瓶颈 |
|------|----------|----------|--------|----------|
| 闪存注意力 | S (<5ms) | ~0.5ms | ~512 字节 | 8 组的 Softmax exp() |
| 相干门 | L (<2ms) | ~0.3ms | ~320 字节 | 每个子载波的 sin/cos |
| 时间压缩 | S (<5ms) | ~0.8ms | ~12 KB | 512 快照 * 24 字节 |
| 稀疏恢复 | H (<10ms) | ~3ms | ~768 字节 | 10 次 ISTA 迭代 * 32 子载波 |
| 最小割人员匹配 | H (<10ms) | ~1.5ms | ~640 字节 | 4x4 成本矩阵 + 特征提取 |
| 最优传输 | S (<5ms) | ~1.5ms | ~1 KB | 8 次 Shell 排序（4 个投影 * 2 个分布） |

所有延迟均针对在 240 MHz 运行 WASM3 解释器的 ESP32-S3 估计。实际性能随子载波计数和帧复杂度而变化。

## 内存布局

所有模块使用固定大小的栈/静态分配。无堆、无 `alloc`、无 `Vec`。这是在 ESP32-S3 上部署 `no_std` WASM 所必需的。

所有 6 个信号模块的总静态内存：约 15 KB，远在 ESP32-S3 可用的 WASM 线性内存之内。