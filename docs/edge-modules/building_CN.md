# 智能建筑模块 -- WiFi-DensePose 边缘智能

> 利用您已有的 WiFi 信号让任何建筑变得更智能。了解哪些房间有人使用，自动控制 HVAC 和照明，统计电梯乘客数量，跟踪会议室使用情况，以及审计能源浪费 -- 所有这些都无需摄像头或徽章。

## 概述

| 模块 | 文件 | 功能 | 事件 ID | 帧预算 |
|------|------|------|---------|--------|
| HVAC 存在检测 | `bld_hvac_presence.rs` | 针对 HVAC 能源管理优化的存在检测 | 310-312 | ~0.5 us/帧 |
| 照明区域控制 | `bld_lighting_zones.rs` | 基于空间占用的每个区域照明控制（开/调暗/关） | 320-322 | ~1 us/帧 |
| 电梯乘客计数 | `bld_elevator_count.rs` | 电梯轿厢内乘客计数（1-12 人） | 330-333 | ~1.5 us/帧 |
| 会议室跟踪 | `bld_meeting_room.rs` | 会议生命周期跟踪与使用率指标 | 340-343 | ~0.3 us/帧 |
| 能源审计 | `bld_energy_audit.rs` | 24x7 小时占用直方图用于调度优化 | 350-352 | ~0.2 us/帧 |

所有模块均针对运行 WASM3（ADR-040 第 3 层）的 ESP32-S3 设计。它们从第 2 层 DSP 接收预处理的 CSI 信号，并通过 `csi_emit_event()` 发送结构化事件。

---

## 模块

### HVAC 存在控制 (`bld_hvac_presence.rs`)

**功能**：告诉您的 HVAC 系统房间是否有人，具有故意不对称的时序 -- 快速到达检测（10 秒）以便快速启动制冷/供暖，以及慢速离开超时（5 分钟）以避免有人短暂外出时过早关闭。还会分类占用者是久坐（办公、阅读）还是活动（行走、锻炼）。

**工作原理**：一个四状态机处理每帧的存在分数和运动能量：

```
Vacant --> ArrivalPending --> Occupied --> DeparturePending --> Vacant
           (10s 防抖)                 (5 分钟超时)
```

运动能量通过指数移动平均（alpha=0.1）进行平滑，并与 0.3 的阈值进行比较，以区分久坐和活动行为。

#### 状态机

| 状态 | 进入条件 | 退出条件 |
|------|----------|----------|
| `Vacant` | 未检测到存在 | 存在分数 > 0.5 |
| `ArrivalPending` | 检测到存在，防抖计数 | 200 连续帧存在 -> Occupied；任何不存在 -> Vacant |
| `Occupied` | 到达防抖完成 | 首帧不存在 -> DeparturePending |
| `DeparturePending` | 存在丢失 | 6000 帧不存在 -> Vacant；任何存在 -> Occupied |

#### 事件

| 事件 ID | 名称 | 值 | 触发时机 |
|---------|------|------|----------|
| 310 | `HVAC_OCCUPIED` | 1.0（有人）或 0.0（无人） | 每 20 帧 |
| 311 | `ACTIVITY_LEVEL` | 0.0-0.99（久坐 + EMA）或 1.0（活动） | 每 20 帧 |
| 312 | `DEPARTURE_COUNTDOWN` | 0.0-1.0（剩余超时比例） | DeparturePending 期间每 20 帧 |

#### API

```rust
use wifi_densepose_wasm_edge::bld_hvac_presence::HvacPresenceDetector;

let mut det = HvacPresenceDetector::new();

// 每帧处理
let events = det.process_frame(presence_score, motion_energy);
// events: &[(event_type: i32, value: f32)]

// 查询
det.state()       // -> HvacState (Vacant|ArrivalPending|Occupied|DeparturePending)
det.is_occupied()  // -> bool (true during Occupied or DeparturePending)
det.activity()     // -> ActivityLevel (Sedentary|Active)
det.motion_ema()   // -> f32 (smoothed motion energy)
```

