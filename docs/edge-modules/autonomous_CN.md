# 量子启发与自主模块 -- WiFi-DensePose 边缘智能

> 受量子计算、神经科学和 AI 规划启发的先进算法。这些模块让 ESP32 做出自主决策，修复自己的网格网络，解释高级场景语义，并使用量子启发搜索探索房间状态。

## 量子启发

| 模块 | 文件 | 功能 | 事件 ID | 预算 |
|------|------|------|---------|------|
| 量子相干 | `qnt_quantum_coherence.rs` | 将 CSI 相位映射到 Bloch 球上以检测突然的环境变化 | 850-852 | H (<10 ms) |
| 干涉搜索 | `qnt_interference_search.rs` | Grover 启发的多假设房间状态分类器 | 855-857 | H (<10 ms) |

---

### 量子相干 (`qnt_quantum_coherence.rs`)

**功能**：将每个子载波的相位映射到量子 Bloch 球上的一个点，并从平均 Bloch 向量幅度计算聚合相干度量。当所有子载波相位对齐时，系统是"相干的"（如量子纯态）。当相位随机散射时，系统是"非相干的"（如最大混合态）。突然的退相干 -- 快速熵峰值 -- 表明环境干扰，如门打开、人进入或家具移动。

**算法**：每个子载波相位被映射到 3D Bloch 向量：
- theta = |phase|（极角）
- phi = sign(phase) * pi/2（方位角）

由于 phi 始终为 +/- pi/2，cos(phi) = 0 且 sin(phi) = +/- 1。这消除了每个子载波的 2 个三角函数调用（每帧 32 个子载波节省 64+ cosf/sinf 调用）。平均 Bloch 向量的 x 分量始终为零。

冯·诺依曼熵：S = -p*log(p) - (1-p)*log(1-p)，其中 p = (1 + |bloch|) / 2。当完全相干时 (|bloch|=1) S=0，当最大混合时 (|bloch|=0) S=ln(2)。EMA 平滑，alpha=0.15。

#### 公共 API

```rust
use wifi_densepose_wasm_edge::qnt_quantum_coherence::QuantumCoherenceMonitor;

let mut mon = QuantumCoherenceMonitor::new();             // const fn
let events = mon.process_frame(&phases);                  // 每帧
let coh = mon.coherence();                                // [0, 1], 1=纯态
let ent = mon.entropy();                                  // [0, ln(2)]
let norm_ent = mon.normalized_entropy();                   // [0, 1]
let bloch = mon.bloch_vector();                           // [f32; 3]
let frames = mon.frame_count();                           // 总帧数
```

#### 事件

| 事件 ID | 常量 | 值 | 频率 |
|---------|------|-----|------|
| 850 | `EVENT_ENTANGLEMENT_ENTROPY` | EMA 平滑的冯·诺依曼熵 [0, ln(2)] | 每 10 帧 |
| 851 | `EVENT_DECOHERENCE_EVENT` | 熵跳跃幅度 (> 0.3) | 检测时 |
| 852 | `EVENT_BLOCH_DRIFT` | 连续 Bloch 向量之间的欧几里得距离 | 每 5 帧 |

#### 配置常量

| 常量 | 值 | 用途 |
|------|-----|------|
| `MAX_SC` | 32 | 最大子载波数 |
| `ALPHA` | 0.15 | EMA 平滑因子 |
| `DECOHERENCE_THRESHOLD` | 0.3 | 熵跳跃阈值 |
| `ENTROPY_EMIT_INTERVAL` | 10 | 熵报告之间的帧数 |
| `DRIFT_EMIT_INTERVAL` | 5 | 漂移报告之间的帧数 |
| `LN2` | 0.693147 | 最大二进制熵 |

#### 示例：通过退相干检测门打开

