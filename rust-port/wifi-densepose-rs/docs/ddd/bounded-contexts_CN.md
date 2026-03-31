# 有界上下文

本文档定义了构成 WiFi-DensePose 系统的五个有界上下文。每个上下文代表一个独特的子域，拥有自己的通用语言、模型和边界。

---

## 1. 信号域（CSI 处理）

### 目的

信号域负责从信道状态信息 (CSI) 数据中获取、验证、预处理和提取特征。它将原始 RF 测量转换为适合姿态推理的结构化信号特征。

### 通用语言（上下文特定）

| 术语 | 定义 |
|------|------|
| CSI 帧 | 跨所有子载波和天线的信道状态信息的单次捕获 |
| 子载波 | OFDM 调制中承载幅度和相位数据的单个频率 bin |
| 幅度 | CSI 测量的信号强度组件 |
| 相位 | CSI 测量的信号时序组件 |
| 多普勒频移 | 移动物体引起的频率变化 |
| 噪声底 | 背景电磁干扰水平 |
| SNR | 信噪比，CSI 数据的质量指标 |

### 核心职责

1. **CSI 获取** - 与硬件接口以接收原始 CSI 字节
2. **帧解析** - 解码供应商特定的 CSI 格式（ESP32、Atheros、Intel）
3. **验证** - 验证帧完整性、天线数量、子载波维度
4. **预处理** - 噪声去除、加窗、归一化
5. **特征提取** - 计算幅度统计、相位差异、相关性、PSD

### 聚合根：CsiFrame

```rust
pub struct CsiFrame {
    id: FrameId,
    device_id: DeviceId,
    session_id: Option<SessionId>,
    timestamp: Timestamp,
    sequence_number: u64,

    // 原始测量
    amplitude: Matrix<f32>,     // [antennas x subcarriers]
    phase: Matrix<f32>,         // [antennas x subcarriers]

    // 信号特性
    frequency: Frequency,       // 中心频率 (Hz)
    bandwidth: Bandwidth,       // 信道带宽 (Hz)
    num_subcarriers: u16,
    num_antennas: u8,

    // 质量指标
    snr: SignalToNoise,
    rssi: Option<Rssi>,
    noise_floor: Option<NoiseFloor>,

    // 处理状态
    status: ProcessingStatus,
    metadata: FrameMetadata,
}
```

### 值对象

```rust
// 带有不变量的验证频率
pub struct Frequency(f64); // Hz，必须 > 0

// 具有常见 WiFi 值的带宽
pub enum Bandwidth {
    Bw20MHz,
    Bw40MHz,
    Bw80MHz,
    Bw160MHz,
}

// 具有合理边界的 SNR
pub struct SignalToNoise(f64); // dB，通常为 -50 到 +50

// 处理 pipeline 状态
pub enum ProcessingStatus {
    Pending,
    Preprocessing,
    FeatureExtraction,
    Completed,
    Failed(ProcessingError),
}
```

### 域服务

```rust
pub trait CsiPreprocessor {
    fn remove_noise(&self, frame: &CsiFrame, threshold: NoiseThreshold) -> Result<CsiFrame>;
    fn apply_window(&self, frame: &CsiFrame, window: WindowFunction) -> Result<CsiFrame>;
    fn normalize_amplitude(&self, frame: &CsiFrame) -> Result<CsiFrame>;
    fn sanitize_phase(&self, frame: &CsiFrame) -> Result<CsiFrame>;
}

pub trait FeatureExtractor {
    fn extract_amplitude_features(&self, frame: &CsiFrame) -> AmplitudeFeatures;
    fn extract_phase_features(&self, frame: &CsiFrame) -> PhaseFeatures;
    fn extract_correlation_features(&self, frame: &CsiFrame) -> CorrelationFeatures;
    fn extract_doppler_features(&self, frames: &[CsiFrame]) -> DopplerFeatures;
    fn compute_power_spectral_density(&self, frame: &CsiFrame) -> PowerSpectralDensity;
}
```

### 出站事件