#### 配置常量

| 常量 | 值 | 描述 |
|------|------|------|
| `ARRIVAL_DEBOUNCE` | 200 帧 (10s) | 确认占用前的连续存在帧 |
| `DEPARTURE_TIMEOUT` | 6000 帧 (5 min) | 宣布无人前的连续不存在帧 |
| `ACTIVITY_THRESHOLD` | 0.3 | 运动 EMA 高于此值 = 活动 |
| `MOTION_ALPHA` | 0.1 | 运动能量的 EMA 平滑因子 |
| `PRESENCE_THRESHOLD` | 0.5 | 考虑有人的最低存在分数 |
| `EMIT_INTERVAL` | 20 帧 (1s) | 事件发射间隔 |

#### 示例：BACnet 集成

```python
# Python 主机从 ESP32 UDP 数据包读取事件
if event_id == 310:  # HVAC_OCCUPIED
    bacnet_write(device_id, "Occupancy", int(value))  # 1=有人, 0=无人
elif event_id == 311:  # ACTIVITY_LEVEL
    if value >= 1.0:
        bacnet_write(device_id, "CoolingSetpoint", 72)  # 活动：更凉爽
    else:
        bacnet_write(device_id, "CoolingSetpoint", 76)  # 久坐：更温暖
elif event_id == 312:  # DEPARTURE_COUNTDOWN
    if value < 0.2:  # 剩余不到 1 分钟
        bacnet_write(device_id, "FanMode", "low")  # 开始降低
```

---

### 照明区域控制 (`bld_lighting_zones.rs`)

**功能**：管理最多 4 个独立照明区域，自动在每个区域之间转换：开（有人且活动）、调暗（有人但久坐超过 10 分钟）和关（无人超过 30 秒）。使用每个区域的方差分析来确定房间哪些区域有人。

**工作原理**：子载波被分为组（每个区域一组）。计算每组的幅度方差并与校准基线进行比较。方差偏差超过阈值表示该区域有人。校准阶段（200 帧 = 10 秒）在空房间中建立基线。

```
Off --> On (检测到占用 + 活动)
On --> Dim (有人但久坐 10 分钟)
On --> Dim (检测到无人，宽限期)
Dim --> Off (无人 30 秒)
Dim --> On (活动恢复)
```

#### 事件

| 事件 ID | 名称 | 值 | 触发时机 |
|---------|------|------|----------|
| 320 | `LIGHT_ON` | zone_id (0-3) | 开启状态转换 |
| 321 | `LIGHT_DIM` | zone_id (0-3) | 调暗状态转换 |
| 322 | `LIGHT_OFF` | zone_id (0-3) | 关闭状态转换 |

定期摘要在值字段中编码 `zone_id + confidence`（整数部分 = 区域，小数部分 = 占用分数）。

#### API

```rust
use wifi_densepose_wasm_edge::bld_lighting_zones::LightingZoneController;

let mut ctrl = LightingZoneController::new();

// 每帧：传递子载波幅度和整体运动能量
let events = ctrl.process_frame(&amplitudes, motion_energy);

// 查询
ctrl.zone_state(zone_id) // -> LightState (Off|Dim|On)
ctrl.n_zones()           // -> usize (活动区域数量, 1-4)
ctrl.is_calibrated()     // -> bool
```

#### 配置常量

| 常量 | 值 | 描述 |
|------|------|------|
| `MAX_ZONES` | 4 | 最大照明区域 |
| `OCCUPANCY_THRESHOLD` | 0.03 | 占用的方差偏差比率 |
| `ACTIVE_THRESHOLD` | 0.25 | 活动分类的运动能量 |
| `DIM_TIMEOUT` | 12000 帧 (10 min) | 久坐后调暗的帧数 |
| `OFF_TIMEOUT` | 600 帧 (30s) | 无人后关闭的帧数 |
| `BASELINE_FRAMES` | 200 帧 (10s) | 校准持续时间 |

