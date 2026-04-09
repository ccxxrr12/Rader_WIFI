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
| **协作机器人安全区** | 检测协作机器人附近的人类存在——在接触前自动减速或停止,即使在障碍物后面 | 每个工作单元2-3个ESP32-S3 | 存在延迟<100毫秒 | [叉车接近](docs/edge-modules/industrial.md)、[周界突破](docs/edge-modules/security.md) |
| **仓库AMR导航** | 自主移动机器人感知拐角处、货架后的人类——无激光雷达遮挡 | 沿通道部署ESP32网格 | 穿透货架检测 | [叉车接近](docs/edge-modules/industrial.md)、[逗留](docs/edge-modules/security.md) |
| **安卓/类人空间感知** | 社交机器人的环境人体姿态感知——无需始终开启摄像头即可检测手势、接近方向和个人空间 | 机载ESP32-S3模块 | 17关键点姿态 | [手势语言](docs/edge-modules/exotic.md)、[情绪检测](docs/edge-modules/exotic.md) |
| **生产线监控** | 每个工位的工人存在、人体工程学姿势警报、班次合规人数统计——穿透设备工作 | 每个区域工业AP | 姿态+呼吸 | [受限空间](docs/edge-modules/industrial.md)、[步态分析](docs/edge-modules/medical.md) |
| **建筑工地安全** | 重型机械周围的禁区执行、脚手架跌倒检测、人员人数统计 | 加固ESP32网格 | 警报<2秒,穿透灰尘 | [恐慌运动](docs/edge-modules/security.md)、[结构振动](docs/edge-modules/industrial.md) |
| **农业机器人** | 在灰尘/雾气田间条件下检测自动收割机附近的农场工人,此时摄像头不可靠 | 防风雨ESP32节点 | 开阔场地范围~10米 | [叉车接近](docs/edge-modules/industrial.md)、[雨水检测](docs/edge-modules/exotic.md) |
| **无人机着陆区** | 验证着陆区域无人——WiFi感知在雨水、灰尘和低光条件下工作,此时向下摄像头失效 | 地面ESP32节点 | 存在:>95%准确率 | [周界突破](docs/edge-modules/security.md)、[尾随](docs/edge-modules/security.md) |
| **洁净室监控** | 无摄像头人员跟踪(摄像头风扇的颗粒污染风险)——通过姿态进行 gown 合规性检查 | 现有洁净室WiFi | 无颗粒排放 | [洁净室](docs/edge-modules/industrial.md)、[ livestock 监控](docs/edge-modules/industrial.md) |

</details>

<details>
<summary><strong>🔥 极端场景</strong>——穿墙、灾难、防御、地下</summary>

这些场景利用WiFi穿透固体材料的能力——混凝土、瓦砾、泥土——这是光学或红外传感器无法到达的地方。WiFi-Mat灾难模块(ADR-001)专门为此级别设计。

| 用例 | 功能 | 硬件 | 关键指标 | 边缘模块 |
|----------|-------------|----------|------------|-------------|
| **搜索与救援(WiFi-Mat)** | 通过呼吸特征检测瓦砾/废墟中的幸存者,START检伤分类,3D定位 | 便携式ESP32网格+笔记本电脑 | 穿透30cm混凝土 | [呼吸窘迫](docs/edge-modules/medical.md)、[癫痫检测](docs/edge-modules/medical.md) |
| **消防** | 在进入前通过烟雾和墙壁定位 occupants;呼吸检测远程确认生命迹象 | 卡车上的便携式网格 | 在零能见度下工作 | [睡眠呼吸暂停](docs/edge-modules/medical.md)、[恐慌运动](docs/edge-modules/security.md) |
| **监狱和安全设施** | 牢房占用验证,痛苦检测(异常生命体征),周边感知——无摄像头盲点 | 专用AP基础设施 | 24/7生命体征 | [心律失常](docs/edge-modules/medical.md)、[逗留](docs/edge-modules/security.md) |
| **军事/战术** | 穿墙人员检测,房间清理确认,对峙距离下的人质生命体征 | 定向WiFi+自定义固件 | 穿墙范围:5米 | [周界突破](docs/edge-modules/security.md)、[武器检测](docs/edge-modules/security.md) |
| **边境和周边安全** | 检测隧道、围栏后、车辆中的人类存在——被动感知,无主动照明暴露位置 | 隐蔽ESP32网格 | 被动/隐蔽 | [周界突破](docs/edge-modules/security.md)、[尾随](docs/edge-modules/security.md) |
| **采矿和地下** | GPS/摄像头失效的隧道中的工人存在,坍塌后的呼吸检测,安全点人数统计 | 加固ESP32网格 | 穿透岩石/泥土 | [受限空间](docs/edge-modules/industrial.md)、[呼吸窘迫](docs/edge-modules/medical.md) |
| **海事和海军** | 通过钢质舱壁的甲板下人员跟踪(有限范围,需要调优),人员落水检测 | 船舶WiFi+ESP32 | 穿透1-2个舱壁 | [结构振动](docs/edge-modules/industrial.md)、[恐慌运动](docs/edge-modules/security.md) |
| **野生动物研究** | 围栏或洞穴中的非侵入性动物活动监测——无光污染,无视觉干扰 | 防风雨ESP32节点 | 零光发射 | [livestock 监控](docs/edge-modules/industrial.md)、[梦境阶段](docs/edge-modules/exotic.md) |

</details>

### 边缘智能 ([ADR-041](docs/adr/ADR-041-wasm-module-collection.md))

直接在ESP32传感器上运行的小型程序——无需互联网,无云费用,即时响应。每个模块是一个微小的WASM文件(5-30 KB),您可以通过空中上传到设备。它读取WiFi信号数据并在不到10毫秒内在本地做出决策。[ADR-041](docs/adr/ADR-041-wasm-module-collection.md)定义了13个类别中的60个模块——所有60个模块均已实现,609个测试通过。

