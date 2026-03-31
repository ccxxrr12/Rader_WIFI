# 空间与时间智能 -- WiFi-DensePose 边缘智能

> 位置感知、活动模式和自主决策在 ESP32 芯片上运行。这些模块确定人们的位置，学习日常 routines，验证安全规则，并让设备计划自己的行动。

## 空间推理

| 模块 | 文件 | 功能 | 事件 ID | 预算 |
|------|------|------|---------|------|
| 页面排名影响 | `spt_pagerank_influence.rs` | 使用互相关 PageRank 找出多人场景中的主导人物 | 760-762 | S (<5 ms) |
| 微型 HNSW | `spt_micro_hnsw.rs` | 用于 CSI 指纹匹配的设备端近似最近邻搜索 | 765-768 | S (<5 ms) |
| 脉冲跟踪器 | `spt_spiking_tracker.rs` | 使用具有 STDP 学习的 LIF 神经元进行生物启发的人员跟踪 | 770-773 | M (<8 ms) |

---

### 页面排名影响 (`spt_pagerank_influence.rs`)

**功能**：使用 Google 用于排名网页的相同数学方法，找出多人场景中具有最强 WiFi 信号影响的人。最多 4 人被建模为图节点；边权重来自其亚载波相位组（每人 8 个子载波）的归一化互相关。

**算法**：4x4 加权邻接图，由 abs(dot-product) / (norm_a * norm_b) 互相关构建。标准 PageRank 幂迭代，阻尼因子 0.85，10 次迭代，列归一化转移矩阵。每次迭代后，排名归一化为总和 1.0。

#### 公共 API

```rust
use wifi_densepose_wasm_edge::spt_pagerank_influence::PageRankInfluence;

let mut pr = PageRankInfluence::new();          // const fn, zero-alloc
let events = pr.process_frame(&phases, 2);      // phases: &[f32], n_persons: usize
let score = pr.rank(0);                         // 人物 0 的 PageRank 分数
let dom = pr.dominant_person();                  // 主导人物的索引
```

#### 事件

| 事件 ID | 常量 | 值 | 频率 |
|---------|------|-----|------|
| 760 | `EVENT_DOMINANT_PERSON` | 人物索引 (0-3) | 每帧 |
| 761 | `EVENT_INFLUENCE_SCORE` | 主导人物的 PageRank 分数 [0, 1] | 每帧 |
| 762 | `EVENT_INFLUENCE_CHANGE` | 编码的 person_id + 带符号增量 (小数) | 当排名变化 > 0.05 时 |

#### 配置常量

| 常量 | 值 | 用途 |
|------|-----|------|
| `MAX_PERSONS` | 4 | 最大跟踪人数 |
| `SC_PER_PERSON` | 8 | 每人组分配的子载波数 |
| `DAMPING` | 0.85 | PageRank 阻尼因子（标准） |
| `PR_ITERS` | 10 | 幂迭代轮数 |
| `CHANGE_THRESHOLD` | 0.05 | 发出变化事件的最小排名变化 |

#### 示例：检测房间中的主导说话者

当多人存在时，移动最多的人会产生最强的 CSI 干扰。PageRank 识别哪个人的信号最强烈地"影响"其他人。

```
帧 1：人物 0 说话（活跃），人物 1 就座
  -> EVENT_DOMINANT_PERSON = 0, EVENT_INFLUENCE_SCORE = 0.62

帧 50：人物 1 站立并行走
  -> EVENT_DOMINANT_PERSON = 1, EVENT_INFLUENCE_SCORE = 0.58
  -> EVENT_INFLUENCE_CHANGE（人物 1 排名增加 0.08）
```

#### 工作原理（分步）

1. 主机报告 `n_persons` 并提供最多 32 个子载波相位
2. 模块分组子载波：人物 0 获取 phases[0..8]，人物 1 获取 phases[8..16]，依此类推
3. 计算每个人物组对之间的互相关（abs 余弦相似度）
4. 构建 4x4 邻接矩阵（无自环）
5. PageRank 幂迭代运行 10 次，阻尼=0.85
6. 排名最高的人物被报告为主导人物
7. 如果任何人的排名自上一帧以来变化超过 0.05，则触发变化事件