#### 示例：DALI/KNX 照明

```python
# 将区域事件映射到 DALI 地址
DALI_ADDR = {0: 1, 1: 2, 2: 3, 3: 4}

if event_id == 320:  # LIGHT_ON
    zone = int(value)
    dali_send(DALI_ADDR[zone], level=254)  # 全亮度
elif event_id == 321:  # LIGHT_DIM
    zone = int(value)
    dali_send(DALI_ADDR[zone], level=80)   # 30% 亮度
elif event_id == 322:  # LIGHT_OFF
    zone = int(value)
    dali_send(DALI_ADDR[zone], level=0)    # 关闭
```

---

### 电梯乘客计数 (`bld_elevator_count.rs`)

**功能**：计算电梯轿厢内的人数（0-12），检测门开关事件，并在计数超过可配置阈值时发出过载警告。利用电梯的有限空间多路径特性，将幅度方差与人数相关联。

**工作原理**：在像电梯这样的小型反射金属盒中，每增加一个人都会增加显著的多路径散射。该模块在空轿厢上进行校准，然后将当前方差与基线方差的比率映射到人数。帧间幅度增量检测突然的几何变化（门开关）。计数估计融合了模块自己的基于方差的估计（40% 权重）和主机的人数提示（60% 权重）（如果可用）。

#### 事件

| 事件 ID | 名称 | 值 | 触发时机 |
|---------|------|------|----------|
| 330 | `ELEVATOR_COUNT` | 人数 (0-12) | 每 10 帧 |
| 331 | `DOOR_OPEN` | 开门时的当前计数 | 检测到门打开时 |
| 332 | `DOOR_CLOSE` | 关门时的当前计数 | 检测到门关闭时 |
| 333 | `OVERLOAD_WARNING` | 当前计数 | 当计数 >= 过载阈值时 |

#### API

```rust
use wifi_densepose_wasm_edge::bld_elevator_count::ElevatorCounter;

let mut ec = ElevatorCounter::new();

// 每帧：幅度、相位、运动能量、主机人数提示
let events = ec.process_frame(&amplitudes, &phases, motion_energy, host_n_persons);

// 查询
ec.occupant_count()    // -> u8 (0-12)
ec.door_state()        // -> DoorState (Open|Closed)
ec.is_calibrated()     // -> bool

// 配置
ec.set_overload_threshold(8); // 设置自定义过载限制
```

#### 配置常量

| 常量 | 值 | 描述 |
|------|------|------|
| `MAX_OCCUPANTS` | 12 | 最大跟踪乘客数 |
| `DEFAULT_OVERLOAD` | 10 | 默认过载警告阈值 |
| `DOOR_VARIANCE_RATIO` | 4.0 | 门检测的增量幅度 |
| `DOOR_DEBOUNCE` | 3 帧 | 门事件的防抖 |
| `DOOR_COOLDOWN` | 40 帧 (2s) | 门事件后的冷却时间 |
| `BASELINE_FRAMES` | 200 帧 (10s) | 空轿厢校准 |

---

### 会议室跟踪器 (`bld_meeting_room.rs`)

**功能**：跟踪会议室使用的完整生命周期 -- 从有人进入，到确认真正的多人会议，再到检测会议结束和房间再次可用。区分实际会议（2+ 人超过 3 秒）和单人短暂使用房间的情况。跟踪峰值人数并计算房间使用率。

**工作原理**：一个四状态机处理存在和人数：

```
Empty --> PreMeeting --> Active --> PostMeeting --> Empty
          (有人进入)        (确认 2+ 人)       (所有人离开,
                           )                 2 分钟冷却)
```

PreMeeting 状态有 3 分钟超时：如果只有一个人留下，房间不会升级为"Active"（不被视为会议）。

#### 事件

