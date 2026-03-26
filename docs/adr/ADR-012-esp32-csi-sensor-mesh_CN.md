# ADR-012: 用于分布式感知的ESP32 CSI传感器网格

## 状态
已接受——部分实现(固件 + 聚合器工作,参见ADR-018)

## 日期
2026-02-28

## 背景

### 硬件现实差距

WiFi-DensePose的Rust和Python管道实现了真实的信号处理(FFT、相位解绕、多普勒提取、相关特征),但系统目前没有定义从**物理WiFi硬件 → CSI字节 → 管道输入**的路径。`csi_extractor.py`和`router_interface.py`模块包含占位符解析器,返回`np.random.rand()`而不是真实解析数据(参见ADR-011)。

为了弥合这一差距,我们需要一个具体的、负担得起的、可重现的硬件平台,产生真实CSI数据并将其流式传输到现有管道中。

### 为什么选择ESP32

| 因素 | ESP32/ESP32-S3 | Intel 5300 (iwl5300) | Atheros AR9580 |
|--------|---------------|---------------------|----------------|
| 成本 | ~$5-15/节点 | ~$50-100(二手网卡) | ~$30-60(二手网卡) |
| 可用性 | 大规模生产,有库存 | 已停产,仅eBay | 已停产,仅eBay |
| CSI支持 | 官方ESP-IDF API | Linux CSI工具(内核修改) | Atheros CSI工具 |
| 外形规格 | 独立MCU | 需要PCIe/Mini-PCIe主机 | 需要PCIe主机 |
| 部署 | 电池/USB,无线 | 仅台式机/笔记本 | 仅台式机/笔记本 |
| 天线配置 | 1-2 TX, 1-2 RX | 3 TX, 3 RX (MIMO) | 3 TX, 3 RX (MIMO) |
| 子载波 | 52-56 (802.11n) | 30(压缩) | 56(完整) |
| 保真度 | 较低(消费级SoC) | 较高(专用网卡) | 较高(专用网卡) |

**ESP32在可部署性上胜出**:这是陌生人可以在Amazon上购买节点、刷写固件并在一个下午内拥有工作的CSI网格的唯一选择。Intel 5300和Atheros卡需要特定硬件、内核修改和旧版操作系统。

### ESP-IDF CSI API

Espressif通过三个关键函数提供官方CSI支持:

```c
// 1. 配置要捕获的CSI数据
wifi_csi_config_t csi_config = {
    .lltf_en = true,         // 长训练字段(最适合CSI)
    .htltf_en = true,        // HT-LTF
    .stbc_htltf2_en = true,  // STBC HT-LTF2
    .ltf_merge_en = true,    // 合并LTF
    .channel_filter_en = false,
    .manu_scale = false,
};
esp_wifi_set_csi_config(&csi_config);

// 2. 注册接收CSI数据的回调
esp_wifi_set_csi_rx_cb(csi_data_callback, NULL);

// 3. 启用CSI收集
esp_wifi_set_csi(true);

// 回调接收:
void csi_data_callback(void *ctx, wifi_csi_info_t *info) {
    // info->rx_ctrl: RSSI、噪声底、信道、辅助信道等
    // info->buf: 原始CSI数据(每子载波的I/Q对)
    // info->len: CSI数据缓冲区长度
    // 典型: 112字节 = 56个子载波 × 2(I,Q) × 每个字节1
}
```

## 决策

我们将构建ESP32 CSI传感器网格作为主要硬件集成路径,具有从固件到聚合器到Rust管道到可视化的完整堆栈。

### 系统架构