```
帧 1-50：空房间，相位稳定在 ~0.1 弧度
  Bloch 向量：(0, 0.10, 0.99) -> 相干性 = 0.995
  熵 ~ 0.005（接近零，纯态）

帧 51：门打开，多径突然变化
  相位散射：[-2.1, 0.8, 1.5, -0.3, ...]
  Bloch 向量：(0, 0.12, 0.34) -> 相干性 = 0.36
  熵跳升至 0.61
  -> EVENT_DECOHERENCE_EVENT = 0.605（跳跃幅度）
  -> EVENT_BLOCH_DRIFT = 0.65（大 Bloch 向量位移）

帧 52-100：新的稳定多径
  相位稳定在新值
  熵通过 EMA 逐渐衰减
  不再有退相干事件
```

#### Bloch 球直觉

将每个子载波视为指南针指针。当房间稳定时，所有指针大致指向同一方向（高相干性，低熵）。当某物改变 WiFi 多径 -- 人进入、门打开、家具移动 -- 指针向不同方向散射（低相干性，高熵）。Bloch 球形式主义以数学精确且计算廉价的方式量化这一点。

---

### 干涉搜索 (`qnt_interference_search.rs`)

**功能**：维护 16 个幅度加权的当前房间状态假设（空，A/B/C/D 区域有人，两人，锻炼，睡觉等），并使用 Grover 启发的 oracle+扩散过程收敛到最可能的状态。

**算法**：受 Grover 量子搜索算法启发，适应经典计算：

1. **Oracle**：CSI 证据（存在、运动、人数）根据一致性将假设幅度乘以增强 (1.3) 或抑制 (0.7) 因子。
2. **Grover 扩散**：将所有幅度反射到其平均值（a_i = 2*mean - a_i），将概率质量集中在 oracle 增强的假设上。负幅度被钳制为零（经典近似）。
3. **归一化**：幅度被重新归一化，使平方和 = 1.0（概率守恒）。

经过足够的迭代后，获胜者以概率 > 0.5（收敛阈值）出现。

#### 16 个假设

| 索引 | 假设 | Oracle 证据 |
|------|------|------------|
| 0 | 空 | presence=0 |
| 1-4 | A/B/C/D 区域有人 | presence=1, 1 人 |
| 5 | 两人 | n_persons=2 |
| 6 | 三人 | n_persons>=3 |
| 7 | 向左移动 | 高运动，移动状态 |
| 8 | 向右移动 | 高运动，移动状态 |
| 9 | 坐着 | 低运动，存在 |
| 10 | 站着 | 低运动，存在 |
| 11 | 跌倒 | 高运动（瞬态） |
| 12 | 锻炼 | 高运动，存在 |
| 13 | 睡觉 | 低运动，存在 |
| 14 | 烹饪 | 中等运动 + 移动 |
| 15 | 工作 | 低运动，存在 |

#### 公共 API

```rust
use wifi_densepose_wasm_edge::qnt_interference_search::{InterferenceSearch, Hypothesis};

let mut search = InterferenceSearch::new();               // const fn, 均匀幅度
let events = search.process_frame(presence, motion_energy, n_persons);
let winner = search.winner();                             // Hypothesis 枚举
let prob = search.winner_probability();                   // [0, 1]
let converged = search.is_converged();                    // prob > 0.5
let amp = search.amplitude(Hypothesis::Sleeping);         // 原始幅度
let p = search.probability(Hypothesis::Exercising);       // amplitude^2
let iters = search.iterations();                          // 总迭代次数
search.reset();                                           // 回到均匀
```

#### 事件

| 事件 ID | 常量 | 值 | 频率 |
|---------|------|-----|------|
| 855 | `EVENT_HYPOTHESIS_WINNER` | 获胜假设索引 (0-15) | 每 10 帧或变化时 |
| 856 | `EVENT_HYPOTHESIS_AMPLITUDE` | 获胜假设概率 | 每 20 帧 |
| 857 | `EVENT_SEARCH_ITERATIONS` | 总 Grover 迭代次数 | 每 50 帧 |

#### 配置常量

| 常量 | 值 | 用途 |
|------|-----|------|
| `N_HYPO` | 16 | 房间状态假设数 |
| `CONVERGENCE_PROB` | 0.5 | 宣布收敛的阈值 |
| `ORACLE_BOOST` | 1.3 | 支持假设的幅度乘数 |
| `ORACLE_DAMPEN` | 0.7 | 矛盾假设的幅度乘数 |
| `MOTION_HIGH_THRESH` | 0.5 | "高运动"的运动能量阈值 |
| `MOTION_LOW_THRESH` | 0.15 | "低运动"的运动能量阈值 |

#### 示例：房间状态分类

```
初始状态：所有 16 个假设的概率为 1/16 = 0.0625

帧 1-30：presence=0, motion=0, n_persons=0
  Oracle 增强空 (索引 0)，抑制所有其他
  扩散将概率质量集中在空
  30 次迭代后：P(空) = 0.72, P(其他) < 0.03
  -> EVENT_HYPOTHESIS_WINNER = 0 (空)

帧 31-60：presence=1, motion=0.8, n_persons=1
  Oracle 增强锻炼、向左移动、向右移动
  Oracle 抑制空、坐着、睡觉
  再 30 次迭代后：P(锻炼) = 0.45
  -> EVENT_HYPOTHESIS_WINNER = 12 (锻炼)
  获胜者改变 -> 事件立即发出

帧 61-90：presence=1, motion=0.05, n_persons=1
  Oracle 增强坐着、睡觉、工作、站着
  Oracle 抑制锻炼、向左移动、向右移动
  -> 收敛转移到静态假设
```

---

## 自主系统

| 模块 | 文件 | 功能 | 事件 ID | 预算 |
|------|------|------|---------|------|
| 心理符号 | `aut_psycho_symbolic.rs` | 使用前向链接符号规则的上下文感知推理 | 880-883 | H (<10 ms) |
| 自修复网格 | `aut_self_healing_mesh.rs` | 监控网格节点健康并通过最小割分析自动重新配置 | 885-888 | S (<5 ms) |

---

### 心理符号推理 (`aut_psycho_symbolic.rs`)

**功能**：使用 16 个前向链接规则的知识库，将原始 CSI 衍生特征解释为高级语义结论。给定存在、运动能量、呼吸率、心率、人数、相干性和一天中的时间，它确定诸如"人休息"、"可能入侵者"、"医疗 distress"或"社交活动"等结论。

**算法**：前向链接规则评估。每个规则有 4 个条件槽（feature_id, comparison_op, threshold）。当所有非禁用条件匹配时，规则触发。置信度传播：最终置信度是规则的基础置信度乘以每个条件的匹配质量分数（特征高于/低于阈值的程度，钳制在 [0.5, 1.0]）。矛盾检测通过保留较高置信度的结论来解决互斥结论。

#### 16 条规则

| 规则 | 结论 | 条件 | 基础置信度 |
|------|------|------|------------|
| R0 | 可能入侵者 | 存在 + 高运动 (>=200) + 夜晚 | 0.80 |
| R1 | 人休息 | 存在 + 低运动 (<30) + 呼吸 10-22 BPM | 0.90 |
| R2 | 宠物或环境 | 无存在 + 运动 (>=15) | 0.60 |
| R3 | 社交活动 | 多人 (>=2) + 高运动 (>=100) | 0.70 |
| R4 | 锻炼 | 1 人 + 高运动 (>=150) + 心率升高 (>=100) | 0.80 |
| R5 | 可能跌倒 | 存在 + 突然静止 (motion<10, prev_motion>=150) | 0.70 |
| R6 | 干扰 | 低相干 (<0.4) + 存在 | 0.50 |
| R7 | 睡觉 | 存在 + 非常低运动 (<5) + 夜晚 + 呼吸 (>=8) | 0.90 |
| R8 | 烹饪活动 | 存在 + 中等运动 (40-120) + 傍晚 | 0.60 |
| R9 | 离开家 | 无存在 + 先前运动 (>=50) + 早晨 | 0.65 |
| R10 | 到家 | 存在 + 运动 (>=60) + 低先前运动 (<15) + 傍晚 | 0.70 |
| R11 | 儿童玩耍 | 多人 (>=2) + 非常高运动 (>=250) + 白天 | 0.60 |
| R12 | 办公桌工作 | 1 人 + 低运动 (<20) + 良好相干 (>=0.6) + 早晨 | 0.75 |
| R13 | 医疗 distress | 存在 + 非常高心率 (>=130) + 低运动 (<15) | 0.85 |
| R14 | 房间空（稳定） | 无存在 + 无运动 (<5) + 良好相干 (>=0.6) | 0.95 |
| R15 | 人群聚集 | 多人 (>=4) + 高运动 (>=120) | 0.70 |

#### 矛盾对

这些结论是互斥的。当两者都触发时，只有置信度更高的那个保留：

| 对 A | 对 B |
|------|------|
| 睡觉 | 锻炼 |
| 睡觉 | 社交活动 |
| 房间空（稳定） | 可能入侵者 |
| 人休息 | 锻炼 |

#### 输入特征

| 索引 | 特征 | 来源 | 范围 |
|------|------|------|------|
| 0 | 存在 | 第 2 层 DSP | 0（缺席）或 1（存在） |
| 1 | 运动能量 | 第 2 层 DSP | 0 到 ~1000 |
| 2 | 呼吸 BPM | 第 2 层生命体征 | 0-60 |
| 3 | 心率 BPM | 第 2 层生命体征 | 0-200 |
| 4 | 人数 | 第 2 层占用 | 0-8 |
| 5 | 相干性 | QuantumCoherenceMonitor 或上游 | 0-1 |
| 6 | 时间段 | 主机时钟 | 0=早晨, 1=下午, 2=傍晚, 3=夜晚 |
| 7 | 先前运动 | 内部（自动跟踪） | 0 到 ~1000 |

#### 公共 API

```rust
use wifi_densepose_wasm_edge::aut_psycho_symbolic::PsychoSymbolicEngine;

let mut engine = PsychoSymbolicEngine::new();             // const fn
engine.set_coherence(0.8);                                // 来自上游模块
let events = engine.process_frame(
    presence, motion, breathing, heartrate, n_persons, time_bucket
);
let rules = engine.fired_rules();                         // u16 位图
let count = engine.fired_count();                         // 触发的规则数
let prev = engine.prev_conclusion();                      // 上一个获胜结论 ID
let contras = engine.contradiction_count();                // 总矛盾数
engine.reset();                                           // 清除状态
```

#### 事件

| 事件 ID | 常量 | 值 | 频率 |
|---------|------|-----|------|
| 880 | `EVENT_INFERENCE_RESULT` | 结论 ID (1-16) | 任何规则触发时 |
| 881 | `EVENT_INFERENCE_CONFIDENCE` | 获胜结论的置信度 [0, 1] | 与结果配对 |
| 882 | `EVENT_RULE_FIRED` | 规则索引 (0-15) | 每个触发的规则 |
| 883 | `EVENT_CONTRADICTION` | 编码对：conclusion_a * 100 + conclusion_b | 矛盾时 |

#### 示例：跌倒检测序列

```
帧 1：人快走
  特征：presence=1, motion=200, breathing=20, HR=90, persons=1, time=1
  R4 (锻炼) 触发：置信度 = 0.80 * 0.75 = 0.60
  -> EVENT_INFERENCE_RESULT = 5 (锻炼)
  -> EVENT_INFERENCE_CONFIDENCE = 0.60

帧 2：突然静止 (prev_motion=200, current motion=3)
  R5 (可能跌倒) 触发：置信度 = 0.70 * 0.85 = 0.595
  R1 (人休息) 也触发：置信度 = 0.90 * 0.50 = 0.45
  这两个之间无矛盾
  -> EVENT_RULE_FIRED = 5 (跌倒规则)
  -> EVENT_RULE_FIRED = 1 (休息规则)
  -> EVENT_INFERENCE_RESULT = 6 (可能跌倒，最高置信度)
  -> EVENT_INFERENCE_CONFIDENCE = 0.595
```

