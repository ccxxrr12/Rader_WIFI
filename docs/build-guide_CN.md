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
- `wifi-densepose-dev` -- 带有`--reload`的API服务器、调试日志、禁用认证(端口8000)
- `postgres` -- PostgreSQL 15 (端口5432)
- `redis` -- 带AOF持久化的Redis 7 (端口6379)
- `prometheus` -- 指标抓取(端口9090)
- `grafana` -- 仪表板(端口3000,登录:admin/admin)
- `nginx` -- 反向代理(端口80、443)

```bash
# 查看日志
docker compose logs -f wifi-densepose

# 在容器内运行测试
docker compose exec wifi-densepose pytest tests/ -v

# 停止所有服务
docker compose down

# 停止并删除卷
docker compose down -v
```

### 生产环境
使用生产Dockerfile阶段,包含4个uvicorn工作进程、启用认证、速率限制和资源限制。

```bash
# 构建生产镜像
docker build --target production -t wifi-densepose:latest .

# 独立运行
docker run -d \
  --name wifi-densepose \
  -p 8000:8000 \
  -e ENVIRONMENT=production \
  -e SECRET_KEY=your-secret-key \
  wifi-densepose:latest
```

对于使用Docker Swarm密钥的完整生产堆栈:

```bash
# 首先创建所需密钥
echo "db_password_here" | docker secret create db_password -
echo "redis_password_here" | docker secret create redis_password -
echo "jwt_secret_here" | docker secret create jwt_secret -
echo "api_key_here" | docker secret create api_key -
echo "grafana_password_here" | docker secret create grafana_password -

# 设置所需环境变量
export DATABASE_URL=postgresql://wifi_user:db_password_here@postgres:5432/wifi_densepose
export REDIS_URL=redis://redis:6379/0
export SECRET_KEY=your-secret-key
export JWT_SECRET=your-jwt-secret
export ALLOWED_HOSTS=your-domain.com
export POSTGRES_DB=wifi_densepose
export POSTGRES_USER=wifi_user

# 使用Docker Swarm部署
docker stack deploy -c docker-compose.prod.yml wifi-densepose
```

生产compose包括:
- 3个API服务器副本,具有滚动更新和回滚
- 资源限制(每个副本2 CPU、4GB RAM)
- 所有服务上的健康检查
- 带轮换的JSON文件日志记录
- 独立的监控网络(overlay)
- 带有告警规则和15天保留期的Prometheus
- 带有预配置数据源和仪表板的Grafana

### Dockerfile阶段
多阶段`Dockerfile`提供四个目标:

| 目标 | 用途 | 命令 |
|--------|-----|---------|
| `development` | 带热重新加载的本地开发 | `docker build --target development .` |
| `production` | 优化的生产镜像 | `docker build --target production .` |
| `testing` | 构建期间运行pytest | `docker build --target testing .` |
| `security` | 运行safety + bandit扫描 | `docker build --target security .` |

---

## 6. ESP32硬件设置

使用ESP32-S3板作为WiFi CSI传感器节点。完整规范参见[ADR-012](adr/ADR-012-esp32-csi-sensor-mesh.md)。

### 材料清单(入门套件--$54)

| 项目 | 数量 | 单价 | 总价 |
|------|-----|-----------|-------|
| ESP32-S3-DevKitC-1 | 3 | $10 | $30 |
| USB-A到USB-C线缆 | 3 | $3 | $9 |
| USB电源适配器(多端口) | 1 | $15 | $15 |
| 消费级WiFi路由器(任意) | 1 | $0 (现有) | $0 |
| 聚合器(笔记本或Pi 4) | 1 | $0 (现有) | $0 |
| **总计** | | | **$54** |

### 前置要求

安装ESP-IDF(Espressif的官方开发框架):

```bash
# 克隆ESP-IDF
mkdir -p ~/esp
cd ~/esp
git clone --recursive https://github.com/espressif/esp-idf.git
cd esp-idf
git checkout v5.2  # 固定到测试版本

# 安装工具
./install.sh esp32s3

# 激活环境(每次会话运行)
. ./export.sh
```

### 刷写节点

