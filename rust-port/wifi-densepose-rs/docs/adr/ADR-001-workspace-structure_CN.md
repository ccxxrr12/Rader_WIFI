# ADR-001: Rust 工作区结构

## 状态
已接受

## 背景
我们需要将 WiFi-DensePose Python 应用程序移植到 Rust，以提高性能、内存安全性和跨平台部署（包括 WASM）。架构必须是模块化的、可维护的，并支持多个部署目标。

## 决策
我们将使用带有 9 个模块化 crate 的 Cargo 工作区：

```
wifi-densepose-rs/
├── Cargo.toml                    # 工作区根目录
├── crates/
│   ├── wifi-densepose-core/      # 核心类型、特征、错误处理
│   ├── wifi-densepose-signal/    # 信号处理（CSI、相位、FFT）
│   ├── wifi-densepose-nn/        # 神经网络（DensePose、转换）
│   ├── wifi-densepose-api/       # REST/WebSocket API（Axum）
│   ├── wifi-densepose-db/        # 数据库层（SQLx）
│   ├── wifi-densepose-config/    # 配置管理
│   ├── wifi-densepose-hardware/  # 硬件抽象
│   ├── wifi-densepose-wasm/      # WASM 绑定
│   └── wifi-densepose-cli/       # CLI 应用程序
```

### Crate 职责

1. **wifi-densepose-core**：所有 crate 共享的基础类型、特征和错误处理
2. **wifi-densepose-signal**：CSI 数据处理、相位净化、FFT、特征提取
3. **wifi-densepose-nn**：使用 ONNX Runtime、Candle 或 tch-rs 的神经网络推理
4. **wifi-densepose-api**：使用 Axum 的 HTTP/WebSocket 服务器
5. **wifi-densepose-db**：使用 SQLx 的数据库操作
6. **wifi-densepose-config**：配置加载和验证
7. **wifi-densepose-hardware**：路由器和硬件接口
8. **wifi-densepose-wasm**：用于浏览器部署的 WebAssembly 绑定
9. **wifi-densepose-cli**：命令行界面

## 影响

### 积极影响
- 清晰的关注点分离
- 独立的 crate 版本控制
- 并行编译
- 选择性功能包含
- 更易于测试和维护
- WASM 目标隔离

### 消极影响
- 更复杂的依赖管理
- 初始设置开销
- 跨 crate 重构复杂性

## 参考资料
- [Cargo 工作区](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- [ruvector crate 结构](https://github.com/ruvnet/ruvector)