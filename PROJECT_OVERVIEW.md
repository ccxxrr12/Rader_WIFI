# Rader_WIFI (π RuView) 项目概述

## 项目简介

Rader_WIFI（也称为π RuView）是一个**边缘AI感知系统**，能够通过WiFi信号实现"穿墙透视"功能。该项目的核心创新在于利用WiFi的**信道状态信息**(CSI)，无需摄像头、可穿戴设备或互联网连接，就能实时重建人体姿态、检测呼吸频率、心率和存在状态。

项目基于学术研究（如卡内基梅隆大学的"DensePose From WiFi"工作），但将其扩展为一个实用的边缘系统，能够在廉价硬件（如ESP32传感器网格，每个节点约1美元）上运行。

## 核心功能

### 感知能力
- **隐私优先的人体姿态估计**：仅使用WiFi信号，无需摄像头
- **生命体征检测**：呼吸率(6-30次/分钟)和心率(40-120次/分钟)
- **多人同时追踪**：每个AP可区分3-5人，多AP系统可线性扩展
- **穿墙感知**：WiFi信号可穿透墙壁、家具和瓦砾，最远可达5米深度
- **灾难救援**：通过瓦砾检测幸存者并进行START分级分类

### 智能特性
- **自学习系统**：从原始WiFi数据中自我学习，无需标注训练集或摄像头引导
- **AI信号处理**：使用注意力网络、图算法和智能压缩替代手工调优的阈值
- **跨环境泛化**：经过对抗域泛化训练，可在任何房间部署而无需重新训练
- **多视角融合**：AI结合每个传感器从不同角度看到的信息，填补盲点

### 性能与部署
- **实时处理**：每帧WiFi信号分析时间不到100微秒
- **高性能Rust实现**：54,000帧/秒的处理管道，比Python版本快810倍
- **一键部署**：支持Docker一键部署（amd64 + arm64/Apple Silicon）
- **完全本地运行**：可在9美元的ESP32上完全独立运行，无需互联网

## 项目架构

### 双代码库设计
项目采用**双代码库架构**：
1. **Python v1版本** (`v1/`目录)：原始实现，主要用于研究和验证
2. **Rust端口** (`rust-port/wifi-densepose-rs/`)：生产级高性能实现

### Rust核心组件
Rust版本包含多个核心crate：
- `wifi-densepose-core`：核心类型、特征和CSI帧原语
- `wifi-densepose-signal`：先进信号处理和RuvSense多静态感知
- `wifi-densepose-nn`：神经网络推理（支持ONNX、PyTorch、Candle后端）
- `wifi-densepose-hardware`：ESP32聚合器和TDM协议固件
- `wifi-densepose-wasm-edge`：WASM边缘模块（65个已实现模块）

### RuvSense信号处理模块
- **多频带融合**：跨信道CSI帧融合和相干性分析
- **相位对齐**：迭代LO相位偏移估计
- **多静态融合**：注意力加权融合和几何多样性
- **相干性门控**：信号质量门控决策
- **姿态追踪**：17关键点卡尔曼追踪器
- **场模型**：SVD房间特征结构和扰动提取

## 应用场景

### 日常应用
- **医疗健康**：老年人跌倒检测、医院患者监护、睡眠呼吸暂停监测
- **零售商业**：客流统计、停留时间分析、队列长度预测
- **智能建筑**：办公室空间利用率、酒店房间占用、HVAC优化

### 专业应用
- **体育健身**：动作计数、姿势纠正、呼吸节奏监测
- **教育儿童**：午睡呼吸监测、游乐场人数统计
- **工业机器人**：协作机器人安全区域、仓库AMR导航

### 极端环境
- **搜救救援**：通过瓦砾检测幸存者
- **消防应急**：烟雾和墙壁后的人员定位
- **军事战术**：穿墙人员检测和人质生命体征监测

## 技术架构特点