---

### 微型 HNSW (`spt_micro_hnsw.rs`)

**功能**：在单层可导航小世界图中存储最多 64 个参考 CSI 指纹向量（每个 8 维），实现快速近似最近邻查找。当传感器看到新的 CSI 模式时，它找到最相似的存储参考并返回其分类标签。

**算法**：HNSW（分层可导航小世界）简化为嵌入式使用的单层。64 个节点，每个节点 4 个邻居，波束搜索宽度 4，最大 8 跳。L2（欧几里得）距离。双向边，当节点满时使用最差邻居替换剪枝。

#### 公共 API

```rust
use wifi_densepose_wasm_edge::spt_micro_hnsw::MicroHnsw;

let mut hnsw = MicroHnsw::new();                     // const fn, zero-alloc
let idx = hnsw.insert(&features_8d, label);           // Option<usize>
let (nearest_id, distance) = hnsw.search(&query_8d);  // (usize, f32)
let events = hnsw.process_frame(&features);            // 每帧查询
let label = hnsw.last_label();                         // u8 或 255=未知
let dist = hnsw.last_match_distance();                 // f32
let n = hnsw.size();                                   // 存储的向量数
```

#### 事件

| 事件 ID | 常量 | 值 | 频率 |
|---------|------|-----|------|
| 765 | `EVENT_NEAREST_MATCH_ID` | 最近存储向量的索引 | 每帧 |
| 766 | `EVENT_MATCH_DISTANCE` | 到最近匹配的 L2 距离 | 每帧 |
| 767 | `EVENT_CLASSIFICATION` | 最近匹配的标签（太远则为 255） | 每帧 |
| 768 | `EVENT_LIBRARY_SIZE` | 存储的参考向量数 | 每帧 |

#### 配置常量

| 常量 | 值 | 用途 |
|------|-----|------|
| `MAX_VECTORS` | 64 | 最大存储参考指纹数 |
| `DIM` | 8 | 每个特征向量的维度 |
| `MAX_NEIGHBORS` | 4 | 图中每个节点的边数 |
| `BEAM_WIDTH` | 4 | 搜索波束宽度（质量 vs 速度） |
| `MAX_HOPS` | 8 | 最大图遍历深度 |
| `MATCH_THRESHOLD` | 2.0 | 分类返回"未知"的距离阈值 |

#### 示例：房间位置指纹识别

预加载已知位置的参考 CSI 指纹，然后实时分类新读数。

```
设置：
  hnsw.insert(&kitchen_fingerprint, 1);   // 标签 1 = 厨房
  hnsw.insert(&bedroom_fingerprint, 2);   // 标签 2 = 卧室
  hnsw.insert(&bathroom_fingerprint, 3);  // 标签 3 = 浴室

运行时：
  帧到达，特征 = [0.32, 0.15, ...]
  -> EVENT_NEAREST_MATCH_ID = 1（厨房参考）
  -> EVENT_MATCH_DISTANCE = 0.45
  -> EVENT_CLASSIFICATION = 1（厨房）
  -> EVENT_LIBRARY_SIZE = 3
```

#### 工作原理（分步）

1. **插入**：新向量添加到位置 `n_vectors`。模块扫描所有现有节点（N<=64，所以线性扫描没问题）以找到 4 个最近邻居。添加双向边；如果节点已有 4 个邻居，则如果新连接更短，则替换最差（最远）的邻居。
2. **搜索**：从入口点开始，波束搜索（宽度 4）探索最多 8 跳的邻居节点。每次跳扩展当前波束的未访问邻居并插入更近的邻居。当没有跳改善波束时，搜索终止。
3. **分类**：如果最近匹配距离低于 `MATCH_THRESHOLD` (2.0)，则返回其标签。否则，255（未知）。

---

### 脉冲跟踪器 (`spt_spiking_tracker.rs`)

**功能**：使用生物启发的脉冲神经网络在 4 个空间区域中跟踪人的位置。32 个泄漏积分放电 (LIF) 神经元（每个子载波一个）馈入 4 个输出神经元（每个区域一个）。尖峰率最高的区域表示人的位置。区域转换测量速度。

