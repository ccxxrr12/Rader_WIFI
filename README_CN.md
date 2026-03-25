# π RuView

<p align="center">
  <a href="https://ruvnet.github.io/RuView/">
    <img src="assets/ruview-small-gemini.jpg" alt="RuView - WiFi DensePose" width="100%">
  </a>
</p>

## **用WiFi + AI实现穿墙透视** ##

**通过信号感知世界。** 无需摄像头。无需可穿戴设备。无需互联网。仅凭物理原理。

### π RuView是一个直接从周围环境中学习的边缘AI感知系统。

它不依赖摄像头或云端模型,而是观察空间中存在的任何信号,如WiFi、跨频段无线电波、运动模式、振动、声音或其他感官输入,并构建对本地正在发生的事情的理解。

该项目建立在[RuVector](https://github.com/ruvnet/ruvector/)之上,因其实现WiFi DensePose而广为人知——这是一种首次在卡内基梅隆大学的"DensePose From WiFi"等学术研究中探索的感知技术。该研究表明WiFi信号可用于重建人体姿态。

RuView将这一概念扩展为实用的边缘系统。通过分析人体运动引起的信道状态信息(CSI)扰动,RuView利用基于物理的信号处理和机器学习实时重建身体位置、呼吸频率、心率和存在状态。

与依赖同步摄像头进行训练的研究系统不同,RuView设计为完全在边缘通过无线电信号和自学习嵌入向量运行。

该系统完全在廉价硬件上运行,如ESP32传感器网格(每个节点约1美元)。小型可编程边缘模块在本地分析信号,并随时间学习房间的射频特征,使系统能够将环境与内部活动区分开来。

由于RuView在靠近其观察的信号处学习,它在运行过程中不断改进。每次部署都会开发周围环境的本地模型,并持续适应,而无需摄像头、标注数据或云基础设施。

在实践中,这意味着普通环境获得了一种新型的空间感知能力。房间、建筑物和设备开始利用已经充满空间的信号来感知存在、运动和生命活动。

### 为低功耗边缘应用而构建

[边缘模块](#edge-intelligence-adr-041)是直接在ESP32传感器上运行的小型程序——无需互联网,无需云费用,即时响应。

[![Rust 1.85+](https://img.shields.io/badge/rust-1.85+-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Tests: 1300+](https://img.shields.io/badge/tests-1300%2B-brightgreen.svg)](https://github.com/ruvnet/RuView)
[![Docker: multi-arch](https://img.shields.io/badge/docker-amd64%20%2B%20arm64-blue.svg)](https://hub.docker.com/r/ruvnet/wifi-densepose)
[![Vital Signs](https://img.shields.io/badge/vital%20signs-breathing%20%2B%20heartbeat-red.svg)](#vital-sign-detection)
[![ESP32 Ready](https://img.shields.io/badge/ESP32--S3-CSI%20streaming-purple.svg)](#esp32-s3-hardware-pipeline)
[![crates.io](https://img.shields.io/crates/v/wifi-densepose-ruvector.svg)](https://crates.io/crates/wifi-densepose-ruvector)

 
> | 功能 | 实现方式 | 速度 |
> |------|-----|-------|
> | **姿态估计** | CSI子载波幅度/相位 → DensePose UV映射 | 54K fps (Rust) |
> | **呼吸检测** | 带通0.1-0.5 Hz → FFT峰值 | 6-30 BPM |
> | **心率** | 带通0.8-2.0 Hz → FFT峰值 | 40-120 BPM |
> | **存在感知** | RSSI方差 + 运动频带功率 | < 1ms延迟 |
> | **穿墙** | 菲涅尔区几何 + 多径建模 | 最深5米 |

```bash
# 30秒开始实时感知——无需工具链
docker pull ruvnet/wifi-densepose:latest
docker run -p 3000:3000 ruvnet/wifi-densepose:latest
# 打开 http://localhost:3000
```

> [!NOTE]
> **需要支持CSI的硬件。** 姿态估计、生命体征和穿墙感知依赖于信道状态信息(CSI)——标准消费级WiFi不公开的每子载波幅度和相位数据。要获得完整功能,您需要支持CSI的硬件(ESP32-S3或研究级网卡)。消费级WiFi笔记本电脑只能提供基于RSSI的存在检测,其能力显著较低。

> **实时CSI捕获的硬件选项:**
>
> | 选项 | 硬件 | 成本 | 完整CSI | 能力 |
> |--------|----------|------|----------|-------------|
> | **ESP32网格**(推荐) | 3-6个ESP32-S3 + WiFi路由器 | ~$54 | 是 | 姿态、呼吸、心跳、运动、存在 |
> | **研究级网卡** | Intel 5300 / Atheros AR9580 | ~$50-100 | 是 | 完整CSI,3x3 MIMO |
> | **任何WiFi** | Windows、macOS或Linux笔记本 | $0 | 否 | 仅RSSI:粗略存在和运动检测 |
>
> 没有硬件?使用确定性参考信号验证信号处理管道:`python v1/data/proof/verify.py`
>
---

## 📖 文档

| 文档 | 描述 |
|----------|-------------|
| [用户指南](docs/user-guide.md) | 分步指南:安装、首次运行、API使用、硬件设置、训练 |
| [构建指南](docs/build-guide.md) | 从源代码构建(Rust和Python) |
| [架构决策](docs/adr/README.md) | 62个ADR——每个技术选择的原因,按领域组织(硬件、信号处理、ML、平台、基础设施) |
| [领域模型](docs/ddd/README.md) | 7个DDD模型(RuvSense、信号处理、训练管道、硬件平台、感知服务器、WiFi-Mat、CHCI)——有界上下文、聚合、领域事件和通用语言 |
| [桌面应用](rust-port/wifi-densepose-rs/crates/wifi-densepose-desktop/README.md) | **开发中**——用于节点管理、OTA更新、WASM部署和网格可视化的Tauri v2桌面应用 |
| [医疗示例](examples/medical/README.md) | 通过60 GHz毫米波雷达实现非接触式血压、心率、呼吸率检测——15美元硬件,无需可穿戴设备 |

---


  <a href="https://ruvnet.github.io/RuView/">
    <img src="assets/v2-screen.png" alt="WiFi DensePose — 实时姿态检测及设置指南" width="800">
  </a>
  <br>
  <em>来自WiFi CSI信号的实时姿态骨架——无需摄像头,无需可穿戴设备</em>
  <br><br>
  <a href="https://ruvnet.github.io/RuView/"><strong>▶ 实时观测站演示</strong></a>
  &nbsp;|&nbsp;
  <a href="https://ruvnet.github.io/RuView/pose-fusion.html"><strong>▶ 双模态姿态融合演示</strong></a>

> [服务器](#-quick-start)对于可视化和聚合是可选的——ESP32[独立运行](#esp32-s3-hardware-pipeline)用于存在检测、生命体征和跌倒警报。
>
> **实时ESP32管道**:连接ESP32-S3节点 → 运行[感知服务器](#sensing-server) → 打开[姿态融合演示](https://ruvnet.github.io/RuView/pose-fusion.html)进行实时双模态姿态估计(网络摄像头 + WiFi CSI)。参见[ADR-059](docs/adr/ADR-059-live-esp32-csi-pipeline.md)。


## 🚀 核心功能

### 感知能力

仅使用房间内已有的WiFi信号,就能透过墙壁看到人、呼吸和心跳。

| | 功能 | 含义 |
|---|---------|---------------|
| 🔒 | **隐私优先** | 仅使用WiFi信号跟踪人体姿态——无摄像头、无视频、无存储图像 |
| 💓 | **生命体征** | 无需任何可穿戴设备即可检测呼吸率(6-30次/分钟)和心率(40-120次/分钟) |
| 👥 | **多人追踪** | 同时追踪多人,每个人都有独立的姿态和生命体征——无硬性软件限制(物理限制:56个子载波时每个AP约3-5人,多AP可更多) |
| 🧱 | **穿墙** | WiFi可穿透墙壁、家具和瓦砾——在摄像头无法工作的地方工作 |
| 🚑 | **灾难救援** | 透过瓦砾检测被困幸存者并分类伤情严重程度(START检伤分类) |
| 📡 | **多静态网格** | 4-6个低成本传感器节点协同工作,结合12+条重叠信号路径,实现360度房间全覆盖,亚英寸精度,无人员混淆([ADR-029](docs/adr/ADR-029-ruvsense-multistatic-sensing-mode.md)) |
| 🌐 | **持久场模型** | 系统学习每个房间的射频特征——然后减去房间以隔离人体运动,检测数天内的漂移,在运动开始前预测意图,标记欺骗尝试([ADR-030](docs/adr/ADR-030-ruvsense-persistent-field-model.md)) |

### 智能特性

系统自主学习并随时间变得更智能——无需手动调优,无需标注数据。

| | 功能 | 含义 |
|---|---------|---------------|
| 🧠 | **自学习** | 从原始WiFi数据中自学——无需标注训练集,无需摄像头引导([ADR-024](docs/adr/ADR-024-contrastive-csi-embedding-model.md)) |
| 🎯 | **AI信号处理** | 注意力网络、图算法和智能压缩替代手动调优的阈值——自动适应每个房间([RuVector](https://github.com/ruvnet/ruvector)) |
| 🌍 | **随处可用** | 一次训练,在任何房间部署——对抗域泛化消除环境偏差,使模型在房间、建筑物和硬件间迁移([ADR-027](docs/adr/ADR-027-cross-environment-domain-generalization.md)) |
| 👁️ | **跨视角融合** | AI结合每个传感器从其自身角度看到的信息——填补盲点和深度歧义,这是任何单一视角无法自行解决的([ADR-031](docs/adr/ADR-031-ruview-sensing-first-rf-mode.md)) |
| 🔮 | **信号线协议** | 6阶段处理管道将原始WiFi信号转换为结构化身体表示——从信号清理到基于图的空间推理,再到最终姿态输出([ADR-033](docs/adr/ADR-033-crv-signal-line-sensing-integration.md)) |
| 🔒 | **QUIC网格安全** | 所有传感器间通信都经过端到端加密,具有篡改检测、重放保护,以及节点移动或掉线时的无缝重连([ADR-032](docs/adr/ADR-032-multistatic-mesh-security-hardening.md)) |
| 🎯 | **自适应分类器** | 记录标注的CSI会话,用纯Rust训练15特征逻辑回归模型,学习房间的独特信号特征——用数据驱动的分类替代手动调优的阈值([ADR-048](docs/adr/ADR-048-adaptive-csi-classifier.md)) |

### 性能与部署

速度足够实时使用,体积足够适合边缘设备,设置足够简单,一键完成。

| | 功能 | 含义 |
|---|---------|---------------|
| ⚡ | **实时** | 每帧分析WiFi信号不到100微秒——足够实时监控 |
| 🦀 | **810倍更快** | 完整Rust重写:54,000帧/秒管道,多架构Docker镜像,1,031+测试 |
| 🐳 | **一键设置** | `docker pull ruvnet/wifi-densepose:latest`——30秒实时感知,无需工具链(amd64 + arm64 / Apple Silicon) |
| 📡 | **完全本地** | 完全在9美元ESP32上运行——无互联网连接,无云账户,无经常性费用。设备端检测存在、生命体征和跌倒,即时响应 |
| 📦 | **可移植模型** | 训练模型打包为单个`.rvf`文件——在边缘、云或浏览器(WASM)上运行 |
| 🔭 | **观测站可视化** | 电影级Three.js仪表板,带5个全息面板——子载波流形、生命体征预言机、存在热图、相位星座、收敛引擎——全部由实时或演示CSI数据驱动([ADR-047](docs/adr/ADR-047-psychohistory-observatory-visualization.md)) |
| 📟 | **AMOLED显示** | 带内置AMOLED屏幕的ESP32-S3板直接在传感器上显示实时存在、生命体征和房间状态——无需手机或PC([ADR-045](docs/adr/ADR-045-amoled-display-support.md)) |

---

## 🔬 工作原理

WiFi路由器向每个房间发射无线电波。当一个人移动——甚至呼吸时——这些波会以不同方式散射。WiFi DensePose读取这种散射模式并重建发生了什么:

```
WiFi路由器 → 无线电波穿过房间 → 击中人体 → 散射
    ↓
ESP32网格(4-6个节点)通过TDM协议在信道1/6/11上捕获CSI
    ↓
多频带融合:3个信道 × 56个子载波 = 每条链路168个虚拟子载波
    ↓
多静态融合:N×(N-1)条链路 → 注意力加权跨视角嵌入
    ↓
相干性门控:接受/拒绝测量 → 无需调优即可稳定数天
    ↓
信号处理:Hampel、SpotFi、Fresnel、BVP、频谱图 → 清洁特征
    ↓
AI骨干(RuVector):注意力、图算法、压缩、场模型
    ↓
信号线协议(CRV):6阶段格式塔 → 感知 → 拓扑 → 相干 → 搜索 → 模型
    ↓
神经网络:处理后的信号 → 17个身体关键点 + 生命体征 + 房间模型
    ↓
输出:实时姿态、呼吸、心率、房间指纹、漂移警报
```

无需训练摄像头——[自学习系统(ADR-024)](docs/adr/ADR-024-contrastive-csi-embedding-model.md)仅从原始WiFi数据引导。[MERIDIAN (ADR-027)](docs/adr/ADR-027-cross-environment-domain-generalization.md)确保模型在任何房间工作,而不仅仅是在训练的房间中。

---

## 🏢 用例与应用

WiFi感知在任何WiFi存在的地方都有效。大多数情况下无需新硬件——只需在现有接入点上安装软件或添加8美元ESP32。由于没有摄像头,部署在设计上避免了隐私法规(GDPR视频、HIPAA成像)。

**扩展性:**每个AP区分约3-5人(56个子载波)。多AP线性倍增——4AP零售网格覆盖约15-20名占用者。无硬性软件限制;实际上限是信号物理。

| | WiFi感知的优势 | 传统替代方案 |
|---|----------------------|----------------------|
| 🔒 | **无视频,无GDPR/HIPAA成像规则** | 摄像头需要同意、标识、数据保留政策 |
| 🧱 | **穿透墙壁、货架、瓦砾工作** | 摄像头每个房间需要视线 |
| 🌙 | **在完全黑暗中工作** | 摄像头需要红外或可见光 |
| 💰 | **每个区域0-8美元**(现有WiFi或ESP32) | 摄像头系统:每个区域200-2,000美元 |
| 🔌 | **WiFi已部署在各处** | PIR/雷达传感器每个房间需要新布线 |

<details>
<summary><strong>🏥 日常应用</strong>——医疗、零售、办公、酒店(通用WiFi)</summary>

| 用例 | 功能 | 硬件 | 关键指标 | 边缘模块 |
|----------|-------------|----------|------------|-------------|
| **老年护理/辅助生活** | 跌倒检测、夜间活动监测、睡眠呼吸率——无需可穿戴设备合规 | 每房间1个ESP32-S3(8美元) | 跌倒警报<2秒 | [睡眠呼吸暂停](docs/edge-modules/medical.md)、[步态分析](docs/edge-modules/medical.md) |
| **医院患者监护** | 非重症床位连续呼吸+心率监测,无需有线传感器;异常时护士警报 | 每病房1-2个AP | 呼吸:6-30 BPM | [呼吸窘迫](docs/edge-modules/medical.md)、[心律失常](docs/edge-modules/medical.md) |
| **急诊室检伤分类** | 自动占用计数+等待时间估计;在候诊区检测患者痛苦(异常呼吸) | 现有医院WiFi | 占用准确率>95% | [队列长度](docs/edge-modules/retail.md)、[恐慌运动](docs/edge-modules/security.md) |
| **零售占用与流量** | 实时客流、按区域停留时间、队列长度——无摄像头、无需选择加入、GDPR友好 | 现有商店WiFi + 1个ESP32 | 停留分辨率~1米 | [客户流量](docs/edge-modules/retail.md)、[停留热图](docs/edge-modules/retail.md) |
| **办公空间利用率** | 哪些办公桌/房间实际被占用、会议室未到场、基于真实存在的HVAC优化 | 现有企业WiFi | 存在延迟<1秒 | [会议室](docs/edge-modules/building.md)、[HVAC存在](docs/edge-modules/building.md) |
| **酒店与酒店业** | 无门传感器的房间占用、迷你吧/浴室使用模式、空房间节能 | 现有酒店WiFi | 15-30% HVAC节能 | [能源审计](docs/edge-modules/building.md)、[照明区域](docs/edge-modules/building.md) |
| **餐厅与餐饮服务** | 餐桌周转跟踪、厨房员工存在、浴室占用显示——用餐区无摄像头 | 现有WiFi | 队列等待±30秒 | [餐桌周转](docs/edge-modules/retail.md)、[队列长度](docs/edge-modules/retail.md) |
| **停车场** | 楼梯间和电梯中行人存在,摄像头有盲点;如果有人逗留则安全警报 | 现有WiFi | 穿透混凝土墙 | [逗留](docs/edge-modules/security.md)、[电梯计数](docs/edge-modules/building.md) |

</details>

<details>
<summary><strong>🏟️ 专业应用</strong>——活动、健身、教育、市政(支持CSI的硬件)</summary>

| 用例 | 功能 | 硬件 | 关键指标 | 边缘模块 |
|----------|-------------|----------|------------|-------------|
| **智能家居自动化** | 穿墙工作的房间级存在触发器(灯光、HVAC、音乐)——无死角,无运动传感器超时 | 2-3个ESP32-S3节点(24美元) | 穿墙范围~5米 | [HVAC存在](docs/edge-modules/building.md)、[照明区域](docs/edge-modules/building.md) |
| **健身与体育** | 动作计数、姿势纠正、运动期间呼吸节奏——无可穿戴设备,更衣室无摄像头 | 3+个ESP32-S3网格 | 姿态:17个关键点 | [呼吸同步](docs/edge-modules/exotic.md)、[步态分析](docs/edge-modules/medical.md) |
| **儿童保育与学校** | 午睡呼吸监测、操场人数统计、受限区域警报——未成年人隐私安全 | 每区域2-4个ESP32-S3 | 呼吸:±1 BPM | [睡眠呼吸暂停](docs/edge-modules/medical.md)、[周界突破](docs/edge-modules/security.md) |
| **活动场所与音乐会** | 人群密度映射、通过呼吸压缩检测挤压风险、紧急疏散流量跟踪 | 多AP网格(4-8个AP) | 每平方米密度 | [客户流量](docs/edge-modules/retail.md)、[恐慌运动](docs/edge-modules/security.md) |
| **体育场与竞技场** | 区域级占用用于动态定价、特许经营人员配备、紧急出口流量建模 | 企业AP网格 | 每个AP网格15-20人 | [停留热图](docs/edge-modules/retail.md)、[队列长度](docs/edge-modules/retail.md) |
| **宗教场所** | 无面部识别的出席计数——隐私敏感的会众、多房间校园跟踪 | 现有WiFi | 区域级准确率 | [电梯计数](docs/edge-modules/building.md)、[能源审计](docs/edge-modules/building.md) |
| **仓库与物流** | 工人安全区域、叉车接近警报、危险区域占用——穿透货架和托盘工作 | 工业AP网格 | 警报延迟<500毫秒 | [叉车接近](docs/edge-modules/industrial.md)、[受限空间](docs/edge-modules/industrial.md) |
| **市政基础设施** | 公共浴室占用(不可能有摄像头)、地铁平台拥挤、紧急情况下避难所人数统计 | 市政WiFi + ESP32 | 实时人数统计 | [客户流量](docs/edge-modules/retail.md)、[逗留](docs/edge-modules/security.md) |
| **博物馆与画廊** | 访客流量热图、展品停留时间、人群瓶颈警报——艺术品附近无摄像头(闪光灯/盗窃风险) | 现有WiFi | 区域停留±5秒 | [停留热图](docs/edge-modules/retail.md)、[货架参与](docs/edge-modules/retail.md) |

</details>

<details>
<summary><strong>🤖 机器人与工业</strong>——自主系统、制造、安卓空间感知</summary>

WiFi感知为机器人和自主系统提供空间感知层,在激光雷达和摄像头失效的地方工作——穿过灰尘、烟雾、雾气和拐角。CSI信号场作为检测环境中人类的"第六感",无需视线。

| 用例 | 功能 | 硬件 | 关键指标 | 边缘模块 |
|----------|-------------|----------|------------|-------------|
