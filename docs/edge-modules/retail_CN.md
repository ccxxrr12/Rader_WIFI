# 零售与 hospitality 模块 -- WiFi-DensePose 边缘智能

> 无需摄像头或同意书即可了解客户行为。统计队列长度、绘制客流热力图、跟踪餐桌周转率、衡量货架互动度 -- 所有这些都通过已有的 WiFi 信号实现。

## 概述

| 模块 | 文件 | 功能 | 事件 ID | 帧预算 |
|------|------|------|---------|--------|
| 队列长度 | `ret_queue_length.rs` | 使用 Little's Law 估算队列长度和等待时间 | 400-403 | ~0.5 us/帧 |
| 停留热力图 | `ret_dwell_heatmap.rs` | 跟踪每个空间区域（3x3 网格）的停留时间 | 410-413 | ~1 us/帧 |
| 客户流动 | `ret_customer_flow.rs` | 方向性客流计数（入口/出口） | 420-423 | ~1.5 us/帧 |
| 餐桌周转 | `ret_table_turnover.rs` | 餐厅餐桌生命周期跟踪，带周转率计算 | 430-433 | ~0.3 us/帧 |
| 货架互动 | `ret_shelf_engagement.rs` | 检测和分类客户与货架的互动 | 440-443 | ~1 us/帧 |

所有模块针对运行 WASM3 的 ESP32-S3（ADR-040 第 3 层）。它们从第 2 层 DSP 接收预处理的 CSI 信号，并通过 `csi_emit_event()` 发出结构化事件。

---

## 模块

### 队列长度估算 (`ret_queue_length.rs`)

**功能**：估算队列中的人数，计算到达率和服务率，使用 Little's Law (L = λ x W) 估算等待时间，并在队列超过可配置阈值时触发警报。

**工作原理**：该模块跟踪帧间人数变化，以检测到达（计数增加或有新存在且方差峰值）和离开（计数减少或存在边缘且低运动）。在 30 秒窗口内，它计算到达率（λ）和服务率（μ），单位为每分钟人数。队列长度通过对原始人数的 EMA 进行平滑处理。等待时间估计为 `队列长度 / (到达率 / 60)`。

#### 事件

| 事件 ID | 名称 | 值 | 触发时机 |
|---------|------|-----|----------|
| 400 | `QUEUE_LENGTH` | 估计队列长度 (0-20) | 每 20 帧 (1 秒) |
| 401 | `WAIT_TIME_ESTIMATE` | 估计等待时间（秒） | 每 600 帧 (30 秒窗口) |
| 402 | `SERVICE_RATE` | 服务率（人/分钟，平滑后） | 每 600 帧 (30 秒窗口) |
| 403 | `QUEUE_ALERT` | 当前队列长度 | 当队列 >= 5 时（一次，低于 4 时重置） |

#### API

```rust
use wifi_densepose_wasm_edge::ret_queue_length::QueueLengthEstimator;

let mut q = QueueLengthEstimator::new();

// 每帧：存在 (0/1)、人数、方差、运动能量
let events = q.process_frame(presence, n_persons, variance, motion_energy);

// 查询
q.queue_length()  // -> u8 (0-20, 平滑后)
q.arrival_rate()  // -> f32 (人/分钟，EMA 平滑)
q.service_rate()  // -> f32 (人/分钟，EMA 平滑)
```

#### 配置常量

| 常量 | 值 | 描述 |
|------|-----|------|
| `REPORT_INTERVAL` | 20 帧 (1 秒) | 队列长度报告间隔 |
| `SERVICE_WINDOW_FRAMES` | 600 帧 (30 秒) | 速率计算窗口 |
| `QUEUE_EMA_ALPHA` | 0.1 | 队列长度的 EMA 平滑系数 |
| `RATE_EMA_ALPHA` | 0.05 | 到达/服务率的 EMA 平滑系数 |
| `JOIN_VARIANCE_THRESH` | 0.05 | 加入检测的方差峰值阈值 |
| `DEPART_MOTION_THRESH` | 0.02 | 离开检测的运动阈值 |
| `QUEUE_ALERT_THRESH` | 5.0 | 触发警报的队列长度 |
| `MAX_QUEUE` | 20 | 跟踪的最大队列长度 |

#### 示例：零售队列管理

```python
# 响应队列事件
if event_id == 400:  # QUEUE_LENGTH
    queue_len = int(value)
    dashboard.update_queue(register_id, queue_len)

elif event_id == 401:  # WAIT_TIME_ESTIMATE
    wait_seconds = value
    signage.show(f"预计等待: {int(wait_seconds / 60)} 分钟")

elif event_id == 403:  # QUEUE_ALERT
    staff_pager.send(f"收银台 {register_id}: 队列中有 {int(value)} 人")
```