| 事件 ID | 名称 | 值 | 触发时机 |
|---------|------|------|----------|
| 340 | `MEETING_START` | 当前人数 | 转换到 Active 时 |
| 341 | `MEETING_END` | 持续时间（分钟） | 转换到 PostMeeting 时 |
| 342 | `PEAK_HEADCOUNT` | 峰值人数 | 会议结束时 + Active 期间定期 |
| 343 | `ROOM_AVAILABLE` | 1.0 | 从 PostMeeting 转换到 Empty 时 |

#### API

```rust
use wifi_densepose_wasm_edge::bld_meeting_room::MeetingRoomTracker;

let mut mt = MeetingRoomTracker::new();

// 每帧：存在 (0/1), 人数, 运动能量
let events = mt.process_frame(presence, n_persons, motion_energy);

// 查询
mt.state()            // -> MeetingState (Empty|PreMeeting|Active|PostMeeting)
mt.peak_headcount()   // -> u8
mt.meeting_count()    // -> u32 (重置后总会议数)
mt.utilization_rate() // -> f32 (会议时间比例, 0.0-1.0)
```

#### 配置常量

| 常量 | 值 | 描述 |
|------|------|------|
| `MEETING_MIN_PERSONS` | 2 | "会议"的最少人数 |
| `PRE_MEETING_TIMEOUT` | 3600 帧 (3 min) | 等待会议形成的最长时间 |
| `POST_MEETING_TIMEOUT` | 2400 帧 (2 min) | 标记房间可用前的冷却时间 |
| `MEETING_MIN_FRAMES` | 6000 帧 (5 min) | 参考最低会议持续时间 |

#### 示例：日历集成

```python
# 同步会议室状态与日历系统
if event_id == 340:  # MEETING_START
    calendar_api.mark_room_in_use(room_id, headcount=int(value))
elif event_id == 341:  # MEETING_END
    duration_min = value
    calendar_api.log_actual_usage(room_id, duration_min)
elif event_id == 343:  # ROOM_AVAILABLE
    calendar_api.mark_room_available(room_id)
    display_screen.show("Room Available")
```

---

### 能源审计 (`bld_energy_audit.rs`)

**功能**：构建 7 天、24 小时占用直方图（168 小时 bins）以识别能源浪费模式。找出哪些小时始终无人（HVAC/照明关闭的候选时间），检测非工作时间占用异常（安全/安全隐患），并报告整体建筑利用率。

**工作原理**：每帧递增相应小时 bin 的计数器。该模块维护自己的模拟时钟（小时/天），通过计数帧来前进（72,000 帧 = 20 Hz 下的 1 小时）。主机可以通过 `set_time()` 设置实时时间。非工作时间定义为 22:00-06:00（正确跨越午夜）。非工作时间持续存在（30+ 秒）会触发警报。

#### 事件

| 事件 ID | 名称 | 值 | 触发时机 |
|---------|------|------|----------|
| 350 | `SCHEDULE_SUMMARY` | 当前小时的占用率 (0.0-1.0) | 每 1200 帧 (1 min) |
| 351 | `AFTER_HOURS_ALERT` | 当前小时 (22-5) | 非工作时间存在 600 帧 (30s) 后 |
| 352 | `UTILIZATION_RATE` | 整体利用率 (0.0-1.0) | 每 1200 帧 (1 min) |

#### API

```rust
use wifi_densepose_wasm_edge::bld_energy_audit::EnergyAuditor;

let mut ea = EnergyAuditor::new();

// 从主机设置实时时间
ea.set_time(0, 8); // 周一 8 AM (天 0-6, 小时 0-23)

// 每帧：存在 (0/1), 人数
let events = ea.process_frame(presence, n_persons);

// 查询
ea.utilization_rate()          // -> f32 (整体)
ea.hourly_rate(day, hour)      // -> f32 (特定时段的占用率)
ea.hourly_headcount(day, hour) // -> f32 (平均人数)
ea.unoccupied_hours(day)       // -> u8 (占用率低于 10% 的小时数)
ea.current_time()              // -> (day, hour)
```

#### 配置常量

