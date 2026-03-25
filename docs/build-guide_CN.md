# WiFi-DensePose 构建和运行指南

涵盖构建、运行和部署系统的所有方式——从零硬件验证到带有3D可视化的完整ESP32网格。

---

## 目录

1. [快速开始(仅验证——无需硬件)](#1-快速开始仅验证----无需硬件)
2. [Python管道(v1/)](#2-python管道v1)
3. [Rust管道(v2)](#3-rust管道v2)
4. [Three.js可视化](#4-threejs可视化)
5. [Docker部署](#5-docker部署)
6. [ESP32硬件设置](#6-esp32硬件设置)
7. [环境特定构建](#7-环境特定构建)

---

## 1. 快速开始(仅验证——无需硬件)

确认信号处理管道真实且确定性的最快方式。仅需要Python 3.8+、numpy和scipy。无需WiFi硬件、GPU或Docker。

```bash
# 从仓库根目录:
./verify
```

这运行三个阶段:

1. **环境检查**——确认Python、numpy、scipy和proof文件存在。
2. **Proof管道回放**——将发布的参考信号通过完整信号处理链(噪声过滤、汉明窗、幅度归一化、基于FFT的多普勒提取、通过scipy.fft的功率谱密度)并计算输出的SHA-256哈希。
3. **生产代码完整性扫描**——扫描`v1/src/`中生产代码里的`np.random.rand` / `np.random.randn`调用(测试辅助程序被排除)。

退出代码:
- `0` PASS——管道哈希与发布的预期哈希匹配
- `1` FAIL——哈希不匹配或错误
- `2` SKIP——没有预期哈希文件可比较

附加标志:

```bash
./verify --verbose         # 详细特征统计和多普勒频谱
./verify --verbose --audit # 完整验证 + 代码库审计

# 或通过make:
make verify
make verify-verbose
make verify-audit
```

如果预期哈希文件缺失,重新生成:

```bash
python3 v1/data/proof/verify.py --generate-hash
```

### 仅验证的最小依赖

```bash
pip install numpy==1.26.4 scipy==1.14.1
```

或安装保证哈希可重现性的固定集合:

```bash
pip install -r v1/requirements-lock.txt
```

锁文件固定:`numpy==1.26.4`、`scipy==1.14.1`、`pydantic==2.10.4`、`pydantic-settings==2.7.1`。

---

## 2. Python管道(v1/)

Python管道位于`v1/`下,提供完整API服务器、信号处理、感知模块和WebSocket流式传输。

### 前置要求

- Python 3.8+
- pip

### 安装(仅验证——轻量级)

```bash
pip install -r v1/requirements-lock.txt
```

这仅安装确定性管道验证所需的四个包。

### 安装(带API服务器的完整管道)

```bash
pip install -r requirements.txt
```

这将引入FastAPI、uvicorn、torch、OpenCV、SQLAlchemy、Redis客户端和所有其他运行时依赖。

### 验证管道

```bash
python3 v1/data/proof/verify.py
```

与`./verify`相同,但直接调用Python脚本,跳过bash包装器的代码库扫描阶段。

### 运行API服务器

```bash
uvicorn v1.src.api.main:app --host 0.0.0.0 --port 8000
```

服务器暴露:
- REST API文档:http://localhost:8000/docs
- 健康检查:http://localhost:8000/health
- 最新姿态:http://localhost:8000/api/v1/pose/latest
- WebSocket姿态流:ws://localhost:8000/ws/pose/stream
- WebSocket分析:ws://localhost:8000/ws/analytics/events

用于开发自动重新加载:

```bash
uvicorn v1.src.api.main:app --host 0.0.0.0 --port 8000 --reload
```

### 使用通用WiFi运行(RSSI感知——无自定义硬件)

通用感知模块(`v1/src/sensing/`)从标准Linux WiFi指标(RSSI、噪声底、链路质量)提取存在和运动特征,无需任何硬件修改。完整设计细节参见[ADR-013](adr/ADR-013-feature-level-sensing-commodity-gear.md)。

要求:
- 任何带有WiFi接口的Linux机器(笔记本、Raspberry Pi等)
- 连接到WiFi接入点(AP是信号源)
- 通过`/proc/net/wireless`进行基本RSSI读取无需root

模块提供:
- `LinuxWifiCollector`——从`/proc/net/wireless`和`iw`命令读取真实RSSI
- `RssiFeatureExtractor`——计算滚动统计、FFT频谱特征、CUSUM变化点检测
- `PresenceClassifier`——基于规则的存在/运动分类

它可以检测什么:
| 能力 | 单接收器 | 3+接收器 |
|-----------|----------------|-------------|
| 二元存在 | 是(90-95%) | 是(90-95%) |
| 粗略运动(静止/移动) | 是(85-90%) | 是(85-90%) |
| 房间级定位 | 否 | 边缘(70-80%) |

它不能检测什么:身体姿态、心跳、可靠呼吸。诚实能力矩阵参见ADR-013。

### Python项目结构

```
v1/
  src/
    api/
      main.py              # FastAPI应用程序入口点
      routers/             # REST端点路由器(姿态、流、健康)
      middleware/           # 认证、速率限制
      websocket/           # WebSocket连接管理器、姿态流
    config/                # 设置、域配置
    sensing/
      rssi_collector.py    # LinuxWifiCollector + SimulatedCollector
      feature_extractor.py # RssiFeatureExtractor (FFT、CUSUM、频谱)
      classifier.py        # PresenceClassifier (基于规则)
      backend.py           # SensingBackend协议
  data/
    proof/
      sample_csi_data.json       # 确定性参考信号
      expected_features.sha256   # 发布的预期哈希
      verify.py                  # 单命令验证脚本
  requirements-lock.txt          # 哈希可重现性的固定依赖
```

---

## 3. Rust管道(v2)

高性能Rust移植,与Python管道相比,完整信号处理链速度提升约810倍。

### 前置要求

- Rust 1.70+(通过[rustup](https://rustup.rs/)安装)
- cargo(包含在Rust中)
- OpenBLAS的系统依赖(ndarray-linalg使用):
  ```bash
  # Ubuntu/Debian
  sudo apt-get install build-essential gfortran libopenblas-dev pkg-config

  # macOS
  brew install openblas
  ```

### 构建

```bash
cd rust-port/wifi-densepose-rs
cargo build --release
```

Release配置文件配置了LTO、单codegen单元和`-O3`以获得最大性能。

### 测试

```bash
cd rust-port/wifi-densepose-rs
cargo test --workspace
```

运行所有workspace crate的107个测试。

### 基准测试

```bash
cd rust-port/wifi-densepose-rs
cargo bench --package wifi-densepose-signal
```

预期吞吐量:
| 操作 | 延迟 | 吞吐量 |
|-----------|---------|------------|
| CSI预处理(4x64) | ~5.19 us | 49-66 Melem/s |
| 相位清理(4x64) | ~3.84 us | 67-85 Melem/s |
| 特征提取(4x64) | ~9.03 us | 7-11 Melem/s |
| 运动检测 | ~186 ns | -- |
| 完整管道 | ~18.47 us | ~54,000 fps |

### Workspace crate

Rust workspace在`crates/`下包含10个crate:

| Crate | 描述 |
|-------|-------------|
| `wifi-densepose-core` | 核心类型、特征和域模型 |
| `wifi-densepose-signal` | 信号处理(FFT、相位解绕、多普勒、相关) |
| `wifi-densepose-nn` | 神经网络推理(ONNX Runtime、candle、tch) |
| `wifi-densepose-api` | 基于Axum的HTTP/WebSocket API服务器 |
| `wifi-densepose-db` | 数据库层(SQLx、PostgreSQL、SQLite、Redis) |
| `wifi-densepose-config` | 配置加载(环境变量、YAML、TOML) |
| `wifi-densepose-hardware` | 硬件适配器(ESP32、Intel 5300、Atheros、UDP、PCAP) |
| `wifi-densepose-wasm` | 浏览器部署的WebAssembly绑定 |
| `wifi-densepose-cli` | 命令行界面 |
| `wifi-densepose-mat` | WiFi-Mat灾难响应模块(搜索和救援) |

构建独立crate:

```bash
# 仅信号处理
cargo build --release --package wifi-densepose-signal

# API服务器
cargo build --release --package wifi-densepose-api

# 灾难响应模块
cargo build --release --package wifi-densepose-mat

# WASM目标(完整说明参见第7节)
cargo build --release --package wifi-densepose-wasm --target wasm32-unknown-unknown
```

---

## 4. Three.js可视化

基于浏览器的3D可视化仪表板,渲染带有24个身体部位的DensePose身体模型、信号可视化和环境渲染。

### 运行

直接在浏览器中打开`ui/viz.html`:

```bash
# macOS
open ui/viz.html

# Linux
xdg-open ui/viz.html

# 或本地服务
python3 -m http.server 3000 --directory ui
# 然后打开 http://localhost:3000/viz.html
```

### WebSocket连接

可视化连接到`ws://localhost:8000/ws/pose`以获取实时姿态数据。如果没有服务器运行,它回退到带有模拟数据的演示模式,因此您仍然可以看到3D渲染。

要查看实时数据:

1. 启动API服务器(Python或Rust)
2. 打开`ui/viz.html`
3. 仪表板将自动连接

---

## 5. Docker部署

### 开发(带热重新加载、Postgres、Redis、Prometheus、Grafana)

```bash
docker compose up
```

这将启动:
