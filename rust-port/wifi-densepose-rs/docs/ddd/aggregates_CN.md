# 聚合根

本文档定义了 WiFi-DensePose 系统中的核心聚合根。每个聚合根是一组领域对象的集合，在数据变更时被视为一个单一单元，其中一个实体被指定为聚合根。

---

## 设计原则

### 聚合根不变量

1. **事务一致性** - 聚合根内的所有变更都是原子性的
2. **标识** - 每个聚合根都有唯一标识符
3. **封装** - 内部实体只能通过根访问
4. **最终一致性** - 跨聚合引用使用 ID，而非直接引用

### Rust 实现模式

```rust
// 带有私有构造函数的聚合根，强制执行不变量
pub struct AggregateRoot {
    id: AggregateId,
    // ... 字段
}

impl AggregateRoot {
    // 强制执行不变量的工厂方法
    pub fn create(params: CreateParams) -> Result<Self, DomainError> {
        // 验证不变量
        Self::validate(&params)?;

        Ok(Self {
            id: AggregateId::generate(),
            // ... 初始化字段
        })
    }

    // 命令返回领域事件
    pub fn handle_command(&mut self, cmd: Command) -> Result<Vec<DomainEvent>, DomainError> {
        // 根据当前状态验证命令
        // 应用状态变更
        // 返回事件
    }
}
```

---

## 1. CsiFrame 聚合根

### 目的

表示从 WiFi 硬件捕获的单个信道状态信息 (CSI)。这是通过信号处理 pipeline 流动的基础数据结构。

### 聚合根：CsiFrame

```rust
use chrono::{DateTime, Utc};
use uuid::Uuid;
use ndarray::Array2;

/// CSI 帧数据的聚合根
#[derive(Debug, Clone)]
pub struct CsiFrame {
    // 标识
    id: FrameId,

    // 关系（通过 ID，而非引用）
    device_id: DeviceId,
    session_id: Option<SessionId>,

    // 时间
    timestamp: DateTime<Utc>,
    sequence_number: u64,

    // 核心 CSI 数据（创建后不可变）
    amplitude: Array2<f32>,  // [antennas, subcarriers]
    phase: Array2<f32>,      // [antennas, subcarriers]

    // 信号参数
    frequency: Frequency,
    bandwidth: Bandwidth,

    // 维度
    num_subcarriers: u16,
    num_antennas: u8,

    // 质量指标
    snr: SignalToNoise,
    rssi: Option<Rssi>,
    noise_floor: Option<NoiseFloor>,

    // 处理状态
    status: ProcessingStatus,
    processed_at: Option<DateTime<Utc>>,

    // 可扩展元数据
    metadata: FrameMetadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FrameId(Uuid);

impl FrameId {
    pub fn generate() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }
}
```

### 值对象

```rust
/// 中心频率（Hz）（必须为正数）
#[derive(Debug, Clone, Copy)]
pub struct Frequency(f64);

impl Frequency {
    pub fn new(hz: f64) -> Result<Self, DomainError> {
        if hz <= 0.0 {
            return Err(DomainError::InvalidFrequency { value: hz });
        }
        Ok(Self(hz))
    }

    pub fn as_hz(&self) -> f64 {
        self.0
    }

    pub fn as_ghz(&self) -> f64 {
        self.0 / 1_000_000_000.0
    }

    /// 常见 WiFi 频率
    pub fn wifi_2_4ghz() -> Self {
        Self(2_400_000_000.0)
    }

    pub fn wifi_5ghz() -> Self {
        Self(5_000_000_000.0)
    }
}

/// 信道带宽
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bandwidth {
    Bw20MHz,
    Bw40MHz,
    Bw80MHz,
    Bw160MHz,
}

impl Bandwidth {
    pub fn as_hz(&self) -> f64 {
        match self {
            Self::Bw20MHz => 20_000_000.0,
            Self::Bw40MHz => 40_000_000.0,
            Self::Bw80MHz => 80_000_000.0,
            Self::Bw160MHz => 160_000_000.0,
        }
    }

    pub fn expected_subcarriers(&self) -> u16 {
        match self {
            Self::Bw20MHz => 56,
            Self::Bw40MHz => 114,
            Self::Bw80MHz => 242,
            Self::Bw160MHz => 484,
        }
    }
}

/// 信噪比（dB）
#[derive(Debug, Clone, Copy)]
pub struct SignalToNoise(f64);

impl SignalToNoise {
    const MIN_DB: f64 = -50.0;
    const MAX_DB: f64 = 50.0;

    pub fn new(db: f64) -> Result<Self, DomainError> {
        if db < Self::MIN_DB || db > Self::MAX_DB {
            return Err(DomainError::InvalidSnr { value: db });
        }
        Ok(Self(db))
    }

    pub fn as_db(&self) -> f64 {
        self.0
    }

    pub fn is_good(&self) -> bool {
        self.0 >= 20.0
    }

    pub fn is_acceptable(&self) -> bool {
        self.0 >= 10.0
    }
}

/// 处理 pipeline 状态
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessingStatus {
    Pending,
    Preprocessing,
    FeatureExtraction,
    Completed,
    Failed { reason: String },
}
```

### 不变量

1. 幅度和相位数组必须具有匹配的维度
2. 维度必须匹配 num_subcarriers x num_antennas
3. 频率必须为正数
4. SNR 必须在合理范围内（-50 到 +50 dB）
5. 序列号在每个会话中单调递增

### 工厂方法

```rust
impl CsiFrame {
    /// 创建带有验证的新 CSI 帧
    pub fn create(params: CreateCsiFrameParams) -> Result<Self, DomainError> {
        // 验证维度
        let (rows, cols) = params.amplitude.dim();
        if rows != params.num_antennas as usize || cols != params.num_subcarriers as usize {
            return Err(DomainError::DimensionMismatch {
                expected_antennas: params.num_antennas,
                expected_subcarriers: params.num_subcarriers,
                actual_rows: rows,
                actual_cols: cols,
            });
        }

        // 验证相位维度与幅度匹配
        if params.amplitude.dim() != params.phase.dim() {
            return Err(DomainError::PhaseDimensionMismatch);
        }

        Ok(Self {
            id: FrameId::generate(),
            device_id: params.device_id,
            session_id: params.session_id,
            timestamp: Utc::now(),
            sequence_number: params.sequence_number,
            amplitude: params.amplitude,
            phase: params.phase,
            frequency: params.frequency,
            bandwidth: params.bandwidth,
            num_subcarriers: params.num_subcarriers,
            num_antennas: params.num_antennas,
            snr: params.snr,
            rssi: params.rssi,
            noise_floor: params.noise_floor,
            status: ProcessingStatus::Pending,
            processed_at: None,
            metadata: params.metadata.unwrap_or_default(),
        })
    }

    /// 从持久化重构（绕过验证）
    pub(crate) fn reconstitute(/* all fields */) -> Self {
        // 由存储库实现使用
        // 假设数据在创建时已验证
    }
}
```

### 命令

```rust
impl CsiFrame {
    /// 将帧标记为正在预处理
    pub fn start_preprocessing(&mut self) -> Result<CsiFramePreprocessingStarted, DomainError> {
        match &self.status {
            ProcessingStatus::Pending => {
                self.status = ProcessingStatus::Preprocessing;
                Ok(CsiFramePreprocessingStarted {
                    frame_id: self.id,
                    timestamp: Utc::now(),
                })
            }
            _ => Err(DomainError::InvalidStateTransition {
                from: format!("{:?}", self.status),
                to: "Preprocessing".to_string(),
            }),
        }
    }

    /// 将帧标记为已提取特征
    pub fn complete_feature_extraction(&mut self) -> Result<CsiFrameProcessed, DomainError> {
        match &self.status {
            ProcessingStatus::Preprocessing | ProcessingStatus::FeatureExtraction => {
                self.status = ProcessingStatus::Completed;
                self.processed_at = Some(Utc::now());
                Ok(CsiFrameProcessed {
                    frame_id: self.id,
                    processed_at: self.processed_at.unwrap(),
                })
            }
            _ => Err(DomainError::InvalidStateTransition {
                from: format!("{:?}", self.status),
                to: "Completed".to_string(),
            }),
        }
    }

    /// 将帧标记为失败
    pub fn fail(&mut self, reason: String) -> CsiFrameProcessingFailed {
        self.status = ProcessingStatus::Failed { reason: reason.clone() };
        CsiFrameProcessingFailed {
            frame_id: self.id,
            reason,
            timestamp: Utc::now(),
        }
    }
}
```

---

## 2. ProcessedSignal 聚合根

### 目的

表示从一个或多个 CSI 帧中提取的特征，准备用于姿态推理。这是信号领域的输出和姿态领域的输入。

### 聚合根：ProcessedSignal

```rust
/// 处理后信号特征的聚合根
#[derive(Debug, Clone)]
pub struct ProcessedSignal {
    // 标识
    id: SignalId,

    // 源帧
    source_frames: Vec<FrameId>,
    device_id: DeviceId,
    session_id: Option<SessionId>,

    // 时间
    timestamp: DateTime<Utc>,
    window_start: DateTime<Utc>,
    window_end: DateTime<Utc>,

    // 提取的特征
    features: SignalFeatures,

    // 人体检测结果
    human_presence: HumanPresenceResult,

    // 质量评估
    quality_score: QualityScore,

    // 处理元数据
    processing_config: ProcessingConfig,
    extraction_time: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SignalId(Uuid);

/// 提取的信号特征集合
#[derive(Debug, Clone)]
pub struct SignalFeatures {
    // 幅度特征
    pub amplitude_mean: Array1<f32>,
    pub amplitude_variance: Array1<f32>,
    pub amplitude_skewness: Array1<f32>,
    pub amplitude_kurtosis: Array1<f32>,

    // 相位特征
    pub phase_difference: Array1<f32>,
    pub phase_unwrapped: Array2<f32>,

    // 相关特征
    pub antenna_correlation: Array2<f32>,
    pub subcarrier_correlation: Array2<f32>,

    // 频域特征
    pub doppler_shift: Array1<f32>,
    pub power_spectral_density: Array1<f32>,
    pub dominant_frequencies: Vec<f32>,

    // 时域特征（如果有多个帧）
    pub temporal_variance: Option<Array1<f32>>,
    pub motion_indicators: Option<MotionIndicators>,
}

/// 人体存在检测结果
#[derive(Debug, Clone)]
pub struct HumanPresenceResult {
    pub detected: bool,
    pub confidence: Confidence,
    pub motion_score: f32,
    pub estimated_count: Option<u8>,
}

/// 信号质量评估
#[derive(Debug, Clone, Copy)]
pub struct QualityScore(f32);

impl QualityScore {
    pub fn new(score: f32) -> Result<Self, DomainError> {
        if score < 0.0 || score > 1.0 {
            return Err(DomainError::InvalidQualityScore { value: score });
        }
        Ok(Self(score))
    }

    pub fn is_usable(&self) -> bool {
        self.0 >= 0.3
    }

    pub fn is_good(&self) -> bool {
        self.0 >= 0.7
    }
}
```

### 工厂方法

```rust
impl ProcessedSignal {
    /// 从提取的特征创建
    pub fn create(
        source_frames: Vec<FrameId>,
        device_id: DeviceId,
        session_id: Option<SessionId>,
        features: SignalFeatures,
        human_presence: HumanPresenceResult,
        processing_config: ProcessingConfig,
        extraction_time: Duration,
    ) -> Result<Self, DomainError> {
        if source_frames.is_empty() {
            return Err(DomainError::NoSourceFrames);
        }

        let quality_score = Self::calculate_quality(&features)?;

        Ok(Self {
            id: SignalId(Uuid::new_v4()),
            source_frames,
            device_id,
            session_id,
            timestamp: Utc::now(),
            window_start: Utc::now(), // TODO: 从帧计算
            window_end: Utc::now(),
            features,
            human_presence,
            quality_score,
            processing_config,
            extraction_time,
        })
    }

    fn calculate_quality(features: &SignalFeatures) -> Result<QualityScore, DomainError> {
        // 基于特征完整性和方差的质量评估
        let amplitude_quality = if features.amplitude_variance.iter().any(|&v| v > 0.0) {
            1.0
        } else {
            0.5
        };

        let phase_quality = if !features.phase_difference.is_empty() {
            1.0
        } else {
            0.3
        };

        let score = 0.6 * amplitude_quality + 0.4 * phase_quality;
        QualityScore::new(score)
    }
}
```

---

## 3. PoseEstimate 聚合根

### 目的

表示姿态推理的输出，包含检测到的人员及其身体配置、关键点和活动分类。

### 聚合根：PoseEstimate