### 边缘智能
项目实现了**65个WASM边缘模块**，可在ESP32上直接运行：
- 医疗健康类：睡眠呼吸暂停、心律失常、步态分析
- 安全监控类：入侵检测、周界突破、徘徊检测
- 智能建筑类：区域占用、HVAC控制、电梯计数
- 信号智能类：CSI信号实时分析和特征提取

### 安全与隐私
- **无视频数据**：避免GDPR/HIPAA成像法规限制
- **QUIC网格安全**：端到端加密、防重放攻击、篡改检测
- **本地处理**：所有敏感数据在设备端处理，无需上传云端

### 可视化界面
项目提供多种可视化界面：
- **Observatory观测站**：基于Three.js的全息仪表板
- **Pose Fusion姿态融合**：WiFi CSI与摄像头的双模态姿态估计
- **移动应用**：支持Android/iOS的移动管理界面

## 硬件要求

### CSI-capable硬件选项
| 选项 | 硬件 | 成本 | 功能 |
|------|------|------|------|
| **ESP32网格**（推荐） | 3-6个ESP32-S3 + WiFi路由器 | ~$54 | 姿态、呼吸、心跳、运动、存在检测 |
| **研究网卡** | Intel 5300 / Atheros AR9580 | ~$50-100 | 完整CSI，3x3 MIMO |
| **普通WiFi** | Windows/macOS/Linux笔记本 | $0 | 仅RSSI：粗略存在和运动检测 |

## 项目文档体系

项目拥有完善的文档体系：
- **62个架构决策记录**(ADR)：详细记录每个技术选择的原因
- **7个领域驱动设计**(DDD)：定义有界上下文、聚合和领域事件
- **用户指南和构建指南**：详细的安装和使用说明
- **边缘模块文档**：65个WASM模块的完整文档

## 项目文件架构