| | 类别 | 示例 |
|---|----------|---------|
| 🏥 | [**医疗与健康**](docs/edge-modules/medical.md) | 睡眠呼吸暂停检测、心律失常、步态分析、癫痫检测 |
| 🔐 | [**安全与安防**](docs/edge-modules/security.md) | 入侵检测、周界突破、逗留、恐慌运动 |
| 🏢 | [**智能建筑**](docs/edge-modules/building.md) | 区域占用、HVAC控制、电梯计数、会议室跟踪 |
| 🛒 | [**零售与酒店**](docs/edge-modules/retail.md) | 队列长度、停留热图、客户流量、餐桌周转 |
| 🏭 | [**工业**](docs/edge-modules/industrial.md) | 叉车接近、受限空间监控、结构振动 |
| 🔮 | [**特殊与研究**](docs/edge-modules/exotic.md) | 睡眠分期、情绪检测、手语、呼吸同步 |
| 📡 | [**信号智能**](docs/edge-modules/signal-intelligence.md) | 清理和锐化原始WiFi信号——聚焦重要区域,过滤噪声,填补缺失数据,跟踪人员身份 |
| 🧠 | [**自适应学习**](docs/edge-modules/adaptive-learning.md) | 传感器随时间自行学习新手势和模式——无需云,更新后仍记得所学内容 |
| 🗺️ | [**空间推理**](docs/edge-modules/spatial-temporal.md) | 找出房间内人员位置,哪些区域最重要,使用基于图的空间逻辑跟踪跨区域移动 |
| ⏱️ | [**时间分析**](docs/edge-modules/spatial-temporal.md) | 学习日常 routine,检测模式打破(有人没起床),随时间验证安全规则是否被遵守 |
| 🛡️ | [**AI安全**](docs/edge-modules/ai-security.md) | 检测信号重放攻击、WiFi干扰、注入尝试,标记可能表明篡改的异常行为 |
| ⚛️ | [**量子启发**](docs/edge-modules/autonomous.md) | 使用量子启发数学映射房间范围的信号相干性,搜索最佳传感器配置 |
| 🤖 | [**自主与特殊**](docs/edge-modules/autonomous.md) | 自我管理传感器网格——自动修复掉线节点,计划自己的行动,探索实验性信号表示 |