---

### 自修复网格 (`aut_self_healing_mesh.rs`)

**功能**：监控 8 节点传感器网格的健康状况，自动检测网络拓扑何时变得脆弱。使用 Stoer-Wagner 最小图割算法找到网格中的最薄弱环节。当最小割值低于阈值时，它识别退化节点并触发重新配置事件。

**算法**：最多 8 个节点的加权图上的 Stoer-Wagner 最小割。边权重是两个端点的最小质量分数 (min(q_i, q_j))。质量分数是每个节点 CSI 相干值的 EMA 平滑 (alpha=0.15)。O(n^3) 复杂度，n=8 时仅 512 次操作。健康和修复模式之间的状态机转换。

#### 公共 API

```rust
use wifi_densepose_wasm_edge::aut_self_healing_mesh::SelfHealingMesh;

let mut mesh = SelfHealingMesh::new();                    // const fn
mesh.update_node_quality(0, coherence);                   // 更新单个节点
let events = mesh.process_frame(&node_qualities);         // 处理所有节点
let q = mesh.node_quality(2);                             // 节点 2 的 EMA 质量
let n = mesh.active_nodes();                              // 计数
let mc = mesh.prev_mincut();                              // 上次最小割值
let healing = mesh.is_healing();                          // 脆弱状态？
let weak = mesh.weakest_node();                           // 节点 ID 或 0xFF
mesh.reset();                                             // 清除状态
```

#### 事件

| 事件 ID | 常量 | 值 | 频率 |
|---------|------|-----|------|
| 885 | `EVENT_NODE_DEGRADED` | 退化节点的索引 (0-7) | 当 min-cut < 0.3 时 |
| 886 | `EVENT_MESH_RECONFIGURE` | 最小割值（脆弱性度量） | 与退化配对 |
| 887 | `EVENT_COVERAGE_SCORE` | 所有活动节点的平均质量 [0, 1] | 每帧 |
| 888 | `EVENT_HEALING_COMPLETE` | 最小割值（现在健康） | 当 min-cut 恢复 >= 0.6 时 |

#### 配置常量

| 常量 | 值 | 用途 |
|------|-----|------|
| `MAX_NODES` | 8 | 最大网格节点数 |
| `QUALITY_ALPHA` | 0.15 | 节点质量的 EMA 平滑 |
| `MINCUT_FRAGILE` | 0.3 | 低于此值，网格被视为脆弱 |
| `MINCUT_HEALTHY` | 0.6 | 高于此值，修复被视为完成 |

#### 状态机

```
                 mincut < 0.3
  [Healthy] ----------------------> [Healing]
      ^                                 |
      |         mincut >= 0.6           |
      +---------------------------------+
```

#### Stoer-Wagner 最小割详情

该算法找到如果移除将图断开为两个组件的边的最小权重。对于 8 节点网格：

1. 从完整的加权邻接矩阵开始
2. 对于每个阶段（共 n-1 个阶段）：
   - 通过重复添加具有最高总边权重的节点来增长集合 A
   - 最后添加的两个节点（prev, last）定义"阶段割" = 到 last 的权重
   - 跟踪所有阶段的全局最小割
   - 合并最后两个节点（合并它们的边权重）
3. 返回 (global_min_cut, 较轻侧的节点)

#### 示例：节点故障和恢复