- `CsiFrameReceived` - 从硬件获取原始帧
- `CsiFrameValidated` - 帧通过完整性检查
- `SignalProcessed` - 特征已提取并准备好用于推理

### 集成点

| 上下文 | 方向 | 机制 |
|---------|------|------|
| 硬件域 | 入站 | 通过异步通道的原始字节 |
| 姿态域 | 出站 | 通过事件总线的 ProcessedSignal |
| 存储域 | 出站 | 通过存储库的持久化 |

---

## 2. 姿态域（DensePose 推理）

### 目的

姿态域是系统的核心。它使用神经网络推理将处理后的 CSI 特征转换为人体姿态估计。该域封装了模态转换算法和 DensePose 模型集成。

### 通用语言（上下文特定）

| 术语 | 定义 |
|------|------|
| 模态转换 | 将 RF 信号特征转换为类视觉表示 |
| DensePose | 将像素映射到身体表面的密集人体姿态估计 |
| 身体部位 | 在分割中识别的解剖区域（头部、躯干、四肢） |
| UV 坐标 | 身体网格上的 2D 表面坐标 |
| 关键点 | 命名的解剖学标记点（鼻子、肩膀、膝盖等） |
| 置信度分数 | 检测正确的概率 |
| 边界框 | 包含检测到的人的矩形区域 |

### 核心职责

1. **模态转换** - 将 CSI 特征转换到视觉特征空间
2. **人体检测** - 识别人类的存在和数量
3. **身体分割** - 将像素/区域分类为身体部位
4. **UV 回归** - 预测连续表面坐标
5. **关键点定位** - 检测解剖学标记点
6. **活动分类** - 推断高级活动（站立、坐着、行走）

### 聚合根：PoseEstimate

```rust
pub struct PoseEstimate {
    id: EstimateId,
    session_id: SessionId,
    frame_id: FrameId,
    timestamp: Timestamp,

    // 检测结果
    persons: Vec<PersonDetection>,
    person_count: u8,

    // 处理元数据
    processing_time: Duration,
    model_version: ModelVersion,
    algorithm: InferenceAlgorithm,

    // 质量评估
    overall_confidence: Confidence,
    is_valid: bool,
}

pub struct PersonDetection {
    person_id: PersonId,
    bounding_box: BoundingBox,
    keypoints: Vec<Keypoint>,
    body_parts: BodyPartSegmentation,
    uv_coordinates: UvMap,
    confidence: Confidence,
    activity: Option<Activity>,
}

pub struct Keypoint {
    name: KeypointName,
    position: Position2D,
    confidence: Confidence,
}

pub enum KeypointName {
    Nose,
    LeftEye,
    RightEye,
    LeftEar,
    RightEar,
    LeftShoulder,
    RightShoulder,
    LeftElbow,
    RightElbow,
    LeftWrist,
    RightWrist,
    LeftHip,
    RightHip,
    LeftKnee,
    RightKnee,
    LeftAnkle,
    RightAnkle,
}
```

### 值对象

```rust
// 置信度分数范围 [0, 1]
pub struct Confidence(f32);

impl Confidence {
    pub fn new(value: f32) -> Result<Self, DomainError> {
        if value < 0.0 || value > 1.0 {
            return Err(DomainError::InvalidConfidence);
        }
        Ok(Self(value))
    }

    pub fn is_high(&self) -> bool {
        self.0 >= 0.8
    }
}

// 归一化坐标 [0, 1] 中的 2D 位置
pub struct Position2D {
    x: NormalizedCoordinate,
    y: NormalizedCoordinate,
}

// 活动分类
pub enum Activity {
    Standing,
    Sitting,
    Walking,
    Lying,
    Falling,
    Unknown,
}
```

### 域服务

```rust
pub trait ModalityTranslator {
    fn translate(&self, signal: &ProcessedSignal) -> Result<VisualFeatures>;
}

pub trait PoseInferenceEngine {
    fn detect_persons(&self, features: &VisualFeatures) -> Vec<PersonDetection>;
    fn segment_body_parts(&self, detection: &PersonDetection) -> BodyPartSegmentation;
    fn regress_uv_coordinates(&self, detection: &PersonDetection) -> UvMap;
    fn classify_activity(&self, detection: &PersonDetection) -> Activity;
}

pub trait HumanPresenceDetector {
    fn detect_presence(&self, signal: &ProcessedSignal) -> HumanPresenceResult;
    fn estimate_count(&self, signal: &ProcessedSignal) -> PersonCount;
}
```

