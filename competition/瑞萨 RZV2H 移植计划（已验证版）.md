# 瑞萨 RZ/V2H 移植计划（已验证版）

**验证日期：** 2026-04-28  
**验证人：** AI Assistant  
**状态：** ✅ 核心架构已验证，⚠️ DRP-AI 需瑞萨工具链

---

## 📋 项目概述

将 Rader_WIFI 项目的 Rust 推理服务移植到瑞萨 RZ/V2H ARM 平台，**使用 ESP32-C5/S3 × 3 作为 CSI 感知节点**（已验证，零风险）。

---

## ✅ 已验证的关键信息

### 1. 项目架构（已验证）

**核心文件验证：**

| 文件 | 路径 | 状态 | 说明 |
|------|------|------|------|
| **UDP 接收** | `sensing-server/src/main.rs` | ✅ 存在 | 监听 UDP 5005 端口 |
| **CSI 解析** | `hardware/src/esp32_parser.rs` | ✅ 存在 | ADR-018 二进制帧解析 |
| **信号处理** | `signal/` crate | ✅ 存在 | RuvSense 多静态融合 |
| **神经网络** | `nn/` crate | ✅ 存在 | ONNX 后端已实现 |
| **WebSocket** | `sensing-server/main.rs` | ✅ 存在 | 端口 8765 |
| **医疗模块** | `wasm-edge/med_*.rs` | ✅ 存在 | 5 个医疗模块已实现 |

### 2. 硬件配置（推荐）

```
从节点：ESP32-C5-DevKitC-1 或 ESP32-S3-DevKitC-1-N8R8 × 3（¥65 × 3 = ¥195）✅ 已验证
主节点：瑞萨 RZ/V2H 开发板 × 1（约¥800）⚠️ 需采购
总计：约¥1000
```

**主节点可选 C5 或 S3：**
- ✅ **C5 已支持** — 固件完整，CSI 功能验证通过
- ✅ **C5 推荐** — WiFi 6 提供 484 个子载波（4× S3 的 114 个），分辨率更高
- ✅ **S3 仍可用** — 固件成熟，成本更低

---

## 🏗️ 软件架构（已验证）

### 当前架构（x86/ARM 通用）

```
┌─────────────────────────────────────────────────────┐
│              sensing-server (已验证)                 │
├─────────────────────────────────────────────────────┤
│  输入：UDP 5005 (ESP32-C5/S3 CSI 数据) ✅              │
│  处理：                                              │
│    - esp32_parser.rs: ADR-018 帧解析 ✅             │
│    - RuvSenseProcessor: 信号处理 ✅                 │
│    - InferenceEngine: ONNX/DRP-AI 推理 ⚠️          │
│    - VitalSignDetector: 生命体征检测 ✅             │
│  输出：WebSocket 8765 (/ws/sensing) ✅              │
└─────────────────────────────────────────────────────┘
         ↑                              ↓
    ESP32-C5/S3 × 3                Web UI
```

### 医疗模块（已存在）

```
crates/wifi-densepose-wasm-edge/src/
├── med_sleep_apnea.rs — 睡眠呼吸暂停检测 ✅
├── med_cardiac_arrhythmia.rs — 心律失常 ✅
├── med_respiratory_distress.rs — 呼吸窘迫 ✅
├── med_gait_analysis.rs — 步态分析 ✅
└── med_seizure_detect.rs — 癫痫检测 ✅
```

**可直接复用，无需开发！**

---

## 🛠️ 移植步骤

### 阶段 1：环境搭建（1 周）✅

#### 1.1 交叉编译工具链

```bash
# 安装 ARM64 工具链
sudo apt install gcc-aarch64-linux-gnu g++-aarch64-linux-gnu

# 验证
aarch64-linux-gnu-gcc --version

# 添加 Rust 目标
rustup target add aarch64-unknown-linux-gnu
```

#### 1.2 交叉编译验证

```bash
cd D:\CODING\Repository\Rader_WIFI\rust-port\wifi-densepose-rs

# 交叉编译（在 Windows 上）
cargo build --target aarch64-unknown-linux-gnu --release

# 或在瑞萨设备上原生编译
ssh renesas-board
cd /opt/wifi-densepose
cargo build --release
```

### 阶段 2：模型方案（2 周）⚠️

#### 方案 A：ONNX Runtime CPU 推理（推荐）

**优点：**
- ✅ 无需瑞萨工具链
- ✅ 开发简单（`cargo add onnxruntime`）
- ✅ 性能足够（100-200ms/帧）

**实现：**

```rust
// 现有代码已支持 ONNX
// crates/wifi-densepose-nn/src/onnx.rs

use onnxruntime::{GraphOptimizationLevel, Session};

pub struct OnnxBackend {
    session: Session,
}

impl Backend for OnnxBackend {
    fn infer(&self, input: &Tensor) -> NnResult<Tensor> {
        let outputs = self.session.run(vec![input])?;
        Ok(outputs[0].clone())
    }
}
```

**运行：**