**算法**：LIF 神经元模型，膜泄漏因子 0.95，阈值 1.0，重置为 0.0。STDP（脉冲时间依赖性可塑性）学习：当 pre+post 在 1 帧内激发时增强 LR=0.01，当仅 pre 激发时抑制 LR=0.005。权重钳制在 [0, 2]。区域尖峰率的 EMA 平滑（alpha=0.1）。

#### 公共 API

```rust
use wifi_densepose_wasm_edge::spt_spiking_tracker::SpikingTracker;

let mut st = SpikingTracker::new();                       // const fn
let events = st.process_frame(&phases, &prev_phases);     // 返回事件
let zone = st.current_zone();                             // i8, -1 表示丢失
let rate = st.zone_spike_rate(0);                         // 区域 0 的 f32
let vel = st.velocity();                                  // EMA 速度
let tracking = st.is_tracking();                          // bool
```

#### 事件

| 事件 ID | 常量 | 值 | 频率 |
|---------|------|-----|------|
| 770 | `EVENT_TRACK_UPDATE` | 区域 ID (0-3) | 当被跟踪时 |
| 771 | `EVENT_TRACK_VELOCITY` | 区域转换/帧 (EMA) | 当被跟踪时 |
| 772 | `EVENT_SPIKE_RATE` | 跨区域的平均尖峰率 [0, 1] | 每帧 |
| 773 | `EVENT_TRACK_LOST` | 最后已知的区域 ID | 当跟踪丢失时 |

#### 配置常量

| 常量 | 值 | 用途 |
|------|-----|------|
| `N_INPUT` | 32 | 输入神经元（每个子载波一个） |
| `N_OUTPUT` | 4 | 输出神经元（每个区域一个） |
| `THRESHOLD` | 1.0 | LIF 激发阈值 |
| `LEAK` | 0.95 | 每帧的膜衰减 |
| `STDP_LR_PLUS` | 0.01 | 增强学习率 |
| `STDP_LR_MINUS` | 0.005 | 抑制学习率 |
| `W_MIN` / `W_MAX` | 0.0 / 2.0 | 权重边界 |
| `MIN_SPIKE_RATE` | 0.05 | 考虑区域活跃的最小速率 |

#### 示例：跟踪区域间的移动

```
帧 1-30：子载波 0-7（区域 0）的强相位变化
  -> EVENT_TRACK_UPDATE = 0, EVENT_SPIKE_RATE = 0.15

帧 31-60：活动转移到子载波 16-23（区域 2）
  -> EVENT_TRACK_UPDATE = 2, EVENT_TRACK_VELOCITY = 0.033
  STDP 加强区域 2 连接，减弱区域 0

帧 61-90：无活动
  -> 尖峰率通过 EMA 衰减
  -> EVENT_TRACK_LOST = 2（最后已知区域）
```

#### 工作原理（分步）

1. 相位增量（|当前 - 先前|）将电流注入 LIF 神经元
2. 每个神经元泄漏（膜 *= 0.95），然后添加电流
3. 如果膜 >= 阈值 (1.0)，神经元激发并重置为 0
4. 输入尖峰通过加权连接传播到输出区域
5. 当累积输入超过阈值时，输出神经元激发
6. STDP 调整权重：相关的 pre+post 激发加强连接，不相关的 pre 激发减弱连接（稀疏迭代跳过沉默神经元，节省 70-90%）
7. 区域尖峰率通过 EMA 平滑；尖峰率最高且高于 `MIN_SPIKE_RATE` 的区域被报告为跟踪位置

---

## 时间分析

| 模块 | 文件 | 功能 | 事件 ID | 预算 |
|------|------|------|---------|------|
| 模式序列 | `tmp_pattern_sequence.rs` | 学习日常活动常规并检测偏差 | 790-793 | S (<5 ms) |
| 时间逻辑守卫 | `tmp_temporal_logic_guard.rs` | 每帧验证 8 个 LTL 安全不变量 | 795-797 | S (<5 ms) |
| GOAP 自主性 | `tmp_goap_autonomy.rs` | 通过 A* 目标导向规划进行自主模块管理 | 800-803 | S (<5 ms) |