```bash
cd firmware/esp32-csi-node

# 设置目标芯片
idf.py set-target esp32s3

# 配置WiFi SSID/密码和聚合器IP
idf.py menuconfig
# 导航到: Component config > WiFi-DensePose CSI Node
#   - 设置WiFi SSID
#   - 设置WiFi密码
#   - 设置聚合器IP地址
#   - 设置节点ID(1、2、3、...)
#   - 设置采样率(10-100 Hz)

# 构建并刷写(连接USB线缆)
idf.py build flash monitor
```

`idf.py monitor`显示实时串行输出,包括CSI回调数据。按`Ctrl+]`退出。

对每个节点重复此操作,递增节点ID。

### 固件项目结构

```
firmware/esp32-csi-node/
  CMakeLists.txt
  sdkconfig.defaults          # 启用CSI的Menuconfig默认值
  main/
    main.c                    # 入口点、WiFi初始化、CSI回调
    csi_collector.c           # CSI数据收集和缓冲
    feature_extract.c         # 设备上FFT和特征提取
    stream_sender.c           # 到聚合器的UDP流
    config.h                  # 节点配置
    Kconfig.projbuild         # Menuconfig选项
  components/
    esp_dsp/                  # 用于FFT的Espressif DSP库
```

每个节点进行设备上特征提取(原始I/Q到幅度 + 相位 + 频谱带),将带宽从每帧~11 KB减少到每帧~470字节。节点通过UDP向聚合器流式传输特征。

### 运行聚合器

聚合器从所有ESP32节点收集UDP流,执行特征级融合(非信号级——参见ADR-012了解原因),并将融合数据输入到Rust或Python管道。

```bash
# 通过Docker启动聚合器和管道
docker compose -f docker-compose.esp32.yml up

# 或直接运行Rust聚合器
cd rust-port/wifi-densepose-rs
cargo run --release --package wifi-densepose-hardware -- --mode esp32-aggregator --port 5000
```

### 使用真实硬件验证

```bash
docker exec aggregator python verify_esp32.py
```

这捕获10秒数据,生成特征JSON,并根据proof包验证哈希。

### ESP32网格能检测和不能检测什么

| 能力 | 1个节点 | 3个节点 | 6个节点 |
|-----------|--------|---------|---------|
| 存在检测 | 良好 | 优秀 | 优秀 |
| 粗略运动 | 良好 | 优秀 | 优秀 |
| 房间级定位 | 无 | 良好 | 优秀 |
| 呼吸 | 边缘 | 良好 | 良好 |
| 心跳 | 差 | 边缘-差 | 边缘 |
| 多人计数 | 无 | 边缘 | 良好 |
| 姿态估计 | 无 | 差 | 边缘 |

---

## 7. 环境特定构建

### 浏览器(WASM)

将Rust管道编译为WebAssembly以在浏览器中执行。边缘部署架构参见[ADR-009](adr/ADR-009-rvf-wasm-runtime-edge-deployment.md)。

前置要求:

```bash
# 安装wasm-pack
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# 或通过cargo
cargo install wasm-pack

# 添加WASM目标
rustup target add wasm32-unknown-unknown
```

构建:

```bash
cd rust-port/wifi-densepose-rs

# 构建WASM包(输出到pkg/)
wasm-pack build crates/wifi-densepose-wasm --target web --release

# 构建时包含灾难响应模块
wasm-pack build crates/wifi-densepose-wasm --target web --release -- --features mat
```

输出`pkg/`目录包含`.wasm`、`.js`胶水和TypeScript定义。在Web项目中导入:

```javascript
import init, { WifiDensePoseWasm } from './pkg/wifi_densepose_wasm.js';

async function main() {
  await init();
  const processor = new WifiDensePoseWasm();
  const result = processor.process_frame(csiJsonString);
  console.log(JSON.parse(result));
}
main();
```

运行WASM测试:

```bash
wasm-pack test --headless --chrome crates/wifi-densepose-wasm
```

按部署配置的容器大小目标:

| 配置 | 大小 | 适用于 |
|---------|------|-------------|
| 浏览器(int8量化) | ~10 MB | Chrome/Firefox仪表板 |
| IoT(int4量化) | ~0.7 MB | ESP32、受限设备 |
| 移动(int8量化) | ~6 MB | iOS/Android WebView |
| 字段(fp16量化) | ~62 MB | 离线灾难平板 |

