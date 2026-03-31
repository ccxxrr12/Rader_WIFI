# ADR-003: 神经网络推理策略

## 状态
已接受

## 背景
WiFi-DensePose 系统需要神经网络推理用于：
1. 模态转换（CSI → 视觉特征）
2. DensePose 估计（身体部位分割 + UV 映射）

我们需要选择支持预训练模型和多个后端的推理策略。

## 决策
我们将实现多后端推理引擎：

### 主要后端：ONNX Runtime（`ort` crate）
- 加载导出到 ONNX 的预训练 PyTorch 模型
- 通过 CUDA/TensorRT 进行 GPU 加速
- 跨平台支持

### 替代后端（功能门控）
- `tch-rs`：PyTorch C++ 绑定
- `candle`：纯 Rust 机器学习框架

### 架构
```rust
pub trait Backend: Send + Sync {
    fn load_model(&mut self, path: &Path) -> NnResult<()>;
    fn run(&self, inputs: HashMap<String, Tensor>) -> NnResult<HashMap<String, Tensor>>;
    fn input_specs(&self) -> Vec<TensorSpec>;
    fn output_specs(&self) -> Vec<TensorSpec>;
}
```

### 功能标志
```toml
[features]
default = ["onnx"]
onnx = ["ort"]
tch-backend = ["tch"]
candle-backend = ["candle-core", "candle-nn"]
cuda = ["ort/cuda"]
tensorrt = ["ort/tensorrt"]
```

## 影响

### 积极影响
- 使用现有的训练模型（无需重新训练）
- 不同部署的多个后端选项
- 可用时进行 GPU 加速
- 功能标志最小化二进制大小

### 消极影响
- 需要 ONNX 模型转换
- ort crate 引入 C++ 依赖
- tch 需要安装 libtorch