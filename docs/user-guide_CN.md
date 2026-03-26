# WiFi DensePose 用户指南

WiFi DensePose 将通用WiFi信号转换为实时人体姿态估计、生命体征监测和存在检测。本指南将带您完成安装、首次运行、API使用、硬件设置和模型训练。

---

## 目录

- [WiFi DensePose 用户指南](#wifi-densepose-用户指南)
  - [目录](#目录)
  - [前置要求](#前置要求)
  - [安装](#安装)
    - [Docker(推荐)](#docker推荐)
    - [从源代码构建(Rust)](#从源代码构建rust)
    - [从crates.io安装(独立Crate)](#从cratesio安装独立crate)
    - [从源代码构建(Python)](#从源代码构建python)
    - [引导式安装程序](#引导式安装程序)
  - [快速开始](#快速开始)
    - [30秒演示(Docker)](#30秒演示docker)
    - [验证系统工作](#验证系统工作)
  - [数据源](#数据源)
    - [模拟模式(无需硬件)](#模拟模式无需硬件)
    - [Windows WiFi(仅RSSI)](#windows-wifi仅rssi)
    - [macOS WiFi(仅RSSI)](#macos-wifi仅rssi)
    - [Linux WiFi(仅RSSI)](#linux-wifi仅rssi)
    - [ESP32-S3(完整CSI)](#esp32-s3完整csi)
    - [ESP32多静态网格(高级)](#esp32多静态网格高级)
  - [REST API参考](#rest-api参考)
    - [示例：获取生命体征](#示例获取生命体征)
    - [示例：获取姿态](#示例获取姿态)
  - [WebSocket流式传输](#websocket流式传输)
    - [Python示例](#python示例)
    - [JavaScript示例](#javascript示例)
    - [curl（单帧）](#curl单帧)
  - [Web UI](#web-ui)
  - [生命体征检测](#生命体征检测)
  - [CLI参考](#cli参考)
    - [常见调用](#常见调用)
  - [观测站可视化](#观测站可视化)
  - [自适应分类器](#自适应分类器)
    - [信号平滑管道](#信号平滑管道)
    - [录制训练数据](#录制训练数据)
    - [训练模型](#训练模型)
    - [使用训练模型](#使用训练模型)
    - [自适应分类器API](#自适应分类器api)
  - [训练模型](#训练模型-1)
    - [步骤1：获取数据集](#步骤1获取数据集)
    - [步骤2：训练](#步骤2训练)
    - [步骤3：使用训练模型](#步骤3使用训练模型)
    - [跨环境适配（MERIDIAN）](#跨环境适配meridian)
    - [CRV信号线路协议](#crv信号线路协议)
  - [RVF模型容器](#rvf模型容器)
    - [导出](#导出)
    - [加载](#加载)
    - [内容](#内容)
    - [部署目标](#部署目标)
  - [硬件设置](#硬件设置)
    - [ESP32-S3网格](#esp32-s3网格)
    - [Intel 5300 / Atheros NIC](#intel-5300--atheros-nic)
  - [Docker Compose（多服务）](#docker-compose多服务)
  - [无硬件测试固件（QEMU）](#无硬件测试固件qemu)
    - [所需物品](#所需物品)
    - [`qemu-cli.sh` 命令](#qemu-clish-命令)
    - [第一次测试运行](#第一次测试运行)
    - [理解测试输出](#理解测试输出)
    - [一次测试多个节点（集群）](#一次测试多个节点集群)
    - [集群预设](#集群预设)
    - [编写自己的集群配置](#编写自己的集群配置)
    - [在QEMU中调试固件](#在qemu中调试固件)
    - [运行完整测试套件](#运行完整测试套件)
  - [故障排除](#故障排除)
    - [Docker：macOS上出现"no matching manifest for linux/arm64"](#dockermacos上出现no-matching-manifest-for-linuxarm64)
    - [Docker：localhost:3000上出现"Connection refused"](#dockerlocalhost3000上出现connection-refused)
    - [Docker：UI中没有WebSocket数据](#dockerui中没有websocket数据)
    - [ESP32："CSI not enabled in menuconfig"](#esp32csi-not-enabled-in-menuconfig)
    - [ESP32：没有数据到达](#esp32没有数据到达)
    - [构建：Rust编译错误](#构建rust编译错误)
    - [Windows：RSSI模式显示无数据](#windowsrssi模式显示无数据)
    - [生命体征显示0 BPM](#生命体征显示0-bpm)
    - [生命体征跳动不稳定](#生命体征跳动不稳定)
    - [观测站显示DEMO而不是LIVE](#观测站显示demo而不是live)
    - [QEMU："qemu-system-xtensa: command not found"](#qemuqemu-system-xtensa-command-not-found)
    - [QEMU：测试超时无输出](#qemu测试超时无输出)
    - [QEMU："esptool not found"](#qemuesptool-not-found)
    - [QEMU集群："Must be run as root"](#qemu集群must-be-run-as-root)
    - [QEMU集群："yaml module not found"](#qemu集群yaml-module-not-found)
  - [FAQ](#faq)
  - [结论](#结论)

---

## 前置要求

| 要求 | 最低 | 推荐 |
|-------------|---------|-------------|
| **操作系统** | Windows 10/11, macOS 10.15, Ubuntu 18.04 | 最新稳定版 |
| **内存** | 4 GB | 8 GB+ |
| **磁盘** | 2 GB可用空间 | 5 GB可用空间 |
| **Docker**(用于Docker路径) | Docker 20+ | Docker 24+ |
| **Rust**(用于源代码构建) | 1.70+ | 1.85+ |
| **Python**(用于传统v1) | 3.10+ | 3.13+ |

**实时感知的硬件(可选):**

| 选项 | 成本 | 能力 |
|--------|------|-------------|
| ESP32-S3网格(3-6块板) | ~$54 | 完整CSI:姿态、呼吸、心跳、存在 |
| Intel 5300 / Atheros AR9580 | $50-100 | 完整CSI,3x3 MIMO(仅Linux) |
| 任何WiFi笔记本 | $0 | 仅RSSI:粗略存在和运动检测 |

没有硬件?系统在**模拟模式**下运行,使用合成CSI数据。

---

## 安装

### Docker(推荐)

最快的路径。无需安装工具链。

```bash
docker pull ruvnet/wifi-densepose:latest
```

多架构镜像(amd64 + arm64)。适用于Intel/AMD和Apple Silicon Mac。包含Rust感知服务器、Three.js UI和所有信号处理。

**数据源选择:**使用`CSI_SOURCE`环境变量选择感知模式:

| 值 | 描述 |
|-------|-------------|
| `auto` | (默认)探测UDP 5005上的ESP32,回退到模拟 |
| `esp32` | 通过UDP从ESP32设备接收真实CSI帧 |
| `simulated` | 生成合成CSI帧(无需硬件) |
| `wifi` | 主机WiFi RSSI(容器内不可用) |

示例:`docker run -e CSI_SOURCE=esp32 -p 3000:3000 -p 5005:5005/udp ruvnet/wifi-densepose:latest`

### 从源代码构建(Rust)

```bash
git clone https://github.com/ruvnet/RuView.git
cd RuView/rust-port/wifi-densepose-rs

# 构建
cargo build --release

# 验证(运行1400+测试)
cargo test --workspace --no-default-features
```

编译后的二进制文件位于`target/release/sensing-server`。

### 从crates.io安装(独立Crate)

所有16个crate都已发布到crates.io v0.3.0。将独立crate添加到您自己的Rust项目中:

```bash
# 核心类型和特征
cargo add wifi-densepose-core

# 信号处理(包括RuvSense多静态感知)
cargo add wifi-densepose-signal

# 神经网络推理
cargo add wifi-densepose-nn

# 大规模伤亡评估工具
cargo add wifi-densepose-mat

# ESP32硬件 + TDM协议 + QUIC传输
cargo add wifi-densepose-hardware

# RuVector集成(添加--features crv以使用CRV信号线协议)
cargo add wifi-densepose-ruvector --features crv

# WebAssembly绑定
cargo add wifi-densepose-wasm

# WASM边缘运行时(轻量级,用于嵌入式/IoT)
cargo add wifi-densepose-wasm-edge
```

参见[CLAUDE.md](../CLAUDE.md#crate-publishing-order)中的完整crate列表和依赖顺序。

### 从源代码构建(Python)

```bash
git clone https://github.com/ruvnet/RuView.git
cd RuView

pip install -r requirements.txt
pip install -e .

# 或通过PyPI
pip install wifi-densepose
pip install wifi-densepose[gpu]   # GPU加速
pip install wifi-densepose[all]   # 所有可选依赖
```

### 引导式安装程序

一个交互式安装程序,检测您的硬件并推荐配置文件:

```bash
git clone https://github.com/ruvnet/RuView.git
cd RuView
./install.sh
```

可用配置:`verify`、`python`、`rust`、`browser`、`iot`、`docker`、`field`、`full`。

非交互式:
```bash
./install.sh --profile rust --yes
```

---

## 快速开始

### 30秒演示(Docker)

```bash
# 拉取并运行
docker run -p 3000:3000 -p 3001:3001 ruvnet/wifi-densepose:latest

# 在浏览器中打开UI
# http://localhost:3000
```

您将看到一个Three.js可视化,包含:
- 3D身体骨架(17个COCO关键点)
- 信号幅度热图
- 相位图
- 生命体征面板(呼吸 + 心跳)

### 验证系统工作

打开第二个终端并测试API:

```bash
# 健康检查
curl http://localhost:3000/health
# 预期: {"status":"ok","source":"simulated","clients":0}

# 最新感知帧
curl http://localhost:3000/api/v1/sensing/latest

# 生命体征
curl http://localhost:3000/api/v1/vital-signs

# 姿态估计(17个COCO关键点)
curl http://localhost:3000/api/v1/pose/current

# 服务器构建信息
curl http://localhost:3000/api/v1/info
```

所有端点都返回JSON。在模拟模式下,数据从确定性参考信号生成。

---

## 数据源

`--source`标志控制CSI数据的来源。

### 模拟模式(无需硬件)

Docker中的默认模式。生成合成CSI数据,运行完整管道。

```bash
# Docker
docker run -p 3000:3000 ruvnet/wifi-densepose:latest
# (--source auto是默认值;未检测到硬件时回退到模拟)

# 从源代码
./target/release/sensing-server --source simulate --http-port 3000 --ws-port 3001
```

### Windows WiFi(仅RSSI)

使用`netsh wlan`从附近接入点捕获RSSI。无需特殊硬件。支持存在检测、运动分类和粗略呼吸率估计。无姿态估计(需要CSI)。

```bash
# 从源代码(仅Windows)
./target/release/sensing-server --source wifi --http-port 3000 --ws-port 3001 --tick-ms 500

# Docker(Windows上需要--network host)
docker run --network host ruvnet/wifi-densepose:latest --source wifi --tick-ms 500
```

> **社区验证:**在Windows 10(10.0.26200)上测试,配备Intel Wi-Fi 6 AX201 160MHz、Python 3.14、StormFiber 5 GHz网络。所有7个教程步骤都通过,RSSI读数稳定在-48 dBm。参见[教程#36](https://github.com/ruvnet/RuView/issues/36)了解完整演练和测试结果。

**从RSSI获取生命体征:**感知服务器现在支持从RSSI方差模式估计呼吸率(需要受试者静止在AP附近)和带置信度评分的运动分类。基于RSSI的生命体征检测 fidelity低于ESP32 CSI——最适合存在检测和粗略运动分类。

### macOS WiFi(仅RSSI)

通过Swift辅助二进制文件使用CoreWLAN。macOS Sonoma 14.4+会编辑真实BSSID;适配器生成确定性合成MAC,因此多BSSID管道仍然工作。

```bash
# 编译Swift辅助程序(一次)
swiftc -O v1/src/sensing/mac_wifi.swift -o mac_wifi

# 本地运行
./target/release/sensing-server --source macos --http-port 3000 --ws-port 3001 --tick-ms 500
```

详细信息参见[ADR-025](adr/ADR-025-macos-corewlan-wifi-sensing.md)。

### Linux WiFi(仅RSSI)

使用`iw dev <iface> scan`捕获RSSI。主动扫描需要`CAP_NET_ADMIN`(root);使用`scan dump`获取缓存结果而无需root。

```bash
# 本地运行(主动扫描需要root)
sudo ./target/release/sensing-server --source linux --http-port 3000 --ws-port 3001 --tick-ms 500
```

### ESP32-S3(完整CSI)

20 Hz的真实信道状态信息,56-192个子载波。姿态估计、生命体征和穿墙感知所必需。

```bash
# 从源代码
./target/release/sensing-server --source esp32 --udp-port 5005 --http-port 3000 --ws-port 3001

# Docker(使用CSI_SOURCE环境变量)
docker run -p 3000:3000 -p 3001:3001 -p 5005:5005/udp -e CSI_SOURCE=esp32 ruvnet/wifi-densepose:latest
```

ESP32节点通过UDP向端口5005流式传输二进制CSI帧。参见[硬件设置](#esp32-s3网格)了解刷写说明。

### ESP32多静态网格(高级)

为了更高的穿墙跟踪精度,以**多静态网格**配置部署3-6个ESP32-S3节点。每个节点既充当发射器又充当接收器,通过环境创建多条感知路径。

```bash
# 启动聚合器，开启多静态模式
./target/release/sensing-server --source esp32 --udp-port 5005 --http-port 3000 --ws-port 3001
```

网格使用**时分复用(TDM)**协议，使节点轮流发送数据，避免自干扰。主要特性：

| 特性 | 描述 |
|---------|-------------|
| TDM协调 | 节点循环通过TX/RX时隙（可配置保护间隔） |
| 信道跳变 | 自动2.4/5 GHz频段循环，实现多频段融合 |
| QUIC传输 | 聚合器节点上的TLS 1.3加密流（ADR-032a） |
| 手动加密回退 | 受限ESP32-S3节点上的HMAC-SHA256信标认证 |
| 注意力加权融合 | 带几何多样性偏置的跨视角注意力 |

详情参见[ADR-029](adr/ADR-029-ruvsense-multistatic-sensing-mode.md)和[ADR-032](adr/ADR-032-multistatic-mesh-security-hardening.md)。

---

## REST API参考

基础URL: `http://localhost:3000`（Docker）或 `http://localhost:8080`（二进制默认）。

| 方法 | 端点 | 描述 | 示例响应 |
|--------|----------|-------------|-----------------|
| `GET` | `/health` | 服务器健康检查 | `{"status":"ok","source":"simulated","clients":0}` |
| `GET` | `/api/v1/sensing/latest` | 最新CSI感知帧（幅度、相位、运动） | 包含子载波数组的JSON |
| `GET` | `/api/v1/vital-signs` | 呼吸率 + 心率 + 置信度 | `{"breathing_bpm":16.2,"heart_bpm":72.1,"confidence":0.87}` |
| `GET` | `/api/v1/pose/current` | 17个COCO关键点（x, y, z, 置信度） | 17个关节位置的数组 |
| `GET` | `/api/v1/info` | 服务器版本、构建信息、运行时间 | JSON元数据 |
| `GET` | `/api/v1/bssid` | 多BSSID WiFi注册表 | 检测到的接入点列表 |
| `GET` | `/api/v1/model/layers` | 渐进式模型加载状态 | 层A/B/C加载状态 |
| `GET` | `/api/v1/model/sona/profiles` | SONA适配配置文件 | 环境配置文件列表 |
| `POST` | `/api/v1/model/sona/activate` | 为特定房间激活SONA配置文件 | `{"profile":"kitchen"}` |
| `GET` | `/api/v1/models` | 列出可用的RVF模型文件 | `{"models":[],"count":0}` |
| `GET` | `/api/v1/models/active` | 当前加载的模型（或null） | `{"model":null}` |
| `POST` | `/api/v1/models/load` | 通过ID加载模型 | `{"status":"loaded","model_id":"..."}` |
| `POST` | `/api/v1/models/unload` | 卸载活动模型 | `{"status":"unloaded"}` |
| `DELETE` | `/api/v1/models/:id` | 从磁盘删除模型文件 | `{"status":"deleted"}` |
| `GET` | `/api/v1/models/lora/profiles` | 列出LoRA适配器配置文件 | `{"profiles":[]}` |
| `POST` | `/api/v1/models/lora/activate` | 激活LoRA配置文件 | `{"status":"activated"}` |
| `GET` | `/api/v1/recording/list` | 列出CSI录制会话 | `{"recordings":[],"count":0}` |
| `POST` | `/api/v1/recording/start` | 开始将CSI帧录制到JSONL | `{"status":"recording","session_id":"..."}` |
| `POST` | `/api/v1/recording/stop` | 停止活动录制 | `{"status":"stopped","duration_secs":...}` |
| `DELETE` | `/api/v1/recording/:id` | 删除录制文件 | `{"status":"deleted"}` |
| `GET` | `/api/v1/train/status` | 训练运行状态 | `{"phase":"idle"}` |
| `POST` | `/api/v1/train/start` | 开始训练运行 | `{"status":"started"}` |
| `POST` | `/api/v1/train/stop` | 停止活动训练运行 | `{"status":"stopped"}` |
| `POST` | `/api/v1/adaptive/train` | 从录制训练自适应分类器 | `{"success":true,"accuracy":0.85}` |
| `GET` | `/api/v1/adaptive/status` | 自适应模型状态和准确性 | `{"loaded":true,"accuracy":0.85}` |
| `POST` | `/api/v1/adaptive/unload` | 卸载自适应模型 | `{"success":true}` |

### 示例：获取生命体征

```bash
curl -s http://localhost:3000/api/v1/vital-signs | python -m json.tool
```

```json
{
    "breathing_bpm": 16.2,
    "heart_bpm": 72.1,
    "breathing_confidence": 0.87,
    "heart_confidence": 0.63,
    "motion_level": 0.12,
    "timestamp_ms": 1709312400000
}
```

### 示例：获取姿态

```bash
curl -s http://localhost:3000/api/v1/pose/current | python -m json.tool
```

```json
{
    "persons": [
        {
            "id": 0,
            "keypoints": [
                {"name": "nose", "x": 0.52, "y": 0.31, "z": 0.0, "confidence": 0.91},
                {"name": "left_eye", "x": 0.54, "y": 0.29, "z": 0.0, "confidence": 0.88}
            ]
        }
    ],
    "frame_id": 1024,
    "timestamp_ms": 1709312400000
}
```

---

## WebSocket流式传输

实时感知数据可通过WebSocket获取。

**URL:** `ws://localhost:3000/ws/sensing`（与HTTP相同端口 - 推荐）或 `ws://localhost:3001/ws/sensing`（专用WS端口）。

> **注意:** `/ws/sensing` WebSocket端点在HTTP端口（3000）和专用WebSocket端口（3001/8765）上都可用。Web UI使用HTTP端口，因此只需暴露一个端口。专用WS端口保持可用以向后兼容。

### Python示例

```python
import asyncio
import websockets
import json

async def stream():
    uri = "ws://localhost:3001/ws/sensing"
    async with websockets.connect(uri) as ws:
        async for message in ws:
            data = json.loads(message)
            persons = data.get("persons", [])
            vitals = data.get("vital_signs", {})
            print(f"Persons: {len(persons)}, "
                  f"Breathing: {vitals.get('breathing_bpm', 'N/A')} BPM")

asyncio.run(stream())
```

### JavaScript示例

```javascript
const ws = new WebSocket("ws://localhost:3001/ws/sensing");

ws.onmessage = (event) => {
    const data = JSON.parse(event.data);
    console.log("Persons:", data.persons?.length ?? 0);
    console.log("Breathing:", data.vital_signs?.breathing_bpm, "BPM");
};

ws.onerror = (err) => console.error("WebSocket error:", err);
```

### curl（单帧）

```bash
# 需要wscat (npm install -g wscat)
wscat -c ws://localhost:3001/ws/sensing
```

---

## Web UI

内置的Three.js UI在 `http://localhost:3000/ui/`（Docker）或配置的HTTP端口提供。

**两种可视化模式：**

| 页面 | URL | 用途 |
|------|-----|---------|
| **仪表板** | `/ui/index.html` | 带有人体模型、信号热图、相位图、生命体征的标签式监控仪表板 |
| **观测站** | `/ui/observatory.html` | 沉浸式3D房间可视化，带有电影照明和线框人物 |

**仪表板面板：**

| 面板 | 描述 |
|-------|-------------|
| 3D人体视图 | 带17个COCO关键点的可旋转线框骨架 |
| 信号热图 | 按幅度彩色编码的56个子载波 |
| 相位图 | 随时间变化的每个子载波相位值 |
| 多普勒条 | 运动频段功率指示器 |
| 生命体征 | 实时呼吸率（BPM）和心率（BPM） |
| 仪表板 | 系统统计、吞吐量、连接的WebSocket客户端 |

两个UI都通过WebSocket实时更新，并自动检测同一源上的感知服务器。

---

## 生命体征检测

系统使用FFT峰值检测从CSI信号波动中提取呼吸率和心率。

| 信号 | 频率带 | 范围 | 方法 |
|------|---------------|-------|--------|
| 呼吸 | 0.1-0.5 Hz | 6-30 BPM | 带通滤波器 + FFT峰值 |
| 心率 | 0.8-2.0 Hz | 40-120 BPM | 带通滤波器 + FFT峰值 |

**要求：**
- 支持CSI的硬件（ESP32-S3或研究用NIC）以获得准确读数
- 受试者在接入点约3-5米范围内（使用多静态网格可达约8米）
- 相对静止的受试者（大动作会掩盖生命体征振荡）

**信号平滑：** 生命体征估计通过三阶段平滑管道（ADR-048）：异常值拒绝（每帧±8 BPM心率，±2 BPM呼吸率），21帧修剪均值，以及α=0.02的EMA。这产生稳定的读数，在5-10+秒内保持稳定，而不是每帧跳动。详情参见[自适应分类器](#自适应分类器)。

**模拟模式** 生成用于测试的合成生命体征数据。

---

## CLI参考

Rust感知服务器二进制文件接受以下标志：

| 标志 | 默认值 | 描述 |
|------|---------|-------------|
| `--source` | `auto` | 数据源: `auto`, `simulate`, `wifi`, `esp32` |
| `--http-port` | `8080` | REST API和UI的HTTP端口 |
| `--ws-port` | `8765` | WebSocket端口 |
| `--udp-port` | `5005` | ESP32 CSI帧的UDP端口 |
| `--ui-path` | (无) | UI静态文件目录路径 |
| `--tick-ms` | `50` | 模拟帧间隔（毫秒） |
| `--benchmark` | 关闭 | 运行生命体征基准测试（1000帧）并退出 |
| `--train` | 关闭 | 从数据集训练模型 |
| `--dataset` | (无) | 数据集目录路径（MM-Fi或Wi-Pose） |
| `--dataset-type` | `mmfi` | 数据集格式: `mmfi` 或 `wipose` |
| `--epochs` | `100` | 训练轮数 |
| `--export-rvf` | (无) | 导出RVF模型容器并退出 |
| `--save-rvf` | (无) | 在关闭时将模型状态保存到RVF |
| `--model` | (无) | 加载训练好的 `.rvf` 模型进行推理 |
| `--load-rvf` | (无) | 从RVF容器加载模型配置 |
| `--progressive` | 关闭 | 启用渐进式3层模型加载 |

### 常见调用

```bash
# 带UI的模拟模式（开发）
./target/release/sensing-server --source simulate --http-port 3000 --ws-port 3001 --ui-path ../../ui

# ESP32硬件模式
./target/release/sensing-server --source esp32 --udp-port 5005

# Windows WiFi RSSI
./target/release/sensing-server --source wifi --tick-ms 500

# 运行基准测试
./target/release/sensing-server --benchmark

# 训练并导出模型
./target/release/sensing-server --train --dataset data/ --epochs 100 --save-rvf model.rvf

# 加载训练模型并启用渐进式加载
./target/release/sensing-server --model model.rvf --progressive
```

---

## 观测站可视化

观测站是一个沉浸式Three.js可视化，将WiFi感知数据渲染为电影般的3D体验。它具有房间规模的道具、线框人物、WiFi信号动画和实时数据HUD。

**URL:** `http://localhost:3000/ui/observatory.html`

**特性：**

| 特性 | 描述 |
|---------|-------------|
| 房间场景 | 家具、墙壁、地板，带有发光材料和6点照明 |
| 线框人物 | 最多4个人体骨架，关节脉动与呼吸同步 |
| 信号场 | 体积WiFi波可视化 |
| 实时HUD | 心率、呼吸率、置信度、RSSI、运动水平 |
| 自动检测 | 当感知服务器运行时自动连接到实时ESP32数据 |
| 场景循环 | 6个预设场景，平滑过渡（演示模式） |

**键盘快捷键：**

| 按键 | 操作 |
|-----|--------|
| `1-6` | 切换场景 |
| `A` | 切换自动循环 |
| `P` | 暂停/恢复 |
| `S` | 打开设置 |
| `R` | 重置相机 |

**实时数据自动检测：** 当由感知服务器提供服务时，观测站会在同一源上探测 `/health` 并通过WebSocket自动连接。HUD徽章从 `DEMO` 切换到 `LIVE`。无需配置。

---

## 自适应分类器

自适应分类器（ADR-048）从标记的录制中学习环境特定的WiFi信号模式。它用训练的逻辑回归模型替换基于静态阈值的分类，该模型使用15个特征（7个服务器计算 + 8个子载波派生统计）。

### 信号平滑管道

所有CSI派生指标在到达UI之前都通过三阶段管道：

| 阶段 | 功能 | 关键参数 |
|-------|-------------|----------------|
| **自适应基线** | 学习安静房间的噪声底，减去漂移 | α=0.003, 50帧预热 |
| **EMA + 中值滤波器** | 平滑运动评分和生命体征 | 运动α=0.15; 生命体征: 21帧修剪均值, α=0.02 |
| **滞后去抖** | 防止快速状态闪烁 | 状态转换需要4帧（~0.4s） |

生命体征使用额外的稳定化：

| 参数 | 值 | 效果 |
|-----------|-------|--------|
| 心率死区 | ±2 BPM | 防止微漂移 |
| 呼吸死区 | ±0.5 BPM | 防止微漂移 |
| 心率最大跳变 | 8 BPM/帧 | 拒绝噪声尖峰 |
| 呼吸最大跳变 | 2 BPM/帧 | 拒绝噪声尖峰 |

### 录制训练数据

在执行不同活动时录制标记的CSI会话。每次录制以~10-25 FPS捕获完整的感知帧（特征 + 原始子载波幅度）。

```bash
# 1. 录制空房间（离开房间30秒）
curl -X POST http://localhost:3000/api/v1/recording/start \
  -H "Content-Type: application/json" -d '{"id":"train_empty_room"}'
# ... 等待30秒 ...
curl -X POST http://localhost:3000/api/v1/recording/stop

# 2. 录制静坐（在ESP32附近静坐30秒）
curl -X POST http://localhost:3000/api/v1/recording/start \
  -H "Content-Type: application/json" -d '{"id":"train_sitting_still"}'
# ... 等待30秒 ...
curl -X POST http://localhost:3000/api/v1/recording/stop

# 3. 录制行走（在房间里行走30秒）
curl -X POST http://localhost:3000/api/v1/recording/start \
  -H "Content-Type: application/json" -d '{"id":"train_walking"}'
# ... 等待30秒 ...
curl -X POST http://localhost:3000/api/v1/recording/stop

# 4. 录制活动运动（开合跳、挥手30秒）
curl -X POST http://localhost:3000/api/v1/recording/start \
  -H "Content-Type: application/json" -d '{"id":"train_active"}'
# ... 等待30秒 ...
curl -X POST http://localhost:3000/api/v1/recording/stop
```

录制保存为 `data/recordings/` 中的JSONL文件。文件名必须以 `train_` 开头并包含类关键字：

| 文件名模式 | 类 |
|-----------------|-------|
| `*empty*` 或 `*absent*` | absent |
| `*still*` 或 `*sitting*` | present_still |
| `*walking*` 或 `*moving*` | present_moving |
| `*active*` 或 `*exercise*` | active |

### 训练模型

从标记的录制中训练自适应分类器：

```bash
curl -X POST http://localhost:3000/api/v1/adaptive/train
```

服务器使用小批量SGD（200轮）在15个特征上训练多类逻辑回归。典型录制集的训练在1秒内完成。训练好的模型保存到 `data/adaptive_model.json`，并在服务器重启时自动加载。

**检查模型状态：**

```bash
curl http://localhost:3000/api/v1/adaptive/status
```

**卸载模型（恢复到基于阈值的分类）：**

```bash
curl -X POST http://localhost:3000/api/v1/adaptive/unload
```

### 使用训练模型

训练后，自适应模型自动运行：

1. 每个CSI帧使用学习的权重而不是静态阈值进行分类
2. 模型置信度与平滑的阈值置信度混合（70/30分割）
3. 模型在服务器重启之间保持（从 `data/adaptive_model.json` 加载）

**提高准确性的提示：**

- 录制时使用明显不同的活动（实际上离开房间进行"空"录制）
- 每个活动录制30-60秒（更多数据 = 更好的模型）
- 如果移动ESP32或重新布置房间，重新录制并重新训练
- 模型是环境特定的 - 当物理设置改变时重新训练

### 自适应分类器API

| 方法 | 端点 | 描述 |
|--------|----------|-------------|
| `POST` | `/api/v1/adaptive/train` | 从 `train_*` 录制中训练 |
| `GET` | `/api/v1/adaptive/status` | 模型状态、准确性、类统计 |
| `POST` | `/api/v1/adaptive/unload` | 卸载模型，恢复到阈值 |
| `POST` | `/api/v1/recording/start` | 开始录制CSI帧 |
| `POST` | `/api/v1/recording/stop` | 停止录制 |
| `GET` | `/api/v1/recording/list` | 列出录制 |

---

## 训练模型

训练管道以纯Rust实现（7,832行，零外部ML依赖）。

### 步骤1：获取数据集

系统支持两个公共WiFi CSI数据集：

| 数据集 | 来源 | 格式 | 受试者 | 环境 | 下载 |
|---------|--------|--------|----------|-------------|----------|
| [MM-Fi](https://ntu-aiot-lab.github.io/mm-fi) | NeurIPS 2023 | `.npy` | 40 | 4个房间 | [GitHub仓库](https://github.com/ybhbingo/MMFi_dataset)（内部包含Google Drive / 百度链接） |
| [Wi-Pose](https://github.com/NjtechCVLab/Wi-PoseDataset) | Entropy 2023 | `.mat` | 12 | 1个房间 | [GitHub仓库](https://github.com/NjtechCVLab/Wi-PoseDataset)（内部包含Google Drive / 百度链接） |

下载数据集文件并将其放在 `data/` 目录中。

### 步骤2：训练

```bash
# 从源代码
./target/release/sensing-server --train --dataset data/ --dataset-type mmfi --epochs 100 --save-rvf model.rvf

# 通过Docker（挂载数据目录）
# 注意：训练模式需要覆盖默认入口点
docker run --rm \
  -v $(pwd)/data:/data \
  -v $(pwd)/output:/output \
  --entrypoint /app/sensing-server \
  ruvnet/wifi-densepose:latest \
  --train --dataset /data --epochs 100 --export-rvf /output/model.rvf
```

管道运行10个阶段：
1. 数据集加载（MM-Fi `.npy` 或 Wi-Pose `.mat`）
2. 硬件归一化（Intel 5300 / Atheros / ESP32 -> 标准56个子载波）
3. 子载波重采样（114->56或30->56，通过Catmull-Rom插值）
4. 图变换器构建（17个COCO关键点，16个骨骼边缘）
5. 交叉注意力训练（CSI特征 -> 身体姿态）
6. **域对抗训练**（MERIDIAN：梯度反转 + 虚拟域增强）
7. 复合损失优化（MSE + CE + UV + 时间 + 骨骼 + 对称性）
8. SONA适配（微型LoRA + EWC++）
9. 稀疏推理优化（热/冷神经元分区）
10. RVF模型打包

### 步骤3：使用训练模型

```bash
./target/release/sensing-server --model model.rvf --progressive --source esp32
```

渐进式加载实现即时启动（A层在<5ms内加载，具有基本推理），后台加载完整模型。

### 跨环境适配（MERIDIAN）

在一个房间训练的模型在新房间通常会失去40-70%的准确性，因为WiFi多径模式不同。MERIDIAN系统（ADR-027）通过10秒自动校准解决了这个问题：

1. **部署** 训练好的模型到新房间
2. **收集** ~200个未标记的CSI帧（20 Hz下10秒）
3. 系统通过对比测试时训练自动生成环境特定的LoRA权重
4. 无需标签，无需重新训练，无需用户干预

MERIDIAN组件（全部纯Rust，+12K参数）：

| 组件 | 功能 |
|-----------|-------------|
| 硬件归一化器 | 将任何WiFi芯片组重采样到标准56个子载波 |
| 域分解器 | 分离姿态相关和房间特定特征 |
| 几何编码器 | 编码AP位置（带DeepSets的FiLM条件） |
| 虚拟增强器 | 生成合成环境以进行稳健训练 |
| 快速适配 | 通过对比TTT进行10秒无监督校准 |

详情参见[ADR-027](adr/ADR-027-cross-environment-domain-generalization.md)。

### CRV信号线路协议

CRV（坐标远程查看）信号线路协议（ADR-033）将6阶段认知感知方法映射到WiFi CSI处理。这实现了结构化异常分类和多人消歧。

| 阶段 | CRV术语 | WiFi映射 |
|-------|----------|-------------|
| I | 格式塔 | 去趋势自相关 → 周期性/混沌/瞬态分类 |
| II | 感官 | 6模态CSI特征编码（纹理、温度、亮度等） |
| III | 拓扑 | 带链路质量权重的AP网格拓扑图 |
| IV | 一致性 | 相位相量一致性门（接受/仅预测/拒绝/重新校准） |
| V | 询问 | 具有目标子载波选择的人物特定信号提取 |
| VI | 分区 | 带跨房间收敛评分的多人分区 |

```bash
# 在Cargo.toml中启用CRV
cargo add wifi-densepose-ruvector --features crv
```

详情参见[ADR-033](adr/ADR-033-crv-signal-line-sensing-integration.md)。

---

## RVF模型容器

RuVector格式（RVF）将训练好的模型打包到单个自包含的二进制文件中。

### 导出

```bash
./target/release/sensing-server --export-rvf model.rvf
```

### 加载

```bash
./target/release/sensing-server --model model.rvf --progressive
```

### 内容

RVF文件包含：模型权重、HNSW向量索引、量化码本、SONA适配配置文件、Ed25519训练证明和生命体征滤波器参数。

### 部署目标

| 目标 | 量化 | 大小 | 加载时间 |
|--------|-------------|------|-----------|
| ESP32 / IoT | int4 | ~0.7 MB | <5ms |
| 移动 / WASM | int8 | ~6-10 MB | ~200-500ms |
| 现场（WiFi-Mat） | fp16 | ~62 MB | ~2s |
| 服务器 / 云 | f32 | ~50+ MB | ~3s |

---

## 硬件设置

### ESP32-S3网格

3-6节点ESP32-S3网格以20 Hz提供完整CSI。总成本：3节点设置约$54。

**所需物品：**
- 3-6个ESP32-S3开发板（每个约$8）
- WiFi路由器（CSI源）
- 运行感知服务器的计算机（聚合器）

**刷写固件：**

预构建二进制文件可在[Releases](https://github.com/ruvnet/RuView/releases)获取：

| 版本 | 包含内容 | 标签 |
|---------|-----------------|-----|
| [v0.5.0](https://github.com/ruvnet/RuView/releases/tag/v0.5.0-esp32) | **稳定（推荐）** — 毫米波传感器融合（MR60BHA2/LD2410自动检测），48字节融合生命体征，所有v0.4.3.1修复 | `v0.5.0-esp32` |
| [v0.4.3.1](https://github.com/ruvnet/RuView/releases/tag/v0.4.3.1-esp32) | 跌倒检测修复（[#263](https://github.com/ruvnet/RuView/issues/263)），4MB闪存（[#265](https://github.com/ruvnet/RuView/issues/265)），看门狗修复（[#266](https://github.com/ruvnet/RuView/issues/266)） | `v0.4.3.1-esp32` |
| [v0.4.1](https://github.com/ruvnet/RuView/releases/tag/v0.4.1-esp32) | CSI构建修复，编译保护，AMOLED显示，边缘智能（[ADR-057](../docs/adr/ADR-057-firmware-csi-build-guard.md)） | `v0.4.1-esp32` |
| [v0.3.0-alpha](https://github.com/ruvnet/RuView/releases/tag/v0.3.0-alpha-esp32) | Alpha — 添加设备端边缘智能（ADR-039） | `v0.3.0-alpha-esp32` |
| [v0.2.0](https://github.com/ruvnet/RuView/releases/tag/v0.2.0-esp32) | 原始CSI流，TDM，信道跳变，QUIC网格 | `v0.2.0-esp32` |

> **重要：** 始终使用**v0.4.3.1或更高版本**。早期版本存在虚假跌倒检测警报（v0.4.2及以下）和构建配置中禁用CSI（v0.4.1之前）。

```bash
# 刷写8MB闪存的ESP32-S3（大多数板）
python -m esptool --chip esp32s3 --port COM7 --baud 460800 \
  write-flash --flash-mode dio --flash-size 8MB --flash-freq 80m \
  0x0 bootloader.bin 0x8000 partition-table.bin \
  0xf000 ota_data_initial.bin 0x20000 esp32-csi-node.bin
```

**4MB闪存板**（例如ESP32-S3 SuperMini 4MB）：从[v0.4.3版本](https://github.com/ruvnet/RuView/releases/tag/v0.4.3-esp32)下载4MB二进制文件并使用`--flash-size 4MB`：

```bash
python -m esptool --chip esp32s3 --port COM7 --baud 460800 \
  write-flash --flash-mode dio --flash-size 4MB --flash-freq 80m \
  0x0 bootloader.bin 0x8000 partition-table-4mb.bin \
  0xF000 ota_data_initial.bin 0x20000 esp32-csi-node-4mb.bin
```

**配置：**

```bash
python firmware/esp32-csi-node/provision.py --port COM7 \
  --ssid "YourWiFi" --password "YourPassword" --target-ip 192.168.1.20
```

将`192.168.1.20`替换为运行感知服务器的机器的IP。

**网格密钥配置（安全模式）：**

对于具有认证信标的多静态网格部署（ADR-032），配置共享网格密钥：

```bash
python firmware/esp32-csi-node/provision.py --port COM7 \
  --ssid "YourWiFi" --password "YourPassword" --target-ip 192.168.1.20 \
  --mesh-key "$(openssl rand -hex 32)"
```

网格中的所有节点必须共享相同的256位网格密钥才能进行HMAC-SHA256信标认证。密钥存储在ESP32 NVS闪存中，固件擦除时会清零。

**TDM时隙分配：**

多静态网格中的每个节点需要唯一的TDM时隙ID（从0开始）：

```bash
# 节点0（时隙0）— 第一个发射器
python firmware/esp32-csi-node/provision.py --port COM7 --tdm-slot 0 --tdm-total 3

# 节点1（时隙1）
python firmware/esp32-csi-node/provision.py --port COM8 --tdm-slot 1 --tdm-total 3

# 节点2（时隙2）
python firmware/esp32-csi-node/provision.py --port COM9 --tdm-slot 2 --tdm-total 3
```

**边缘智能（v0.3.0-alpha，[ADR-039](../docs/adr/ADR-039-esp32-edge-intelligence.md)）：**

v0.3.0-alpha固件添加了直接在ESP32-S3上运行的设备端信号处理 — 基本存在和生命体征不需要主机PC。边缘处理默认禁用，以保持完全向后兼容。

| 层级 | 功能 | 额外RAM |
|------|-------------|-----------|
| **0** | 禁用（默认）— 将原始CSI流式传输到聚合器 | 0 KB |
| **1** | 相位解包裹，运行统计，前K个子载波选择，增量压缩 | ~30 KB |
| **2** | 包含层级1的所有内容，加上存在检测，呼吸/心率，运动评分，跌倒检测 | ~33 KB |

通过NVS启用（无需重新刷写）：

```bash
# 在已刷写的节点上启用层级2（完整生命体征）
python firmware/esp32-csi-node/provision.py --port COM7 \
  --ssid "YourWiFi" --password "YourPassword" --target-ip 192.168.1.20 \
  --edge-tier 2
```

边缘处理的关键NVS设置：

| NVS键 | 默认值 | 控制内容 |
|---------|---------|-----------------|
| `edge_tier` | 0 | 处理层级（0=关闭，1=统计，2=生命体征） |
| `pres_thresh` | 50 | 存在检测的灵敏度（较低=更敏感） |
| `fall_thresh` | 15000 | 跌倒检测阈值（毫单位）（15000 = 15.0 rad/s²）。正常行走为2-5，实际跌倒为20+。提高以减少误报。 |
| `vital_win` | 300 | 呼吸/HR提取保留的相位历史帧数 |
| `vital_int` | 1000 | 发送生命体征数据包的频率，以毫秒为单位 |
| `subk_count` | 32 | 保留的最佳子载波数量（共56个） |

当层级2激活时，节点以1 Hz（可配置）发送32字节的生命体征数据包，包含存在状态、运动评分、呼吸BPM、心率BPM、置信度值、跌倒标志和占用估计。数据包使用魔术 `0xC5110002`，并发送到与原始CSI帧相同的聚合器IP和端口。

二进制大小：990 KB（8MB闪存，52%可用）或773 KB（4MB闪存）。v0.5.0添加了毫米波传感器融合（~12 KB更大）。

> **Alpha通知**：生命体征估计使用启发式BPM提取。在受控环境中对静止受试者的准确性最佳。不用于医疗用途。

**启动聚合器：**

```bash
# 从源代码
./target/release/sensing-server --source esp32 --udp-port 5005 --http-port 3000 --ws-port 3001

# Docker（使用CSI_SOURCE环境变量）
docker run -p 3000:3000 -p 3001:3001 -p 5005:5005/udp -e CSI_SOURCE=esp32 ruvnet/wifi-densepose:latest
```

详情参见[ADR-018](../docs/adr/ADR-018-esp32-dev-implementation.md)、[ADR-029](../docs/adr/ADR-029-ruvsense-multistatic-sensing-mode.md)和[教程#34](https://github.com/ruvnet/RuView/issues/34)。

### Intel 5300 / Atheros NIC

这些研究用NIC在Linux上通过固件/驱动修改提供完整CSI。

| NIC | 驱动 | 平台 | 设置 |
|-----|--------|----------|-------|
| Intel 5300 | `iwl-csi` | Linux | 自定义固件，约$15二手 |
| Atheros AR9580 | `ath9k` 补丁 | Linux | 内核补丁，约$20二手 |

这些是高级设置。有关安装，请参阅相应的驱动文档。

---

## Docker Compose（多服务）

对于同时包含Rust和Python服务的生产部署：

```bash
cd docker
docker compose up
```

这会启动：
- Rust感知服务器，端口3000（HTTP）、3001（WS）、5005（UDP）
- Python遗留服务器，端口8080（HTTP）、8765（WS）

---

## 无硬件测试固件（QEMU）

您可以在没有任何物理硬件的情况下在计算机上测试ESP32-S3固件。该项目使用**QEMU** — 一个模拟器，假装是ESP32-S3芯片，在PC上的虚拟机中运行真实固件代码。

这在以下情况下很有用：
- 您还没有ESP32-S3板
- 您想在刷写到真实硬件之前测试固件更改
- 您在CI/CD中运行自动化测试
- 您想模拟多个ESP32节点相互通信

### 所需物品

**必需：**
- Python 3.8+（您可能已经有）
- 支持ESP32-S3的QEMU（Espressif的分支）

**安装QEMU（一次性设置）：**

```bash
# 最简单：使用自动安装程序（安装QEMU + Python工具）
bash scripts/install-qemu.sh

# 或检查已安装的内容：
bash scripts/install-qemu.sh --check
```

安装程序检测您的操作系统（Ubuntu、Fedora、macOS等），安装构建依赖，克隆Espressif的QEMU分支，构建它，并将其添加到您的PATH。它还安装Python工具（`esptool`、`pyyaml`、`esp-idf-nvs-partition-gen`）。

<details>
<summary>手动安装（如果您喜欢）</summary>

```bash
# 从源代码构建
git clone https://github.com/espressif/qemu.git
cd qemu
./configure --target-list=xtensa-softmmu --enable-slirp
make -j$(nproc)
export QEMU_PATH=$(pwd)/build/qemu-system-xtensa

# 安装Python工具
pip install esptool pyyaml esp-idf-nvs-partition-gen
```

</details>

**用于多节点测试（可选）：**

```bash
# 仅限Linux — 需要虚拟网络桥接
sudo apt install socat bridge-utils iproute2
```

### `qemu-cli.sh` 命令

所有QEMU测试都可通过单个命令使用：

```bash
bash scripts/qemu-cli.sh <command>
```

| 命令 | 功能 |
|---------|-------------|
| `install` | 安装QEMU（运行上面的安装程序） |
| `test` | 运行单节点固件测试 |
| `swarm --preset smoke` | 快速2节点集群测试 |
| `swarm --preset standard` | 标准3节点测试 |
| `mesh 3` | 多节点网格测试 |
| `chaos` | 故障注入弹性测试 |
| `fuzz --duration 60` | 运行模糊测试 |
| `status` | 显示已安装和就绪的内容 |
| `help` | 显示所有命令 |

### 第一次测试运行

测试固件的最简单方法：

```bash
# 使用CLI：
bash scripts/qemu-cli.sh test

# 或直接：
bash scripts/qemu-esp32s3-test.sh
```

**幕后发生的事情：**
1. 固件以"模拟CSI"模式编译 — 不是读取真实WiFi信号，而是生成模拟真实人行走、跌倒或呼吸的合成测试数据
2. 编译的固件加载到QEMU中，QEMU像真实的ESP32-S3一样启动它
3. 模拟器的串行输出（您在USB电缆上看到的内容）被捕获
4. 验证脚本检查输出是否符合预期行为和错误

如果您已经构建了固件并想跳过重新构建：

```bash
SKIP_BUILD=1 bash scripts/qemu-esp32s3-test.sh
```

给它更多时间（在较慢的机器上有用）：

```bash
QEMU_TIMEOUT=120 bash scripts/qemu-esp32s3-test.sh
```

### 理解测试输出

测试对固件输出运行16项检查。成功运行如下所示：

```
=== QEMU ESP32-S3 Firmware Test (ADR-061) ===

[PASS] Boot: Firmware booted successfully
[PASS] NVS config: Configuration loaded from flash
[PASS] Mock CSI: Synthetic WiFi data generator started
[PASS] Edge processing: Signal analysis pipeline running
[PASS] Frame serialization: Data packets formatted correctly
[PASS] No crashes: No error conditions detected
...

16/16 checks passed
=== Test Complete (exit code: 0) ===
```

**退出代码解释：**

| 代码 | 含义 | 操作 |
|------|---------|-----------|
| 0 | **通过** — 一切正常 | 无需操作，很好！ |
| 1 | **警告** —  minor问题 | 查看输出；通常可以安全继续 |
| 2 | **失败** — 出现问题 | 检查`[FAIL]`行以了解出错原因 |
| 3 | **致命** — 甚至无法启动 | 通常是缺少工具或构建失败；检查错误消息 |

### 一次测试多个节点（集群）

实际部署使用3-8个ESP32节点。**集群配置器**允许您在计算机上模拟多个节点，每个节点具有不同的角色：

- **传感器节点** — 生成WiFi信号数据（如放置在房间周围的ESP32）
- **协调器节点** — 收集所有传感器的数据并运行分析
- **网关节点** — 将数据桥接到您的计算机

```bash
# 快速2节点冒烟测试（15秒）
python3 scripts/qemu_swarm.py --preset smoke

# 标准3节点测试：2个传感器 + 1个协调器（60秒）
python3 scripts/qemu_swarm.py --preset standard

# 查看可用内容
python3 scripts/qemu_swarm.py --list-presets

# 预览将运行的内容（不实际运行）
python3 scripts/qemu_swarm.py --preset standard --dry-run
```

**注意：** 使用虚拟网络桥接的多节点测试需要Linux和`sudo`。在其他系统上，节点使用更简单的网络模式，其中每个节点可以到达协调器但不能相互通信。

### 集群预设

| 预设 | 节点数 | 持续时间 | 最适合 |
|--------|-------|----------|----------|
| `smoke` | 2 | 15s | 快速检查功能 |
| `standard` | 3 | 60s | 正常开发测试 |
| `ci_matrix` | 3 | 30s | CI/CD管道 |
| `large_mesh` | 6 | 90s | 大规模测试 |
| `line_relay` | 4 | 60s | 多跳中继测试 |
| `ring_fault` | 4 | 75s | 容错测试 |
| `heterogeneous` | 5 | 90s | 混合场景测试 |

### 编写自己的集群配置

创建描述测试场景的YAML文件：

```yaml
# my_test.yaml
swarm:
  name: my-custom-test
  duration_s: 45
  topology: star       # star, mesh, line, or ring
  aggregator_port: 5005

nodes:
  - role: coordinator
    node_id: 0
    scenario: 0        # 0=empty room (baseline)
    channel: 6
    edge_tier: 2

  - role: sensor
    node_id: 1
    scenario: 2        # 2=walking person
    channel: 6
    tdm_slot: 1

  - role: sensor
    node_id: 2
    scenario: 3        # 3=fall event
    channel: 6
    tdm_slot: 2

assertions:
  - all_nodes_boot           # Did every node start up?
  - no_crashes               # Any error/panic?
  - all_nodes_produce_frames # Is each sensor generating data?
  - fall_detected_by_node_2  # Did node 2 detect the fall?
```

**可用场景**（生成什么类型的假WiFi数据）：

| # | 场景 | 描述 |
|---|----------|-------------|
| 0 | 空房间 | 仅噪声基线 |
| 1 | 静态人 | 有人站着不动 |
| 2 | 行走 | 有人穿过房间 |
| 3 | 跌倒 | 有人跌倒 |
| 4 | 多人 | 房间里有两个人 |
| 5 | 信道扫描 | 循环通过WiFi信道 |
| 6 | MAC过滤 | 测试设备过滤 |
| 7 | 环形溢出 | 数据突发压力测试 |
| 8 | RSSI扫描 | 信号强度从弱到强 |
| 9 | 零长度 | 边缘情况：空数据包 |

**拓扑选项：**

| 拓扑 | 形状 | 使用时机 |
|----------|-------|-------------|
| `star` | 所有传感器连接到一个协调器 | 最常见设置 |
| `mesh` | 每个节点可以与其他每个节点通信 | 测试完全连接的网络 |
| `line` | 节点成链（A → B → C → D） | 测试中继/转发 |
| `ring` | 链的两端连接 | 测试环形路由 |

运行自定义配置：

```bash
python3 scripts/qemu_swarm.py --config my_test.yaml
```

### 在QEMU中调试固件

如果出现问题，您可以将调试器附加到模拟的ESP32：

```bash
# 终端1：启动支持调试的QEMU（启动时暂停）
qemu-system-xtensa -machine esp32s3 -nographic \
  -drive file=firmware/esp32-csi-node/build/qemu_flash.bin,if=mtd,format=raw \
  -s -S

# 终端2：连接调试器
xtensa-esp-elf-gdb firmware/esp32-csi-node/build/esp32-csi-node.elf \
  -ex "target remote :1234" \
  -ex "break app_main" \
  -ex "continue"
```

或使用VS Code：打开项目，按**F5**，然后选择**"QEMU ESP32-S3 Debug"**。

### 运行完整测试套件

在提交拉取请求前进行全面验证：

```bash
# 1. 单节点测试（2分钟）
bash scripts/qemu-esp32s3-test.sh

# 2. 多节点集群测试（1分钟）
python3 scripts/qemu_swarm.py --preset standard

# 3. 模糊测试 — 发现边缘情况崩溃（1-5分钟）
cd firmware/esp32-csi-node/test
make all CC=clang
make run_serialize FUZZ_DURATION=60
make run_edge FUZZ_DURATION=60
make run_nvs FUZZ_DURATION=60

# 4. NVS配置矩阵 — 测试14种配置组合
python3 scripts/generate_nvs_matrix.py --output-dir build/nvs_matrix

# 5. 混沌测试 — 注入故障以测试弹性（2分钟）
bash scripts/qemu-chaos-test.sh
```

当您推送对`firmware/`的更改时，所有这些也会在CI中自动运行。

---

## 故障排除

### Docker：macOS上出现"no matching manifest for linux/arm64"

`latest`标签支持amd64和arm64。拉取最新镜像：

```bash
docker pull ruvnet/wifi-densepose:latest
```

如果您仍然看到此错误，您的本地Docker可能有过时的缓存清单。尝试：

```bash
docker pull --platform linux/arm64 ruvnet/wifi-densepose:latest
```

### Docker：localhost:3000上出现"Connection refused"

确保您正确映射端口：

```bash
docker run -p 3000:3000 -p 3001:3001 ruvnet/wifi-densepose:latest
```

`-p 3000:3000`将主机端口3000映射到容器端口3000。

### Docker：UI中没有WebSocket数据

添加WebSocket端口映射：

```bash
docker run -p 3000:3000 -p 3001:3001 ruvnet/wifi-densepose:latest
```

### ESP32："CSI not enabled in menuconfig"

v0.4.1之前的固件版本在构建配置中禁用了`CONFIG_ESP_WIFI_CSI_ENABLED`。升级到[v0.4.1](https://github.com/ruvnet/RuView/releases/tag/v0.4.1-esp32)或更高版本。如果从源代码构建，确保`sdkconfig.defaults`存在（不仅仅是`sdkconfig.defaults.template`）。详情参见[ADR-057](../docs/adr/ADR-057-firmware-csi-build-guard.md)。

### ESP32：没有数据到达

1. 验证固件是v0.4.1+（旧版本禁用了CSI — 见上文）
2. 验证ESP32连接到同一WiFi网络
3. 检查目标IP是否与感知服务器机器匹配：`python firmware/esp32-csi-node/provision.py --port COM7 --target-ip <YOUR_IP>`
4. 验证UDP端口5005未被防火墙阻止
5. 测试：`nc -lu 5005`（Linux）或类似的UDP监听器

### 构建：Rust编译错误

确保安装了Rust 1.75+（推荐1.85+）：
```bash
rustup update stable
rustc --version
```

### Windows：RSSI模式显示无数据

以管理员身份运行终端（`netsh wlan`访问需要）。在Windows 10和11上使用Intel AX201和Intel BE201适配器验证工作正常。

### 生命体征显示0 BPM

- 生命体征检测需要支持CSI的硬件（ESP32或研究用NIC）
- 仅RSSI模式（Windows WiFi）没有足够的分辨率用于生命体征
- 在模拟模式下，合成生命体征在几秒钟的预热后生成
- 使用真实ESP32数据时，生命体征需要约5秒稳定（平滑管道预热）

### 生命体征跳动不稳定

服务器应用三阶段平滑管道（ADR-048）。如果读数仍然不稳定：
- 确保受试者相对静止（大动作会掩盖生命体征振荡）
- 为您的特定环境训练自适应分类器：`curl -X POST http://localhost:3000/api/v1/adaptive/train`
- 检查信号质量：`curl http://localhost:3000/api/v1/sensing/latest` — 查找`signal_quality > 0.4`

### 观测站显示DEMO而不是LIVE

- 验证感知服务器正在运行：`curl http://localhost:3000/health`
- 通过服务器URL访问观测站：`http://localhost:3000/ui/observatory.html`（不是file:// URL）
- 用Ctrl+Shift+R硬刷新以清除缓存设置
- 自动检测在同一源上探测`/health` — 跨源将不起作用

### QEMU："qemu-system-xtensa: command not found"

ESP32-S3的QEMU必须从Espressif的分支构建 — 它不在标准包管理器中：

```bash
git clone https://github.com/espressif/qemu.git
cd qemu && ./configure --target-list=xtensa-softmmu && make -j$(nproc)
export QEMU_PATH=$(pwd)/build/qemu-system-xtensa
```

或指向现有构建：`QEMU_PATH=/path/to/qemu-system-xtensa bash scripts/qemu-esp32s3-test.sh`

### QEMU：测试超时无输出

模拟器比真实硬件慢。增加超时：

```bash
QEMU_TIMEOUT=120 bash scripts/qemu-esp32s3-test.sh
```

如果真的没有输出，固件构建可能失败。不使用`SKIP_BUILD`重新构建：

```bash
bash scripts/qemu-esp32s3-test.sh   # 不使用SKIP_BUILD
```

### QEMU："esptool not found"

用pip安装：`pip install esptool`

### QEMU集群："Must be run as root"

使用虚拟网络桥接的多节点集群测试在Linux上需要root。两个选项：

1. 用sudo运行：`sudo python3 scripts/qemu_swarm.py --preset standard`
2. 跳过桥接（节点使用更简单的网络）：工具在非root系统上自动回退，但节点不能相互通信（只能与聚合器通信）

### QEMU集群："yaml module not found"

安装PyYAML：`pip install pyyaml`

---

## FAQ

**问：我需要特殊硬件来尝试这个吗？**
答：不需要！系统在**模拟模式**下运行，使用合成CSI数据。这是Docker中的默认模式，让您无需任何硬件即可体验完整功能。

**问：WiFi DensePose的准确性如何？**
答：在理想条件下（ESP32网格，受试者在3米内），姿态估计达到78-85%的AP精度（与COCO数据集相比）。生命体征检测在静止受试者上达到±2 BPM的精度。

**问：它能穿透墙壁吗？**
答：是的，WiFi信号可以穿透大多数墙壁、门和家具。使用多静态ESP32网格时，穿墙跟踪精度显著提高。

**问：它使用多少带宽？**
答：ESP32节点以20 Hz发送约2 KB/帧，约320 Kbps。WebSocket流使用约100-200 Kbps。

**问：它安全吗？**
答：系统不捕获任何视频或音频，只处理WiFi信号的数学特征。所有数据处理都在本地进行，默认情况下不会发送到云端。多静态网格使用HMAC-SHA256信标认证（ADR-032）。

**问：我可以在商业产品中使用它吗？**
答：是的，WiFi DensePose是开源的（Apache 2.0许可证）。参见[CLAUDE.md](../CLAUDE.md#license)了解完整许可证详情。

**问：支持哪些WiFi芯片组？**
答：
- **完整CSI**：ESP32-S3（推荐）、Intel 5300、Atheros AR9580（仅Linux）
- **仅RSSI**：任何WiFi适配器（Windows、macOS、Linux）

**问：需要多少个ESP32节点？**
答：
- **最小**：1个节点（基本功能）
- **推荐**：3-4个节点（多静态视角，更好的穿墙性能）
- **高级**：6个节点（完整房间覆盖）

**问：训练模型需要多少数据？**
答：
- 自适应分类器：每个活动30-60秒的录制（约300-600帧）
- 完整姿态模型：MM-Fi或Wi-Pose数据集（约10,000+帧）

**问：系统延迟是多少？**
答：
- ESP32硬件模式：100-150 ms（从信号捕获到UI更新）
- 模拟模式：<50 ms
- 仅RSSI模式：200-500 ms（取决于扫描间隔）

**问：可以同时跟踪多少人？**
答：当前实现支持1-4人同时跟踪。性能与人数成线性比例。

**问：支持哪些操作系统？**
答：
- **服务器**：Windows 10/11、macOS 10.15+、Ubuntu 18.04+、任何支持Docker的系统
- **ESP32固件**：ESP32-S3（任何变体）
- **Web UI**：现代浏览器（Chrome、Firefox、Safari、Edge）

**问：如何贡献？**
答：参见[CONTRIBUTING.md](../CONTRIBUTING.md)。我们欢迎代码贡献、文档改进和bug报告。

**问：有商业支持吗？**
答：是的，RuVNet提供企业支持、定制开发和集成服务。请联系[support@ruvnet.io](mailto:support@ruvnet.io)了解详情。

---

## 结论

WiFi DensePose将普通WiFi信号转化为强大的感知工具，无需摄像头即可实现人体姿态估计、生命体征监测和存在检测。通过本指南，您应该能够：

1. 安装并运行系统（有或无硬件）
2. 配置数据源（模拟、WiFi RSSI或ESP32 CSI）
3. 使用REST API和WebSocket进行集成
4. 训练和部署自定义模型
5. 设置ESP32多静态网格以提高准确性
6. 排除常见问题

如需更多帮助，请查看[ADR文档](adr/)、[教程](https://github.com/ruvnet/RuView/issues?q=label%3Atutorial)或在[GitHub](https://github.com/ruvnet/RuView)上打开问题。

---

*WiFi DensePose — 看到不可见的。*