### IoT (ESP32)

完整ESP32设置参见[第6节](#6-esp32硬件设置)。固件在设备本身上运行(C,使用ESP-IDF编译)。Rust聚合器在主机机器上运行。

要将WASM运行时部署到Raspberry Pi或类似设备:

```bash
# 为ARM交叉编译
rustup target add aarch64-unknown-linux-gnu
cargo build --release --package wifi-densepose-cli --target aarch64-unknown-linux-gnu
```

### 服务器(Docker)

参见[第5节](#5-docker部署)。

快速参考:

```bash
# 开发
docker compose up

# 生产独立
docker build --target production -t wifi-densepose:latest .
docker run -d -p 8000:8000 wifi-densepose:latest

# 生产堆栈(Swarm)
docker stack deploy -c docker-compose.prod.yml wifi-densepose
```

### 服务器(直接——无Docker)

```bash
# 1. 安装Python依赖
pip install -r requirements.txt

# 2. 设置环境变量(从example.env复制)
cp example.env .env
# 使用您的设置编辑.env

# 3. 使用uvicorn运行(生产)
uvicorn v1.src.api.main:app \
  --host 0.0.0.0 \
  --port 8000 \
  --workers 4

# 或运行Rust API服务器
cd rust-port/wifi-densepose-rs
cargo run --release --package wifi-densepose-api
```

### 开发(本地带热重新加载)

Python:

```bash
# 创建虚拟环境
python3 -m venv venv
source venv/bin/activate

# 安装所有依赖,包括开发工具
pip install -r requirements.txt

# 带自动重新加载运行
uvicorn v1.src.api.main:app --host 0.0.0.0 --port 8000 --reload

# 在另一个终端运行验证
./verify --verbose

# 运行测试
pytest tests/ -v
pytest --cov=wifi_densepose --cov-report=html
```

Rust:

```bash
cd rust-port/wifi-densepose-rs

# 以调试模式构建(编译更快)
cargo build

# 带输出运行测试
cargo test --workspace -- --nocapture

# 监视模式(需要cargo-watch)
cargo install cargo-watch
cargo watch -x 'test --workspace' -x 'build --release'

# 运行基准测试
cargo bench --package wifi-densepose-signal
```

两者(可视化 + API):

```bash
# 终端1: 启动API服务器
uvicorn v1.src.api.main:app --host 0.0.0.0 --port 8000 --reload

# 终端2: 提供可视化
python3 -m http.server 3000 --directory ui

# 打开 http://localhost:3000/viz.html —— 它连接到 ws://localhost:8000/ws/pose
```

---

## 附录:关键文件位置

| 文件 | 用途 |
|------|---------|
| `./verify` | 信任终止开关——单命令管道proof |
| `Makefile` | `make verify`、`make verify-verbose`、`make verify-audit` |
| `v1/requirements-lock.txt` | 用于哈希可重现性的固定Python依赖 |
| `requirements.txt` | 完整Python依赖(API服务器、torch等) |
| `v1/data/proof/verify.py` | Python验证脚本 |
| `v1/data/proof/sample_csi_data.json` | 确定性参考信号 |
| `v1/data/proof/expected_features.sha256` | 发布的预期哈希 |
| `v1/src/api/main.py` | FastAPI应用程序入口点 |
| `v1/src/sensing/` | 消费级WiFi感知模块(RSSI) |
| `rust-port/wifi-densepose-rs/Cargo.toml` | Rust workspace根目录 |
| `ui/viz.html` | Three.js 3D可视化 |
| `Dockerfile` | 多阶段Docker构建(dev/prod/test/security) |
| `docker-compose.yml` | 开发堆栈(Postgres、Redis、Prometheus、Grafana) |
| `docker-compose.prod.yml` | 生产堆栈(Swarm、密钥、资源限制) |
| `docs/adr/ADR-009-rvf-wasm-runtime-edge-deployment.md` | WASM边缘部署架构 |
| `docs/adr/ADR-012-esp32-csi-sensor-mesh.md` | ESP32固件和网格规范 |
| `docs/adr/ADR-013-feature-level-sensing-commodity-gear.md` | 消费级WiFi(RSSI)感知 |