### 出站事件

- `PoseEstimated` - 姿态推理成功完成
- `PersonDetected` - 新人进入检测区域
- `PersonLost` - 人离开检测区域
- `ActivityChanged` - 人的活动分类发生变化
- `MotionDetected` - 观察到显著运动
- `FallDetected` - 识别到潜在的跌倒事件

### 集成点

| 上下文 | 方向 | 机制 |
|---------|------|------|
| 信号域 | 入站 | ProcessedSignal 事件 |
| 流传输域 | 出站 | PoseEstimate 广播 |
| 存储域 | 出站 | 通过存储库的持久化 |

---

## 3. 流传输域（WebSocket，实时）

### 目的

流传输域通过 WebSocket 连接管理向客户端的实时数据传递。它处理连接生命周期、消息路由、按区域/主题过滤，并维护流传输服务质量。

### 通用语言（上下文特定）

| 术语 | 定义 |
|------|------|
| 连接 | 与客户端的活动 WebSocket 会话 |
| 流类型 | 数据流类别（姿态、CSI、警报、状态） |
| 区域 | 用于过滤姿态数据的逻辑或物理区域 |
| 订阅 | 客户端对特定流/区域的表达兴趣 |
| 广播 | 发送给所有匹配订阅者的消息 |
| 心跳 | 定期 ping 以验证连接活跃性 |
| 背压 | 当客户端无法跟上时的流量控制 |

### 核心职责

1. **连接管理** - 接受、跟踪和关闭 WebSocket 连接
2. **订阅处理** - 管理客户端对流和区域的订阅
3. **消息路由** - 向匹配的订阅者传递事件
4. **服务质量** - 处理背压、缓冲、重连
5. **指标收集** - 跟踪延迟、吞吐量、错误率

### 聚合根：Session

```rust
pub struct Session {
    id: SessionId,
    client_id: ClientId,

    // 连接详情
    connected_at: Timestamp,
    last_activity: Timestamp,
    remote_addr: Option<IpAddr>,
    user_agent: Option<String>,

    // 订阅状态
    stream_type: StreamType,
    zone_subscriptions: Vec<ZoneId>,
    filters: SubscriptionFilters,

    // 会话状态
    status: SessionStatus,
    message_count: u64,

    // 质量指标
    latency_stats: LatencyStats,
    error_count: u32,
}

pub enum StreamType {
    Pose,
    Csi,
    Alerts,
    SystemStatus,
    All,
}

pub enum SessionStatus {
    Active,
    Paused,
    Reconnecting,
    Completed,
    Failed(SessionError),
    Cancelled,
}

pub struct SubscriptionFilters {
    min_confidence: Option<Confidence>,
    max_persons: Option<u8>,
    include_keypoints: bool,
    include_segmentation: bool,
    throttle_ms: Option<u32>,
}
```

### 值对象

```rust
// 带有验证的区域标识符
pub struct ZoneId(String);

impl ZoneId {
    pub fn new(id: impl Into<String>) -> Result<Self, DomainError> {
        let id = id.into();
        if id.is_empty() || id.len() > 64 {
            return Err(DomainError::InvalidZoneId);
        }
        Ok(Self(id))
    }
}

// 延迟跟踪
pub struct LatencyStats {
    min_ms: f64,
    max_ms: f64,
    avg_ms: f64,
    p99_ms: f64,
    samples: u64,
}
```

### 域服务