---

### 停留热力图 (`ret_dwell_heatmap.rs`)

**功能**：将感应区域划分为 3x3 网格（9 个区域），跟踪客户在每个区域停留的时间。识别"热点区域"（停留时间最长）和"冷点区域"（停留时间最短）。当空间为空时发出会话摘要，实现商店布局优化。

**工作原理**：子载波被分为 9 组，每组对应一个区域。每个区域的方差通过 EMA 平滑并与阈值比较。当方差超过阈值且检测到存在时，停留时间以每帧 0.05 秒的速度累积。会话在有人进入时开始，在 100 帧（5 秒）空窗后结束。

#### 事件

| 事件 ID | 名称 | 值编码 | 触发时机 |
|---------|------|---------|----------|
| 410 | `DWELL_ZONE_UPDATE` | `zone_id * 1000 + dwell_seconds` | 每 600 帧 (30 秒) 每个占用区域 |
| 411 | `HOT_ZONE` | `zone_id + dwell_seconds/1000` | 每 600 帧 (30 秒) |
| 412 | `COLD_ZONE` | `zone_id + dwell_seconds/1000` | 每 600 帧 (30 秒) |
| 413 | `SESSION_SUMMARY` | 会话持续时间（秒） | 占用后空间变空时 |

**DWELL_ZONE_UPDATE 值解码**：区域 ID 编码在千位。例如，`value = 2015.5` 表示区域 2，停留时间 15.5 秒。

#### API

```rust
use wifi_densepose_wasm_edge::ret_dwell_heatmap::DwellHeatmapTracker;

let mut t = DwellHeatmapTracker::new();

// 每帧：存在 (0/1)、每个子载波的方差、运动能量、人数
let events = t.process_frame(presence, &variances, motion_energy, n_persons);

// 查询
t.zone_dwell(zone_id)       // -> f32 (当前会话中的秒数)
t.zone_total_dwell(zone_id) // -> f32 (所有会话的总秒数)
t.is_zone_occupied(zone_id) // -> bool
t.is_session_active()       // -> bool
```

#### 配置常量

| 常量 | 值 | 描述 |
|------|-----|------|
| `NUM_ZONES` | 9 | 空间区域（3x3 网格） |
| `REPORT_INTERVAL` | 600 帧 (30 秒) | 热力图更新间隔 |
| `ZONE_OCCUPIED_THRESH` | 0.015 | 区域占用的方差阈值 |
| `ZONE_EMA_ALPHA` | 0.12 | 区域方差的 EMA 平滑系数 |
| `EMPTY_FRAMES_FOR_SUMMARY` | 100 帧 (5 秒) | 会话结束前的空置持续时间 |
| `MAX_EVENTS` | 12 | 每帧最大事件数 |

#### 区域布局

3x3 网格映射到物理空间：

```
+-------+-------+-------+
|  Z0   |  Z1   |  Z2   |
|       |       |       |
+-------+-------+-------+
|  Z3   |  Z4   |  Z5   |
|       |       |       |
+-------+-------+-------+
|  Z6   |  Z7   |  Z8   |
|       |       |       |
+-------+-------+-------+
   Near    Mid      Far
```

子载波平均分配：27 个子载波时，每个区域获得 3 个子载波。低索引子载波对应较近的菲涅尔区。

---

### 客户流动计数 (`ret_customer_flow.rs`)

**功能**：使用方向性相位梯度分析计算通过门口或通道的进入和离开人数。维护累计进入/离开计数并报告净占用率（进入 - 离开，钳制为零）。每小时发出流量摘要。

**工作原理**：子载波分为两组：低索引（靠近入口）和高索引（远离入口）。人穿过感应区域会导致不对称的相位速度模式 -- 进入时低侧组的相位变化先于高侧组，离开时则相反。方向性梯度（低梯度 - 高梯度）通过 EMA 平滑并阈值化。结合运动能量和幅度峰值检测，可区分真实穿越和噪声。

```
进入：正平滑梯度（低侧相位领先）
离开：负平滑梯度（高侧相位领先）
```

#### 事件

| 事件 ID | 名称 | 值 | 触发时机 |
|---------|------|-----|----------|
| 420 | `INGRESS` | 累计进入计数 | 每次检测到进入时 |
| 421 | `EGRESS` | 累计离开计数 | 每次检测到离开时 |
| 422 | `NET_OCCUPANCY` | 当前净占用率 (>= 0) | 穿越时 + 每 100 帧 |
| 423 | `HOURLY_TRAFFIC` | `ingress * 1000 + egress` | 每 72000 帧 (1 小时) |