```rust
/// 姿态估计结果的聚合根
#[derive(Debug, Clone)]
pub struct PoseEstimate {
    // 标识
    id: EstimateId,

    // 源引用
    signal_id: SignalId,
    session_id: SessionId,
    zone_id: Option<ZoneId>,

    // 时间
    timestamp: DateTime<Utc>,
    frame_number: u64,

    // 检测结果
    persons: Vec<PersonDetection>,
    person_count: u8,

    // 处理元数据
    processing_time: Duration,
    model_version: ModelVersion,
    algorithm: InferenceAlgorithm,

    // 质量指标
    overall_confidence: Confidence,
    is_valid: bool,

    // 估计过程中生成的事件
    detected_events: Vec<PoseEvent>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EstimateId(Uuid);

/// 带有完整姿态信息的检测人员
#[derive(Debug, Clone)]
pub struct PersonDetection {
    pub person_id: PersonId,
    pub bounding_box: BoundingBox,
    pub keypoints: KeypointSet,
    pub body_parts: Option<BodyPartSegmentation>,
    pub uv_coordinates: Option<UvMap>,
    pub confidence: Confidence,
    pub activity: Activity,
    pub velocity: Option<Velocity2D>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PersonId(u32);

/// 解剖学关键点集合
#[derive(Debug, Clone)]
pub struct KeypointSet {
    keypoints: HashMap<KeypointName, Keypoint>,
}

impl KeypointSet {
    pub fn new() -> Self {
        Self { keypoints: HashMap::new() }
    }

    pub fn add(&mut self, keypoint: Keypoint) {
        self.keypoints.insert(keypoint.name, keypoint);
    }

    pub fn get(&self, name: KeypointName) -> Option<&Keypoint> {
        self.keypoints.get(&name)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Keypoint> {
        self.keypoints.values()
    }

    pub fn visible_count(&self) -> usize {
        self.keypoints.values().filter(|k| k.is_visible()).count()
    }
}

/// 单个解剖学关键点
#[derive(Debug, Clone)]
pub struct Keypoint {
    pub name: KeypointName,
    pub position: Position2D,
    pub confidence: Confidence,
    pub is_occluded: bool,
}

impl Keypoint {
    pub fn is_visible(&self) -> bool {
        !self.is_occluded && self.confidence.value() > 0.5
    }
}

/// 遵循 COCO 格式的命名关键点位置
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

impl KeypointName {
    pub fn all() -> [Self; 17] {
        [
            Self::Nose,
            Self::LeftEye, Self::RightEye,
            Self::LeftEar, Self::RightEar,
            Self::LeftShoulder, Self::RightShoulder,
            Self::LeftElbow, Self::RightElbow,
            Self::LeftWrist, Self::RightWrist,
            Self::LeftHip, Self::RightHip,
            Self::LeftKnee, Self::RightKnee,
            Self::LeftAnkle, Self::RightAnkle,
        ]
    }
}
```

### 值对象

```rust
/// 置信度分数 [0, 1]
#[derive(Debug, Clone, Copy)]
pub struct Confidence(f32);

impl Confidence {
    pub fn new(value: f32) -> Result<Self, DomainError> {
        if value < 0.0 || value > 1.0 {
            return Err(DomainError::InvalidConfidence { value });
        }
        Ok(Self(value))
    }

    pub fn value(&self) -> f32 {
        self.0
    }

    pub fn is_high(&self) -> bool {
        self.0 >= 0.8
    }

    pub fn is_medium(&self) -> bool {
        self.0 >= 0.5 && self.0 < 0.8
    }

    pub fn is_low(&self) -> bool {
        self.0 < 0.5
    }
}

/// 归一化坐标 [0, 1] 中的 2D 位置
#[derive(Debug, Clone, Copy)]
pub struct Position2D {
    x: NormalizedCoordinate,
    y: NormalizedCoordinate,
}

#[derive(Debug, Clone, Copy)]
pub struct NormalizedCoordinate(f32);

impl NormalizedCoordinate {
    pub fn new(value: f32) -> Result<Self, DomainError> {
        if value < 0.0 || value > 1.0 {
            return Err(DomainError::CoordinateOutOfRange { value });
        }
        Ok(Self(value))
    }
}

/// 矩形边界框
#[derive(Debug, Clone, Copy)]
pub struct BoundingBox {
    pub x: NormalizedCoordinate,
    pub y: NormalizedCoordinate,
    pub width: f32,
    pub height: f32,
}

impl BoundingBox {
    pub fn area(&self) -> f32 {
        self.width * self.height
    }

    pub fn center(&self) -> Position2D {
        Position2D {
            x: NormalizedCoordinate(self.x.0 + self.width / 2.0),
            y: NormalizedCoordinate(self.y.0 + self.height / 2.0),
        }
    }
}

/// 分类活动
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Activity {
    Standing,
    Sitting,
    Walking,
    Running,
    Lying,
    Falling,
    Unknown,
}

impl Activity {
    pub fn is_alert_worthy(&self) -> bool {
        matches!(self, Activity::Falling)
    }

    pub fn is_mobile(&self) -> bool {
        matches!(self, Activity::Walking | Activity::Running)
    }
}
```

### 命令和事件生成

