# ADR-002: 信号处理库选择

## 状态
已接受

## 背景
CSI 信号处理需要 FFT 操作、复数处理和矩阵操作。我们需要选择提供 Python/NumPy 等效功能的适当 Rust 库。

## 决策
我们将使用以下库：

| 库 | 用途 | Python 等效 |
|---------|---------|-------------------|
| `ndarray` | N 维数组 | NumPy |
| `rustfft` | FFT 操作 | numpy.fft |
| `num-complex` | 复数 | complex |
| `num-traits` | 数值特征 | - |

### 关键实现

1. **相位净化**：多种解缠方法（标准、自定义、Itoh、质量引导）
2. **CSI 处理**：幅度/相位提取、时间平滑、汉明窗
3. **特征提取**：多普勒、PSD、幅度、相位、相关性特征
4. **运动检测**：基于方差的自适应阈值

## 影响

### 积极影响
- 纯 Rust 实现（无 FFI 开销）
- WASM 兼容（rustfft 是纯 Rust）
- 带有 ndarray 的 NumPy 风格 API
- 具有 SIMD 优化的高性能

### 消极影响
- ndarray-linalg 需要 BLAS 后端进行高级操作
- ndarray 模式的学习曲线

## 参考资料
- [ndarray 文档](https://docs.rs/ndarray)
- [rustfft 文档](https://docs.rs/rustfft)