# 边缘智能模块——WiFi-DensePose

> 60个WASM模块直接在ESP32传感器上运行。无需互联网,无云费用,即时响应。每个模块都是一个小文件(5-30 KB),读取WiFi信号数据并在10毫秒内本地做出决策。

## 快速开始

```bash
# 为ESP32构建所有模块
cd rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge
cargo build --target wasm32-unknown-unknown --release

# 运行所有632个测试
cargo test --features std

# 将模块上传到您的ESP32
python scripts/wasm_upload.py --port COM7 --module target/wasm32-unknown-unknown/release/module_name.wasm
```

## 模块类别

| | 类别 | 模块 | 测试 | 文档 |
|---|----------|---------|-------|---------------|
| | **核心** | 7 | 81 | [core.md](core.md) |
| | **医疗与健康** | 5 | 38 | [medical.md](medical.md) |
| | **安全与安防** | 6 | 42 | [security.md](security.md) |
| | **智能建筑** | 5 | 38 | [building.md](building.md) |
| | **零售与酒店** | 5 | 38 | [retail.md](retail.md) |
| | **工业** | 5 | 38 | [industrial.md](industrial.md) |
| | **特殊与研究** | 10 | ~60 | [exotic.md](exotic.md) |
| | **信号智能** | 6 | 54 | [signal-intelligence.md](signal-intelligence.md) |
| | **自适应学习** | 4 | 42 | [adaptive-learning.md](adaptive-learning.md) |
| | **空间与时间** | 6 | 56 | [spatial-temporal.md](spatial-temporal.md) |
| | **AI安全** | 2 | 20 | [ai-security.md](ai-security.md) |
| | **量子与自主** | 4 | 30 | [autonomous.md](autonomous.md) |
| | **总计** | **65** | **632** | |

## 工作原理

1. **WiFi信号在房间内的人和物体上反射**,创建独特的模式
2. **ESP32芯片读取这些模式**作为信道状态信息(CSI)——52个描述每个WiFi信道如何变化的数字
3. **WASM模块分析模式**以检测特定事物:有人跌倒、房间被占用、呼吸频率变化
4. **事件在本地发出**——无云往返,响应时间低于10毫秒

## 架构

```
WiFi路由器 ──── 无线电波 ────→ ESP32-S3传感器
                                      │
                                      ▼
                              ┌──────────────┐
                              │  第0-2层    │  C固件:相位解绕,
                              │  DSP引擎  │  统计,前K选择
                              └──────┬───────┘
                                      │ CSI帧(52个子载波)
                                      ▼
                              ┌──────────────┐
                              │   WASM3      │  微型解释器
                              │   运行时    │  (60 KB开销)
                              └──────┬───────┘
                                      │
                          ┌───────────┼───────────┐
                          ▼           ▼           ▼
                    ┌──────────┐ ┌──────────┐ ┌──────────┐
                    │ 模块A │ │ 模块B │ │ 模块C │
                    │ (5-30KB) │ │ (5-30KB) │ │ (5-30KB) │
                    └────┬─────┘ └────┬─────┘ └────┬─────┘
                         │           │           │
                         └───────────┼───────────┘
                                     ▼
                              事件 + 警报
                         (UDP到聚合器或本地)
```

## 主机API

每个模块通过12个函数与ESP32通信:

| 函数 | 返回 | 描述 |
|----------|---------|-------------|
| `csi_get_phase(i)` | `f32` | 子载波`i`的WiFi信号相位角 |
| `csi_get_amplitude(i)` | `f32` | 子载波`i`的信号强度 |
| `csi_get_variance(i)` | `f32` | 子载波`i`波动的程度 |
| `csi_get_bpm_breathing()` | `f32` | 呼吸频率(BPM) |
| `csi_get_bpm_heartrate()` | `f32` | 心率(BPM) |
| `csi_get_presence()` | `i32` | 有人在吗?(0/1) |
| `csi_get_motion_energy()` | `f32` | 整体运动水平 |
| `csi_get_n_persons()` | `i32` | 估计人数 |
| `csi_get_timestamp()` | `i32` | 当前时间戳(毫秒) |
| `csi_emit_event(id, val)` | — | 向主机发送检测结果 |
| `csi_log(ptr, len)` | — | 向串行控制台记录消息 |
| `csi_get_phase_history(buf, max)` | `i32` | 用于趋势分析的过去相位值 |

## 事件ID注册表

| 范围 | 类别 | 示例事件 |
|-------|----------|---------------|
| 0-99 | 核心 | 检测到手势、相干性分数、异常 |
| 100-199 | 医疗 | 呼吸暂停、心动过缓、心动过速、癫痫 |
| 200-299 | 安全 | 入侵、周边突破、徘徊、恐慌 |
| 300-399 | 智能建筑 | 区域占用、暖通空调、照明、电梯、会议 |
| 400-499 | 零售 | 队列长度、停留区域、客户流、周转率 |
| 500-599 | 工业 | 接近警告、受限空间、振动 |
| 600-699 | 特殊 | 睡眠阶段、情绪、手势语言、雨 |
| 700-729 | 信号智能 | 注意力、相干性门、压缩、恢复 |
| 730-759 | 自适应学习 | 学习的手势、吸引子、适应、EWC |
| 760-789 | 空间推理 | 影响、HNSW匹配、峰值跟踪 |
| 790-819 | 时间分析 | 模式、LTL违规、GOAP目标 |
| 820-849 | AI安全 | 重放攻击、注入、干扰、行为 |
| 850-879 | 量子启发 | 纠缠、退相干、假设 |
| 880-899 | 自主 | 推理、规则触发、网格重新配置 |

## 模块开发

### 添加新模块

1. 按照模式创建`src/your_module.rs`:
   ```rust
   #![cfg_attr(not(feature = "std"), no_std)]
   #[cfg(not(feature = "std"))]
   use libm::fabsf;

   pub struct YourModule { /* 仅固定大小字段 */ }

   impl YourModule {
       pub const fn new() -> Self { /* ... */ }
       pub fn process_frame(&mut self, /* 输入 */) -> &[(i32, f32)] { /* ... */ }
   }
   ```

2. 将`pub mod your_module;`添加到`lib.rs`
3. 将事件常量添加到`lib.rs`中的`event_types`
4. 使用`#[cfg(test)] mod tests { ... }`添加测试
5. 运行`cargo test --features std`

### 约束

- **无堆分配**:使用固定大小数组,而非`Vec`或`String`
- **无`std`**:使用`libm`进行数学函数
- **预算层级**: L(<2ms)、S(<5ms)、H(<10ms)每帧
- **二进制大小**:每个模块应为5-30 KB的WASM

## 参考文献

- [ADR-039](../adr/ADR-039-esp32-edge-intelligence.md) — 边缘处理层级
- [ADR-040](../adr/ADR-040-wasm-programmable-sensing.md) — WASM运行时设计
- [ADR-041](../adr/ADR-041-wasm-module-collection.md) — 完整模块规范
- [源代码](../../rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/)
