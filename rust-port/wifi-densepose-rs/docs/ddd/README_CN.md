# WiFi-DensePose 领域驱动设计文档

## 概述

本文档描述了 WiFi-DensePose Rust 端口的领域驱动设计（DDD）架构。该系统使用 WiFi 信道状态信息（CSI）执行非侵入式人体姿态估计，将射频信号转换为人体定位数据。

## 战略设计

### 核心领域

**姿态估计领域**代表提供独特价值的核心业务逻辑。该领域将 WiFi CSI 信号转换为与 DensePose 兼容的人体表示。模态转换（从射频到视觉特征）和姿态推断的算法构成了系统的竞争优势。

### 支持领域

1. **信号领域** - CSI 采集和预处理
2. **流领域** - 实时数据传输基础设施
3. **存储领域** - 持久化和检索机制
4. **硬件领域** - 设备抽象和管理

### 通用领域

- 认证和授权
- 日志记录和监控
- 配置管理

## 战术设计模式

### 聚合根

每个限界上下文包含执行不变量和维护一致性的聚合根：

- **CsiFrame** - 带有验证规则的原始信号数据
- **ProcessedSignal** - 准备好进行推断的特征提取信号
- **PoseEstimate** - 带有置信度评分的推断结果
- **Session** - 客户端连接生命周期管理
- **Device** - 带有状态机的硬件抽象

### 领域事件

事件通过事件驱动架构在限界上下文之间流动：

```
CsiFrameReceived -> SignalProcessed -> PoseEstimated -> (MotionDetected | FallDetected)
```

### 仓库

每个聚合根都有对应的仓库用于持久化：

- `CsiFrameRepository`
- `SessionRepository`
- `DeviceRepository`
- `PoseEstimateRepository`

### 领域服务

跨聚合操作由领域服务处理：

- `PoseEstimationService` - 编排 CSI 到姿态的流程
- `CalibrationService` - 硬件校准工作流
- `AlertService` - 运动和跌倒检测警报

## 上下文映射

```
                    +------------------+
                    |  Pose Domain     |
                    |  (Core Domain)   |
                    +--------+---------+
                             |
              +--------------+---------------+
              |              |               |
    +---------v----+  +------v------+  +-----v-------+
    | Signal Domain|  | Streaming   |  | Storage     |
    | (Upstream)   |  | Domain      |  | Domain      |
    +---------+----+  +------+------+  +------+------+
              |              |                |
              +--------------+----------------+
                             |
                    +--------v--------+
                    | Hardware Domain |
                    | (Foundation)    |
                    +-----------------+
```

### 关系

| 上游 | 下游 | 关系 |
|------|------|------|
| 硬件 | 信号 | 遵循者 |
| 信号 | 姿态 | 客户-供应商 |
| 姿态 | 流 | 发布语言 |
| 姿态 | 存储 | 共享内核 |

## 架构原则

### 1. 六边形架构

每个限界上下文遵循六边形（端口和适配器）架构：

```
                    +--------------------+
                    |    Application     |
                    |      Services      |
                    +---------+----------+
                              |
              +---------------+---------------+
              |                               |
    +---------v---------+           +---------v---------+
    |   Domain Layer    |           |   Domain Layer    |
    |  (Entities, VOs,  |           |   (Aggregates,    |
    |   Domain Events)  |           |    Repositories)  |
    +---------+---------+           +---------+---------+
              |                               |
    +---------v---------+           +---------v---------+
    | Infrastructure    |           | Infrastructure    |
    | (Adapters: DB,    |           | (Adapters: API,   |
    |  Hardware, MQ)    |           |  WebSocket)       |
    +-------------------+           +-------------------+
```

### 2. CQRS（命令查询责任分离）

系统分离读写操作：

- **命令**：`ProcessCsiFrame`、`CreateSession`、`UpdateDeviceConfig`
- **查询**：`GetCurrentPose`、`GetSessionHistory`、`GetDeviceStatus`

### 3. 事件溯源（可选）

为了审计和重放能力，CSI 处理事件可以存储为事件日志：

```rust
pub enum DomainEvent {
    CsiFrameReceived(CsiFrameReceivedEvent),
    SignalProcessed(SignalProcessedEvent),
    PoseEstimated(PoseEstimatedEvent),
    MotionDetected(MotionDetectedEvent),
    FallDetected(FallDetectedEvent),
}
```

## Rust 实现指南

### 模块结构

```
wifi-densepose-rs/
  crates/
    wifi-densepose-core/         # 共享内核
      src/
        domain/
          entities/
          value_objects/
          events/
    wifi-densepose-signal/       # 信号限界上下文
      src/
        domain/
        application/
        infrastructure/
    wifi-densepose-nn/           # 姿态限界上下文
      src/
        domain/
        application/
        infrastructure/
    wifi-densepose-api/          # 流限界上下文
      src/
        domain/
        application/
        infrastructure/
    wifi-densepose-db/           # 存储限界上下文
      src/
        domain/
        application/
        infrastructure/
    wifi-densepose-hardware/     # 硬件限界上下文
      src/
        domain/
        application/
        infrastructure/
```

### 类型驱动设计

利用 Rust 的类型系统编码领域不变量：

```rust
// 领域标识符的新类型模式
pub struct DeviceId(Uuid);
pub struct SessionId(Uuid);
pub struct FrameId(u64);

// 通过枚举实现状态机
pub enum DeviceState {
    Disconnected,
    Connecting(ConnectionAttempt),
    Connected(ActiveConnection),
    Streaming(StreamingSession),
    Error(DeviceError),
}

// 经过验证的值对象
pub struct Frequency {
    hz: f64, // 不变量：始终 > 0
}

impl Frequency {
    pub fn new(hz: f64) -> Result<Self, DomainError> {
        if hz <= 0.0 {
            return Err(DomainError::InvalidFrequency);
        }
        Ok(Self { hz })
    }
}
```

### 错误处理

领域错误与基础设施错误分开：

```rust
#[derive(Debug, thiserror::Error)]
pub enum SignalDomainError {
    #[error("Invalid CSI frame: {0}")]
    InvalidFrame(String),

    #[error("Signal quality below threshold: {snr} dB")]
    LowSignalQuality { snr: f64 },

    #[error("Calibration required for device {device_id}")]
    CalibrationRequired { device_id: DeviceId },
}
```

## 测试策略

### 单元测试
- 值对象不变量
- 聚合业务规则
- 领域服务逻辑

### 集成测试
- 仓库实现
- 上下文间通信
- 事件发布/订阅

### 属性测试
- 信号处理算法
- 姿态估计准确性
- 事件排序保证

## 参考资料

- Evans, Eric. *Domain-Driven Design: Tackling Complexity in the Heart of Software*. Addison-Wesley, 2003.
- Vernon, Vaughn. *Implementing Domain-Driven Design*. Addison-Wesley, 2013.
- Millett, Scott and Tune, Nick. *Patterns, Principles, and Practices of Domain-Driven Design*. Wrox, 2015.

## 文档索引

1. [限界上下文](./bounded-contexts_CN.md) - 详细的上下文定义
2. [聚合根](./aggregates_CN.md) - 聚合根规范
3. [领域事件](./domain-events_CN.md) - 事件目录和模式
4. [通用语言](./ubiquitous-language_CN.md) - 领域术语词汇表