---

### 模式序列 (`tmp_pattern_sequence.rs`)

**功能**：学习日常活动常规并在发生变化时发出警报。每分钟被离散化为运动符号（Empty, Still, LowMotion, HighMotion, MultiPerson），存储在 24 小时环形缓冲区（1440 条目）中。今天和昨天之间的每小时 LCS（最长公共子序列）比较产生常规置信度分数。如果奶奶通常在 8 点前进入厨房但未移动，它会注意到。

**算法**：两行动态规划 LCS，O(n) 内存（60 条目比较窗口）。从每帧累积中选择多数投票符号。两天历史缓冲区，带日期滚动。

#### 公共 API

```rust
use wifi_densepose_wasm_edge::tmp_pattern_sequence::PatternSequenceAnalyzer;

let mut psa = PatternSequenceAnalyzer::new();            // const fn
psa.on_frame(presence, motion, n_persons);               // 每 CSI 帧调用 (~20 Hz)
let events = psa.on_timer();                             // ~1 Hz 调用
let conf = psa.routine_confidence();                     // [0, 1]
let n = psa.pattern_count();                             // 存储的模式
let min = psa.current_minute();                          // 0-1439
let day = psa.day_offset();                              // 自开始以来的天数
```

#### 事件

| 事件 ID | 常量 | 值 | 频率 |
|---------|------|-----|------|
| 790 | `EVENT_PATTERN_DETECTED` | 检测到的模式的 LCS 长度 | 每小时 |
| 791 | `EVENT_PATTERN_CONFIDENCE` | 常规置信度 [0, 1] | 每小时 |
| 792 | `EVENT_ROUTINE_DEVIATION` | 偏差发生的分钟索引 | 每分钟（当偏差时） |
| 793 | `EVENT_PREDICTION_NEXT` | 预测的下一分钟符号（来自昨天） | 每分钟 |

#### 配置常量

| 常量 | 值 | 用途 |
|------|-----|------|
| `DAY_LEN` | 1440 | 每天的分钟数 |
| `MAX_PATTERNS` | 32 | 最大存储模式模板数 |
| `PATTERN_LEN` | 16 | 每个模式的最大符号数 |
| `LCS_WINDOW` | 60 | 比较窗口（1 小时） |
| `THRESH_STILL` / `THRESH_LOW` / `THRESH_HIGH` | 0.05 / 0.3 / 0.7 | 运动离散化阈值 |

#### 符号

| 符号 | 值 | 条件 |
|------|-----|------|
| Empty | 0 | 无存在 |
| Still | 1 | 存在，运动 < 0.05 |
| LowMotion | 2 | 存在，0.3 < 运动 <= 0.7 |
| HighMotion | 3 | 存在，运动 > 0.7 |
| MultiPerson | 4 | 超过 1 人存在 |

#### 示例：老年人护理常规监控

```
第 1 天：学习阶段
  07:00 - Still（人在床上）
  07:30 - HighMotion（准备）
  08:00 - LowMotion（早餐）
  -> 模式存储在历史缓冲区

第 2 天：比较活跃
  07:00 - Still（正常）
  07:30 - Still（偏差！预期 HighMotion）
    -> EVENT_ROUTINE_DEVIATION = 450（7:30 分钟）
    -> EVENT_PREDICTION_NEXT = 3（预期 HighMotion）
  08:30 - Still（仍然无活动）
    -> 护理人员通过 DEVIATION 事件收到通知
```

---

### 时间逻辑守卫 (`tmp_temporal_logic_guard.rs`)

**功能**：将 8 个安全规则编码为线性时间逻辑 (LTL) 状态机。G 规则（"全局"）在任何单帧上被违反。F 规则（"最终"）有截止日期。每帧，守卫检查所有规则并发出带有反例帧索引的违规。

**算法**：每个规则的状态机（Satisfied/Pending/Violated）。G 规则使用即时布尔检查。F 规则使用截止日期计数器（基于帧）。反例跟踪记录违规首次发生的帧索引。