```rust
impl PoseEstimate {
    /// 从推理结果创建新的姿态估计
    pub fn create(
        signal_id: SignalId,
        session_id: SessionId,
        zone_id: Option<ZoneId>,
        persons: Vec<PersonDetection>,
        processing_time: Duration,
        model_version: ModelVersion,
    ) -> Result<(Self, Vec<DomainEvent>), DomainError> {
        let person_count = persons.len() as u8;
        let overall_confidence = Self::calculate_overall_confidence(&persons);

        let mut events = Vec::new();
        let mut detected_events = Vec::new();

        // 检查运动
        if persons.iter().any(|p| p.velocity.map(|v| v.is_significant()).unwrap_or(false)) {
            let event = PoseEvent::MotionDetected {
                timestamp: Utc::now(),
                zone_id: zone_id.clone(),
            };
            detected_events.push(event.clone());
            events.push(DomainEvent::MotionDetected(MotionDetectedEvent {
                zone_id: zone_id.clone(),
                person_count,
                timestamp: Utc::now(),
            }));
        }

        // 检查跌倒
        for person in &persons {
            if person.activity == Activity::Falling && person.confidence.is_high() {
                let event = PoseEvent::FallDetected {
                    person_id: person.person_id,
                    confidence: person.confidence,
                    timestamp: Utc::now(),
                };
                detected_events.push(event);
                events.push(DomainEvent::FallDetected(FallDetectedEvent {
                    person_id: person.person_id,
                    zone_id: zone_id.clone(),
                    confidence: person.confidence,
                    timestamp: Utc::now(),
                }));
            }
        }

        // 主要估计事件
        events.push(DomainEvent::PoseEstimated(PoseEstimatedEvent {
            estimate_id: EstimateId(Uuid::new_v4()),
            signal_id,
            person_count,
            overall_confidence,
            timestamp: Utc::now(),
        }));

        let estimate = Self {
            id: EstimateId(Uuid::new_v4()),
            signal_id,
            session_id,
            zone_id,
            timestamp: Utc::now(),
            frame_number: 0, // TODO: 跟踪帧编号
            persons,
            person_count,
            processing_time,
            model_version,
            algorithm: InferenceAlgorithm::DensePose,
            overall_confidence,
            is_valid: true,
            detected_events,
        };

        Ok((estimate, events))
    }

    fn calculate_overall_confidence(persons: &[PersonDetection]) -> Confidence {
        if persons.is_empty() {
            return Confidence(0.0);
        }
        let sum: f32 = persons.iter().map(|p| p.confidence.value()).sum();
        Confidence(sum / persons.len() as f32)
    }
}
```

---

## 4. Session 聚合根

### 目的

表示用于实时流的客户端连接会话。跟踪连接生命周期、订阅和传递指标。

### 聚合根：Session

```rust
/// 流会话的聚合根
#[derive(Debug)]
pub struct Session {
    // 标识
    id: SessionId,
    client_id: ClientId,

    // 连接详情
    connected_at: DateTime<Utc>,
    last_activity: DateTime<Utc>,
    remote_addr: Option<IpAddr>,
    user_agent: Option<String>,

    // 订阅状态
    stream_type: StreamType,
    zone_subscriptions: HashSet<ZoneId>,
    filters: SubscriptionFilters,

    // 会话状态（状态机）
    status: SessionStatus,

    // 指标
    messages_sent: u64,
    messages_failed: u64,
    bytes_sent: u64,
    latency_samples: Vec<Duration>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SessionId(Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClientId(Uuid);

/// 会话生命周期状态
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionStatus {
    /// 初始连接，尚未订阅
    Connecting,

    /// 积极接收数据
    Active,

    /// 客户端暂时暂停
    Paused,

    /// 连接丢失，尝试重新连接
    Reconnecting { attempts: u8, last_attempt: DateTime<Utc> },

    /// 优雅关闭
    Completed { ended_at: DateTime<Utc> },

    /// 错误终止
    Failed { reason: String, failed_at: DateTime<Utc> },

    /// 客户端发起的取消
    Cancelled { cancelled_at: DateTime<Utc> },
}

/// 客户端订阅偏好
#[derive(Debug, Clone, Default)]
pub struct SubscriptionFilters {
    pub min_confidence: Option<Confidence>,
    pub max_persons: Option<u8>,
    pub include_keypoints: bool,
    pub include_segmentation: bool,
    pub include_uv_coordinates: bool,
    pub throttle_interval: Option<Duration>,
    pub activity_filter: Option<Vec<Activity>>,
}
```

### 状态转换

