# 医疗与健康模块——WiFi-DensePose边缘智能

> 使用WiFi信号进行非接触式健康监测。无需可穿戴设备,无需摄像头——只需一个ESP32传感器读取从人体反射的WiFi信号,以检测呼吸问题、心律问题、行走困难和癫痫。

## 重要免责声明

这些模块是**研究工具,而非FDA批准的医疗设备**。它们应该补充——而非替代——专业医疗监测。从WiFi CSI衍生的生命体征本质上比临床仪器(ECG、脉搏血氧仪、呼吸带)更嘈杂。误报和漏报将会发生。在根据警报采取行动之前,始终使用临床级设备验证发现。

## 概述

| 模块 | 文件 | 功能 | 事件ID | 预算 |
|--------|------|-------------|-----------|--------|
| 睡眠呼吸暂停检测 | `med_sleep_apnea.rs` | 检测呼吸停止>10秒的呼吸暂停发作;跟踪AHI分数 | 100-102 | L (< 2 ms) |
| 心律失常 | `med_cardiac_arrhythmia.rs` | 检测心动过速、心动过缓、漏跳、HRV异常 | 110-113 | S (< 5 ms) |
| 呼吸窘迫 | `med_respiratory_distress.rs` | 检测呼吸急促、呼吸困难、潮式呼吸、复合窘迫分数 | 120-123 | H (< 10 ms) |
| 步态分析 | `med_gait_analysis.rs` | 提取步频、不对称、拖曳、慌张步态、跌倒风险分数 | 130-134 | H (< 10 ms) |
| 癫痫检测 | `med_seizure_detect.rs` | 使用相位区分(跌倒与震颤)检测强直-阵挛癫痫 | 140-143 | S (< 5 ms) |

所有模块:
- 编译为WASM的`no_std`(ESP32 WASM3运行时)
- 使用`const fn new()`进行零成本初始化
- 通过`&[(i32, f32)]`切片返回事件(无堆分配)
- 包括NaN和除零保护
- 实现冷却定时器以防止事件泛滥

---

## 模块

### 睡眠呼吸暂停检测(`med_sleep_apnea.rs`)

**功能**:从主机CSI管道监测呼吸频率,并检测当呼吸在连续10秒以上降至4 BPM以下时,表明呼吸暂停发作。它跟踪所有发作并计算呼吸暂停-低通气指数(AHI)——每小时监测睡眠时间的呼吸暂停事件数。AHI是睡眠呼吸暂停严重程度的临床标准指标。

**临床基础**:阻塞性和中枢性睡眠呼吸暂停定义为气流停止10秒或更长时间。该模块使用4 BPM的呼吸频率阈值(基本上是接近零的呼吸),并具有10秒发作延迟以确认停止是持续的。AHI严重程度分类:< 5正常,5-15轻度,15-30中度,> 30重度。

**工作原理**:
1. 每秒检查呼吸BPM是否低于4.0
2. 增加连续低呼吸计数器
3. 在连续10秒后,宣布呼吸暂停开始(回溯到呼吸首次下降时)
4. 当呼吸恢复到4 BPM以上时,记录发作及其持续时间
5. 每5分钟,计算AHI = (总发作数) / (监测小时数)
6. 仅在检测到存在时监测;如果受试者在呼吸暂停期间离开,则结束发作

#### API

| 项目 | 类型 | 描述 |
|------|------|-------------|
| `SleepApneaDetector` | struct | 主检测器状态 |
| `SleepApneaDetector::new()` | `const fn` | 创建零状态检测器 |
| `process_frame(breathing_bpm, presence, variance)` | method | 以~1 Hz处理一帧;返回事件切片 |
| `ahi()` | method | 当前AHI值 |
| `episode_count()` | method | 记录的呼吸暂停发作总数 |
| `monitoring_seconds()` | method | 存在激活的总秒数 |
| `in_apnea()` | method | 当前是否在呼吸暂停发作中 |
| `APNEA_BPM_THRESH` | const | 4.0 BPM -- 低于此值计为呼吸暂停 |
| `APNEA_ONSET_SECS` | const | 10秒 -- 宣布呼吸暂停的最短持续时间 |
| `AHI_REPORT_INTERVAL` | const | 300秒(5分钟) -- AHI重新计算的频率 |
| `MAX_EPISODES` | const | 256 -- 每次会话存储的最大发作数 |