#### 8 个安全规则

| 规则 | 类型 | 描述 | 违规条件 |
|------|------|------|----------|
| R0 | G | 房间空时无跌倒警报 | `presence==0 AND fall_alert` |
| R1 | G | 无人存在时无入侵警报 | `intrusion_alert AND presence==0` |
| R2 | G | 无人检测时无人员 ID 活跃 | `n_persons==0 AND person_id_active` |
| R3 | G | 相干性过低时无生命体征 | `coherence<0.3 AND vital_signs_active` |
| R4 | F | 持续运动必须在 300 秒内停止 | 连续 6000 帧运动 > 0.1 |
| R5 | F | 快速呼吸必须在 5 秒内触发警报 | 连续 100 帧呼吸 > 40 BPM |
| R6 | G | 心率不得超过 150 BPM | `heartrate_bpm > 150` |
| R7 | G-F | 癫痫发作后 60 秒内无正常步态 | 癫痫发作后 < 1200 帧报告正常步态 |

#### 公共 API

```rust
use wifi_densepose_wasm_edge::tmp_temporal_logic_guard::{TemporalLogicGuard, FrameInput};

let mut guard = TemporalLogicGuard::new();               // const fn
let events = guard.on_frame(&input);                     // 每帧检查
let satisfied = guard.satisfied_count();                 // 多少规则 OK
let state = guard.rule_state(4);                         // Satisfied/Pending/Violated
let vio = guard.violation_count(0);                      // 规则 0 的总违规数
let frame = guard.last_violation_frame(3);               // 最后违规的帧索引
```

#### 事件

| 事件 ID | 常量 | 值 | 频率 |
|---------|------|-----|------|
| 795 | `EVENT_LTL_VIOLATION` | 规则索引 (0-7) | 违规时 |
| 796 | `EVENT_LTL_SATISFACTION` | 当前满足的规则数 | 每 200 帧 |
| 797 | `EVENT_COUNTEREXAMPLE` | 违规发生的帧索引 | 与违规配对 |

---

### GOAP 自主性 (`tmp_goap_autonomy.rs`)

**功能**：让 ESP32 根据当前情况自主决定激活或停用哪些传感模块。使用目标导向行动计划 (GOAP)，通过 A* 搜索 8 位布尔世界状态，找到实现最高优先级未满足目标的最便宜行动序列。

**算法**：8 位世界状态的 A* 搜索。6 个优先级目标，8 个带有前置条件和效果的行动，编码为位掩码。最大计划深度 4，开放集容量 32。每 60 秒重新计划。

#### 世界状态属性

| 位 | 属性 | 含义 |
|-----|------|------|
| 0 | `has_presence` | 检测到房间占用 |
| 1 | `has_motion` | 运动能量高于阈值 |
| 2 | `is_night` | 夜间时段 |
| 3 | `multi_person` | 超过 1 人存在 |
| 4 | `low_coherence` | 信号质量下降 |
| 5 | `high_threat` | 威胁分数高于阈值 |
| 6 | `has_vitals` | 生命体征监控活跃 |
| 7 | `is_learning` | 模式学习活跃 |

#### 目标（优先级顺序）

| # | 目标 | 优先级 | 条件 |
|---|------|--------|------|
| 0 | 监控健康 | 0.9 | 实现 `has_vitals = true` |
| 1 | 保护空间 | 0.8 | 实现 `has_presence = true` |
| 2 | 统计人数 | 0.7 | 实现 `multi_person = false` |
| 3 | 学习模式 | 0.5 | 实现 `is_learning = true` |
| 4 | 节省能源 | 0.3 | 实现 `is_learning = false` |
| 5 | 自检 | 0.1 | 实现 `low_coherence = false` |

#### 行动

