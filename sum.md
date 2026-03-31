# RuView 项目全景解析

## 一、项目定位与核心原理

### 1.1 项目定位
**Rader_WIFI（蟺 RuView）** 是一个**边缘 AI 感知系统**，通过 WiFi 信道状态信息（CSI）实现"透视感知"——无需摄像头、无需可穿戴设备、无需互联网连接，即可实时检测人体姿态、呼吸频率、心率、 Presence 甚至通过墙壁感知生命体征。

**核心价值主张**：
- **隐私优先**：仅使用 WiFi 信号，无视频、无图像存储
- **边缘智能**：可在 $8 的 ESP32 上独立运行
- **自学习能力**：从原始 WiFi 数据中自举学习，无需人工标注
- **跨环境泛化**：训练一次，部署到任何房间

### 1.2 物理原理（底层机制）

```
WiFi 路由器 → 射频波穿透空间 → 遇到人体散射 → ESP32 Mesh 收集 CSI
     ↓                                                          ↓
  环境背景场                                                 多径散射模式
     ↓                                                          ↓
  人体移动/呼吸 → 改变电磁场分布 → 接收端相位/幅度变化 → 信号特征提取
```

**关键物理现象**：
- **SpotFi 原理**：使用两个天线的共轭乘积消除 CFO/SFO/PDD（载波/采样频率/路径时延偏移）
- **Fresnel 区域模型**：椭球形区域反射/衍射模型，用于估算呼吸导致的边界交叉
- **Body Velocity Profile (BVP)**：多普勒导出的速度-时间 2D 矩阵
- **相干门控（Coherence Gate）**：识别并剔除无效测量，确保系统数天稳定

**CSI 能力**（Channel State Information）：
- 每个子载波的幅度 + 相位（256+ 子载波）
- 多输入多输出（MIMO）通道矩阵
- 传统 WiFi 仅提供 RSSI（信号强度），无法用于精细感知

### 1.3 技术栈对比

| 组件 | 实现方式 | 优势 |
|------|---------|------|
| **Python v1** | v1/ 目录 | 研发验证、快速原型 |
| **Rust 生产版** | rust-port/wifi-densepose-rs/ | 54,000 FPS 性能、内存安全、WASM 边缘部署 |
| **ESP32 固件** | firmware/esp32-csi-node/ | 无云运行、实时检测（<10ms 延迟） |

---

## 二、系统架构（双层设计）

### 2.1 整体架构图

```
┌─────────────────────────────────────────────────────────────────────┐
│                           宿主层 (Host Layer)                          │
│  ┌─────────────┐  ┌──────────────┐  ┌─────────────┐  ┌───────────┐ │
│  │   WiFi Scanner │  │   CSI Streamer │  │   Aggregator │  │   Server  │ │
│  └───────┬───────┘  └───────┬───────┘  └───────┬───────┘  └─────┬─────┘ │
└──────────┼──────────────────┼──────────────────┼───────────────┼──────┘
           │                  │                  │               │
           ▼                  ▼                  ▼               ▼
┌─────────────────────────────────────────────────────────────────────┐
│                        边缘层 (Edge Layer)                            │
│  ┌────────────────────────────────────────────────────────────────┐ │
│  │ ESP32 Sensor Mesh (4-6 nodes, ~$54 total)                       │ │
│  │  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌───────────┐ │ │
│  │  │  CSI Node A  │ │  CSI Node B  │ │  CSI Node C  │ │  ...     │ │ │
│  │  │  - CSI 捕获  │ │  - CSI 捕获  │ │  - CSI 捕获  │ │  ...     │ │ │
│  │  │  - Edge DSP  │ │  - Edge DSP  │ │  - Edge DSP  │ │  ...     │ │ │
│  │  │  - WASM 执行 │ │  - WASM 执行 │ │  - WASM 执行 │ │  ...     │ │ │
│  │  └──────┬───────┘ └──────┬───────┘ └──────┬───────┘ └─────┬─────┘ │ │
│  └─────────┼────────────────┼────────────────┼───────────────┼───────┘
└────────────┼────────────────┼────────────────┼───────────────┼──────
             │                  │                  │               │
             ▼                  ▼                  ▼               ▼
┌─────────────────────────────────────────────────────────────────────┐
│                         物理层 (Physical Layer)                        │
│  WiFi 2.4/5 GHz 射频波 +人体运动 → CSI 矩阵 (幅度+相位)              │
└─────────────────────────────────────────────────────────────────────┘
```

### 2.2 技术栈分层