#### 发出的事件

| 事件ID | 常量 | 值 | 临床意义 |
|----------|----------|-------|-----------------|
| 100 | `EVENT_APNEA_START` | 当前呼吸BPM | 呼吸已停止或降至4 BPM以下超过10秒 |
| 101 | `EVENT_APNEA_END` | 持续时间(秒) | 呼吸暂停发作后呼吸已恢复 |
| 102 | `EVENT_AHI_UPDATE` | AHI分数(事件/小时) | 周期性严重程度指标;>5 = 轻度,>15 = 中度,>30 = 重度 |

#### 状态机

```
                          存在丢失
    [监测中] -----> [未监测] (无事件,计数器暂停)
         |                    |
         | bpm < 4.0          | 恢复存在
         v                    v
    [低呼吸计数器]  [监测中]
         |
         | count >= 10s
         v
    [呼吸暂停中] ---------> [发作结束] (bpm >= 4.0 或 存在丢失)
         |                      |
         |                      v
         |               [记录发作,发出APNEA_END]
         |
         +-- 发出APNEA_START (一次)
```

#### 配置

| 参数 | 默认值 | 临床范围 | 描述 |
|-----------|---------|----------------|-------------|
| `APNEA_BPM_THRESH` | 4.0 | 0-6 BPM | 怀疑呼吸暂停的呼吸频率阈值 |
| `APNEA_ONSET_SECS` | 10 | 10-20 s | 宣布呼吸暂停前的低呼吸秒数 |
| `AHI_REPORT_INTERVAL` | 300 | 60-3600 s | AHI重新计算和发出的频率 |
| `MAX_EPISODES` | 256 | -- | 发作历史的固定缓冲区大小 |
| `PRESENCE_ACTIVE` | 1 | -- | 监测的最小存在标志值 |

#### 示例用法

```rust
use wifi_densepose_wasm_edge::med_sleep_apnea::*;

let mut detector = SleepApneaDetector::new();

// 正常呼吸 -- 无事件
let events = detector.process_frame(14.0, 1, 0.1);
assert!(events.is_empty());

// 模拟呼吸暂停:连续15秒输入低BPM
for _ in 0..15 {
    let events = detector.process_frame(1.0, 1, 0.1);
    for &(event_id, value) in events {
        match event_id {
            EVENT_APNEA_START => println!("检测到呼吸暂停! BPM: {}", value),
            _ => {}
        }
    }
}
assert!(detector.in_apnea());

// 恢复正常呼吸
let events = detector.process_frame(14.0, 1, 0.1);
for &(event_id, value) in events {
    match event_id {
        EVENT_APNEA_END => println!("呼吸暂停在{}秒后结束", value),
        _ => {}
    }
}

println!("发作数: {}", detector.episode_count());
println!("AHI: {:.1}", detector.ahi());
```

#### 教程:设置卧室睡眠监测

1. **ESP32放置**:将ESP32-S3安装在距离床1-2米的墙壁或天花板上,位于胸部高度。传感器应与睡眠区域有视线。避免放置在金属物体或产生CSI干扰的移动风扇附近。

2. **WiFi路由器**:确保稳定的WiFi AP在范围内。ESP32监测从人体反射的WiFi信号的CSI(信道状态信息)。AP应位于床的传感器对面,以获得最佳身体反射捕获。

3. **固件配置**:使用启用的第2层边缘处理(提供呼吸BPM)刷写ESP32固件。睡眠呼吸暂停WASM模块作为第3层算法在第2层生命体征输出之上运行。

4. **阈值调整**:默认4 BPM阈值是保守的(接近完全停止)。对于更敏感的检测器,降低到6-8 BPM,但预期浅呼吸会有更多误报。10秒发作延迟符合临床呼吸暂停定义。

5. **读取AHI结果**:AHI每5分钟发出一次。整夜(7-8小时)后,最终AHI值代表夜间严重程度。与临床阈值比较:< 5(正常),5-15(轻度),15-30(中度),> 30(重度)。

6. **限制**:基于WiFi的呼吸检测在受试者相对静止(睡眠)时效果最佳。辗转反侧可能导致呼吸检测的瞬时丢失,这可能掩盖或错误触发呼吸暂停事件。单夜研究应始终通过临床多导睡眠图确认。

---

### 心律失常检测(`med_cardiac_arrhythmia.rs`)