```rust
impl Session {
    /// 创建新会话
    pub fn create(
        client_id: ClientId,
        stream_type: StreamType,
        remote_addr: Option<IpAddr>,
        user_agent: Option<String>,
    ) -> (Self, SessionStartedEvent) {
        let session = Self {
            id: SessionId(Uuid::new_v4()),
            client_id,
            connected_at: Utc::now(),
            last_activity: Utc::now(),
            remote_addr,
            user_agent: user_agent.clone(),
            stream_type,
            zone_subscriptions: HashSet::new(),
            filters: SubscriptionFilters::default(),
            status: SessionStatus::Connecting,
            messages_sent: 0,
            messages_failed: 0,
            bytes_sent: 0,
            latency_samples: Vec::new(),
        };

        let event = SessionStartedEvent {
            session_id: session.id,
            client_id,
            stream_type,
            timestamp: Utc::now(),
        };

        (session, event)
    }

    /// 订阅设置后激活会话
    pub fn activate(&mut self) -> Result<(), DomainError> {
        match &self.status {
            SessionStatus::Connecting | SessionStatus::Reconnecting { .. } => {
                self.status = SessionStatus::Active;
                self.last_activity = Utc::now();
                Ok(())
            }
            _ => Err(DomainError::InvalidStateTransition {
                from: format!("{:?}", self.status),
                to: "Active".to_string(),
            }),
        }
    }

    /// 暂停流
    pub fn pause(&mut self) -> Result<(), DomainError> {
        match &self.status {
            SessionStatus::Active => {
                self.status = SessionStatus::Paused;
                Ok(())
            }
            _ => Err(DomainError::CannotPause),
        }
    }

    /// 恢复流
    pub fn resume(&mut self) -> Result<(), DomainError> {
        match &self.status {
            SessionStatus::Paused => {
                self.status = SessionStatus::Active;
                self.last_activity = Utc::now();
                Ok(())
            }
            _ => Err(DomainError::CannotResume),
        }
    }

    /// 处理连接丢失
    pub fn connection_lost(&mut self) -> Result<(), DomainError> {
        match &self.status {
            SessionStatus::Active | SessionStatus::Paused => {
                self.status = SessionStatus::Reconnecting {
                    attempts: 0,
                    last_attempt: Utc::now(),
                };
                Ok(())
            }
            _ => Err(DomainError::AlreadyDisconnected),
        }
    }

    /// 优雅完成会话
    pub fn complete(&mut self) -> Result<SessionEndedEvent, DomainError> {
        match &self.status {
            SessionStatus::Active | SessionStatus::Paused => {
                let ended_at = Utc::now();
                self.status = SessionStatus::Completed { ended_at };

                Ok(SessionEndedEvent {
                    session_id: self.id,
                    duration: ended_at - self.connected_at,
                    messages_sent: self.messages_sent,
                    reason: "completed".to_string(),
                    timestamp: ended_at,
                })
            }
            _ => Err(DomainError::SessionNotActive),
        }
    }

    /// 更新订阅过滤器
    pub fn update_filters(&mut self, filters: SubscriptionFilters) -> Result<SubscriptionUpdatedEvent, DomainError> {
        if !self.is_active() {
            return Err(DomainError::SessionNotActive);
        }

        self.filters = filters.clone();
        self.last_activity = Utc::now();

        Ok(SubscriptionUpdatedEvent {
            session_id: self.id,
            filters,
            timestamp: Utc::now(),
        })
    }

    /// 订阅区域
    pub fn subscribe_to_zone(&mut self, zone_id: ZoneId) -> Result<(), DomainError> {
        if !self.is_active() {
            return Err(DomainError::SessionNotActive);
        }

        self.zone_subscriptions.insert(zone_id);
        self.last_activity = Utc::now();
        Ok(())
    }

    /// 记录成功的消息传递
    pub fn record_message_sent(&mut self, bytes: u64, latency: Duration) {
        self.messages_sent += 1;
        self.bytes_sent += bytes;
        self.last_activity = Utc::now();

        // 保留最后 100 个延迟样本
        if self.latency_samples.len() >= 100 {
            self.latency_samples.remove(0);
        }
        self.latency_samples.push(latency);
    }

    /// 记录失败的传递
    pub fn record_message_failed(&mut self) {
        self.messages_failed += 1;
    }

    // 查询

    pub fn is_active(&self) -> bool {
        matches!(self.status, SessionStatus::Active)
    }

    pub fn is_subscribed_to_zone(&self, zone_id: &ZoneId) -> bool {
        self.zone_subscriptions.is_empty() || self.zone_subscriptions.contains(zone_id)
    }

    pub fn average_latency(&self) -> Option<Duration> {
        if self.latency_samples.is_empty() {
            return None;
        }
        let sum: Duration = self.latency_samples.iter().sum();
        Some(sum / self.latency_samples.len() as u32)
    }
}
```

---

## 5. Device 聚合根

### 目的

表示能够提取 CSI 的物理 WiFi 硬件设备。管理设备生命周期、配置和健康状态。

### 聚合根：Device