**HOURLY_TRAFFIC 解码**：`ingress = int(value / 1000)`，`egress = int(value % 1000)`。

#### API

```rust
use wifi_densepose_wasm_edge::ret_customer_flow::CustomerFlowTracker;

let mut cf = CustomerFlowTracker::new();

// 每帧：每个子载波的相位、幅度、方差、运动能量
let events = cf.process_frame(&phases, &amplitudes, variance, motion_energy);

// 查询
cf.net_occupancy()    // -> i32 (进入 - 离开，钳制为 0)
cf.total_ingress()    // -> u32 (累计进入)
cf.total_egress()     // -> u32 (累计离开)
cf.current_gradient() // -> f32 (平滑后的方向性梯度)
```

#### 配置常量

| 常量 | 值 | 描述 |
|------|-----|------|
| `PHASE_GRADIENT_THRESH` | 0.15 | 穿越的最小梯度幅度 |
| `MOTION_THRESH` | 0.03 | 有效穿越的最小运动能量 |
| `AMPLITUDE_SPIKE_THRESH` | 1.5 | 幅度变化比例因子 |
| `CROSSING_DEBOUNCE` | 10 帧 (0.5 秒) | 穿越事件之间的去抖 |
| `GRADIENT_EMA_ALPHA` | 0.2 | 梯度的 EMA 平滑系数 |
| `OCCUPANCY_REPORT_INTERVAL` | 100 帧 (5 秒) | 净占用率报告间隔 |

#### 示例：商店占用显示

```python
# 商店入口的实时占用计数器
if event_id == 422:  # NET_OCCUPANCY
    occupancy = int(value)
    display.show(f"当前店内人数: {occupancy}")

    if occupancy >= max_capacity:
        door_signal.set("等待")
    else:
        door_signal.set("进入")

elif event_id == 423:  # HOURLY_TRAFFIC
    ingress = int(value / 1000)
    egress = int(value % 1000)
    analytics.log_hourly(hour, ingress, egress)
```

---

### 餐桌周转跟踪 (`ret_table_turnover.rs`)

**功能**：跟踪餐厅餐桌的完整生命周期 -- 从客人坐下，用餐，到离开和清理。测量就座持续时间并计算滚动周转率（每小时周转次数）。设计用于每张餐桌或餐桌组一个 ESP32 节点。

**工作原理**：五状态机处理存在、运动能量和人数：

```
空桌 --> 用餐 --> 离开 --> 冷却 --> 空桌
  |       (2秒          (运动      (30秒         |
  |       去抖)    增加)    清理)     |
  |                                              |
  +----------------------------------------------+
          (短暂离开：保持在用餐状态)
```

`就座`状态在枚举中为完整性而存在，但转换直接处理（空桌 -> 用餐，经过去抖）。`离开`状态检测客人何时显示增加的运动和减少的人数。空置需要 5 秒的确认 absence 以避免短暂 bathroom breaks 引起的误触发。

#### 事件

| 事件 ID | 名称 | 值 | 触发时机 |
|---------|------|-----|----------|
| 430 | `TABLE_SEATED` | 就座时的人数 | 40 帧去抖后 |
| 431 | `TABLE_VACATED` | 就座持续时间（秒） | 100 帧 absence 去抖后 |
| 432 | `TABLE_AVAILABLE` | 1.0 | 30 秒清理冷却后 |
| 433 | `TURNOVER_RATE` | 每小时周转率（滚动） | 每 6000 帧 (5 分钟) |

#### API

```rust
use wifi_densepose_wasm_edge::ret_table_turnover::TableTurnoverTracker;

let mut tt = TableTurnoverTracker::new();

// 每帧：存在 (0/1)、运动能量、人数
let events = tt.process_frame(presence, motion_energy, n_persons);

// 查询
tt.state()             // -> TableState (Empty|Seating|Eating|Departing|Cooldown)
tt.total_turnovers()   // -> u32 (累计周转次数)
tt.session_duration_s() // -> f32 (当前会话长度，秒)
tt.turnover_rate()     // -> f32 (每小时周转率，滚动窗口)
```

#### 状态机

| 状态 | 进入条件 | 退出条件 |
|------|----------|----------|
| `Empty` | 餐桌空闲 | 40 帧 (2 秒) 连续存在 |
| `Eating` | 客人确认就座 | 100 帧 (5 秒) absence -> 冷却；高运动 + 人数减少 -> 离开 |
| `Departing` | 高运动且人数下降 | 100 帧 absence -> 冷却；运动稳定 -> 回到用餐 |
| `Cooldown` | 餐桌空置，清理期 | 600 帧 (30 秒) -> 空桌；冷却期间存在 -> 用餐（快速重新就座） |

