# 域事件

本文档目录了 WiFi-DensePose 系统中的所有域事件。域事件表示域内发生的重要事件，系统的其他部分可能需要对这些事件做出反应。

---

## 事件设计原则

### 事件结构

所有域事件遵循一致的结构：

```rust
/// 所有域事件的基础特征
pub trait DomainEvent: Send + Sync + 'static {
    /// 唯一事件类型标识符
    fn event_type(&self) -> &'static str;

    /// 事件发生时间
    fn occurred_at(&self) -> DateTime<Utc>;

    /// 用于追踪的关联 ID
    fn correlation_id(&self) -> Option<Uuid>;

    /// 产生事件的聚合 ID
    fn aggregate_id(&self) -> String;

    /// 事件模式版本用于演化
    fn version(&self) -> u32 { 1 }
}

/// 用于序列化和传输的事件信封
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope<E: DomainEvent> {
    pub id: Uuid,
    pub event_type: String,
    pub aggregate_id: String,
    pub aggregate_type: String,
    pub sequence_number: u64,
    pub occurred_at: DateTime<Utc>,
    pub correlation_id: Option<Uuid>,
    pub causation_id: Option<Uuid>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub payload: E,
}
```

### 事件命名约定

- 使用过去时态：`CsiFrameReceived`，而不是 `ReceiveCsiFrame`
- 包含聚合名称：`Device` + `Connected` = `DeviceConnected`
- 具体明确：`FallDetected`，而不是 `AlertRaised`

---

## 信号域事件

### CsiFrameReceived

当从硬件接收到原始 CSI 数据时发出。

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsiFrameReceived {
    /// 唯一帧标识符
    pub frame_id: FrameId,

    /// 源设备
    pub device_id: DeviceId,

    /// 关联会话（如果有）
    pub session_id: Option<SessionId>,

    /// 帧序列号
    pub sequence_number: u64,

    /// 接收时间戳
    pub timestamp: DateTime<Utc>,

    /// 帧维度
    pub num_subcarriers: u16,
    pub num_antennas: u8,

    /// 信号质量
    pub snr_db: f64,

    /// 原始数据大小（字节）
    pub payload_size: usize,
}

impl DomainEvent for CsiFrameReceived {
    fn event_type(&self) -> &'static str { "signal.csi_frame_received" }
    fn occurred_at(&self) -> DateTime<Utc> { self.timestamp }
    fn correlation_id(&self) -> Option<Uuid> { self.session_id.map(|s| s.0) }
    fn aggregate_id(&self) -> String { self.frame_id.0.to_string() }
}
```

**生产者：** 硬件域（CSI 提取器）
**消费者：** 信号域（预处理器），存储域（如果启用持久化）

---

### CsiFrameValidated

当 CSI 帧通过完整性验证时发出。

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsiFrameValidated {
    pub frame_id: FrameId,
    pub device_id: DeviceId,
    pub timestamp: DateTime<Utc>,

    /// 验证结果
    pub quality_score: f32,
    pub is_complete: bool,
    pub validation_time_us: u64,

    /// 检测到的问题（如果有）
    pub warnings: Vec<ValidationWarning>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    pub code: String,
    pub message: String,
    pub severity: WarningSeverity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WarningSeverity {
    Info,
    Warning,
    Error,
}
```

**生产者：** 信号域（验证器）
**消费者：** 信号域（预处理器）

---

### SignalProcessed