```rust
/// 硬件设备的聚合根
#[derive(Debug)]
pub struct Device {
    // 标识
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

    // 状态机
    status: DeviceStatus,

    // 健康跟踪
    last_seen: Option<DateTime<Utc>>,
    health_checks: VecDeque<HealthCheckResult>,
    consecutive_failures: u8,

    // 配置
    config: DeviceConfig,
    calibration: Option<CalibrationData>,

    // 元数据
    tags: HashSet<String>,
    custom_properties: HashMap<String, serde_json::Value>,

    // 时间戳
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeviceId(Uuid);

/// 设备状态机
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceStatus {
    /// 未连接到网络
    Disconnected,

    /// 尝试建立连接
    Connecting { started_at: DateTime<Utc> },

    /// 已连接并就绪
    Connected { connected_at: DateTime<Utc> },

    /// 积极流传输 CSI 数据
    Streaming { stream_started_at: DateTime<Utc>, frames_sent: u64 },

    /// 运行校准程序
    Calibrating { calibration_id: CalibrationId, progress: u8 },

    /// 计划维护
    Maintenance { reason: String },

    /// 错误状态
    Error { error: DeviceError, occurred_at: DateTime<Utc> },
}

/// 设备硬件能力
#[derive(Debug, Clone)]
pub struct DeviceCapabilities {
    pub max_subcarriers: u16,
    pub max_antennas: u8,
    pub supported_bandwidths: Vec<Bandwidth>,
    pub supported_frequencies: Vec<FrequencyBand>,
    pub max_sampling_rate_hz: u32,
    pub supports_mimo: bool,
    pub supports_beamforming: bool,
}

/// 设备配置
#[derive(Debug, Clone)]
pub struct DeviceConfig {
    pub sampling_rate_hz: u32,
    pub subcarriers: u16,
    pub antennas: u8,
    pub bandwidth: Bandwidth,
    pub channel: WifiChannel,
    pub tx_power: Option<TxPower>,
    pub gain: Option<f32>,
}
```

### 值对象

```rust
/// MAC 地址
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MacAddress([u8; 6]);

impl MacAddress {
    pub fn parse(s: &str) -> Result<Self, DomainError> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 6 {
            return Err(DomainError::InvalidMacFormat);
        }

        let mut bytes = [0u8; 6];
        for (i, part) in parts.iter().enumerate() {
            bytes[i] = u8::from_str_radix(part, 16)
                .map_err(|_| DomainError::InvalidMacFormat)?;
        }
        Ok(Self(bytes))
    }

    pub fn to_string(&self) -> String {
        format!(
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5]
        )
    }
}

/// 设备类型枚举
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceType {
    Esp32,
    Esp32S3,
    AtherosRouter,
    IntelNic5300,
    IntelNic5500,
    Nexmon,
    PicoScenes,
    Custom(String),
}

/// WiFi 频段
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrequencyBand {
    Band2_4GHz,
    Band5GHz,
    Band6GHz,
}

/// WiFi 信道
#[derive(Debug, Clone, Copy)]
pub struct WifiChannel {
    pub number: u8,
    pub band: FrequencyBand,
}

impl WifiChannel {
    pub fn frequency(&self) -> Frequency {
        match self.band {
            FrequencyBand::Band2_4GHz => {
                // 2.4 GHz 频段：信道 1-14
                let base_mhz = 2412.0;
                let offset_mhz = (self.number as f64 - 1.0) * 5.0;
                Frequency::new((base_mhz + offset_mhz) * 1_000_000.0).unwrap()
            }
            FrequencyBand::Band5GHz => {
                // 5 GHz 频段：各种信道
                let mhz = 5000.0 + (self.number as f64 * 5.0);
                Frequency::new(mhz * 1_000_000.0).unwrap()
            }
            FrequencyBand::Band6GHz => {
                // 6 GHz 频段
                let mhz = 5950.0 + (self.number as f64 * 5.0);
                Frequency::new(mhz * 1_000_000.0).unwrap()
            }
        }
    }
}
```

### 命令

