# π RuView — 基于WiFi CSI感知与端侧LLM的方舱生命体征感知与监护系统

> 第九届全国大学生嵌入式芯片与系统设计竞赛 · 瑞萨赛道
> 硬件：瑞萨 RZ/V2H + 3× ESP32-C5-DevKitC-1-N8R8

---

## 快速开始

### 硬件连接

```
                     TP-Link 千兆路由器 (192.168.1.0/24, SSID: RuView-Triage)
                              │
          ┌───────────────────┼───────────────────┐
          │                   │                   │
    ┌─────▼─────┐      ┌──────▼──────┐      ┌─────▼─────┐
    │ ESP32-C5  │      │ 瑞萨 RZ/V2H │      │ ESP32-C5  │
    │  节点 #2  │      │  (主控+AI)  │      │  节点 #3  │
    │ .1.11     │      │  192.168.1.1│      │ .1.12     │
    └───────────┘      │             │      └───────────┘
                       │  7" HDMI 触屏│
    ┌─────▼─────┐      └─────────────┘
    │ ESP32-C5  │
    │  节点 #1  │
    │ .1.10     │
    └───────────┘
```

### 一键启动

```bash
# 1. 烧录固件 (每个节点修改 node_id)
cd firmware/esp32-c5-csi-node
python ../../scripts/provision.py --chip esp32c5 --node-id 1 --port COM3

# 2. RZ/V2H 上启动服务
ssh root@192.168.1.1
cd /opt/ruview && ./deploy.sh

# 3. 浏览器打开仪表盘
# http://192.168.1.1:8080/ui/triage.html    ← 分诊仪表盘
# http://192.168.1.1:8080/ui/index.html     ← 3D 可视化
```

---

## 系统架构

```
CSI 感知层               AI 计算层                   展示层
─────────────────────    ──────────────────────    ────────────────
ESP32-C5 ×3              RZ/V2H                    7" 触屏 / Web
  │                        │                         │
  ├─ CSI 采集 (484子载波)  │                         │
  ├─ 2.4/5GHz 双频         │                         │
  │                        │                         │
  ├─ UDP 5005 ────────────►├─ Rust Signal Pipeline   │
  │  (ADR-018 二进制帧)    │  • FFT 呼吸率/心率      │
  │                        │  • START 分诊            │
  │                        │  • 伤员追踪 (Kalman)     │
  │                        │  • WiFi 三角定位         │
  │                        │  • 告警生成              │
  │                        │  • ONNX DensePose (可选) │
  │                        │                         │
  │                        ├─ WebSocket 8765 ────────►├─ Triage Dashboard
  │                        │                         │  • 2D 伤员地图
  │                        │                         │  • 生命体征卡片
  │                        │                         │  • 分诊统计 + 告警
  │                        │                         │
  │                        │                         ├─ 3D Visualization
  │                        │                         │  • Three.js 骨架
  │                        │                         │  • 实时运动追踪
```

---

## 核心功能

| 功能 | 实现 | 状态 |
|------|------|:--:|
| WiFi CSI 采集 | ESP32-C5 固件 (WiFi 6, 484 子载波) | ✅ |
| 呼吸率检测 | FFT 频域分析 (6-30 BPM) | ✅ |
| 心率检测 | 振幅微变频谱 (40-120 BPM) | ✅ |
| 人体存在检测 | CSI 振幅方差 + 自适应阈值 | ✅ |
| 多人区分 | MinCut 子载波分区 | ✅ |
| 人员定位 | WiFi RSSI 三角测量 | ✅ |
| **START 分诊** | 红/黄/绿/黑 + 自动评估 + LLM 辅助解释 | ✅ |
| **伤员追踪** | Kalman 滤波 + 生命周期管理 | ✅ |
| **群体伤情评估** | 严重程度 + 救援资源计算 | ✅ |
| **实时告警** | 恶化自动检测 + 推送 | ✅ |
| **端侧 LLM** | CSI 生命体征→自然语言病历报告 | 🔧 方案设计中 |
| 3D 骨架重建 | ONNX DensePose (可选按钮) | ✨ |
| 10 个医疗 WASM 模块 | 睡眠呼吸暂停/心律失常等 | ✅ |

---

## 目录结构

```
π-RuView-Competition/
├── README.md                          ← 本文件
├── deploy.sh                          ← 一键部署脚本
├── firmware/
│   └── esp32-c5-csi-node/            ← C5 CSI 固件 (完整)
├── rust-server/
│   ├── Cargo.toml                     ← Rust 工作区配置
│   └── crates/
│       ├── wifi-densepose-core/       ← 基础类型
│       ├── wifi-densepose-signal/     ← CSI 信号处理
│       ├── wifi-densepose-vitals/     ← 生命体征提取
│       ├── wifi-densepose-hardware/   ← CSI 帧解析
│       ├── wifi-densepose-nn/         ← ONNX 推理
│       ├── wifi-densepose-mat/        ← 分诊系统
│       ├── wifi-densepose-sensing-server/ ← 主服务
│       ├── wifi-densepose-config/     ← 系统配置
│       └── wifi-densepose-wasm-edge/  ← 边缘模块
├── ui/                                ← Web 3D 可视化
├── scripts/
│   └── provision.py                   ← C5 烧录脚本
└── docs/                              ← 竞赛设计文档
    ├── 竞赛改造方案.md
    ├── 竞赛差距分析.md
    ├── 竞赛准备清单.md
    ├── ML架构详解.md
    ├── ESP32-C5 移植审计报告.md
    ├── ESP32-C5 移植指南.md
    ├── 瑞萨 RZV2H 移植计划.md
    └── triage-ui/
        └── triage.html                ← 分诊仪表盘
```

---

## 技术亮点

- **WiFi 6 CSI**: ESP32-C5 484 子载波，4× 传统 S3 方案精度
- **端侧 LLM**: 生命体征→自然语言病历报告 (Qwen2.5-0.5B / Candle 推理)
- **Rust 54K FPS**: 信号处理管道比 Python 快 810 倍
- **START 分诊**: 标准战场分诊协议，自动伤员优先级评估
- **全本地部署**: 数据不出方舱，隐私安全，野外可用
- **瑞萨 DRP-AI**: 可选硬件推理加速

---

## 比赛文档

| 文档 | 内容 |
|------|------|
| `docs/竞赛改造方案.md` | 从开源项目到竞赛版本的完整改造计划 |
| `docs/竞赛差距分析.md` | 竞赛需求 vs 项目现有能力对比 |
| `docs/竞赛准备清单.md` | PPT/视频/展板等竞赛材料清单 |
| `docs/ML架构详解.md` | DensePose 模型架构 + 训练 + 推理 |
| `docs/ESP32-C5 移植审计报告.md` | C5 移植 39 处修改审计 |
| `docs/瑞萨 RZV2H 移植计划.md` | RZ/V2H 主控移植计划 |
| `docs/端侧LLM方案设计.md` | 端侧 LLM 伤病报告方案 |
| `docs/目录审计报告.md` | 目录完整性审计 |

---

## 许可证

MIT OR Apache-2.0