#### A. **Rust 生产栈**（核心性能层）
| Crate | 职责 | 关键算法/技术 |
|-------|------|---------------|
| `wifi-densepose-core` | 核心类型定义 | `CsiFrame`, `PoseEstimate`, `Keypoint` |
| `wifi-densepose-signal` | 信号处理 | SpotFi 共轭乘积、Hampel 滤波、Fresnel 模型、BVP |
| `wifi-densepose-nn` | 神经网络推理 | ONNX、PyTorch、Candle 后端 |
| `wifi-densepose-hardware` | 硬件接口 | ESP32 TDM 协议、QUIC 传输、CSI 帧解析 |
| `wifi-densepose-wasm-edge` | WASM 边缘模块 | **65 个已实现的 WASM 模块** |
| `wifi-densepose-ruvector` | RuVector 核心 | Attention、GNN 图算法、MinCut partitioning |
| `wifi-densepose-mat` | 灾难检测 | WiFi-Mat 模块、START 分类、定位 |
| `wifi-densepose-vitals` | 生命体征 | 呼吸率（6-30 BPM）、心率（40-120 BPM） |
| `wifi-densepose-desktop` | 桌面应用 | Tauri v2 + WASM 部署 |

#### B. **ESP32 固件栈**（边缘运行层）
| 组件 | 功能 |
|------|------|
| `csi_collector.c` | CSI 采集（802.11n/ac） |
| `edge_processing.c` | 边缘处理（DSP pipeline） |
| `stream_sender.c` | UDP 流发送（AHR-018 格式） |
| `wasm_runtime.c` | WASM3 运行时（无栈执行） |
| `ota_update.c` | OTA 更新（HTTP 服务器） |
| `display_task.c` | AMOLED 显示（本地反馈） |
| `nvs_config.c` | NVS 配置（持久化） |

#### C. **Python v1 栈**（研发验证层）
| 模块 | 功能 |
|------|------|
| `v1/src/api/` | FastAPI REST 服务 |
| `v1/src/core/csi_processor.py` | CSI 处理器 |
| `v1/src/core/phase_sanitizer.py` | 相位清洗（无折、离群值、平滑、低通） |
| `v1/src/sensing/` | 感知底层（RSSI 收集、特征提取、分类器） |
| `v1/src/backend.py` | 感知后端协议 |

---

## 三、核心模块关系与数据流

### 3.1 Signal-Line Protocol (6 阶段流水线)

这是项目的核心算法设计（ADR-033）：

```
Stage I: CSI Gestalt Classification
         ↓ (Poincaré 双曲嵌入)
Stage II: CSI Sensory Feature Extraction
         ↓ (多头注意力向量)
Stage III: AP Mesh Spatial Topology
         ↓ (GNN 图拓扑编码)
Stage IV: Coherence Gating (AOL 检测)
         ↓ (SNN 时间编码)
Stage V: Pose Interrogation
         ↓ (可微搜索 + 注意力)
Stage VI: Multi-Person Partitioning
         ↓ (MinCut 分割 + 质心)
Cross-Session: Multi-Room Convergence
         ↓ (跨房间身份匹配)
```

**具体模块映射**：

| CRV 阶段 | RuView 模块 | 文件位置 |
|---------|-------------|----------|
| Stage I | `CsiGestaltClassifier` | `ruvsense/multiband.rs` |
| Stage II | `CsiSensoryEncoder` | `ruvsense/phase_align.rs` |
| Stage III | `MeshTopologyEncoder` | `ruvsense/multistatic.rs` |
| Stage IV | `CoherenceAolDetector` | `ruvsense/coherence_gate.rs` |
| Stage V | `PoseInterrogator` | `ruvsense/field_model.rs` |
| Stage VI | `PersonPartitioner` | 训练流水线（ruvector-mincut） |
| Cross-Session | `MultiViewerConvergence` | `ruvsense/cross_room.rs` |

### 3.2 多视角融合架构（ADR-031）

```
        Room A AP          Room B AP          Room C AP
           │                    │                    │
           ▼                    ▼                    ▼
      [Node A1]            [Node B1]            [Node C1]
      [Node A2]  ← quic →  [Node B2]  ← quic →  [Node C2]
           │                    │                    │
           ▼                    ▼                    ▼
    多视角嵌入1          多视角嵌入2          多视角嵌入3
           \                    │                    /
            \                   │                   /
             \                  │                  /
              ▼                 ▼                 ▼
        Cross-Viewpoint Fusion Engine
                      │
                      ▼
              3D 人体姿态 + 生命体征
```

**关键技术**：
- **Quic Mesh Security**：端到端加密、防重放、防篡改
- **多视角一致性**：不同节点视角互补，消除盲区
- **最小化交叉验证**：通过跨视角 agreement 确定唯一 person ID

### 3.3 数据流处理（CSI → Pose）