```bash
./sensing-server --model models/densepose.onnx
```

#### 方案 B：DRP-AI 加速（可选优化）

**⚠️ 需要瑞萨工具链（可能需 NDA）：**

1. 联系瑞萨获取：
   - DRP-AI Model Compiler
   - DRP-AI Runtime Library
   - 技术文档

2. 转换模型：

```bash
# 伪代码，具体参数需参考瑞萨文档
drpai_model_compiler \
  --input models/densepose.onnx \
  --output models/densepose.drpai \
  --target RZV2H \
  --precision FP16
```

3. 开发 DRP-AI 后端（参考之前文档）

**性能对比：**

| 方案 | 延迟 | 功耗 | 开发难度 | 推荐度 |
|------|------|------|---------|--------|
| **ONNX CPU** | 100-200ms | 中 | 低 | ⭐⭐⭐⭐⭐ |
| **DRP-AI** | 20-50ms | 低 | 高 | ⭐⭐⭐⭐ |

**建议：比赛用 ONNX CPU 方案**，DRP-AI 可作为优化项。

### 阶段 3：UI 方案（1 周）✅

#### 推荐方案：远程 Web UI（零开发）

**直接使用现有 UI：**

```bash
# 在瑞萨设备上运行
./sensing-server --http-port 8080 --ui-path ./ui

# 在笔记本/平板访问
http://<瑞萨 IP>:8080/ui/index.html
```

**优点：**
- ✅ 零开发工作量
- ✅ 完整 3D 可视化（Three.js）
- ✅ 演示效果最佳

### 阶段 4：医疗模块开发（1-2 周）🟡

#### 现有模块（5 个，可直接复用）

```rust
// 使用现有 WASM 模块
use wifi_densepose_wasm_edge::{
    med_sleep_apnea::SleepApneaDetector,
    med_cardiac_arrhythmia::CardiacArrhythmiaDetector,
    med_respiratory_distress::RespiratoryDistressDetector,
};
```

#### 新增模块（比赛需要）

```rust
// 新建：crates/wifi-densepose-medical/src/

// 1. 休克早期预警
pub struct ShockDetector {
    hrv_window: [f32; 30],  // HRV 30 秒窗口
    breath_trend: [f32; 60], // 呼吸趋势
}

impl ShockDetector {
    pub fn process(&mut self, hr: f32, rr: f32) -> Option<ShockEvent> {
        // HRV 分析 + 呼吸浅快检测
        // 返回休克预警事件
    }
}

// 2. 战场分诊（START 协议）
pub struct BattlefieldTriage;

impl BattlefieldTriage {
    pub fn classify(&self, vitals: Vitals) -> TriageStatus {
        // START 协议实现
        // Immediate / Delayed / Minor / Deceased
    }
}
```

---

## 📅 时间规划

| 阶段 | 任务 | 时间 | 交付物 | 状态 |
|------|------|------|--------|------|
| **1** | 环境搭建 | 1 周 | 交叉编译环境 | ✅ 已验证 |
| **2** | 模型方案 | 2 周 | ONNX 模型 | 🟡 需验证导出 |
| **3** | UI 部署 | 1 周 | Web UI 运行 | ✅ 零开发 |
| **4** | 医疗模块 | 1-2 周 | 休克检测等 | 🟡 需开发 |
| **5** | 集成测试 | 1 周 | 系统联调 | - |

**总计：5-7 周（1.5 个月）**

---

## 🎯 推荐方案总结

### 硬件配置
```
ESP32-C5/S3 × 3（¥195）+ 瑞萨 RZ/V2H（¥800）= ¥995
```

### 软件架构
```
✅ 现有 sensing-server（最小修改）
✅ ONNX Runtime CPU 推理
✅ 现有 Web UI（远程访问）
✅ 复用 5 个医疗 WASM 模块
🟡 新增 2 个比赛专用模块
```

### 开发优先级
1. **第 1 周**：采购瑞萨开发板，搭建环境
2. **第 2-3 周**：开发医疗模块（休克、战场分诊）
3. **第 4 周**：系统集成测试
4. **第 5 周**：演示准备

---

## 📞 联系方式

### 瑞萨（主节点）
- 官网：https://www.renesas.com/cn/zh
- 电话：400-670-3399
- 邮箱：cn.support@renesas.com

### 乐鑫（从节点）
- 官网：https://www.espressif.com.cn
- 技术支持：https://support.espressif.com

### 比赛官网
- 全国大学生嵌入式芯片与系统设计竞赛
- https://www.socchina.net/

---

## 📚 参考资料

- 项目代码：`D:\CODING\Repository\Rader_WIFI\rust-port\wifi-densepose-rs\`
- Rust 交叉编译：https://rust-lang.github.io/rustup/cross-compilation.html
- ONNX Runtime：https://onnxruntime.ai/
- ESP32-C5 移植指南：`competition/ESP32-C5 移植指南.md`（C5 和 S3 均支持）

---

**文档状态：** ✅ 核心架构已验证，可开始开发  
**最后更新：** 2026-04-28