```rust
impl Device {
    /// 注册新设备
    pub fn register(
        name: DeviceName,
        device_type: DeviceType,
        mac_address: MacAddress,
        capabilities: DeviceCapabilities,
    ) -> (Self, DeviceRegisteredEvent) {
        let now = Utc::now();
        let device = Self {
            id: DeviceId(Uuid::new_v4()),
            name: name.clone(),
            device_type: device_type.clone(),
            mac_address,
            ip_address: None,
            firmware_version: None,
            hardware_version: None,
            capabilities,
            location: None,
            zone_id: None,
            status: DeviceStatus::Disconnected,
            last_seen: None,
            health_checks: VecDeque::with_capacity(10),
            consecutive_failures: 0,
            config: DeviceConfig::default(),
            calibration: None,
            tags: HashSet::new(),
            custom_properties: HashMap::new(),
            created_at: now,
            updated_at: now,
        };

        let event = DeviceRegisteredEvent {
            device_id: device.id,
            name,
            device_type,
            mac_address,
            timestamp: now,
        };

        (device, event)
    }

    /// 连接设备
    pub fn connect(&mut self) -> Result<DeviceConnectingEvent, DomainError> {
        match &self.status {
            DeviceStatus::Disconnected | DeviceStatus::Error { .. } => {
                self.status = DeviceStatus::Connecting { started_at: Utc::now() };
                self.updated_at = Utc::now();

                Ok(DeviceConnectingEvent {
                    device_id: self.id,
                    timestamp: Utc::now(),
                })
            }
            _ => Err(DomainError::DeviceAlreadyConnected),
        }
    }

    /// 确认连接已建立
    pub fn connection_established(&mut self) -> Result<DeviceConnectedEvent, DomainError> {
        match &self.status {
            DeviceStatus::Connecting { .. } => {
                let now = Utc::now();
                self.status = DeviceStatus::Connected { connected_at: now };
                self.last_seen = Some(now);
                self.consecutive_failures = 0;
                self.updated_at = now;

                Ok(DeviceConnectedEvent {
                    device_id: self.id,
                    timestamp: now,
                })
            }
            _ => Err(DomainError::InvalidStateTransition {
                from: format!("{:?}", self.status),
                to: "Connected".to_string(),
            }),
        }
    }

    /// 开始流传输 CSI 数据
    pub fn start_streaming(&mut self) -> Result<DeviceStreamingStartedEvent, DomainError> {
        match &self.status {
            DeviceStatus::Connected { .. } => {
                let now = Utc::now();
                self.status = DeviceStatus::Streaming {
                    stream_started_at: now,
                    frames_sent: 0,
                };
                self.updated_at = now;

                Ok(DeviceStreamingStartedEvent {
                    device_id: self.id,
                    config: self.config.clone(),
                    timestamp: now,
                })
            }
            _ => Err(DomainError::DeviceNotConnected),
        }
    }

    /// 停止流传输
    pub fn stop_streaming(&mut self) -> Result<DeviceStreamingStoppedEvent, DomainError> {
        match &self.status {
            DeviceStatus::Streaming { frames_sent, .. } => {
                let frames = *frames_sent;
                let now = Utc::now();
                self.status = DeviceStatus::Connected { connected_at: now };
                self.updated_at = now;

                Ok(DeviceStreamingStoppedEvent {
                    device_id: self.id,
                    frames_sent: frames,
                    timestamp: now,
                })
            }
            _ => Err(DomainError::DeviceNotStreaming),
        }
    }

    /// 应用配置
    pub fn configure(&mut self, config: DeviceConfig) -> Result<DeviceConfiguredEvent, DomainError> {
        // 验证配置是否符合能力
        if config.subcarriers > self.capabilities.max_subcarriers {
            return Err(DomainError::ConfigExceedsCapabilities {
                field: "subcarriers".to_string(),
            });
        }
        if config.antennas > self.capabilities.max_antennas {
            return Err(DomainError::ConfigExceedsCapabilities {
                field: "antennas".to_string(),
            });
        }
        if !self.capabilities.supported_bandwidths.contains(&config.bandwidth) {
            return Err(DomainError::UnsupportedBandwidth);
        }

        self.config = config.clone();
        self.updated_at = Utc::now();

        Ok(DeviceConfiguredEvent {
            device_id: self.id,
            config,
            timestamp: Utc::now(),
        })
    }

    /// 记录健康检查结果
    pub fn record_health_check(&mut self, result: HealthCheckResult) {
        // 保留最后 10 次检查
        if self.health_checks.len() >= 10 {
            self.health_checks.pop_front();
        }

        if result.is_healthy {
            self.consecutive_failures = 0;
        } else {
            self.consecutive_failures += 1;
        }

        self.health_checks.push_back(result);
        self.last_seen = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    // 查询

    pub fn is_healthy(&self) -> bool {
        self.consecutive_failures < 3 && !matches!(self.status, DeviceStatus::Error { .. })
    }

    pub fn is_streaming(&self) -> bool {
        matches!(self.status, DeviceStatus::Streaming { .. })
    }

    pub fn uptime(&self) -> Option<Duration> {
        match &self.status {
            DeviceStatus::Connected { connected_at } |
            DeviceStatus::Streaming { stream_started_at: connected_at, .. } => {
                Some((Utc::now() - *connected_at).to_std().unwrap_or_default())
            }
            _ => None,
        }
    }
}
```

---

## 跨聚合引用

聚合根之间仅通过 ID 引用，从不通过直接对象引用：

```rust
// 正确：通过 ID 引用
pub struct CsiFrame {
    device_id: DeviceId,      // 仅 ID
    session_id: Option<SessionId>,  // 仅 ID
}

// 错误：直接引用（永远不要这样做）
pub struct CsiFrame {
    device: Device,           // 错误：创建耦合
    session: Option<Session>, // 错误：违反边界
}
```

## 存储库模式

每个聚合根都有相应的存储库接口：

```rust
#[async_trait]
pub trait AggregateRepository<A, ID> {
    async fn find_by_id(&self, id: &ID) -> Result<Option<A>, RepositoryError>;
    async fn save(&self, aggregate: &A) -> Result<(), RepositoryError>;
    async fn delete(&self, id: &ID) -> Result<bool, RepositoryError>;
}
```