```
Raw CSI Frame (from ESP32)
    │
    ├─→ Conjugate Multiplication (SpotFi) → CsiRatio (clean phase)
    │
    ├─→ Hampel Filter → Outlier-Free Signal
    │
    ├─→ Phase Sanitizer
    │    ├─→ Unwrap (remove 2π jumps)
    │    ├─→ Outlier removal (Z-score)
    │    ├─→ Smoothing (moving average)
    │    └─→ Low-pass filter (Butterworth)
    │
    ├─→ Feature Extraction
    │    ├─→ STFT Spectrogram (2D TF)
    │    ├─→ Subcarrier Selection (variance ranking)
    │    └─→ Body Velocity Profile (Doppler)
    │
    ├─→ Motion Detection
    │    ├─→ Fresnel Zone Model (breathing estimate)
    │    └─→ Motion Score (composite variance/correlation)
    │
    ├─→ RuVector AI Backbone
    │    ├─→ Graph Attention Network
    │    ├─→ MinCut Person Assignment
    │    └─→ Persistent Field Model (room fingerprint)
    │
    ├─→ CRV Signal Line Protocol (6-stage)
    │    ├─→ Stage I: Gestalt Classification (Poincaré)
    │    ├─→ Stage II: Sensory Vectors (Attention)
    │    ├─→ Stage III: GNN Topology
    │    ├─→ Stage IV: Coherence Gate
    │    ├─→ Stage V: Interrogation
    │    └─→ Stage VI: MinCut Partitioning
    │
    └─→ Output
         ├─→ 17 Body Keypoints (COCO format)
         ├─→ Breathing Rate (FFT 0.1-0.5 Hz)
         ├─→ Heart Rate (FFT 0.8-2.0 Hz)
         ├─→ Room Fingerprint (drift tracking)
         └─→ Fall/Vital Alert (edge modules)
```

---

## 四、WASM Edge Modules（边缘可编程感知）

### 4.1 架构设计（ADR-040）

```
┌─────────────────────────────────────────────────────────────────┐
│                    ESP32-S3 (WASM3 Runtime)                       │
│  ┌───────────────────────────────────────────────────────────┐   │
│  │  Runtime Environment (wasm3_runtime.c)                    │   │
│  │  - WASM3 interpreter (no-stack)                           │   │
│  │  - 12-function host API (csi_read, detect_human, etc.)   │   │
│  │  - 5-30 KB module size (flash-friendly)                  │   │
│  └──────────────┬────────────────────────────────────────────┘   │
                 │
                 ▼
┌─────────────────────────────────────────────────────────────────┐
│                    65 Implemented Modules                        │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────┐    │
│  │  Medical    │ │  Security   │ │  Building   │ │ Retail  │    │
│  │  - Sleep    │ │  - Intrusion│ │  - Occupancy│ │ Queue   │    │
│  │  - Arrhythmia│ │  - Perimeter│ │  - HVAC     │ │ Dwell   │    │
│  │  - Gait     │ │  - Loitering│ │  - Elevator │ │ Flow    │    │
│  └─────────────┘ └─────────────┘ └─────────────┘ └─────────┘    │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────┐    │
│  │ Industrial  │ │  Exotic     │ │ Signal Intell.│ │ Adaptive│    │
│  │ - Forklift  │ │  - Sleep    │ │ - CSI Sharp.│ │ Learning  │    │
│  │ - Vibration │ │  - Emotion  │ │ - Noise Fill│ │ Temporal  │    │
│  └─────────────┘ └─────────────┘ └─────────────┘ └─────────┘    │
└─────────────────────────────────────────────────────────────────┘
```

**模块特征**：
- **no_std Rust**：嵌入式友好，无标准库依赖
- **12-function Host API**：`csi_read`, `detect_human`, `get_vitals`, `send_alert`, `log`, `random`, `sleep_ms`, `clock`, `flash_read`, `flash_write`, `wifi_scan`, `ota_signal`
- **609 个测试通过**：每个模块独立单元测试

### 4.2 典型模块实现（Intrusion Detection）

```rust
// rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/intrusion.rs

#[no_mangle]
pub extern "C" fn on_csi_frame(frame_ptr: *const u8, len: usize) -> i32 {
    // 1. 读取 CSI 帧（host API）
    let csi_data = host::csi_read(frame_ptr, len);
    
    // 2. 提取特征（幅度方差、相位变化率）
    let amplitude_var = variance(csi_data.amplitude());
    let phase_rate = phase_derivative(csi_data.phase());
    
    // 3. 分类（逻辑回归，15 特征）
    let score = logistic_regression(&[
        amplitude_var,
        phase_rate,
        // ... 13 more features ...
    ]);
    
    // 4. 判定（阈值 0.7）
    if score > 0.7 {
        host::send_alert(AlertType::Intrusion, score);
        host::log(&format!("Intrusion detected: {:.2}", score));
    }
    
    0  // success
}
```

---

## 五、关键技术亮点

### 5.1 自学习嵌入模型（ADR-024）

** Contrastive CSI Embedding Model **

```
Raw WiFi Data
    │
    ├─→ Multi-Band Fusion (3 channels × 56 subcarriers = 168 virtual)
    ├─→ Phase Sanitization (unwrap + outlier + smooth)
    ├─→ Subcarrier Selection (variance ranking)
    │
    └─→ ruvector-spectral: Spectral Embedding
         │
         ├─→ Node2Vec (graph node embeddings)
         ├─→ Spectral Positional (Laplacian eigenvectors)
         └─→ Temporal (dynamic embedding evolution)
              │
              ▼
    Contrastive Loss (InfoNCE)
         │
         ▼
    Learned Embedding Space (clustering-friendly)
```

**关键创新**：
- **无监督自举**：从原始 WiFi 数据中学习，无需标签
- **对比学习