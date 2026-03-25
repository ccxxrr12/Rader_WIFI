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