```rust
pub trait ConnectionManager {
    async fn connect(&self, socket: WebSocket, config: ConnectionConfig) -> Result<SessionId>;
    async fn disconnect(&self, session_id: &SessionId) -> Result<()>;
    async fn update_subscription(&self, session_id: &SessionId, filters: SubscriptionFilters) -> Result<()>;
    fn get_active_sessions(&self) -> Vec<&Session>;
}

pub trait MessageRouter {
    async fn broadcast(&self, message: StreamMessage, filter: BroadcastFilter) -> BroadcastResult;
    async fn send_to_session(&self, session_id: &SessionId, message: StreamMessage) -> Result<()>;
    async fn send_to_zone(&self, zone_id: &ZoneId, message: StreamMessage) -> BroadcastResult;
}

pub trait StreamBuffer {
    fn buffer_message(&mut self, message: StreamMessage);
    fn get_recent(&self, count: usize) -> Vec<&StreamMessage>;
    fn clear(&mut self);
}
```

### 出站事件

- `SessionStarted` - 客户端已连接并订阅
- `SessionEnded` - 客户端已断开连接
- `SubscriptionUpdated` - 客户端更改了过滤偏好
- `MessageDelivered` - 确认成功传递
- `DeliveryFailed` - 消息无法传递

### 集成点

| 上下文 | 方向 | 机制 |
|---------|------|------|
| 姿态域 | 入站 | PoseEstimate 事件 |
| 信号域 | 入站 | ProcessedSignal 事件（如果启用了 CSI 流） |
| API 层 | 双向 | WebSocket 升级，REST 管理 |

---

## 4. 存储域（持久化）

### 目的

存储域处理所有持久化操作，包括保存 CSI 帧、姿态估计、会话记录和设备配置。它为聚合根提供存储库，并支持实时写入和历史查询。

### 通用语言（上下文特定）

| 术语 | 定义 |
|------|------|
| 存储库 | 聚合持久化操作的接口 |
| 实体 | 具有标识的持久域对象 |
| 查询 | 针对存储数据的读取操作 |
| 迁移 | 架构演化脚本 |
| 事务 | 原子工作单元 |
| 聚合存储 | 聚合根的持久层 |

### 核心职责

1. **CRUD 操作** - 所有聚合的创建、读取、更新、删除
2. **查询支持** - 时间范围查询、过滤、聚合
3. **事务管理** - 确保操作一致性
4. **架构演化** - 处理数据库迁移
5. **性能优化** - 索引、分区、缓存

### 存储库接口

```rust
#[async_trait]
pub trait CsiFrameRepository {
    async fn save(&self, frame: &CsiFrame) -> Result<FrameId>;
    async fn save_batch(&self, frames: &[CsiFrame]) -> Result<Vec<FrameId>>;
    async fn find_by_id(&self, id: &FrameId) -> Result<Option<CsiFrame>>;
    async fn find_by_session(&self, session_id: &SessionId, limit: usize) -> Result<Vec<CsiFrame>>;
    async fn find_by_time_range(&self, start: Timestamp, end: Timestamp) -> Result<Vec<CsiFrame>>;
    async fn delete_older_than(&self, cutoff: Timestamp) -> Result<u64>;
}

#[async_trait]
pub trait PoseEstimateRepository {
    async fn save(&self, estimate: &PoseEstimate) -> Result<EstimateId>;
    async fn find_by_id(&self, id: &EstimateId) -> Result<Option<PoseEstimate>>;
    async fn find_by_session(&self, session_id: &SessionId) -> Result<Vec<PoseEstimate>>;
    async fn find_by_zone_and_time(&self, zone_id: &ZoneId, start: Timestamp, end: Timestamp) -> Result<Vec<PoseEstimate>>;
    async fn get_statistics(&self, start: Timestamp, end: Timestamp) -> Result<PoseStatistics>;
}

#[async_trait]
pub trait SessionRepository {
    async fn save(&self, session: &Session) -> Result<SessionId>;
    async fn update(&self, session: &Session) -> Result<()>;
    async fn find_by_id(&self, id: &SessionId) -> Result<Option<Session>>;
    async fn find_active(&self) -> Result<Vec<Session>>;
    async fn find_by_device(&self, device_id: &DeviceId) -> Result<Vec<Session>>;
    async fn mark_completed(&self, id: &SessionId, end_time: Timestamp) -> Result<()>;
}

#[async_trait]
pub trait DeviceRepository {
    async fn save(&self, device: &Device) -> Result<DeviceId>;
    async fn update(&self, device: &Device) -> Result<()>;
    async fn find_by_id(&self, id: &DeviceId) -> Result<Option<Device>>;
    async fn find_by_mac(&self, mac: &MacAddress) -> Result<Option<Device>>;
    async fn find_all(&self) -> Result<Vec<Device>>;
    async fn find_by_status(&self, status: DeviceStatus) -> Result<Vec<Device>>;
}
```