| 常量 | 值 | 描述 |
|------|------|------|
| `FRAMES_PER_HOUR` | 72000 | 20 Hz 下一小时的帧数 |
| `SUMMARY_INTERVAL` | 1200 帧 (1 min) | 发出摘要的频率 |
| `AFTER_HOURS_START` | 22 (10 PM) | 非工作时间窗口开始 |
| `AFTER_HOURS_END` | 6 (6 AM) | 非工作时间窗口结束 |
| `USED_THRESHOLD` | 0.1 | 考虑一小时"使用"的最低占用率 |
| `AFTER_HOURS_ALERT_FRAMES` | 600 帧 (30s) | 持续存在后发出警报 |

#### 示例：能源优化报告

```python
# 生成每周能源优化报告
for day in range(7):
    unused = auditor.unoccupied_hours(day)
    print(f"{DAY_NAMES[day]}: {unused} hours could have HVAC off")

    for hour in range(24):
        rate = auditor.hourly_rate(day, hour)
        if rate < 0.1:
            print(f"  {hour:02d}:00 - unused ({rate:.0%} occupancy)")
```

---

## 集成指南

### 连接到 BACnet / HVAC 系统

所有五个建筑模块通过标准 `csi_emit_event()` 接口发出事件。典型的集成路径：

1. **ESP32 固件** 从 WASM 模块接收事件
2. **UDP 数据包** 将事件传输到聚合服务器（端口 5005）
3. **传感服务器** (`wifi-densepose-sensing-server`) 通过 REST API 公开事件
4. **BMS 集成脚本** 轮询 API 并写入 BACnet/Modbus 对象

关键 BACnet 对象映射：

| 模块 | BACnet 对象类型 | 属性 |
|------|----------------|------|
| HVAC 存在检测 | Binary Value | Occupancy (310: 1=有人) |
| HVAC 存在检测 | Analog Value | Activity Level (311: 0-1) |
| 照明区域控制 | Multi-State Value | Zone State (320-322: Off/Dim/On) |
| 电梯乘客计数 | Analog Value | Occupant Count (330: 0-12) |
| 会议室跟踪 | Binary Value | Room In Use (340/343) |
| 能源审计 | Analog Value | Utilization Rate (352: 0-1.0) |

### 照明控制集成（DALI、KNX）

`bld_lighting_zones` 模块发出区域级别的开/调暗/关转换。将每个区域映射到 DALI 地址组或 KNX 组地址：

- 事件 320 (LIGHT_ON) -> DALI 命令 `DAPC(254)` 或 KNX `DPT_Switch ON`
- 事件 321 (LIGHT_DIM) -> DALI 命令 `DAPC(80)` 或 KNX `DPT_Scaling 30%`
- 事件 322 (LIGHT_OFF) -> DALI 命令 `DAPC(0)` 或 KNX `DPT_Switch OFF`

### BMS（建筑管理系统）集成

对于结合所有五个模块的完整 BMS 集成：

```
ESP32 节点（每个房间/区域）
    |
    v  UDP 事件
聚合服务器
    |
    v  REST API / WebSocket
BMS 网关脚本
    |
    +-- HVAC 控制器 (BACnet/Modbus)
    +-- 照明控制器 (DALI/KNX)
    +-- 电梯显示面板
    +-- 会议室预订系统
    +-- 能源仪表板
```

### 部署考虑

- **校准**：照明和电梯模块需要在空房间/轿厢中进行 10 秒校准。在已知无人期间安排校准。
- **时钟同步**：能源审计模块需要在启动时调用 `set_time()`。在聚合器上使用 NTP 或通过主机 API 传递时间戳。
- **多个 ESP32**：对于开放式办公室，每个区域部署一个 ESP32。每个 ESP32 运行自己的 HVAC 存在检测和照明区域实例。聚合器合并区域级数据。
- **事件率**：所有模块将事件限制为最多每秒一次发射（EMIT_INTERVAL = 20 帧）。每个模块的总带宽低于 100 字节/秒。