| # | 行动 | 前置条件 | 效果 | 成本 |
|---|------|----------|------|------|
| 0 | 激活生命体征 | 需要存在 | 设置 `has_vitals` | 2 |
| 1 | 激活入侵检测 | 无 | 设置 `has_presence` | 1 |
| 2 | 激活占用检测 | 需要存在 | 清除 `multi_person` | 2 |
| 3 | 激活手势学习 | 低相干性必须为 false | 设置 `is_learning` | 3 |
| 4 | 停用重型模块 | 无 | 清除 `is_learning` + `has_vitals` | 1 |
| 5 | 运行相干性检查 | 无 | 清除 `low_coherence` | 2 |
| 6 | 进入低功耗 | 无 | 清除 `is_learning` + `has_motion` | 1 |
| 7 | 运行自检 | 无 | 清除 `low_coherence` + `high_threat` | 3 |

#### 公共 API

```rust
use wifi_densepose_wasm_edge::tmp_goap_autonomy::GoapPlanner;

let mut planner = GoapPlanner::new();                    // const fn
planner.update_world(presence, motion, n_persons,
                     coherence, threat, has_vitals, is_night);
let events = planner.on_timer();                         // ~1 Hz 调用
let ws = planner.world_state();                          // u8 位掩码
let goal = planner.current_goal();                       // 目标索引或 0xFF
let len = planner.plan_len();                            // 当前计划中的步骤
planner.set_goal_priority(0, 0.95);                      // 动态调整
```

#### 事件

| 事件 ID | 常量 | 值 | 频率 |
|---------|------|-----|------|
| 800 | `EVENT_GOAL_SELECTED` | 目标索引 (0-5) | 重新计划时 |
| 801 | `EVENT_MODULE_ACTIVATED` | 激活模块的行动索引 | 计划步骤时 |
| 802 | `EVENT_MODULE_DEACTIVATED` | 停用模块的行动索引 | 计划步骤时 |
| 803 | `EVENT_PLAN_COST` | 计划行动序列的总成本 | 重新计划时 |

#### 示例：自主夜间模式转换

```
18:00 - 世界状态：presence=1, motion=0, night=0, vitals=1
  目标 0（监控健康）满足，目标 1（保护空间）满足
  -> 选择目标 2（统计人数，优先级 0.7）

22:00 - 世界状态：presence=0, motion=0, night=1
  -> 选择目标 1（保护空间，优先级 0.8）
  -> 计划：[行动 1: 激活入侵检测]（成本=1）
  -> EVENT_GOAL_SELECTED = 1
  -> EVENT_MODULE_ACTIVATED = 1（入侵检测）
  -> EVENT_PLAN_COST = 1

03:00 - 无存在，检测到低相干性
  -> 选择目标 5（自检，优先级 0.1）
  -> 计划：[行动 5: 运行相干性检查]（成本=2）
```

---

## 内存布局摘要

所有模块使用固定大小的数组和静态事件缓冲区。无堆分配。

| 模块 | 状态大小（约） | 静态事件缓冲区 |
|------|--------------|---------------|
| 页面排名影响 | ~192 字节（4x4 邻接 + 2x4 排名 + 元数据） | 8 条目 |
| 微型 HNSW | ~3.5 KB（64 节点 x 48 字节 + 元数据） | 4 条目 |
| 脉冲跟踪器 | ~1.1 KB（32x4 权重 + 膜 + 速率） | 4 条目 |
| 模式序列 | ~3.2 KB（2x1440 历史 + 32 模式 + LCS 行） | 4 条目 |
| 时间逻辑守卫 | ~120 字节（8 规则 + 计数器） | 12 条目 |
| GOAP 自主性 | ~1.6 KB（32 开放集节点 + 目标 + 计划） | 4 条目 |

## 与主机固件集成

这些模块通过 WASM3 主机 API 从 ESP32 第 2 层 DSP 管道接收数据：

```
ESP32 固件 (C)          WASM3 运行时            WASM 模块 (Rust)
       |                         |                         |
  CSI 帧到达              |                         |
  第 2 层 DSP 运行                |                         |
       |--- csi_get_phase() ---->|--- host_get_phase() --->|
       |--- csi_get_presence() ->|--- host_get_presence()->|
       |                         |     process_frame()     |
       |<-- csi_emit_event() ----|<-- host_emit_event() ---|
       |                         |                         |
  转发到聚合器          |                         |
```

模块可以通过 OTA（ADR-040）热加载，无需重新刷新固件。