#### 配置常量

| 常量 | 值 | 描述 |
|------|-----|------|
| `SEATED_DEBOUNCE_FRAMES` | 40 帧 (2 秒) | 标记就座前的确认 |
| `VACATED_DEBOUNCE_FRAMES` | 100 帧 (5 秒) | 空置前的 absence 确认 |
| `AVAILABLE_COOLDOWN_FRAMES` | 600 帧 (30 秒) | 标记可用前的清理时间 |
| `EATING_MOTION_THRESH` | 0.1 | 低于此值的运动 = 稳定/用餐 |
| `ACTIVE_MOTION_THRESH` | 0.3 | 高于此值的运动 = 到达/离开 |
| `TURNOVER_REPORT_INTERVAL` | 6000 帧 (5 分钟) | 速率报告间隔 |
| `MAX_TURNOVERS` | 50 | 速率的滚动窗口缓冲区 |

#### 示例：餐厅运营仪表板

```python
# 餐厅餐桌管理
if event_id == 430:  # TABLE_SEATED
    party_size = int(value)
    kitchen.notify(f"餐桌 {table_id}: {party_size} 位客人就座")
    pos.start_timer(table_id)

elif event_id == 431:  # TABLE_VACATED
    duration_s = value
    analytics.log_seating(table_id, duration_s, peak_persons)
    staff.alert(f"餐桌 {table_id}: 需要清理 ({duration_s/60:.0f} 分钟使用)")

elif event_id == 432:  # TABLE_AVAILABLE
    hostess_display.mark_available(table_id)

elif event_id == 433:  # TURNOVER_RATE
    rate = value
    manager_dashboard.update(table_id, turnovers_per_hour=rate)
```

---

### 货架互动检测 (`ret_shelf_engagement.rs`)

**功能**：检测客户何时停在货架前，并分类其互动水平：浏览（不到 5 秒）、考虑（5-30 秒）或深度互动（超过 30 秒）。还检测伸手动作（手/臂向货架移动）。利用人站立但与产品互动时产生高频相位扰动且平移运动低的原理。

**工作原理**：关键洞察是区分两种 CSI 相位变化：
- **平移运动**（行走）：所有子载波的大型均匀相移
- **局部互动**（伸手、检查）：帧间相位差的高空间方差

该模块计算每个子载波相位差的标准偏差。高标准差且整体运动低表示货架互动。伸手动作会产生超过更高阈值的高频扰动突发。

#### 互动分类

| 级别 | 持续时间 | 描述 | 事件 ID |
|------|----------|------|---------|
| 无 | -- | 无互动（缺席或行走） | -- |
| 浏览 | < 5 秒 | 短暂 glance，路过兴趣 | 440 |
| 考虑 | 5-30 秒 | 检查，阅读标签，比较 | 441 |
| 深度互动 | > 30 秒 | 扩展互动，决策制定 | 442 |

`REACH_DETECTED` 事件 (443) 在客户站立时检测到突然的高频相位突发时独立触发。

#### 事件

| 事件 ID | 名称 | 值 | 触发时机 |
|---------|------|-----|----------|
| 440 | `SHELF_BROWSE` | 互动持续时间（秒） | 分类时（带冷却） |
| 441 | `SHELF_CONSIDER` | 互动持续时间（秒） | 级别升级时 |
| 442 | `SHELF_ENGAGE` | 互动持续时间（秒） | 级别升级时 |
| 443 | `REACH_DETECTED` | 相位扰动幅度 | 每次伸手突发 |

#### API

```rust
use wifi_densepose_wasm_edge::ret_shelf_engagement::ShelfEngagementDetector;

let mut se = ShelfEngagementDetector::new();

// 每帧：存在 (0/1)、运动能量、方差、每个子载波的相位
let events = se.process_frame(presence, motion_energy, variance, &phases);

// 查询
se.engagement_level()     // -> EngagementLevel (None|Browse|Consider|DeepEngage)
se.engagement_duration_s() // -> f32 (秒)
se.total_browse_events()   // -> u32
se.total_consider_events() // -> u32
se.total_engage_events()   // -> u32
se.total_reach_events()    // -> u32
```

#### 配置常量