```
┌─────────────────────────────────────────────────────────────────────┐
│                   ESP32 CSI传感器网格                              │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐                          │
│  │ ESP32    │  │ ESP32    │  │ ESP32    │  ... (3-6个节点)         │
│  │ 节点1   │  │ 节点2   │  │ 节点3   │                          │
│  │          │  │          │  │          │                          │
│  │ CSI Rx   │  │ CSI Rx   │  │ CSI Rx   │  ← 来自消费路由器的    │
│  │ FFT      │  │ FFT      │  │ FFT      │     WiFi帧             │
│  │ 特征    │  │ 特征    │  │ 特征    │                          │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘                          │
│       │              │              │                                │
│       │    UDP/TCP流(WiFi或辅助信道)               │
│       │              │              │                                │
│       ▼              ▼              ▼                                │
│  ┌─────────────────────────────────────────┐                        │
│  │           聚合器                     │                        │
│  │  (笔记本 / Raspberry Pi / 种子设备)  │                        │
│  │                                          │                        │
│  │  1. 从所有节点接收CSI流  │                        │
│  │  2. 时间戳对齐(每节点)       │                        │
│  │  3. 特征级融合                │                        │
│  │  4. 输入到Rust/Python管道      │                        │
│  │  5. 为可视化提供WebSocket    │                        │
│  └──────────────────┬──────────────────────┘                        │
│                      │                                               │
│                      ▼                                               │
│  ┌─────────────────────────────────────────┐                        │
│  │        WiFi-DensePose管道           │                        │
│  │                                          │                        │
│  │  CsiProcessor → FeatureExtractor →      │                        │
│  │  MotionDetector → PoseEstimator →       │                        │
│  │  Three.js可视化                 │                        │
│  └─────────────────────────────────────────┘                        │
└─────────────────────────────────────────────────────────────────────┘
```

### 节点固件规范

**ESP-IDF项目**: `firmware/esp32-csi-node/`

```
firmware/esp32-csi-node/
├── CMakeLists.txt
├── sdkconfig.defaults      # 启用CSI的Menuconfig默认值(git忽略)
├── main/
│   ├── CMakeLists.txt
│   ├── main.c              # 入口点、NVS配置、WiFi初始化、CSI回调
│   ├── csi_collector.c     # CSI收集、混杂模式、ADR-018序列化
│   ├── csi_collector.h
│   ├── nvs_config.c        # 来自NVS的运行时配置(WiFi凭据、目标IP)
│   ├── nvs_config.h
│   ├── stream_sender.c     # 到聚合器的UDP流
│   ├── stream_sender.h
│   └── Kconfig.projbuild   # Menuconfig选项
└── README.md               # 刷写说明(已验证工作)
```

> **实现说明**:设备上特征提取(`feature_extract.c`)已推迟。
> 当前固件以ADR-018二进制格式流式传输原始I/Q数据;特征提取
> 发生在Rust聚合器中。这简化了固件并保持ESP32代码
> 在200行C代码以下。

**设备上处理**(减少带宽,节点进行预处理):

```c
// feature_extract.c
typedef struct {
    uint32_t timestamp_ms;      // 本地单调时间戳
    uint8_t  node_id;           // 此节点的ID
    int8_t   rssi;              // 接收信号强度
    int8_t   noise_floor;       // 噪声底估计
    uint8_t  channel;           // WiFi信道
    float    amplitude[56];     // 每子载波的|CSI|(来自I/Q)
    float    phase[56];         // 每子载波的arg(CSI)
    float    doppler_energy;    // 来自时域FFT的运动能量
    float    breathing_band;    // 0.1-0.5 Hz频带功率
    float    motion_band;       // 0.5-3 Hz频带功率
} csi_feature_frame_t;
// 大小:每帧~470字节
// 在100 Hz时:每节点~47 KB/s,6个节点~280 KB/s
```

**关键固件设计决策**:

1. **设备上特征提取**:原始CSI I/Q → 幅度 + 相位 + 频谱带。这将带宽从原始~11 KB/帧减少到~470字节/帧。

2. **单调时间戳**:每个节点使用自己的单调时钟。节点之间不尝试NTP同步——时钟漂移在聚合器处通过融合特征而非原始相位来处理(参见下方的"时钟漂移"部分)。

3. **UDP流式传输**:低延迟、容错丢失。可接受丢失帧;通过序列号保持排序。

4. **可配置采样率**:通过menuconfig 10-100 Hz。100 Hz用于运动检测,10 Hz足以用于占用检测。

### 聚合器规范

聚合器运行在任何具有WiFi/以太网的机器上连接到节点:

```rust
// 在wifi-densepose-rs中,新模块:crates/wifi-densepose-hardware/src/esp32/
pub struct Esp32Aggregator {
    /// 用于节点流的UDP套接字
    socket: UdpSocket,

    /// 每节点状态(最后时间戳、特征缓冲区、漂移估计)
    nodes: HashMap<u8, NodeState>,

    /// 融合特征帧的环形缓冲区
    fused_buffer: VecDeque<FusedFrame>,

    /// 到管道的通道
    pipeline_tx: mpsc::Sender<CsiData>,
}

/// 来自所有节点的一个时间窗口的融合帧
pub struct FusedFrame {
    /// 时间戳(聚合器本地,单调)
    timestamp: Instant,

    /// 每节点特征(如果节点掉线则可能有间隙)
    node_features: Vec<Option<CsiFeatureFrame>>,

    /// 跨节点相关性(由聚合器计算)
    cross_node_correlation: Array2<f64>,

    /// 融合运动能量(跨节点最大值)
    fused_motion_energy: f64,

    /// 融合呼吸频带(相位对齐处的相干和)
    fused_breathing_band: f64,
}
```

### 时钟漂移处理

ESP32晶体振荡器漂移约20-50 ppm。1小时内，两个节点可能会偏离72-180ms。这使得跨节点的原始相位对齐变得不可能。

**解决方案**：特征级融合，而非信号级融合。

```
信号级（对ESP32错误）：
  跨节点对齐原始I/Q样本 → 需要<1µs同步 → 不切实际

特征级（对ESP32正确）：
  每个节点：原始CSI → 幅度 + 相位 + 频谱特征（本地）
  聚合器：收集特征 → 关联 → 融合决策
  无需跨节点相位对齐
```

具体来说：
- **运动能量**：取跨节点最大值（任何节点检测到运动 = 运动）
- **呼吸频带**：使用SNR最高的节点作为主要节点，其他节点作为佐证
- **位置**：跨节点幅度比率估计位置（无需相位）

### 不同部署的感知能力

| 能力 | 1个节点 | 3个节点 | 6个节点 | 证据 |
|-----------|--------|---------|---------|----------|
| 存在检测 | 良好 | 优秀 | 优秀 | 单节点RSSI方差 |
| 粗略运动 | 良好 | 优秀 | 优秀 | 多普勒能量 |
| 房间级位置 | 无 | 良好 | 优秀 | 幅度比率 |
| 呼吸 | 边缘 | 良好 | 良好 | 0.1-0.5 Hz频带，位置敏感 |
| 心跳 | 差 | 差-边缘 | 边缘 | 需要理想位置，低噪声 |
| 多人计数 | 无 | 边缘 | 良好 | 空间多样性 |
| 姿态估计 | 无 | 差 | 边缘 | 需要模型 + 足够的多样性 |

**诚实评估**：ESP32 CSI的保真度低于Intel 5300或Atheros。心跳检测对位置敏感且不可靠。呼吸检测在良好放置时有效。运动和存在检测是可靠的。

### 故障模式和缓解措施

| 故障模式 | 严重程度 | 缓解措施 |
|-------------|----------|------------|
| 多径在杂乱房间中占主导 | 高 | 网格多样性：3+个节点从不同角度 |
| 人遮挡节点和路由器之间的路径 | 中 | 网格：其他节点仍有清晰路径 |
| 时钟漂移破坏跨节点融合 | 中 | 仅特征级融合；无跨节点相位对齐 |
| 高流量期间UDP包丢失 | 低 | 序列号，<100ms间隙的插值 |
| ESP32 WiFi驱动与CSI的bug | 中 | 固定ESP-IDF版本，在已知良好的板上测试 |
| 节点电源故障 | 低 | 聚合器优雅处理丢失的节点 |

### 材料清单（入门套件）

| 物品 | 数量 | 单价 | 总计 |
|------|----------|-----------|-------|
| ESP32-S3-DevKitC-1 | 3 | $10 | $30 |
| USB-A转USB-C线缆 | 3 | $3 | $9 |
| USB电源适配器（多端口） | 1 | $15 | $15 |
| 消费级WiFi路由器（任何） | 1 | $0（现有） | $0 |
| 聚合器（笔记本电脑或Pi 4） | 1 | $0（现有） | $0 |
| **总计** | | | **$54** |

### 最小构建规格（克隆-刷写-运行）

**选项A：使用预构建二进制文件（无需工具链）**