### 查询对象

```rust
pub struct TimeRangeQuery {
    start: Timestamp,
    end: Timestamp,
    zone_ids: Option<Vec<ZoneId>>,
    device_ids: Option<Vec<DeviceId>>,
    limit: Option<usize>,
    offset: Option<usize>,
}

pub struct PoseStatistics {
    total_detections: u64,
    successful_detections: u64,
    failed_detections: u64,
    average_confidence: f32,
    average_processing_time_ms: f32,
    unique_persons: u32,
    activity_distribution: HashMap<Activity, f32>,
}

pub struct AggregatedPoseData {
    timestamp: Timestamp,
    interval_seconds: u32,
    total_persons: u32,
    zones: HashMap<ZoneId, ZoneOccupancy>,
}
```

### 集成点

| 上下文 | 方向 | 机制 |
|---------|------|------|
| 所有域 | 入站 | 存储库特征实现 |
| 基础设施 | 出站 | SQLx、Redis 适配器 |

---

## 5. 硬件域（设备管理）

### 目的

硬件域抽象物理 WiFi 设备（路由器、ESP32、Intel NIC）并管理其生命周期。它处理设备发现、连接建立、配置和健康监控。

### 通用语言（上下文特定）

| 术语 | 定义 |
|------|------|
| 设备 | 能够提取 CSI 的物理 WiFi 硬件 |
| 固件 | 在设备上运行的软件 |
| MAC 地址 | 唯一硬件标识符 |
| 校准 | 调整设备以获得准确 CSI 的过程 |
| 健康检查 | 设备状态的定期验证 |
| 驱动程序 | 硬件的软件接口 |

### 核心职责

1. **设备发现** - 扫描网络以寻找兼容设备
2. **连接管理** - 建立和维护硬件连接
3. **配置** - 应用和持久化设备设置
4. **健康监控** - 跟踪设备状态和性能
5. **固件管理** - 版本跟踪、更新协调

### 聚合根：Device

```rust
pub struct Device {
    id: DeviceId,

    // 识别
    name: DeviceName,
    device_type: DeviceType,
    mac_address: MacAddress,
    ip_address: Option<IpAddress>,

    // 硬件详情
    firmware_version: Option<FirmwareVersion>,
    hardware_version: Option<HardwareVersion>,
    capabilities: DeviceCapabilities,

    // 位置
    location: Option<Location>,
    zone_id: Option<ZoneId>,

    // 状态
    status: DeviceStatus,
    last_seen: Option<Timestamp>,
    error_count: u32,

    // 配置
    config: DeviceConfig,
    calibration: Option<CalibrationData>,
}

pub enum DeviceType {
    Esp32,
    AtheriosRouter,
    IntelNic,
    Nexmon,
    Custom(String),
}

pub enum DeviceStatus {
    Disconnected,
    Connecting,
    Connected,
    Streaming,
    Calibrating,
    Maintenance,
    Error(DeviceError),
}

pub struct DeviceCapabilities {
    max_subcarriers: u16,
    max_antennas: u8,
    supported_bandwidths: Vec<Bandwidth>,
    supported_frequencies: Vec<Frequency>,
    csi_rate_hz: u32,
}

pub struct DeviceConfig {
    sampling_rate: u32,
    subcarriers: u16,
    antennas: u8,
    bandwidth: Bandwidth,
    channel: WifiChannel,
    gain: Option<f32>,
    custom_params: HashMap<String, serde_json::Value>,
}
```

### 值对象