| 常量 | 值 | 描述 |
|------|-----|------|
| `BROWSE_THRESH_S` | 5.0 秒 (100 帧) | 浏览的互动时间 |
| `CONSIDER_THRESH_S` | 30.0 秒 (600 帧) | 考虑的互动时间 |
| `STILL_MOTION_THRESH` | 0.08 | 低于此值的运动 = 站立静止 |
| `PHASE_PERTURBATION_THRESH` | 0.04 | 互动的相位方差 |
| `REACH_BURST_THRESH` | 0.15 | 伸手检测的相位突发 |
| `STILL_DEBOUNCE` | 10 帧 (0.5 秒) | 计数前的静止确认 |
| `ENGAGEMENT_COOLDOWN` | 60 帧 (3 秒) | 互动事件之间的冷却 |

#### 示例：计划图分析

```python
# 货架性能分析
shelf_stats = defaultdict(lambda: {"browse": 0, "consider": 0, "engage": 0, "reaches": 0})

if event_id == 440:  # SHELF_BROWSE
    shelf_stats[shelf_id]["browse"] += 1
elif event_id == 441:  # SHELF_CONSIDER
    shelf_stats[shelf_id]["consider"] += 1
elif event_id == 442:  # SHELF_ENGAGE
    shelf_stats[shelf_id]["engage"] += 1
    duration_s = value
    if duration_s > 60:
        analytics.flag_decision_difficulty(shelf_id)
elif event_id == 443:  # REACH_DETECTED
    shelf_stats[shelf_id]["reaches"] += 1

# 转化漏斗：浏览 -> 考虑 -> 互动
# 考虑到互动的低比率 = 货架放置或定价不佳
```

---

## 使用场景

### 零售商店布局优化

在关键位置部署 ESP32 节点：
- **入口**：客户流动模块统计客流量和高峰时段
- **收银通道**：队列长度模块监控等待时间，触发"打开收银台"警报
- **过道**：停留热力图识别高流量区域用于优质产品放置
- **端架/展示**：货架互动测量哪些展示将注意力转化为互动

```
                    入口
                  (CustomerFlow)
                       |
        +--------------+--------------+
        |              |              |
   过道 1         过道 2        过道 3
 (DwellHeatmap)  (DwellHeatmap) (DwellHeatmap)
        |              |              |
   [货架 A]       [货架 B]      [货架 C]
 (ShelfEngage)   (ShelfEngage)  (ShelfEngage)
        |              |              |
        +--------------+--------------+
                       |
                  收银区域
                 (QueueLength x3)
```

### 餐厅运营

部署每张餐桌的 ESP32 节点加上入口/出口节点：

- **入口**：客户流动跟踪客户到达
- **每张餐桌**：餐桌周转监控就座生命周期
- **接待台**：队列长度估算 walk-ins 的等待时间
- **厨房视角**：停留热力图识别服务员流量模式

关键指标：
- 每张餐桌的平均就座持续时间
- 每小时周转率（效率）
- 高峰 vs 非高峰利用率
- 等待时间 vs 聚会规模相关性

### 购物中心分析

多层、多区域部署：

- **购物中心入口** (4-8 节点)：客户流动用于总客流量 + 方向性
- **美食广场**：餐桌周转 + 每家餐厅的队列长度
- **锚店入口**：每家商店的客户流动
- **公共区域**：停留热力图用于座位区利用率
- ** kiosks/ pop-ups**：货架互动用于促销展示效果

### 活动场馆管理

- **入口**：客户流动用于进出计数，容量监控
- ** concession stands**：队列长度带员工调度警报
- **座位区**：停留热力图用于区域利用率
- **商品区**：货架互动用于产品兴趣

---

## 集成架构

```
ESP32 节点（每个区域）
    |
    v  UDP 事件 (端口 5005)
感应服务器 (wifi-densepose-sensing-server)
    |
    v  REST API + WebSocket
+---+---+---+---+
|   |   |   |   |
v   v   v   v   v
POS 仪表板  员工   分析
             传呼机  后端
```

### 事件数据包格式

每个事件是一个 `(event_type: i32, value: f32)` 对。每帧的多个事件被打包到单个 UDP 数据包中。感应服务器反序列化并通过以下方式暴露它们：

- `GET /api/v1/sensing/latest` -- 最新原始事件
- `GET /api/v1/sensing/events?type=400-403` -- 按事件类型过滤
- WebSocket `/ws/events` -- 实时流

### 隐私考虑

这些模块处理 WiFi CSI 数据（通道幅度和相位），而非视频或个人识别信息。没有 MAC 地址、设备标识符或个人跟踪数据离开 ESP32。所有输出都是聚合指标：计数、持续时间、区域标签。这使得 WiFi 感应适用于有严格隐私要求的司法管辖区（GDPR、CCPA），在这些地区基于摄像头的分析需要同意书或影响评估。