```
Rader_WIFI/
├── v1/                                    # Python v1版本（研究验证）
│   ├── src/
│   │   ├── api/                             # FastAPI应用程序
│   │   │   ├── main.py                     # 应用入口点
│   │   │   ├── routers/                    # REST端点路由器
│   │   │   ├── middleware/                 # 认证、速率限制
│   │   │   └── websocket/                  # WebSocket连接管理
│   │   ├── core/                           # 核心CSI处理
│   │   │   ├── csi_processor.py            # CSI处理器
│   │   │   ├── phase_sanitizer.py          # 相位清理
│   │   │   └── router_interface.py         # 路由器接口
│   │   ├── sensing/                        # 感知后端
│   │   │   ├── rssi_collector.py           # RSSI收集器
│   │   │   ├── feature_extractor.py        # 特征提取
│   │   │   ├── classifier.py               # 分类器
│   │   │   └── backend.py                # 感知后端协议
│   │   ├── hardware/                       # 硬件接口
│   │   │   ├── csi_extractor.py            # CSI提取器
│   │   │   └── router_interface.py         # 路由器接口
│   │   ├── database/                       # 数据库层
│   │   │   ├── models.py                  # 数据模型
│   │   │   └── migrations/               # 数据库迁移
│   │   ├── services/                       # 业务服务
│   │   │   ├── pose_service.py            # 姿态服务
│   │   │   ├── stream_service.py           # 流服务
│   │   │   └── hardware_service.py        # 硬件服务
│   │   └── models/                         # 神经网络模型
│   │       ├── densepose_head.py           # DensePose头
│   │       └── modality_translation.py     # 模态转换
│   ├── tests/                             # 测试套件
│   │   ├── unit/                         # 单元测试
│   │   ├── integration/                  # 集成测试
│   │   ├── e2e/                         # 端到端测试
│   │   └── performance/                  # 性能测试
│   ├── docs/                             # v1文档
│   │   ├── api/                         # API文档
│   │   ├── developer/                   # 开发者文档
│   │   └── user-guide/                 # 用户指南
│   ├── data/                             # 数据目录
│   │   └── proof/                      # 验证数据
│   └── requirements-lock.txt               # 锁定依赖
│
├── rust-port/wifi-densepose-rs/            # Rust端口（生产级）
│   ├── crates/                           # Rust crates
│   │   ├── ruv-neural/                  # RuVector神经网络
│   │   │   ├── ruv-neural-core/         # 核心库
│   │   │   │   ├── src/
│   │   │   │   │   ├── brain.rs         # 大脑接口
│   │   │   │   │   ├── embedding.rs    # 嵌入向量
│   │   │   │   │   ├── graph.rs        # 图结构
│   │   │   │   │   ├── rvf.rs          # RVF模型
│   │   │   │   │   ├── sensor.rs       # 传感器接口
│   │   │   │   │   ├── signal.rs       # 信号处理
│   │   │   │   │   └── topology.rs    # 拓扑分析
│   │   │   ├── ruv-neural-decoder/      # 解码器
│   │   │   │   ├── src/
│   │   │   │   │   ├── clinical.rs     # 临床解码
│   │   │   │   │   ├── knn_decoder.rs # KNN解码器
│   │   │   │   │   └── pipeline.rs     # 解码管道
│   │   │   ├── ruv-neural-embed/       # 嵌入模块
│   │   │   │   ├── src/
│   │   │   │   │   ├── node2vec.rs     # Node2Vec
│   │   │   │   │   ├── spectral_embed.rs # 谱嵌入
│   │   │   │   │   └── temporal.rs     # 时态嵌入
│   │   │   ├── ruv-neural-graph/       # 图算法
│   │   │   │   ├── src/
│   │   │   │   │   ├── atlas.rs       # 图集
│   │   │   │   │   ├── dynamics.rs    # 动态图
│   │   │   │   │   └── spectral.rs    # 谱分析
│   │   │   ├── ruv-neural-mincut/      # 最小割算法
│   │   │   │   ├── src/
│   │   │   │   │   ├── stoer_wagner.rs # Stoer-Wagner算法
│   │   │   │   │   ├── spectral_cut.rs # 谱割
│   │   │   │   │   └── coherence.rs   # 相干性
│   │   │   ├── ruv-neural-memory/      # 记忆系统
│   │   │   │   ├── src/
│   │   │   │   │   ├── hnsw.rs        # HNSW索引
│   │   │   │   │   ├── store.rs       # 向量存储
│   │   │   │   │   └── session.rs     # 会话管理
│   │   │   ├── ruv-neural-sensor/      # 传感器模块
│   │   │   │   ├── src/
│   │   │   │   │   ├── calibration.rs # 校准
│   │   │   │   │   ├── nv_diamond.rs  # NV金刚石
│   │   │   │   │   └── eeg.rs        # EEG接口
│   │   │   ├── ruv-neural-signal/      # 信号处理
│   │   │   │   ├── src/
│   │   │   │   │   ├── filter.rs      # 滤波器
│   │   │   │   │   ├── hilbert.rs     # 希尔伯特变换
│   │   │   │   │   └── spectral.rs   # 谱分析
│   │   │   ├── ruv-neural-viz/         # 可视化
│   │   │   │   ├── src/
│   │   │   │   │   ├── animation.rs   # 动画
│   │   │   │   │   ├── colormap.rs    # 颜色映射
│   │   │   │   │   └── layout.rs      # 布局
│   │   │   ├── ruv-neural-wasm/        # WASM绑定
│   │   │   │   └── src/
│   │   │   │       ├── graph_wasm.rs   # 图WASM
│   │   │   │       └── streaming.rs  # 流式处理
│   │   │   └── ruv-neural-cli/         # CLI工具
│   │   │       └── src/commands/
│   │   │           ├── analyze.rs       # 分析命令
│   │   │           ├── mincut.rs        # 最小割命令
│   │   │           ├── pipeline.rs      # 管道命令
│   │   │           └── simulate.rs     # 模拟命令
│   │   ├── wifi-densepose-core/         # 核心库
│   │   │   └── src/
│   │   │       ├── types.rs            # 核心类型
│   │   │       ├── traits.rs           # 特征定义
│   │   │       └── utils.rs           # 工具函数
│   │   ├── wifi-densepose-hardware/     # 硬件接口
│   │   │   └── src/
│   │   │       ├── esp32/             # ESP32协议
│   │   │       │   ├── tdm.rs        # TDM协议
│   │   │       │   ├── secure_tdm.rs # 安全TDM
│   │   │       │   └── quic_transport.rs # QUIC传输
│   │   │       ├── aggregator/        # 聚合器
│   │   │       ├── esp32_parser.rs     # ESP32解析器
│   │   │       └── csi_frame.rs       # CSI帧
│   │   ├── wifi-densepose-nn/          # 神经网络
│   │   │   └── src/
│   │   │       ├── densepose.rs       # DensePose模型
│   │   │       ├── onnx.rs           # ONNX推理
│   │   │       └── inference.rs      # 推理引擎
│   │   ├── wifi-densepose-mat/         # WiFi-Mat灾难检测
│   │   │   └── src/
│   │   │       ├── detection/         # 检测模块
│   │   │       │   ├── breathing.rs   # 呼吸检测
│   │   │       │   ├── heartbeat.rs   # 心跳检测
│   │   │       │   ├── movement.rs    # 运动检测
│   │   │       │   └── pipeline.rs    # 检测管道
│   │   │       ├── localization/      # 定位模块
│   │   │       │   ├── triangulation.rs # 三角测量
│   │   │       │   ├── depth.rs      # 深度估计
│   │   │       │   └── fusion.rs     # 多传感器融合
│   │   │       ├── alerting/          # 警报模块
│   │   │       │   ├── dispatcher.rs  # 警报分发
│   │   │       │   └── triage_service.rs # 检伤服务
│   │   │       ├── tracking/          # 追踪模块
│   │   │       │   ├── kalman.rs     # 卡尔曼滤波
│   │   │       │   ├── lifecycle.rs  # 生命周期
│   │   │       │   └── tracker.rs    # 追踪器
│   │   │       └── domain/           # 领域模型
│   │   │           ├── survivor.rs    # 幸存者实体
│   │   │           ├── disaster_event.rs # 灾难事件
│   │   │           └── vital_signs.rs # 生命体征
│   │   ├── wifi-densepose-desktop/     # 桌面应用
│   │   │   ├── src/
│   │   │   │   ├── commands/          # Tauri命令
│   │   │   │   │   ├── discovery.rs  # 设备发现
│   │   │   │   │   ├── flash.rs      # 固件刷写
│   │   │   │   │   ├── provision.rs  # 设备配置
│   │   │   │   │   ├── server.rs     # 服务器
│   │   │   │   │   └── wasm.rs       # WASM管理
│   │   │   ├── domain/               # 领域逻辑
│   │   │   │   ├── config.rs       # 配置
│   │   │   │   ├── node.rs         # 节点模型
│   │   │   │   └── firmware.rs    # 固件模型
│   │   │   └── ui/                   # React UI
│   │   │       ├── src/
│   │   │       │   ├── pages/        # 页面组件
│   │   │       │   │   ├── Dashboard.tsx
│   │   │       │   │   ├── Nodes.tsx
│   │   │       │   │   ├── Sensing.tsx
│   │   │       │   │   ├── EdgeModules.tsx
│   │   │       │   │   ├── FlashFirmware.tsx
│   │   │       │   │   ├── OtaUpdate.tsx
│   │   │       │   │   └── Settings.tsx
│   │   │       │   ├── components/   # UI组件
│   │   │       │   │   ├── NodeCard.tsx
│   │   │       │   │   ├── Sidebar.tsx
│   │   │       │   │   └── StatusBadge.tsx
│   │   │       │   └── hooks/       # React Hooks
│   │   │       │       ├── useNodes.ts
│   │   │       │       └── useServer.ts
│   │   ├── wifi-densepose-cli/          # 命令行工具
│   │   │   └── src/
│   │   │       ├── main.rs            # CLI入口
│   │   │       └── mat.rs             # WiFi-Mat命令
│   │   ├── wifi-densepose-api/          # API库
│   │   ├── wifi-densepose-config/       # 配置库
│   │   └── wifi-densepose-db/          # 数据库库
│   └── Cargo.toml                      # Workspace配置
│
├── firmware/                            # ESP32固件
│   ├── esp32-csi-node/                 # ESP32 CSI节点
│   │   ├── main/
│   │   │   ├── main.c                 # 主程序
│   │   │   ├── csi_collector.c         # CSI收集
│   │   │   ├── csi_collector.h
│   │   │   ├── edge_processing.c       # 边缘处理
│   │   │   ├── edge_processing.h
│   │   │   ├── stream_sender.c         # 流发送
│   │   │   ├── stream_sender.h
│   │   │   ├── nvs_config.c           # NVS配置
│   │   │   ├── nvs_config.h
│   │   │   ├── wasm_runtime.c          # WASM运行时
│   │   │   ├── wasm_runtime.h
│   │   │   ├── display_task.c           # 显示任务
│   │   │   ├── display_ui.c            # 显示UI
│   │   │   ├── power_mgmt.c            # 电源管理
│   │   │   ├── ota_update.c           # OTA更新
│   │   │   └── rvf_parser.c           # RVF解析器
│   │   ├── components/
│   │   │   └── wasm3/                # WASM3解释器
│   │   ├── test/                       # 测试套件
│   │   │   ├── corpus/                # Fuzz语料库
│   │   │   └── stubs/                # 测试桩
│   │   ├── CMakeLists.txt
│   │   ├── sdkconfig.defaults          # ESP-IDF配置
│   │   └── README.md
│   └── esp32-hello-world/             # 测试固件
│
├── docs/                               # 文档目录
│   ├── adr/                           # 架构决策记录(62个)
│   │   ├── ADR-001-wifi-mat-disaster-detection.md
│   │   ├── ADR-012-esp32-csi-sensor-mesh.md
│   │   ├── ADR-014-sota-signal-processing.md
│   │   ├── ADR-024-contrastive-csi-embedding-model.md
│   │   ├── ADR-029-ruvsense-multistatic-sensing-mode.md
│   │   ├── ADR-030-ruvsense-persistent-field-model.md
│   │   ├── ADR-037-multi-person-pose-detection.md
│   │   ├── ADR-039-esp32-edge-intelligence.md
│   │   ├── ADR-040-wasm-programmable-sensing.md
│   │   ├── ADR-041-wasm-module-collection.md
│   │   └── ... (共62个ADR)
│   ├── ddd/                           # 领域驱动设计(7个)
│   │   ├── README.md
│   │   ├── ruvsense-domain-model.md
│   │   ├── signal-processing-domain-model.md
│   │   ├── training-pipeline-domain-model.md
│   │   ├── hardware-platform-domain-model.md
│   │   ├── sensing-server-domain-model.md
│   │   ├── wifi-mat-domain-model.md
│   │   └── chci-domain-model.md
│   ├── edge-modules/                   # 边缘模块文档
│   │   ├── README.md
│   │   ├── core.md                    # 核心模块(7个)
│   │   ├── medical.md                 # 医疗模块(5个)
│   │   ├── security.md                # 安全模块(6个)
│   │   ├── building.md                # 建筑模块(5个)
│   │   ├── retail.md                  # 零售模块(5个)
│   │   ├── industrial.md              # 工业模块(5个)
│   │   ├── exotic.md                  # 特殊模块(10个)
│   │   ├── signal-intelligence.md     # 信号智能(6个)
│   │   ├── adaptive-learning.md       # 自适应学习(4个)
│   │   ├── spatial-temporal.md        # 空间时间(6个)
│   │   ├── ai-security.md             # AI安全(2个)
│   │   └── autonomous.md             # 自主(4个)
│   ├── research/                      # 研究文档(20个)
│   │   ├── 00-rf-topological-sensing-index.md
│   │   ├── 01-rf-graph-theory-foundations.md
│   │   ├── 02-csi-edge-weight-computation.md
│   │   ├── 03-attention-mechanisms-rf-sensing.md
│   │   ├── 04-transformer-architectures-graph-sensing.md
│   │   ├── 05-sublinear-mincut-algorithms.md
│   │   ├── 06-esp32-mesh-hardware-constraints.md
│   │   ├── 07-contrastive-learning-rf-coherence.md
│   │   ├── 08-temporal-graph-evolution-ruvector.md
│   │   ├── 09-resolution-spatial-granularity.md
│   │   ├── 10-system-architecture-prototype.md
│   │   ├── 11-quantum-level-sensors.md
│   │   ├── 12-quantum-biomedical-sensing.md
│   │   ├── 13-nv-diamond-neural-magnetometry.md
│   │   └── ... (共20个研究文档)
│   ├── user-guide.md                 # 用户指南
│   ├── user-guide_CN.md              # 用户指南(中文)
│   ├── build-guide.md                # 构建指南
│   ├── build-guide_CN.md             # 构建指南(中文)
│   └── wifi-mat-user-guide.md       # WiFi-Mat指南
│
├── plans/                              # 项目规划
│   ├── overview.md                    # 总览
│   ├── overview_CN.md                 # 总览(中文)
│   ├── phase1-specification/          # 第一阶段规范
│   │   ├── system-requirements.md    # 系统需求
│   │   ├── functional-spec.md        # 功能规范
│   │   ├── technical-spec.md         # 技术规范
│   │   └── api-spec.md             # API规范
│   ├── phase2-architecture/          # 第二阶段架构
│   │   ├── system-architecture.md    # 系统架构
│   │   ├── neural-network-architecture.md # 神经网络架构
│   │   ├── hardware-integration.md   # 硬件集成
│   │   └── api-architecture.md      # API架构
│   └── ui-pose-detection-rebuild.md  # UI重建
│
├── docker/                             # Docker配置
│   ├── Dockerfile.python               # Python Dockerfile
│   ├── Dockerfile.rust                # Rust Dockerfile
│   ├── docker-compose.yml              # Docker Compose
│   └── .dockerignore
│
├── assets/                             # 资产文件
│   ├── ruview-small.jpg               # 项目截图
│   ├── ruview-small-gemini.jpg        # Gemini版本截图
│   ├── screen.png                     # 屏幕截图
│   ├── wifi-densepose-demo.zip        # 演示包
│   └── wifi-mat.zip                 # WiFi-Mat包
│
├── .github/                            # GitHub配置
│   └── workflows/                     # CI/CD工作流
│       ├── ci.yml                     # 持续集成
│       ├── cd.yml                     # 持续部署
│       ├── firmware-ci.yml            # 固件CI
│       ├── firmware-qemu.yml           # QEMU测试
│       ├── security-scan.yml          # 安全扫描
│       └── update-submodules.yml       # 子模块更新
│
├── .claude/                            # Claude AI配置
│   ├── agents/                        # AI代理配置
│   ├── commands/                      # AI命令
│   ├── helpers/                      # AI助手
│   ├── skills/                       # AI技能
│   └── settings.json                 # Claude设置
│
├── .vscode/                            # VSCode配置
│   └── launch.json                   # 调试配置
│
├── PROJECT_OVERVIEW.md                 # 项目概述(本文档)
├── README.md                          # 项目README
├── README_CN.md                       # 项目README(中文)
├── CHANGELOG.md                       # 变更日志
├── CLAUDE.md                         # Claude说明
├── LICENSE                            # 许可证
├── Makefile                          # Make配置
├── .gitignore                        # Git忽略
├── .gitmodules                       # Git子模块
├── .dockerignore                     # Docker忽略
├── .mcp.json                         # MCP配置
└── deploy.sh                         # 部署脚本
```

这个项目代表了WiFi感知技术从学术研究到实际应用的重要跨越，通过边缘AI和自学习系统，实现了真正实用的非视觉感知解决方案。