所有已实现的模块都是`no_std` Rust,共享[通用实用库](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/vendor_common.rs),并通过12函数API与主机通信。完整文档:[**边缘模块指南**](docs/edge-modules/README.md)。见下面的[完整已实现模块列表](#edge-module-list)。

<details id="edge-module-list">
<summary><strong>🧩 边缘智能 — <a href="docs/edge-modules/README.md">所有65个模块已实现</a></strong> (ADR-041完成)</summary>

所有60个模块均已实现、测试(609个测试通过)并准备部署。它们编译为`wasm32-unknown-unknown`,通过WASM3在ESP32-S3上运行,并共享[通用实用库](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/vendor_common.rs)。源码: [`crates/wifi-densepose-wasm-edge/src/`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/)

**核心模块** (ADR-040旗舰+早期实现):

| 模块 | 文件 | 功能 |
|--------|------|-------------|
| 手势分类器 | [`gesture.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/gesture.rs) | 用于手势的DTW模板匹配 |
| 相干性过滤器 | [`coherence.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/coherence.rs) | 信号质量的相位相干性门控 |
| 对抗检测器 | [`adversarial.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/adversarial.rs) | 检测物理上不可能的信号模式 |
| 入侵检测器 | [`intrusion.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/intrusion.rs) | 人类与非人类运动分类 |
| 占用计数器 | [`occupancy.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/occupancy.rs) | 区域级人员计数 |
| 生命趋势 | [`vital_trend.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/vital_trend.rs) | 长期呼吸和心率趋势 |
| RVF解析器 | [`rvf.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/rvf.rs) | RVF容器格式解析 |

**供应商集成模块** (24个模块,ADR-041类别7):

**📡 信号智能** — 实时CSI分析和特征提取

| 模块 | 文件 | 功能 | 预算 |
|--------|------|-------------|--------|
| 闪光注意力 | [`sig_flash_attention.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/sig_flash_attention.rs) | 8个子载波组的平铺注意力——发现空间焦点区域和熵 | S (<5ms) |
| 相干性门 | [`sig_coherence_gate.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/sig_coherence_gate.rs) | 带滞后的Z-score相量门控:接受/仅预测/拒绝/重新校准 | L (<2ms) |
| 时间压缩 | [`sig_temporal_compress.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/sig_temporal_compress.rs) | 3层自适应量化(8位热/5位温/3位冷) | L (<2ms) |
| 稀疏恢复 | [`sig_sparse_recovery.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/sig_sparse_recovery.rs) | 用于丢失子载波的ISTA L1重建 | H (<10ms) |
| 人员匹配 | [`sig_mincut_person_match.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/sig_mincut_person_match.rs) | 用于多人跟踪的匈牙利简化二分分配 | S (<5ms) |
| 最优传输 | [`sig_optimal_transport.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/sig_optimal_transport.rs) | 带4个投影的切片Wasserstein-1距离 | L (<2ms) |

**🧠 自适应学习** — 无云连接的设备学习

| 模块 | 文件 | 功能 | 预算 |
|--------|------|-------------|--------|
| DTW手势学习 | [`lrn_dtw_gesture_learn.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/lrn_dtw_gesture_learn.rs) | 用户可教的手势识别——3次练习协议,16个模板 | S (<5ms) |
| 异常吸引子 | [`lrn_anomaly_attractor.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/lrn_anomaly_attractor.rs) | 带Lyapunov指数的4D动力系统吸引子分类 | H (<10ms) |
| 元适应 | [`lrn_meta_adapt.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/lrn_meta_adapt.rs) | 带安全回滚的爬山自优化 | L (<2ms) |
| EWC终身学习 | [`lrn_ewc_lifelong.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/lrn_ewc_lifelong.rs) | 弹性权重巩固——学习新任务时记住过去的任务 | S (<5ms) |

**🗺️ 空间推理** — 位置、接近度和影响映射

| 模块 | 文件 | 功能 | 预算 |
|--------|------|-------------|--------|
| PageRank影响 | [`spt_pagerank_influence.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/spt_pagerank_influence.rs) | 带幂迭代PageRank的4x4互相关图 | L (<2ms) |
| 微型HNSW | [`spt_micro_hnsw.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/spt_micro_hnsw.rs) | 用于最近邻搜索的64向量可导航小世界图 | S (<5ms) |
| 尖峰跟踪器 | [`spt_spiking_tracker.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/spt_spiking_tracker.rs) | 32个LIF神经元+4个输出区域神经元,带STDP学习 | S (<5ms) |

**⏱️ 时间分析** — 活动模式、逻辑验证、自主规划

| 模块 | 文件 | 功能 | 预算 |
|--------|------|-------------|--------|
| 模式序列 | [`tmp_pattern_sequence.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/tmp_pattern_sequence.rs) | 活动常规检测和偏差警报 | S (<5ms) |
| 时间逻辑守卫 | [`tmp_temporal_logic_guard.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/tmp_temporal_logic_guard.rs) | CSI事件流的LTL公式验证 | S (<5ms) |
| GOAP自主 | [`tmp_goap_autonomy.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/tmp_goap_autonomy.rs) | 用于自主模块管理的目标导向行动规划 | S (<5ms) |

**🛡️ AI安全** — 篡改检测和行为异常分析

| 模块 | 文件 | 功能 | 预算 |
|--------|------|-------------|--------|
| 提示护盾 | [`ais_prompt_shield.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/ais_prompt_shield.rs) | FNV-1a重放检测,注入检测(10x幅度),干扰(SNR) | L (<2ms) |
| 行为分析器 | [`ais_behavioral_profiler.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/ais_behavioral_profiler.rs) | 带Mahalanobis异常评分的6D行为分析 | S (<5ms) |

**⚛️ 量子启发** — 应用于CSI分析的量子计算隐喻

| 模块 | 文件 | 功能 | 预算 |
|--------|------|-------------|--------|
| 量子相干性 | [`qnt_quantum_coherence.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/qnt_quantum_coherence.rs) | Bloch球映射,Von Neumann熵,退相干检测 | S (<5ms) |
| 干涉搜索 | [`qnt_interference_search.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/qnt_interference_search.rs) | 16个房间状态假设,带Grover启发的预言+扩散 | S (<5ms) |

**🤖 自主系统** — 自管理和自修复行为

| 模块 | 文件 | 功能 | 预算 |
|--------|------|-------------|--------|
| 心理符号 | [`aut_psycho_symbolic.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/aut_psycho_symbolic.rs) | 带矛盾检测的16规则前向链接知识库 | S (<5ms) |
| 自修复网格 | [`aut_self_healing_mesh.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/aut_self_healing_mesh.rs) | 8节点网格,带健康跟踪,降级/恢复,覆盖修复 | S (<5ms) |

**🔮 特殊(供应商)** — 用于CSI解释的新型数学模型

| 模块 | 文件 | 功能 | 预算 |
|--------|------|-------------|--------|
| 时间晶体 | [`exo_time_crystal.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/exo_time_crystal.rs) | 256帧历史中的自相关次谐波检测 | S (<5ms) |
| 双曲空间 | [`exo_hyperbolic_space.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/exo_hyperbolic_space.rs) | 带32个参考位置的Poincare球嵌入,双曲距离 | S (<5ms) |

**🏥 医疗与健康** (类别1) — 非接触式健康监测

| 模块 | 文件 | 功能 | 预算 |
|--------|------|-------------|--------|
| 睡眠呼吸暂停 | [`med_sleep_apnea.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/med_sleep_apnea.rs) | 检测睡眠期间的呼吸暂停 | S (<5ms) |
| 心律失常 | [`med_cardiac_arrhythmia.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/med_cardiac_arrhythmia.rs) | 监测心率的不规则节律 | S (<5ms) |
| 呼吸窘迫 | [`med_respiratory_distress.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/med_respiratory_distress.rs) | 异常呼吸模式警报 | S (<5ms) |
| 步态分析 | [`med_gait_analysis.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/med_gait_analysis.rs) | 跟踪行走模式并检测变化 | S (<5ms) |
| 癫痫检测 | [`med_seizure_detect.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/med_seizure_detect.rs) | 强直-阵挛癫痫识别的6状态机 | S (<5ms) |

**🔐 安全与安防** (类别2) — 周界和威胁检测

| 模块 | 文件 | 功能 | 预算 |
|--------|------|-------------|--------|
| 周界突破 | [`sec_perimeter_breach.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/sec_perimeter_breach.rs) | 检测边界穿越,带接近/离开 | S (<5ms) |
| 武器检测 | [`sec_weapon_detect.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/sec_weapon_detect.rs) | 通过CSI幅度变化检测金属异常 | S (<5ms) |
| 尾随 | [`sec_tailgating.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/sec_tailgating.rs) | 检测接入点的未授权跟随 | S (<5ms) |
| 逗留 | [`sec_loitering.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/sec_loitering.rs) | 当有人在区域停留过长时间时警报 | S (<5ms) |
| 恐慌运动 | [`sec_panic_motion.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/sec_panic_motion.rs) | 检测逃离、挣扎或恐慌运动 | S (<5ms) |

**🏢 智能建筑** (类别3) — 自动化和能源效率

| 模块 | 文件 | 功能 | 预算 |
|--------|------|-------------|--------|
| HVAC存在 | [`bld_hvac_presence.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/bld_hvac_presence.rs) | 基于占用的HVAC控制,带离开倒计时 | S (<5ms) |
| 照明区域 | [`bld_lighting_zones.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/bld_lighting_zones.rs) | 基于区域活动的自动调暗/关闭照明 | S (<5ms) |
| 电梯计数 | [`bld_elevator_count.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/bld_elevator_count.rs) | 计数进入/离开的人员,带超载警告 | S (<5ms) |
| 会议室 | [`bld_meeting_room.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/bld_meeting_room.rs) | 跟踪会议生命周期:开始,人数,结束,可用性 | S (<5ms) |
| 能源审计 | [`bld_energy_audit.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/bld_energy_audit.rs) | 跟踪下班后使用和房间利用率 | S (<5ms) |

**🛒 零售与酒店** (类别4) — 无摄像头的客户洞察

| 模块 | 文件 | 功能 | 预算 |
|--------|------|-------------|--------|
| 队列长度 | [`ret_queue_length.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/ret_queue_length.rs) | 估计队列大小和等待时间 | S (<5ms) |
| 停留热图 | [`ret_dwell_heatmap.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/ret_dwell_heatmap.rs) | 显示人们花费时间的地方(热点/冷点) | S (<5ms) |
| 客户流量 | [`ret_customer_flow.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/ret_customer_flow.rs) | 计数进出并跟踪净占用 | S (<5ms) |
| 餐桌周转 | [`ret_table_turnover.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/ret_table_turnover.rs) | 餐厅餐桌生命周期:就座,用餐, vacate | S (<5ms) |
| 货架参与 | [`ret_shelf_engagement.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/ret_shelf_engagement.rs) | 检测浏览,考虑和伸手拿产品 | S (<5ms) |

**🏭 工业与专业** (类别5) — 安全和合规

| 模块 | 文件 | 功能 | 预算 |
|--------|------|-------------|--------|
| 叉车接近 | [`ind_forklift_proximity.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/ind_forklift_proximity.rs) | 当人们离车辆太近时警告 | S (<5ms) |
| 受限空间 | [`ind_confined_space.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/ind_confined_space.rs) | OSHA合规的工人监控,带提取警报 | S (<5ms) |
| 洁净室 | [`ind_clean_room.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/ind_clean_room.rs) | 占用限制和湍流运动检测 | S (<5ms) |
| livestock 监控 | [`ind_livestock_monitor.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/ind_livestock_monitor.rs) | 动物存在,静止和逃脱警报 | S (<5ms) |
| 结构振动 | [`ind_structural_vibration.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/ind_structural_vibration.rs) | 地震事件,机械共振,结构漂移 | S (<5ms) |

**🔮 特殊与研究** (类别6) — 实验性感知应用

| 模块 | 文件 | 功能 | 预算 |
|--------|------|-------------|--------|
| 梦境阶段 | [`exo_dream_stage.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/exo_dream_stage.rs) | 非接触式睡眠阶段分类(清醒/浅/深/REM) | S (<5ms) |
| 情绪检测 | [`exo_emotion_detect.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/exo_emotion_detect.rs) | 从微动作检测唤醒,压力和冷静 | S (<5ms) |
| 手势语言 | [`exo_gesture_language.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/exo_gesture_language.rs) | 通过WiFi进行手语字母识别 | S (<5ms) |
| 音乐指挥 | [`exo_music_conductor.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/exo_music_conductor.rs) | 从指挥手势跟踪 tempo 和动态 | S (<5ms) |
| 植物生长 | [`exo_plant_growth.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/exo_plant_growth.rs) | 监控植物生长,昼夜节律,枯萎检测 | S (<5ms) |
| 幽灵猎人 | [`exo_ghost_hunter.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/exo_ghost_hunter.rs) | 环境异常分类( draft/insect/wind/unknown) | S (<5ms) |
| 雨水检测 | [`exo_rain_detect.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/exo_rain_detect.rs) | 通过信号散射检测雨水开始,强度和停止 | S (<5ms) |
| 呼吸同步 | [`exo_breathing_sync.rs`](rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/exo_breathing_sync.rs) | 检测多人之间的同步呼吸 | S (<5ms) |

</details>

---

<details>
<summary><strong>🧠 自学习WiFi AI (ADR-024)</strong> — 自适应识别、自我优化和智能异常检测</summary>

每个穿过房间的WiFi信号都会创建该空间的唯一指纹。WiFi-DensePose已经读取这些指纹来跟踪人员,但直到现在它在每次读取后都会丢弃内部"理解"。自学习WiFi AI捕获并保留这种理解作为紧凑、可重用的向量——并为每个新环境不断优化自身。

**简单来说它做什么:**
- 将任何WiFi信号转换为128数字"指纹",唯一描述房间内发生的事情
- 完全从原始WiFi数据中自学——无需摄像头,无需标记,无需人类监督
- 仅使用WiFi识别房间,检测入侵者,识别人,分类活动
- 在8美元的ESP32芯片上运行(整个模型适合55 KB内存)
- 在单一计算中同时产生身体姿态跟踪和环境指纹

**关键能力**

| 功能 | 工作原理 | 重要性 |
|------|-------------|----------------|
| **自监督学习** | 模型观察WiFi信号并自学"相似"和"不同"的样子,无需任何人类标记数据 | 部署到任何地方——只需插入WiFi传感器并等待10分钟 |
| **房间识别** | 每个房间产生独特的WiFi指纹模式 | 无需GPS或信标即可知道某人在哪个房间 |
| **异常检测** | 意外人员或事件创建与之前所见不匹配的指纹 | 自动入侵和跌倒检测作为免费副产品 |
| **人员重新识别** | 每个人以略微不同的方式干扰WiFi,创建个人签名 | 无需摄像头即可跨会话跟踪个人 |
| **环境适应** | MicroLoRA适配器(每个房间1,792参数)为每个新空间微调模型 | 适应新房间数据最少——比从头重新训练少93% |
| **记忆保存** | EWC++正则化记住预训练期间学到的内容 | 切换到新任务不会擦除先前的知识 |
| **困难负样本挖掘** | 训练专注于最困惑的示例以更快学习 | 相同训练数据量下更好的准确性 |

**架构**

```
WiFi Signal [56 channels] → Transformer + Graph Neural Network
                                  ├→ 128-dim environment fingerprint (for search + identification)
                                  └→ 17-joint body pose (for human tracking)
```

**快速开始**

```bash
# 步骤1: 从原始WiFi数据学习(无需标签)
cargo run -p wifi-densepose-sensing-server -- --pretrain --dataset data/csi/ --pretrain-epochs 50

# 步骤2: 用姿态标签微调以获得完整能力
cargo run -p wifi-densepose-sensing-server -- --train --dataset data/mmfi/ --epochs 100 --save-rvf model.rvf

# 步骤3: 使用模型——从实时WiFi提取指纹
cargo run -p wifi-densepose-sensing-server -- --model model.rvf --embed

# 步骤4: 搜索——找到相似环境或检测异常
cargo run -p wifi-densepose-sensing-server -- --model model.rvf --build-index env
```

**训练模式**

| 模式 | 需要什么 | 获得什么 |
|------|--------------|-------------|
| 自监督 | 仅原始WiFi数据 | 理解WiFi信号结构的模型 |
| 监督 | WiFi数据+身体姿态标签 | 完整姿态跟踪+环境指纹 |
| 跨模态 | WiFi数据+摄像头 footage | 与视觉理解对齐的指纹 |

**指纹索引类型**

| 索引 | 存储内容 | 实际用途 |
|-------|---------------|----------------|
| `env_fingerprint` | 平均房间指纹 | "这是厨房还是卧室?" |
| `activity_pattern` | 活动边界 | "有人在做饭、睡觉还是锻炼?" |
| `temporal_baseline` | 正常条件 | "这个房间刚刚发生了不寻常的事情" |
| `person_track` | 个人移动签名 | "A人刚进入客厅" |

**模型大小**

| 组件 | 参数 | 内存(在ESP32上) |
|-----------|-----------|-------------------|
| Transformer骨干 | ~28,000 | 28 KB |
| 嵌入投影头 | ~25,000 | 25 KB |
| 每房间MicroLoRA适配器 | ~1,800 | 2 KB |
| **总计** | **~55,000** | **55 KB** (of 520 KB available) |

自学习系统建立在[AI骨干(RuVector)](#ai-backbone-ruvector)信号处理层之上——注意力、图算法和压缩——在顶部添加对比学习。

完整架构详情见[`docs/adr/ADR-024-contrastive-csi-embedding-model.md`](docs/adr/ADR-024-contrastive-csi-embedding-model.md)。

</details>

---

## 📦 安装

<details>
<summary><strong>引导安装程序</strong> — 交互式硬件检测和配置文件选择</summary>

```bash
./install.sh
```

安装程序逐步完成7个步骤:系统检测、工具链检查、WiFi硬件扫描、配置文件推荐、依赖安装、构建和验证。

| 配置文件 | 安装内容 | 大小 | 要求 |
|---------|-----------------|------|-------------|
| `verify` | 仅管道验证 | ~5 MB | Python 3.8+ |
| `python` | 完整Python API服务器+感知 | ~500 MB | Python 3.8+ |
| `rust` | Rust管道(~810倍更快) | ~200 MB | Rust 1.70+ |
| `browser` | 浏览器内执行的WASM | ~10 MB | Rust + wasm-pack |
| `iot` | ESP32传感器网格+聚合器 |  varies | Rust + ESP-IDF |
| `docker` | 基于Docker的部署 | ~1 GB | Docker |
| `field` | WiFi-Mat灾难响应工具包 | ~62 MB | Rust + wasm-pack |
| `full` | 所有可用内容 | ~2 GB | 所有工具链 |

```bash
# 非交互式
./install.sh --profile rust --yes

# 仅硬件检查
./install.sh --check-only
```

</details>

<details>
<summary><strong>从源代码</strong> — Rust(主要)或Python</summary>

```bash
git clone https://github.com/ruvnet/RuView.git
cd RuView

# Rust (主要 — 810倍更快)
cd rust-port/wifi-densepose-rs
cargo build --release
cargo test --workspace

# Python (传统v1)
pip install -r requirements.txt
pip install -e .

# 或通过pip
pip install wifi-densepose
pip install wifi-densepose[gpu]   # GPU加速
pip install wifi-densepose[all]   # 所有可选依赖
```

</details>

<details>
<summary><strong>Docker</strong> — 预构建镜像,无需工具链</summary>

```bash
# Rust感知服务器(132 MB — 推荐)
docker pull ruvnet/wifi-densepose:latest
docker run -p 3000:3000 -p 3001:3001 -p 5005:5005/udp ruvnet/wifi-densepose:latest

# Python感知管道(569 MB)
docker pull ruvnet/wifi-densepose:python
docker run -p 8765:8765 -p 8080:8080 ruvnet/wifi-densepose:python

# 通过docker-compose运行两者
cd docker && docker compose up

# 导出RVF模型
docker run --rm -v $(pwd):/out ruvnet/wifi-densepose:latest --export-rvf /out/model.rvf
```

| 镜像 | 标签 | 平台 | 端口 |
|-------|-----|-----------|-------|
| `ruvnet/wifi-densepose` | `latest`, `rust` | linux/amd64, linux/arm64 | 3000 (REST), 3001 (WS), 5005/udp (ESP32) |
| `ruvnet/wifi-densepose` | `python` | linux/amd64 | 8765 (WS), 8080 (UI) |

</details>

<details>
<summary><strong>系统要求</strong></summary>

- **Rust**: 1.70+ (主要运行时 — 通过[rustup](https://rustup.rs/)安装)
- **Python**: 3.8+ (用于验证和传统v1 API)
- **操作系统**: Linux (Ubuntu 18.04+), macOS (10.15+), Windows 10+
- **内存**: 最小4GB RAM, 推荐8GB+
- **存储**: 2GB可用空间用于模型和数据
- **网络**: 具有CSI能力的WiFi接口(可选 — 安装程序检测您拥有的设备)
- **GPU**: 可选 (NVIDIA CUDA或Apple Metal)

</details>

<details>
<summary><strong>Rust Crates</strong> — crates.io上的独立crates</summary>

Rust工作区由15个crates组成,均发布到[crates.io](https://crates.io/):

```bash
# 将单个crates添加到您的Cargo.toml
cargo add wifi-densepose-core       # 类型, traits, 错误
cargo add wifi-densepose-signal     # CSI信号处理(6个SOTA算法)
cargo add wifi-densepose-nn         # 神经推理(ONNX, PyTorch, Candle)
cargo add wifi-densepose-vitals     # 生命体征提取(呼吸+心率)
cargo add wifi-densepose-mat        # 灾难响应(MAT幸存者检测)
cargo add wifi-densepose-hardware   # ESP32, Intel 5300, Atheros传感器
cargo add wifi-densepose-train      # 训练管道(MM-Fi数据集)
cargo add wifi-densepose-wifiscan   # 多BSSID WiFi扫描
cargo add wifi-densepose-ruvector   # RuVector v2.0.4集成层(ADR-017)
```

| Crate | 描述 | RuVector | crates.io |
|-------|-------------|----------|-----------|
| [`wifi-densepose-core`](https://crates.io/crates/wifi-densepose-core) | 基础类型, traits和工具 | -- | [![crates.io](https://img.shields.io/crates/v/wifi-densepose-core.svg)](https://crates.io/crates/wifi-densepose-core) |
| [`wifi-densepose-signal`](https://crates.io/crates/wifi-densepose-signal) | SOTA CSI信号处理(SpotFi, FarSense, Widar 3.0) | `mincut`, `attn-mincut`, `attention`, `solver` | [![crates.io](https://img.shields.io/crates/v/wifi-densepose-signal.svg)](https://crates.io/crates/wifi-densepose-signal) |
| [`wifi-densepose-nn`](https://crates.io/crates/wifi-densepose-nn) | 多后端推理(ONNX, PyTorch, Candle) | -- | [![crates.io](https://img.shields.io/crates/v/wifi-densepose-nn.svg)](https://crates.io/crates/wifi-densepose-nn) |
| [`wifi-densepose-train`](https://crates.io/crates/wifi-densepose-train) | 带MM-Fi数据集的训练管道(NeurIPS 2023) | **All 5** | [![crates.io](https://img.shields.io/crates/v/wifi-densepose-train.svg)](https://crates.io/crates/wifi-densepose-train) |
| [`wifi-densepose-mat`](https://crates.io/crates/wifi-densepose-mat) | 大规模伤亡评估工具(灾难幸存者检测) | `solver`, `temporal-tensor` | [![crates.io](https://img.shields.io/crates/v/wifi-densepose-mat.svg)](https://crates.io/crates/wifi-densepose-mat) |
| [`wifi-densepose-ruvector`](https://crates.io/crates/wifi-densepose-ruvector) | RuVector v2.0.4集成层 — 7个信号+MAT集成点(ADR-017) | **All 5** | [![crates.io](https://img.shields.io/crates/v/wifi-densepose-ruvector.svg)](https://crates.io/crates/wifi-densepose-ruvector) |
| [`wifi-densepose-vitals`](https://crates.io/crates/wifi-densepose-vitals) | 生命体征:呼吸(6-30 BPM), 心率(40-120 BPM) | -- | [![crates.io](https://img.shields.io/crates/v/wifi-densepose-vitals.svg)](https://crates.io/crates/wifi-densepose-vitals) |
| [`wifi-densepose-hardware`](https://crates.io/crates/wifi-densepose-hardware) | ESP32, Intel 5300, Atheros CSI传感器接口 | -- | [![crates.io](https://img.shields.io/crates/v/wifi-densepose-hardware.svg)](https://crates.io/crates/wifi-densepose-hardware) |
| [`wifi-densepose-wifiscan`](https://crates.io/crates/wifi-densepose-wifiscan) | 多BSSID WiFi扫描(Windows, macOS, Linux) | -- | [![crates.io](https://img.shields.io/crates/v/wifi-densepose-wifiscan.svg)](https://crates.io/crates/wifi-densepose-wifiscan) |
| [`wifi-densepose-wasm`](https://crates.io/crates/wifi-densepose-wasm) | 浏览器部署的WebAssembly绑定 | -- | [![crates.io](https://img.shields.io/crates/v/wifi-densepose-wasm.svg)](https://crates.io/crates/wifi-densepose-wasm) |
| [`wifi-densepose-sensing-server`](https://crates.io/crates/wifi-densepose-sensing-server) | Axum服务器:UDP摄取, WebSocket广播 | -- | [![crates.io](https://img.shields.io/crates/v/wifi-densepose-sensing-server.svg)](https://crates.io/crates/wifi-densepose-sensing-server) |
| [`wifi-densepose-cli`](https://crates.io/crates/wifi-densepose-cli) | 用于MAT灾难扫描的命令行工具 | -- | [![crates.io](https://img.shields.io/crates/v/wifi-densepose-cli.svg)](https://crates.io/crates/wifi-densepose-cli) |
| [`wifi-densepose-api`](https://crates.io/crates/wifi-densepose-api) | REST + WebSocket API层 | -- | [![crates.io](https://img.shields.io/crates/v/wifi-densepose-api.svg)](https://crates.io/crates/wifi-densepose-api) |
| [`wifi-densepose-config`](https://crates.io/crates/wifi-densepose-config) | 配置管理 | -- | [![crates.io](https://img.shields.io/crates/v/wifi-densepose-config.svg)](https://crates.io/crates/wifi-densepose-config) |
| [`wifi-densepose-db`](https://crates.io/crates/wifi-densepose-db) | 数据库持久化(PostgreSQL, SQLite, Redis) | -- | [![crates.io](https://img.shields.io/crates/v/wifi-densepose-db.svg)](https://crates.io/crates/wifi-densepose-db) |

所有crates都与[RuVector v2.0.4](https://github.com/ruvnet/ruvector)集成 — 见下面的[AI骨干](#ai-backbone-ruvector)。

**[rUv Neural](rust-port/wifi-densepose-rs/crates/ruv-neural/)** — 一个单独的12-crate工作区,用于脑网络拓扑分析、神经解码和医疗感知。见模型与训练中的[rUv Neural](#ruv-neural)。

</details>

---

## 🚀 快速开始

<details open>
<summary><strong>3个命令完成首次API调用</strong></summary>

### 1. 安装

```bash
# 最快路径 — Docker
docker pull ruvnet/wifi-densepose:latest
docker run -p 3000:3000 ruvnet/wifi-densepose:latest

# 或从源代码(Rust)
./install.sh --profile rust --yes
```

### 2. 启动系统

```python
from wifi_densepose import WiFiDensePose

system = WiFiDensePose()
system.start()
poses = system.get_latest_poses()
print(f"Detected {len(poses)} persons")
system.stop()
```

### 3. REST API

```bash
# 健康检查
curl http://localhost:3000/health

# 最新感知帧
curl http://localhost:3000/api/v1/sensing/latest

# 生命体征
curl http://localhost:3000/api/v1/vital-signs

# 姿态估计
curl http://localhost:3000/api/v1/pose/current

# 服务器信息
curl http://localhost:3000/api/v1/info
```

### 4. 实时WebSocket

```python
import asyncio, websockets, json

async def stream():
    async with websockets.connect("ws://localhost:3001/ws/sensing") as ws:
        async for msg in ws:
            data = json.loads(msg)
            print(f"Persons: {len(data.get('persons', []))}")

asyncio.run(stream())
```

</details>

---

## 📋 目录

<details open>
<summary><strong>📡 信号处理与感知</strong> — 从原始WiFi帧到生命体征</summary>

信号处理堆栈将原始WiFi信道状态信息转换为可操作的人体感知数据。从20 Hz捕获的56-192个子载波复数值开始，管道应用研究级算法(SpotFi相位校正、Hampel异常值拒绝、菲涅尔区建模)提取呼吸率、心率、运动水平和多人身体姿态——全部在纯Rust中实现，无外部ML依赖。

| 部分 | 描述 | 文档 |
|---------|-------------|------|
| [核心功能](#key-features) | 感知、智能和性能与部署能力 | — |
| [工作原理](#how-it-works) | 端到端管道:无线电波 → CSI捕获 → 信号处理 → AI → 姿态+生命体征 | — |
| [ESP32-S3硬件管道](#esp32-s3-hardware-pipeline) | 20 Hz CSI流、二进制帧解析、刷写与配置 | [ADR-018](docs/adr/ADR-018-esp32-dev-implementation.md) · [教程 #34](https://github.com/ruvnet/RuView/issues/34) |
| [生命体征检测](#vital-sign-detection) | 呼吸6-30 BPM, 心跳40-120 BPM, FFT峰值检测 | [ADR-021](docs/adr/ADR-021-vital-sign-detection-rvdna-pipeline.md) |
| [WiFi扫描域层](#wifi-scan-domain-layer) | 8阶段RSSI管道, 多BSSID指纹识别, Windows WiFi | [ADR-022](docs/adr/ADR-022-windows-wifi-enhanced-fidelity-ruvector.md) · [教程 #36](https://github.com/ruvnet/RuView/issues/36) |
| [WiFi-Mat灾难响应](#wifi-mat-disaster-response) | 搜索与救援, START检伤分类, 穿透瓦砾3D定位 | [ADR-001](docs/adr/ADR-001-wifi-mat-disaster-detection.md) · [用户指南](docs/wifi-mat-user-guide.md) |
| [SOTA信号处理](#sota-signal-processing) | SpotFi, Hampel, 菲涅尔, STFT频谱图, 子载波选择, BVP | [ADR-014](docs/adr/ADR-014-sota-signal-processing.md) |

</details>

<details>
<summary><strong>🧠 模型与训练</strong> — DensePose管道, RVF容器, SONA适应, RuVector集成</summary>

神经管道使用带交叉注意力的图转换器将CSI特征矩阵映射到17个COCO身体关键点和DensePose UV坐标。模型打包为单文件`.rvf`容器，具有渐进式加载(层A即时, 层B预热, 层C完整)。SONA(自优化神经架构)通过micro-LoRA + EWC++实现持续的设备端适应，无灾难性遗忘。信号处理由5个[RuVector](https://github.com/ruvnet/ruvector) crates(v2.0.4)提供支持，在Rust工作区中有7个集成点，加上6个额外的供应商crates用于推理和图智能。

| 部分 | 描述 | 文档 |
|---------|-------------|------|
| [RVF模型容器](#rvf-model-container) | 带Ed25519签名的二进制打包, 渐进式3层加载, SIMD量化 | [ADR-023](docs/adr/ADR-023-trained-densepose-model-ruvector-pipeline.md) |
| [训练与微调](#training--fine-tuning) | 8阶段纯Rust管道(7,832行), MM-Fi/Wi-Pose预训练, 6项复合损失, SONA LoRA | [ADR-023](docs/adr/ADR-023-trained-densepose-model-ruvector-pipeline.md) |
| [RuVector Crates](#ruvector-crates) | 来自[ruvector](https://github.com/ruvnet/ruvector)的11个供应商Rust crates: 注意力, 最小割, 求解器, GNN, HNSW, 时间压缩, 稀疏推理 | [GitHub](https://github.com/ruvnet/ruvector) · [源码](vendor/ruvector/) |
| [rUv Neural](#ruv-neural) | 12-crate脑拓扑分析生态系统: 神经解码, 量子传感器集成, 认知状态分类, BCI输出 | [README](rust-port/wifi-densepose-rs/crates/ruv-neural/README.md) |
| [AI骨干(RuVector)](#ai-backbone-ruvector) | 5个AI能力替代手动调优阈值: 注意力, 图最小割, 稀疏求解器, 分层压缩 | [crates.io](https://crates.io/crates/wifi-densepose-ruvector) |
| [自学习WiFi AI (ADR-024)](#self-learning-wifi-ai-adr-024) | 对比自监督学习, 房间指纹识别, 异常检测, 55 KB模型 | [ADR-024](docs/adr/ADR-024-contrastive-csi-embedding-model.md) |
| [跨环境泛化 (ADR-027)](docs/adr/ADR-027-cross-environment-domain-generalization.md) | 域对抗训练, 几何条件推理, 硬件归一化, 零样本部署 | [ADR-027](docs/adr/ADR-027-cross-environment-domain-generalization.md) |

</details>

<details>
<summary><strong>🖥️ 使用与配置</strong> — CLI标志, API端点, 硬件设置</summary>

Rust感知服务器是主要接口，提供全面的CLI，带有数据源选择、模型加载、训练、基准测试和RVF导出的标志。REST API(Axum)和WebSocket服务器提供实时数据访问。Python v1 CLI仍然可用于传统工作流。

| 部分 | 描述 | 文档 |
|---------|-------------|------|
| [CLI使用](#cli-usage) | `--source`, `--train`, `--benchmark`, `--export-rvf`, `--model`, `--progressive` | — |
| [REST API & WebSocket](#rest-api--websocket) | 6个REST端点(sensing, vitals, BSSID, SONA), WebSocket实时流 | — |
| [硬件支持](#hardware-support-1) | ESP32-S3 ($8), Intel 5300 ($15), Atheros AR9580 ($20), Windows RSSI ($0) | [ADR-012](docs/adr/ADR-012-esp32-csi-sensor-mesh.md) · [ADR-013](docs/adr/ADR-013-feature-level-sensing-commodity-gear.md) |

</details>

<details>
<summary><strong>⚙️ 开发与测试</strong> — 542+测试, CI, 部署</summary>

项目在7个crate套件中维护542+纯Rust测试，零模拟——每个测试都针对真实算法实现运行。无硬件模拟模式(`--source simulate`)支持无需物理设备的全栈测试。Docker镜像发布在Docker Hub上，实现零设置部署。

| 部分 | 描述 | 文档 |
|---------|-------------|------|
| [测试](#testing) | 7个测试套件: sensing-server (229), signal (83), mat (139), wifiscan (91), RVF (16), vitals (18) | — |
| [部署](#deployment) | Docker镜像(132 MB Rust / 569 MB Python), docker-compose, 环境变量 | — |
| [贡献](#contributing) | Fork → branch → test → PR工作流, Rust和Python开发设置 | — |

</details>

<details>
<summary><strong>📊 性能与基准测试</strong> — 测量吞吐量, 延迟, 资源使用</summary>

所有基准测试均在Rust感知服务器上使用`cargo bench`和内置的`--benchmark` CLI标志进行测量。Rust v2实现比Python v1基线提供810x端到端加速，运动检测达到5,400x改进。生命体征检测器在单线程基准测试中处理11,665帧/秒。

| 部分 | 描述 | 关键指标 |
|---------|-------------|------------|
| [性能指标](#performance-metrics) | 生命体征, CSI管道, 运动检测, Docker镜像, 内存 | 11,665 fps生命体征 · 54K fps管道 |
| [Rust vs Python](#python-vs-rust) | 5项操作的并排基准测试 | **810x**全管道加速 |

</details>

<details>
<summary><strong>📄 元信息</strong> — 许可证, 更新日志, 支持</summary>

WiFi DensePose是MIT许可的开源项目，由[ruvnet](https://github.com/ruvnet)开发。该项目自2025年3月以来一直在积极开发中，3个主要版本提供了Rust移植、SOTA信号处理、灾难响应模块和端到端训练管道。

| 部分 | 描述 | 链接 |
|---------|-------------|------|
| [更新日志](#changelog) | v3.0.0 (AETHER AI + Docker), v2.0.0 (Rust移植 + SOTA + WiFi-Mat) | [CHANGELOG.md](CHANGELOG.md) |
| [许可证](#license) | MIT许可证 | [LICENSE](LICENSE) |
| [支持](#support) | 错误报告, 功能请求, 社区讨论 | [Issues](https://github.com/ruvnet/RuView/issues) · [Discussions](https://github.com/ruvnet/RuView/discussions) |

</details>

---

<details>
<summary><strong>🌍 跨环境泛化 (ADR-027 — 项目MERIDIAN)</strong> — 训练一次, 在任何房间部署无需重新训练</summary>

| 功能 | 工作原理 | 重要性 |
|------|-------------|----------------|
| **梯度反转层** | 一个对抗分类器尝试猜测信号来自哪个房间;主网络被训练以欺骗它 | 强制模型丢弃房间特定的捷径 |
| **几何编码器(FiLM)** | 发射器/接收器位置通过傅里叶编码并作为缩放+移位条件注入到每一层 | 模型知道硬件的*位置*, 因此不需要记忆布局 |
| **硬件归一化器** | 将任何芯片组的CSI重采样为标准56子载波格式，具有标准化幅度 | Intel 5300和ESP32数据对模型看起来相同 |
| **虚拟域增强** | 生成具有随机房间比例、墙壁反射、散射体和噪声分布的合成环境 | 即使只有2-3个房间的数据，训练也能看到1000s个房间 |
| **快速适应(TTT)** | 对比测试时训练，使用LoRA权重生成从几个未标记帧 | 零样本部署——模型在到达时自调优 |
| **跨域评估器** | 在所有训练环境中进行留一评估，具有每个环境的PCK/OKS指标 | 证明泛化，而不仅仅是记忆 |

**架构**

```
CSI Frame [any chipset]
    │
    ▼
HardwareNormalizer ──→ canonical 56 subcarriers, N(0,1) amplitude
    │
    ▼
CSI Encoder (existing) ──→ latent features
    │
    ├──→ Pose Head ──→ 17-joint pose (environment-invariant)
    │
    ├──→ Gradient Reversal Layer ──→ Domain Classifier (adversarial)
    │         λ ramps 0→1 via cosine/exponential schedule
    │
    └──→ Geometry Encoder ──→ FiLM conditioning (scale + shift)
              Fourier positional encoding → DeepSets → per-layer modulation
```

**安全加固:**
- 有界校准缓冲区(最大10,000帧)防止内存耗尽
- `adapt()`返回`Result<_, AdaptError>`——对不良输入无panic
- 原子实例计数器确保跨线程的唯一权重初始化
- 所有增强参数的除零保护

完整架构详情见[`docs/adr/ADR-027-cross-environment-domain-generalization.md`](docs/adr/ADR-027-cross-environment-domain-generalization.md)。

</details>

<details>
<summary><strong>🔍 独立能力审计 (ADR-028)</strong> — 1,031测试, SHA-256证明, 自验证见证包</summary>

[3-agent并行审计](docs/adr/ADR-028-esp32-capability-audit.md)独立验证了本仓库中的每一个声明——ESP32硬件、信号处理、神经网络、训练管道、部署和安全性。结果:

```
Rust tests:     1,031 passed, 0 failed
Python proof:   VERDICT: PASS (SHA-256: 8c0680d7...)
Bundle verify:  7/7 checks PASS
```

**33行证明矩阵:** 31项能力验证为YES, 2项在审计时未测量(基准吞吐量, Kubernetes部署)。

**自己验证** (无需硬件):
```bash
# 运行所有测试
cd rust-port/wifi-densepose-rs && cargo test --workspace --no-default-features

# 运行确定性证明
python v1/data/proof/verify.py

# 生成+验证见证包
bash scripts/generate-witness-bundle.sh
cd dist/witness-bundle-ADR028-*/ && bash VERIFY.sh
```

| 文档 | 包含内容 |
|----------|-----------------|
| [ADR-028](docs/adr/ADR-028-esp32-capability-audit.md) | 完整审计: ESP32规格, 信号算法, NN架构, 训练阶段, 部署基础设施 |
| [见证日志](docs/WITNESS-LOG-028.md) | 11个可重现验证步骤 + 33行证明矩阵,每行带证据 |
| [`generate-witness-bundle.sh`](scripts/generate-witness-bundle.sh) | 创建包含测试日志、证明输出、固件哈希、crate版本、VERIFY.sh的自包含tar.gz |

</details>

<details>
<summary><strong>📡 多静态感知 (ADR-029/030/031 — 项目RuvSense + RuView)</strong> — 多个ESP32节点融合视角用于生产级姿态、跟踪和特殊感知</summary>

单个WiFi接收器可以跟踪人员，但有盲点——躯干后面的肢体不可见，深度不明确，两个在相似范围内的人会产生重叠信号。RuvSense通过将多个ESP32节点协调成一个**多静态网格**来解决这个问题，其中每个节点同时作为发射器和接收器，从N个设备创建N×(N-1)个测量链路。

**简单来说它做什么:**
- 4个ESP32-S3节点(总计$48)提供12个TX-RX测量链路，覆盖360度
- 每个节点在WiFi信道1/6/11之间跳转，将有效带宽从20→60 MHz增加三倍
- 相干性门控自动拒绝噪声帧——无需手动调优，稳定数天
- 20 Hz的双人跟踪，10分钟内零身份交换
- 房间本身成为持久模型——系统记忆、预测和解释

**三个ADR, 一个管道:**

| ADR | 代号 | 添加内容 |
|-----|----------|-------------|
| [ADR-029](docs/adr/ADR-029-ruvsense-multistatic-sensing-mode.md) | **RuvSense** | 信道跳转, TDM协议, 多节点融合, 相干性门控, 17关键点Kalman跟踪器 |
| [ADR-030](docs/adr/ADR-030-ruvsense-persistent-field-model.md) | **RuvSense Field** | 房间电磁特征结构(SVD), RF断层扫描, 纵向漂移检测, 意图预测, 手势识别, 对抗检测 |
| [ADR-031](docs/adr/ADR-031-ruview-sensing-first-rf-mode.md) | **RuView** | 带几何偏置的跨视角注意力, 视角多样性优化, 嵌入级融合 |

**架构**

```
4x ESP32-S3 nodes ($48)     TDM: each transmits in turn, all others receive
        │                    Channel hop: ch1→ch6→ch11 per dwell (50ms)
        ▼
Per-Node Signal Processing   Phase sanitize → Hampel → BVP → subcarrier select
        │                    (ADR-014, unchanged per viewpoint)
        ▼
Multi-Band Frame Fusion      3 channels × 56 subcarriers = 168 virtual subcarriers
        │                    Cross-channel phase alignment via NeumannSolver
        ▼
Multistatic Viewpoint Fusion  N nodes → attention-weighted fusion → single embedding
        │                    Geometric bias from node placement angles
        ▼
Coherence Gate               Accept / PredictOnly / Reject / Recalibrate
        │                    Prevents model drift, stable for days
        ▼
Persistent Field Model       SVD baseline → body = observation - environment
        │                    RF tomography, drift detection, intention signals
        ▼
Pose Tracker + DensePose     17-keypoint Kalman, re-ID via AETHER embeddings
                             Multi-person min-cut separation, zero ID swaps
```

**七个特殊感知层级 (ADR-030)**

| 层级 | 能力 | 检测内容 |
|------|-----------|-----------------|
| 1 | 场正常模式 | 通过SVD的房间电磁特征结构 |
| 2 | 粗略RF断层扫描 | 来自链路衰减的3D占用体积 |
| 3 | 意图引导信号 | 动作前200-500ms的预运动预测 |
| 4 | 纵向生物力学 | 数天/周内的个人运动变化 |
| 5 | 跨房间连续性 | 无摄像头跨房间身份保留 |
| 6 | 不可见交互 | 穿墙多用户手势控制 |
| 7 | 对抗检测 | 物理上不可能的信号识别 |

</details>