```
帧 1：所有 4 个节点健康
  qualities = [0.9, 0.85, 0.88, 0.92]
  覆盖范围 = 0.89
  最小割 = 0.85（远高于 0.6）
  -> EVENT_COVERAGE_SCORE = 0.89

帧 50：节点 1 开始退化
  qualities = [0.9, 0.20, 0.88, 0.92]
  EMA 平滑的 quality[1] 逐渐下降
  最小割降至 0.20（边权重使用 min(q_i, q_j)）
  最小割 < 0.3 -> 脆弱！
  -> EVENT_NODE_DEGRADED = 1
  -> EVENT_MESH_RECONFIGURE = 0.20
  -> 网格进入修复模式

  主机固件现在可以：
  - 增加节点 1 的传输功率
  - 绕过节点 1 路由流量
  - 唤醒备份节点
  - 提醒操作员

帧 100：节点 1 恢复（天线重新定位）
  qualities = [0.9, 0.85, 0.88, 0.92]
  最小割爬回 0.85
  最小割 >= 0.6 -> 健康！
  -> EVENT_HEALING_COMPLETE = 0.85
```

---

## 量子启发算法如何帮助 WiFi 传感

这些模块使用量子计算隐喻 -- 不是因为 ESP32 是量子计算机，而是因为量子力学的数学框架自然映射到 CSI 信号分析：

**Bloch 球 / 相干性**：WiFi 子载波相位的行为类似于量子相位。当多径稳定时，所有相位对齐（纯态）。当环境变化时，相位随机化（混合态）。冯·诺依曼熵精确量化这一点，提供比跟踪单个子载波相位更鲁棒的单一标量"变化检测器"。

**Grover 算法 / 假设搜索**：oracle+扩散循环是组合来自多个噪声传感器的证据的原则性方法。与其硬编码"如果 motion > 0.5 则锻炼"，Grover 启发的搜索让多个假设竞争。证据逐渐放大正确假设，同时抑制不正确的假设。这比单一阈值对噪声 CSI 数据更鲁棒。

**为什么不使用经典统计？** 你可以。但量子启发的公式在嵌入式硬件上有三个实际优势：

1. **固定内存**：Bloch 向量始终为 3 个浮点数。假设数组始终为 16 个浮点数。无需动态分配。
2. **优雅降级**：如果 CSI 数据有噪声，Grover 搜索不会崩溃或立即给出错误答案 -- 它只是收敛更慢。
3. **可组合性**：Bloch 球模块的相干性分数直接馈入时间逻辑守卫（规则 3："当相干 < 0.3 时无生命体征"）和心理符号引擎（特征 5：相干性）。这创建了一个管道，其中量子启发的度量为经典推理提供信息。

---

## 内存布局

| 模块 | 状态大小（约） | 静态事件缓冲区 |
|------|--------------|---------------|
| 量子相干 | ~40 字节（3D Bloch 向量 + 2 个熵浮点数 + 计数器） | 3 条目 |
| 干涉搜索 | ~80 字节（16 个幅度 + 计数器） | 3 条目 |
| 心理符号 | ~24 字节（位图 + 计数器 + prev_motion） | 8 条目 |
| 自修复网格 | ~360 字节（8x8 邻接 + 8 个质量 + 状态） | 6 条目 |

所有模块使用固定大小的数组和静态事件缓冲区。无堆分配。完全 no_std 兼容，可在 ESP32-S3 上部署 WASM3。

---

## 跨模块集成

这些模块设计为在管道中协同工作：

```
CSI 帧（第 2 层 DSP）
    |
    v
[量子相干] --coherence--> [心理符号引擎]
    |                                     |
    v                                     v
[干涉搜索]              [推理结果]
    |                                     |
    v                                     v
[房间状态假设]            [GOAP 规划器]
                                         |
                                         v
                                   [模块激活/停用]
                                         |
                                         v
                                   [自修复网格]
                                         |
                                         v
                                   [重新配置事件]
```

量子相干监视器将其相干性分数馈入：
- **心理符号引擎**：作为特征 5（相干性），启用规则 R3（干扰）和 R6（低相干性）
- **时间逻辑守卫**：规则 3 检查"当相干 < 0.3 时无生命体征"
- **自修复网格**：节点质量可以从相干性推导

GOAP 规划器使用推理结果决定激活哪些模块（例如，当有人存在时激活生命体征监控，当房间空时进入低功耗模式）。