```bash
# 从GitHub Release v0.1.0-esp32下载二进制文件
# 用esptool刷写（pip install esptool）
python -m esptool --chip esp32s3 --port COM7 --baud 460800 \
  write-flash --flash-mode dio --flash-size 4MB \
  0x0 bootloader.bin 0x8000 partition-table.bin 0x10000 esp32-csi-node.bin

# 配置WiFi凭据（无需重新编译）
python scripts/provision.py --port COM7 \
  --ssid "YourWiFi" --password "secret" --target-ip 192.168.1.20

# 运行聚合器
cargo run -p wifi-densepose-hardware --bin aggregator -- --bind 0.0.0.0:5005 --verbose
```

**选项B：使用Docker从源代码构建（无需安装ESP-IDF）**

```bash
# 步骤1：编辑WiFi凭据
vim firmware/esp32-csi-node/sdkconfig.defaults

# 步骤2：用Docker构建
cd firmware/esp32-csi-node
MSYS_NO_PATHCONV=1 docker run --rm -v "$(pwd):/project" -w /project \
  espressif/idf:v5.2 bash -c "idf.py set-target esp32s3 && idf.py build"

# 步骤3：刷写
cd build
python -m esptool --chip esp32s3 --port COM7 --baud 460800 \
  write-flash --flash-mode dio --flash-size 4MB \
  0x0 bootloader/bootloader.bin 0x8000 partition_table/partition-table.bin \
  0x10000 esp32-csi-node.bin

# 步骤4：运行聚合器
cargo run -p wifi-densepose-hardware --bin aggregator -- --bind 0.0.0.0:5005 --verbose
```

**已验证**：20 Hz CSI流，64/128/192子载波帧，RSSI -47至-88 dBm。
参见教程：https://github.com/ruvnet/wifi-densepose/issues/34

### ESP32的现实证明

**现场验证**使用ESP32-S3-DevKitC-1（CP2102，MAC 3C:0F:02:EC:C2:28）：
- 18秒内693帧（约21.6 fps）
- 序列号连续（零帧丢失）
- 存在检测确认：每秒钟幅度方差的运动评分10/10
- 帧类型：64 sc（148 B），128 sc（276 B），192 sc（404 B）
- 20个Rust测试 + 6个Python测试通过

预构建二进制文件：https://github.com/ruvnet/wifi-densepose/releases/tag/v0.1.0-esp32

## 影响

### 积极
- **$54入门套件**：获取真实CSI数据的最低可能障碍
- **大量可用硬件**：ESP32板在全球有库存
- **真实数据路径**：用实际硬件输入消除所有`np.random.rand()`占位符
- **证明工件**：捕获的CSI + 预期哈希证明管道处理真实数据
- **可扩展网格**：添加节点以获得更多覆盖范围，无需更改软件
- **特征级融合**：避免跨节点相位同步的不可能问题

### 消极
- **比研究NIC保真度低**：ESP32 CSI比Intel 5300噪声更大
- **心跳检测不可靠**：微多普勒分辨率不足以进行一致的心跳检测
- **ESP-IDF学习曲线**：固件开发需要嵌入式C知识
- **WiFi干扰**：与数据流量共享同一信道的节点会增加噪声
- **位置敏感性**：呼吸检测需要仔细的节点定位

### 与其他ADR的交互
- **ADR-011**（现实证明）：ESP32为证明包提供真实CSI捕获
- **ADR-008**（分布式共识）：网格节点可以使用简化的Raft进行配置分发
- **ADR-003**（RVF容器）：聚合器以RVF格式存储CSI特征
- **ADR-004**（HNSW）：来自ESP32网格的环境指纹输入HNSW索引

## 参考资料

- [Espressif ESP-CSI仓库](https://github.com/espressif/esp-csi)
- [ESP-IDF WiFi CSI API](https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-guides/wifi.html#wi-fi-channel-state-information)
- [ESP32 CSI研究论文](https://ieeexplore.ieee.org/document/9439871)
- [使用ESP32进行WiFi感知：教程](https://arxiv.org/abs/2207.07859)
- ADR-011：Python现实证明和模拟消除
- ADR-018：ESP32开发实现（二进制帧格式规范）
- [预构建固件版本v0.1.0-esp32](https://github.com/ruvnet/wifi-densepose/releases/tag/v0.1.0-esp32)
- [逐步教程（Issue #34）](https://github.com/ruvnet/wifi-densepose/issues/34)