当 CSI 特征已提取且信号准备好用于推理时发出。

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalProcessed {
    /// 处理后信号标识符
    pub signal_id: SignalId,

    /// 源帧
    pub source_frames: Vec<FrameId>,

    /// 源设备
    pub device_id: DeviceId,

    /// 关联会话
    pub session_id: Option<SessionId>,

    /// 处理时间戳
    pub timestamp: DateTime<Utc>,

    /// 处理窗口
    pub window_start: DateTime<Utc>,
    pub window_end: DateTime<Utc>,

    /// 特征摘要（非完整数据）
    pub feature_summary: FeatureSummary,

    /// 人体存在检测
    pub human_detected: bool,
    pub presence_confidence: f32,
    pub estimated_person_count: Option<u8>,

    /// 质量指标
    pub quality_score: f32,

    /// 处理性能
    pub processing_time_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureSummary {
    pub amplitude_mean: f32,
    pub amplitude_std: f32,
    pub phase_variance: f32,
    pub dominant_frequency_hz: f32,
    pub motion_indicator: f32,
}
```

**生产者：** 信号域（特征提取器）
**消费者：** 姿态域（推理引擎），流传输域（如果启用 CSI 流）

---

### SignalProcessingFailed

当信号处理失败时发出。

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalProcessingFailed {
    pub frame_id: FrameId,
    pub device_id: DeviceId,
    pub timestamp: DateTime<Utc>,

    /// 错误详情
    pub error_code: String,
    pub error_message: String,
    pub error_category: ProcessingErrorCategory,

    /// 恢复建议
    pub recoverable: bool,
    pub suggested_action: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessingErrorCategory {
    InvalidData,
    InsufficientQuality,
    CalibrationRequired,
    ResourceExhausted,
    InternalError,
}
```

**生产者：** 信号域
**消费者：** 监控，警报

---

## 姿态域事件

### PoseEstimated

当姿态推理成功完成时发出。

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoseEstimated {
    /// 估计标识符
    pub estimate_id: EstimateId,

    /// 源信号
    pub signal_id: SignalId,

    /// 会话上下文
    pub session_id: SessionId,

    /// 区域（如果适用）
    pub zone_id: Option<ZoneId>,

    /// 估计时间戳
    pub timestamp: DateTime<Utc>,

    /// 会话中的帧编号
    pub frame_number: u64,

    /// 检测结果摘要
    pub person_count: u8,
    pub persons: Vec<PersonSummary>,

    /// 置信度指标
    pub overall_confidence: f32,

    /// 处理性能
    pub processing_time_ms: f64,
    pub model_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonSummary {
    pub person_id: PersonId,
    pub bounding_box: BoundingBoxDto,
    pub confidence: f32,
    pub activity: String,
    pub keypoint_count: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundingBoxDto {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}
```

**生产者：** 姿态域（推理引擎）
**消费者：** 流传输域，存储域，监控

---

### PersonDetected

当新人进入检测区域时发出。

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonDetected {
    /// 人员标识符（跟踪 ID）
    pub person_id: PersonId,

    /// 检测上下文
    pub session_id: SessionId,
    pub zone_id: Option<ZoneId>,
    pub estimate_id: EstimateId,

    /// 检测详情
    pub timestamp: DateTime<Utc>,
    pub confidence: f32,
    pub bounding_box: BoundingBoxDto,

    /// 初始活动分类
    pub initial_activity: String,

    /// 入口点（如果可跟踪）
    pub entry_position: Option<Position2DDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position2DDto {
    pub x: f32,
    pub y: f32,
}
```

**生产者：** 姿态域（跟踪器）
**消费者：** 流传输域，分析，警报

---

### PersonLost

当跟踪的人离开检测区域时发出。

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonLost {
    /// 人员标识符
    pub person_id: PersonId,

    /// 上下文
    pub session_id: SessionId,
    pub zone_id: Option<ZoneId>,

    /// 时间
    pub timestamp: DateTime<Utc>,
    pub first_seen: DateTime<Utc>,
    pub duration_seconds: f64,

    /// 离开详情
    pub last_position: Option<Position2DDto>,
    pub last_activity: String,

    /// 跟踪统计
    pub total_frames_tracked: u64,
    pub average_confidence: f32,
}
```

**生产者：** 姿态域（跟踪器）
**消费者：** 流传输域，分析

---

### ActivityChanged

当人的分类活动发生变化时发出。

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityChanged {
    pub person_id: PersonId,
    pub session_id: SessionId,
    pub zone_id: Option<ZoneId>,
    pub timestamp: DateTime<Utc>,

    /// 活动转换
    pub previous_activity: String,
    pub new_activity: String,

    /// 对新分类的置信度
    pub confidence: f32,

    /// 之前活动的持续时间
    pub previous_activity_duration_seconds: f64,
}
```

**生产者：** 姿态域（活动分类器）
**消费者：** 流传输域，分析，警报（针对某些转换）

---

### MotionDetected

当在区域中检测到显著运动时发出。

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotionDetected {
    /// 事件标识
    pub event_id: Uuid,

    /// 上下文
    pub session_id: Option<SessionId>,
    pub zone_id: Option<ZoneId>,
    pub device_id: DeviceId,

    /// 检测详情
    pub timestamp: DateTime<Utc>,
    pub motion_score: f32,
    pub motion_type: MotionType,

    /// 相关人员（如果可识别）
    pub person_ids: Vec<PersonId>,
    pub person_count: u8,

    /// 运动特征
    pub velocity_estimate: Option<f32>,
    pub direction: Option<f32>, // 角度（弧度）
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MotionType {
    /// 一般运动
    General,
    /// 步行运动模式
    Walking,
    /// 跑步运动模式
    Running,
    /// 突然/快速运动
    Sudden,
    /// 重复运动
    Repetitive,
}
```

**生产者：** 姿态域，信号域（用于基于 CSI 的运动）
**消费者：** 流传输域，警报，分析

---

### FallDetected

当检测到潜在的跌倒事件时发出。这是一个关键警报事件。

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallDetected {
    /// 事件标识
    pub event_id: Uuid,

    /// 涉及的人员
    pub person_id: PersonId,

    /// 上下文
    pub session_id: SessionId,
    pub zone_id: Option<ZoneId>,

    /// 检测详情
    pub timestamp: DateTime<Utc>,
    pub confidence: f32,

    /// 跌倒特征
    pub fall_type: FallType,
    pub duration_ms: Option<u64>,
    pub impact_severity: ImpactSeverity,

    /// 位置信息
    pub fall_location: Option<Position2DDto>,
    pub pre_fall_activity: String,

    /// 验证状态
    pub requires_verification: bool,
    pub auto_alert_sent: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FallType {
    /// 向前跌倒
    Forward,
    /// 向后跌倒
    Backward,
    /// 侧面跌倒
    Lateral,
    /// 逐渐降低（坐/躺）
    Gradual,
    /// 未知模式
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImpactSeverity {
    Low,
    Medium,
    High,
    Critical,
}
```

**生产者：** 姿态域（跌倒检测器）
**消费者：** 警报（高优先级），流传输域，存储域

---

## 流传输域事件

### SessionStarted

当客户端建立流传输会话时发出。

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStarted {
    pub session_id: SessionId,
    pub client_id: ClientId,
    pub timestamp: DateTime<Utc>,

    /// 连接详情
    pub stream_type: String,
    pub remote_addr: Option<String>,
    pub user_agent: Option<String>,

    /// 初始订阅
    pub zone_subscriptions: Vec<String>,
    pub filters: SubscriptionFiltersDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionFiltersDto {
    pub min_confidence: Option<f32>,
    pub max_persons: Option<u8>,
    pub include_keypoints: bool,
    pub include_segmentation: bool,
    pub throttle_ms: Option<u32>,
}
```

**生产者：** 流传输域（连接管理器）
**消费者：** 监控，分析

---

### SessionEnded

当流传输会话终止时发出。

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEnded {
    pub session_id: SessionId,
    pub client_id: ClientId,
    pub timestamp: DateTime<Utc>,

    /// 会话持续时间
    pub started_at: DateTime<Utc>,
    pub duration_seconds: f64,

    /// 终止原因
    pub reason: SessionEndReason,
    pub error_message: Option<String>,

    /// 会话统计
    pub messages_sent: u64,
    pub messages_failed: u64,
    pub bytes_sent: u64,
    pub average_latency_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionEndReason {
    ClientDisconnect,
    ServerShutdown,
    Timeout,
    Error,
    Evicted,
}
```

**生产者：** 流传输域（连接管理器）
**消费者：** 监控，分析

---

### SubscriptionUpdated

当客户端更改其订阅过滤器时发出。

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionUpdated {
    pub session_id: SessionId,
    pub timestamp: DateTime<Utc>,

    /// 旧过滤器
    pub previous_filters: SubscriptionFiltersDto,

    /// 新过滤器
    pub new_filters: SubscriptionFiltersDto,

    /// 区域更改
    pub zones_added: Vec<String>,
    pub zones_removed: Vec<String>,
}
```

**生产者：** 流传输域
**消费者：** 监控

---

### MessageDelivered

用于跟踪消息传递（可选，高容量）。

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageDelivered {
    pub session_id: SessionId,
    pub message_id: Uuid,
    pub timestamp: DateTime<Utc>,

    pub message_type: String,
    pub payload_bytes: usize,
    pub latency_ms: f64,
}
```

**生产者：** 流传输域
**消费者：** 指标收集器

---

### MessageDeliveryFailed

当消息传递失败时发出。

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageDeliveryFailed {
    pub session_id: SessionId,
    pub message_id: Uuid,
    pub timestamp: DateTime<Utc>,

    pub message_type: String,
    pub error_code: String,
    pub error_message: String,
    pub retry_count: u8,
    pub will_retry: bool,
}
```

**生产者：** 流传输域
**消费者：** 监控，警报

---

## 硬件域事件

### DeviceDiscovered

当在网络上发现新设备时发出。

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceDiscovered {
    pub discovery_id: Uuid,
    pub timestamp: DateTime<Utc>,

    /// 设备识别
    pub mac_address: String,
    pub ip_address: Option<String>,
    pub device_type: String,

    /// 发现的能力
    pub capabilities: DeviceCapabilitiesDto,

    /// 固件信息
    pub firmware_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCapabilitiesDto {
    pub max_subcarriers: u16,
    pub max_antennas: u8,
    pub supported_bandwidths: Vec<String>,
    pub max_sampling_rate_hz: u32,
}
```

**生产者：** 硬件域（发现服务）
**消费者：** 设备管理 UI，自动配置

---

### DeviceConnected

当与设备建立连接时发出。

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConnected {
    pub device_id: DeviceId,
    pub timestamp: DateTime<Utc>,

    /// 连接详情
    pub ip_address: String,
    pub protocol: String,
    pub connection_time_ms: u64,

    /// 设备状态
    pub firmware_version: Option<String>,
    pub current_config: DeviceConfigDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConfigDto {
    pub sampling_rate_hz: u32,
    pub subcarriers: u16,
    pub antennas: u8,
    pub bandwidth: String,
    pub channel: u8,
}
```

**生产者：** 硬件域（设备连接器）
**消费者：** 信号域，监控

---

### DeviceDisconnected

当与设备的连接丢失时发出。

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceDisconnected {
    pub device_id: DeviceId,
    pub timestamp: DateTime<Utc>,

    /// 断开连接详情
    pub reason: DisconnectReason,
    pub error_message: Option<String>,

    /// 会话统计
    pub connected_since: DateTime<Utc>,
    pub uptime_seconds: f64,
    pub frames_transmitted: u64,
    pub errors_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DisconnectReason {
    Graceful,
    ConnectionLost,
    Timeout,
    Error,
    MaintenanceMode,
}
```

**生产者：** 硬件域
**消费者：** 信号域，警报，监控

---

### DeviceConfigured

当应用设备配置时发出。

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConfigured {
    pub device_id: DeviceId,
    pub timestamp: DateTime<Utc>,

    /// 应用的配置
    pub config: DeviceConfigDto,

    /// 之前的配置
    pub previous_config: Option<DeviceConfigDto>,

    /// 配置来源
    pub source: ConfigurationSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConfigurationSource {
    Api,
    AutoConfig,
    Calibration,
    Default,
}
```

**生产者：** 硬件域（配置器）
**消费者：** 监控

---

### DeviceCalibrated

当设备校准完成时发出。

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCalibrated {
    pub device_id: DeviceId,
    pub calibration_id: Uuid,
    pub timestamp: DateTime<Utc>,

    /// 校准结果
    pub success: bool,
    pub calibration_type: String,
    pub duration_seconds: f64,

    /// 校准参数
    pub noise_floor_db: f64,
    pub antenna_offsets: Vec<f64>,
    pub phase_correction: Vec<f64>,

    /// 质量指标
    pub quality_before: f32,
    pub quality_after: f32,
    pub improvement_percent: f32,
}
```

**生产者：** 硬件域（校准服务）
**消费者：** 信号域，监控

---

### DeviceHealthChanged

当设备健康状态变化时发出。

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceHealthChanged {
    pub device_id: DeviceId,
    pub timestamp: DateTime<Utc>,

    /// 健康转换
    pub previous_status: String,
    pub new_status: String,

    /// 健康指标
    pub cpu_usage_percent: Option<f32>,
    pub memory_usage_percent: Option<f32>,
    pub temperature_celsius: Option<f32>,
    pub error_rate: Option<f32>,

    /// 连续失败
    pub failure_count: u8,

    /// 建议操作
    pub recommended_action: Option<String>,
}
```

**生产者：** 硬件域（健康监控器）
**消费者：** 警报，监控

---

### DeviceError

当设备遇到错误条件时发出。

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceError {
    pub device_id: DeviceId,
    pub timestamp: DateTime<Utc>,

    /// 错误详情
    pub error_code: String,
    pub error_message: String,
    pub error_category: DeviceErrorCategory,

    /// 上下文
    pub operation: String,
    pub stack_trace: Option<String>,

    /// 恢复
    pub recoverable: bool,
    pub retry_after_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeviceErrorCategory {
    Connection,
    Configuration,
    Hardware,
    Firmware,
    Protocol,
    Resource,
    Unknown,
}
```

**生产者：** 硬件域
**消费者：** 警报，监控，自动恢复

---

## 事件流程图

### CSI 到姿态 Pipeline

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           事件流：CSI 到姿态                           │
└─────────────────────────────────────────────────────────────────────────────┘

  硬件          信号域         姿态域         流传输
  ─────────         ─────────────         ───────────         ─────────

     │                    │                    │                   │
     │ CsiFrameReceived   │                    │                   │
     │───────────────────>│                    │                   │
     │                    │                    │                   │
     │                    │ CsiFrameValidated  │                   │
     │                    │─────────┐          │                   │
     │                    │         │          │                   │
     │                    │<────────┘          │                   │
     │                    │                    │                   │
     │                    │ SignalProcessed    │                   │
     │                    │───────────────────>│                   │
     │                    │                    │                   │
     │                    │                    │ PoseEstimated     │
     │                    │                    │──────────────────>│
     │                    │                    │                   │
     │                    │                    │ [if detected]     │
     │                    │                    │                   │
     │                    │                    │ MotionDetected    │
     │                    │                    │──────────────────>│
     │                    │                    │                   │
     │                    │                    │ FallDetected      │
     │                    │                    │──────────────────>│
     │                    │                    │                   │
```

### 会话生命周期

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        事件流：会话生命周期                        │
└─────────────────────────────────────────────────────────────────────────────┘

  客户端              流传输域              姿态域
  ──────              ────────────────              ───────────

     │                       │                           │
     │  WebSocket 连接    │                           │
     │──────────────────────>│                           │
     │                       │                           │
     │                       │ SessionStarted            │
     │                       │───────────┐               │
     │                       │           │               │
     │                       │<──────────┘               │
     │                       │                           │
     │  订阅区域           │                           │
     │──────────────────────>│                           │
     │                       │                           │
     │                       │ SubscriptionUpdated       │
     │                       │───────────┐               │
     │                       │           │               │
     │                       │<──────────┘               │
     │                       │                           │
     │                       │          PoseEstimated    │
     │                       │<──────────────────────────│
     │                       │                           │
     │  姿态数据            │                           │
     │<──────────────────────│                           │
     │                       │                           │
     │  断开连接           │                           │
     │──────────────────────>│                           │
     │                       │                           │
     │                       │ SessionEnded              │
     │                       │───────────┐               │
     │                       │           │               │
     │                       │<──────────┘               │
```

---

## 事件总线实现

### 事件发布器

```rust
/// 发布域事件的特征
#[async_trait]
pub trait EventPublisher: Send + Sync {
    /// 发布单个事件
    async fn publish<E: DomainEvent + Serialize>(&self, event: E) -> Result<(), EventError>;

    /// 原子发布多个事件
    async fn publish_batch<E: DomainEvent + Serialize>(&self, events: Vec<E>) -> Result<(), EventError>;
}

/// 开发用内存事件总线
pub struct InMemoryEventBus {
    subscribers: RwLock<HashMap<String, Vec<Box<dyn EventHandler>>>>,
}

/// 生产用基于 Redis 的事件总线
pub struct RedisEventBus {
    client: redis::Client,
    stream_name: String,
}

/// 高吞吐量基于 Kafka 的事件总线
pub struct KafkaEventBus {
    producer: FutureProducer,
    topic_prefix: String,
}
```

### 事件处理器

```rust
/// 处理域事件的特征
#[async_trait]
pub trait EventHandler: Send + Sync {
    /// 此处理器感兴趣的事件类型
    fn event_types(&self) -> Vec<&'static str>;

    /// 处理事件
    async fn handle(&self, event: EventEnvelope<serde_json::Value>) -> Result<(), EventError>;
}

/// 跌倒检测警报的示例处理器
pub struct FallAlertHandler {
    notifier: Arc<dyn AlertNotifier>,
}

#[async_trait]
impl EventHandler for FallAlertHandler {
    fn event_types(&self) -> Vec<&'static str> {
        vec!["pose.fall_detected"]
    }

    async fn handle(&self, event: EventEnvelope<serde_json::Value>) -> Result<(), EventError> {
        let fall_event: FallDetected = serde_json::from_value(event.payload)?;

        if fall_event.confidence > 0.8 {
            self.notifier.send_alert(Alert {
                severity: AlertSeverity::Critical,
                title: "Fall Detected".to_string(),
                message: format!(
                    "Person {} detected falling in zone {:?}",
                    fall_event.person_id.0,
                    fall_event.zone_id
                ),
                timestamp: fall_event.timestamp,
            }).await?;
        }

        Ok(())
    }
}
```

---

## 事件版本控制

事件随时间演变。使用显式版本控制：

```rust
/// PoseEstimated 版本 1
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoseEstimatedV1 {
    pub estimate_id: EstimateId,
    pub person_count: u8,
    pub confidence: f32,
    pub timestamp: DateTime<Utc>,
}

/// 版本 2 添加区域支持
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoseEstimatedV2 {
    pub estimate_id: EstimateId,
    pub signal_id: SignalId,  // 新增
    pub zone_id: Option<ZoneId>,  // 新增
    pub person_count: u8,
    pub persons: Vec<PersonSummary>,  // 从仅计数更改
    pub overall_confidence: f32,  // 重命名
    pub timestamp: DateTime<Utc>,
}

/// 用于迁移的事件升级器
pub trait EventUpgrader {
    fn upgrade_v1_to_v2(v1: PoseEstimatedV1) -> PoseEstimatedV2 {
        PoseEstimatedV2 {
            estimate_id: v1.estimate_id,
            signal_id: SignalId(Uuid::nil()),  // 未知
            zone_id: None,  // V1 中不可用
            person_count: v1.person_count,
            persons: vec![],  // 无法重建
            overall_confidence: v1.confidence,
            timestamp: v1.timestamp,
        }
    }
}
```

---

## 事件溯源支持

对于需要完整审计跟踪的聚合：

```rust
/// 事件存储接口
#[async_trait]
pub trait EventStore: Send + Sync {
    /// 将事件追加到聚合流
    async fn append(
        &self,
        aggregate_type: &str,
        aggregate_id: &str,
        expected_version: u64,
        events: Vec<EventEnvelope<serde_json::Value>>,
    ) -> Result<u64, EventStoreError>;

    /// 加载聚合的所有事件
    async fn load(
        &self,
        aggregate_type: &str,
        aggregate_id: &str,
    ) -> Result<Vec<EventEnvelope<serde_json::Value>>, EventStoreError>;

    /// 从特定版本加载事件
    async fn load_from_version(
        &self,
        aggregate_type: &str,
        aggregate_id: &str,
        from_version: u64,
    ) -> Result<Vec<EventEnvelope<serde_json::Value>>, EventStoreError>;
}

/// 从事件重建聚合
pub trait EventSourced: Sized {
    fn apply(&mut self, event: &dyn DomainEvent);

    fn replay(events: Vec<EventEnvelope<serde_json::Value>>) -> Result<Self, ReplayError>;
}
```