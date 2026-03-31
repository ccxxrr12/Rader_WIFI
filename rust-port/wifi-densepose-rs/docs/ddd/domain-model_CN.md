# 领域驱动设计：WiFi-DensePose 领域模型

## 有界上下文

### 1. 信号域
**目的**：原始 CSI 数据获取和预处理

**聚合根**：
- `CsiFrame`：来自 WiFi 硬件的原始 CSI 测量
- `ProcessedSignal`：清理和特征提取的信号

**值对象**：
- `Amplitude`：信号强度测量
- `Phase`：相位角度测量
- `SubcarrierData`：每子载波信息
- `Timestamp`：测量时间

**域服务**：
- `CsiProcessor`：预处理原始 CSI 数据
- `PhaseSanitizer`：解缠和清理相位数据
- `FeatureExtractor`：提取信号特征

### 2. 姿态域
**目的**：从处理后的信号进行人体姿态估计

**聚合根**：
- `PoseEstimate`：完整的 DensePose 输出
- `InferenceSession`：神经网络会话状态

**值对象**：
- `BodyPart`：标记的身体段（躯干、手臂、腿部等）
- `UVCoordinate`：表面映射坐标
- `Keypoint`：身体关节位置
- `Confidence`：预测置信度分数

**域服务**：
- `ModalityTranslator`：CSI → 视觉特征翻译
- `DensePoseHead`：身体部位分割和 UV 回归

### 3. 流传输域
**目的**：实时数据传递给客户端

**聚合根**：
- `Session`：客户端连接与历史
- `StreamConfig`：客户端流传输偏好

**值对象**：
- `WebSocketMessage`：类型化消息负载
- `ConnectionState`：活跃/空闲/断开

**域服务**：
- `StreamManager`：管理客户端连接
- `BroadcastService`：向订阅者推送更新

### 4. 存储域
**目的**：持久化和检索

**聚合根**：
- `Recording`：捕获的 CSI 会话
- `ModelArtifact`：神经网络权重

**存储库**：
- `SessionRepository`：会话 CRUD 操作
- `RecordingRepository`：记录存储
- `ModelRepository`：模型管理

### 5. 硬件域
**目的**：物理设备管理

**聚合根**：
- `Device`：WiFi 路由器/接收器
- `Antenna`：单个天线配置

**域服务**：
- `DeviceManager`：设备发现和控制
- `CsiExtractor`：原始 CSI 提取

## 上下文映射

```
┌─────────────────────────────────────────────────────────────┐
│                      WiFi-DensePose                         │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌──────────────┐     ┌──────────────┐     ┌─────────────┐ │
│  │   Hardware   │────▶│    Signal    │────▶│    Pose     │ │
│  │   Domain     │     │    Domain    │     │   Domain    │ │
│  └──────────────┘     └──────────────┘     └─────────────┘ │
│         │                    │                    │        │
│         │                    │                    │        │
│         ▼                    ▼                    ▼        │
│  ┌──────────────────────────────────────────────────────┐  │
│  │                   Storage Domain                      │  │
│  └──────────────────────────────────────────────────────┘  │
│         │                    │                    │        │
│         ▼                    ▼                    ▼        │
│  ┌──────────────────────────────────────────────────────┐  │
│  │                  Streaming Domain                     │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## 通用语言

| 术语 | 定义 |
|------|------------|
| CSI | 信道状态信息 - WiFi 信号属性 |
| Subcarrier | OFDM 中的单个频率组件 |
| Phase Unwrapping | 纠正 2π 相位不连续性 |
| DensePose | 带有 UV 映射的密集人体姿态估计 |
| Modality Translation | 将 CSI 特征转换为视觉特征 |
| Body Part | 15 个标记的人体身体段之一 |
| UV Mapping | 3D 身体的 2D 表面参数化 |