```rust
// 带有验证的 MAC 地址
pub struct MacAddress([u8; 6]);

impl MacAddress {
    pub fn parse(s: &str) -> Result<Self, DomainError> {
        // 解析 "AA:BB:CC:DD:EE:FF" 格式
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 6 {
            return Err(DomainError::InvalidMacAddress);
        }
        let mut bytes = [0u8; 6];
        for (i, part) in parts.iter().enumerate() {
            bytes[i] = u8::from_str_radix(part, 16)
                .map_err(|_| DomainError::InvalidMacAddress)?;
        }
        Ok(Self(bytes))
    }
}

// 物理位置
pub struct Location {
    name: String,
    room_id: Option<String>,
    coordinates: Option<Coordinates3D>,
}

pub struct Coordinates3D {
    x: f64,
    y: f64,
    z: f64,
}
```

### 域服务

```rust
pub trait DeviceDiscovery {
    async fn scan(&self, timeout: Duration) -> Vec<DiscoveredDevice>;
    async fn identify(&self, address: &IpAddress) -> Option<DeviceType>;
}

pub trait DeviceConnector {
    async fn connect(&self, device: &Device) -> Result<DeviceConnection>;
    async fn disconnect(&self, device_id: &DeviceId) -> Result<()>;
    async fn reconnect(&self, device_id: &DeviceId) -> Result<DeviceConnection>;
}

pub trait DeviceConfigurator {
    async fn apply_config(&self, device_id: &DeviceId, config: &DeviceConfig) -> Result<()>;
    async fn read_config(&self, device_id: &DeviceId) -> Result<DeviceConfig>;
    async fn reset_to_defaults(&self, device_id: &DeviceId) -> Result<()>;
}

pub trait CalibrationService {
    async fn start_calibration(&self, device_id: &DeviceId) -> Result<CalibrationSession>;
    async fn get_calibration_status(&self, session_id: &CalibrationSessionId) -> CalibrationStatus;
    async fn apply_calibration(&self, device_id: &DeviceId, data: &CalibrationData) -> Result<()>;
}

pub trait HealthMonitor {
    async fn check_health(&self, device_id: &DeviceId) -> HealthStatus;
    async fn get_metrics(&self, device_id: &DeviceId) -> DeviceMetrics;
}
```

### 出站事件

- `DeviceDiscovered` - 在网络上发现新设备
- `DeviceConnected` - 连接已建立
- `DeviceDisconnected` - 连接丢失
- `DeviceConfigured` - 配置已应用
- `DeviceCalibrated` - 校准已完成
- `DeviceHealthChanged` - 状态变更（健康/不健康）
- `DeviceError` - 检测到错误条件

### 集成点

| 上下文 | 方向 | 机制 |
|---------|------|------|
| 信号域 | 出站 | 通过通道的原始 CSI 字节 |
| 存储域 | 出站 | 设备持久化 |
| API 层 | 双向 | 用于管理的 REST 端点 |

---

## 上下文集成模式

### 防腐层

当与供应商特定的 CSI 格式集成时，信号域使用防腐层来转换外部格式：

```rust
pub trait CsiParser: Send + Sync {
    fn parse(&self, raw: &[u8]) -> Result<CsiFrame>;
    fn device_type(&self) -> DeviceType;
}

pub struct Esp32Parser;
pub struct AtheriosParser;
pub struct IntelParser;

pub struct ParserRegistry {
    parsers: HashMap<DeviceType, Box<dyn CsiParser>>,
}
```

### 发布语言

姿态域以标准化格式发布事件，供其他上下文使用：

```rust
#[derive(Serialize, Deserialize)]
pub struct PoseEventPayload {
    pub event_type: String,
    pub version: String,
    pub timestamp: DateTime<Utc>,
    pub correlation_id: Uuid,
    pub payload: PoseEstimate,
}
```

### 共享内核

`wifi-densepose-core` crate 包含所有上下文使用的共享类型：

- 标识符：`DeviceId`、`SessionId`、`FrameId`、`EstimateId`
- 时间戳：`Timestamp`、`Duration`
- 常见错误：`DomainError`
- 配置：`ConfigurationLoader`