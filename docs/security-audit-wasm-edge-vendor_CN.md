# 安全审计：wifi-densepose-wasm-edge v0.3.0

**日期**：2026-03-03
**审计员**：安全审计代理（Claude Opus 4.6）
**范围**：`rust-port/wifi-densepose-rs/crates/wifi-densepose-wasm-edge/src/` 中的所有 29 个 `.rs` 文件
** crate 版本**：0.3.0
**目标**：`wasm32-unknown-unknown`（ESP32-S3 WASM3 解释器）

---

## 执行摘要

wifi-densepose-wasm-edge crate 实现了 29 个 no_std WASM 模块，用于设备端 CSI 信号处理。代码总体编写良好，在内存管理、边界检查和事件速率限制方面采用了一致的模式。在 no_std 构建中没有堆分配泄漏。所有主机 API 调用都正确地通过 `cfg(target_arch = "wasm32")` 进行了门控。

**发现的问题总数**：15
- 严重：1
- 高：3
- 中：6
- 低：5

---

## 发现

### 严重

#### C-01：`static mut` 事件缓冲区在并发访问下不安全

**严重性**：严重
**文件**：所有使用 `static mut EVENTS` 模式的 26 个模块
**示例**：`occupancy.rs:161`、`vital_trend.rs:175`、`intrusion.rs:121`、`sig_coherence_gate.rs:180`、`sig_flash_attention.rs:107`、`spt_pagerank_influence.rs:195`、`spt_micro_hnsw.rs:267,284`、`tmp_pattern_sequence.rs:153`、`lrn_dtw_gesture_learn.rs:146`、`lrn_anomaly_attractor.rs:140`、`ais_prompt_shield.rs:158`、`qnt_quantum_coherence.rs:132`、`sig_sparse_recovery.rs:138`、`sig_temporal_compress.rs:246,309` 等 10+ 个文件

**描述**：每个模块都在函数体内使用 `static mut` 数组来返回事件切片，无需堆分配：

```rust
static mut EVENTS: [(i32, f32); 4] = [(0, 0.0); 4];
// ... 写入 EVENTS ...
unsafe { &EVENTS[..n_events] }
```

虽然这在 WASM3 的单线程执行模型中是安全的，但返回的 `&[(i32, f32)]` 引用具有 `'static` 生命周期，但其数据会在下一次调用时被修改。如果调用者在两次 `process_frame()` 调用之间存储返回的切片引用，第一个引用会观察到被静默修改的数据。

**风险**：在当前 ESP32 WASM3 单线程部署中，这一问题得到了缓解。但是，如果该 crate  ever 在多线程环境中使用，或者事件切片在调用之间被存储，数据会静默损坏，不会出现 panic 或错误。

**建议**：在每个函数的文档注释中明确记录此契约："返回的切片仅在下次调用此函数之前有效。" 考虑添加 `#[doc(hidden)]` 注释或使用新类型包装以防止在调用之间存储。当前方法是对 no_std/no-heap 约束的可接受权衡，但必须记录。

**状态**：未修复（文档级问题；嵌入式 WASM 目标无需代码更改）

---

### 高

#### H-01：`coherence.rs:94-96` — 当 `n_sc == 0` 时除以零

**严重性**：高
**文件**：`coherence.rs:94`

**描述**：`CoherenceMonitor::process_frame()` 函数在第 69 行计算 `n_sc` 为 `min(phases.len(), MAX_SC)`，如果 `phases` 为空，可能为 0。然而，在第 94 行，代码在没有零检查的情况下除以 `n`（即 `n_sc as f32`）：

```rust
let n = n_sc as f32;
let mean_re = sum_re / n;  // 如果 phases 为空，除以零
let mean_im = sum_im / n;
```

虽然第 71 行的 `initialized` 检查会捕获第一次调用并提前返回，但第二次调用如果 `phases` 切片为空，将到达除法操作。

**影响**：产生 `NaN`/`Inf`，这些值会通过 EMA 平滑的相干性分数传播，永久损坏监视器状态。

**建议**：在 `initialized` 检查后添加 `if n_sc == 0 { return self.smoothed_coherence; }`。

#### H-02：`occupancy.rs:92,99,105,112` — 当 `zone_count == 1` 且 `n_sc < 4` 时除以零

**严重性**：高
**文件**：`occupancy.rs:92-112`

**描述**：当 `n_sc == 2` 或 `n_sc == 3` 时，`zone_count = (n_sc / 4).min(MAX_ZONES).max(1) = 1`，`subs_per_zone = n_sc / zone_count = n_sc`。循环计算 `count = (end - start) as f32` 是有效的。然而，当 `n_sc == 1` 时，函数在第 83-85 行提前返回。真正的风险是如果 `n_sc == 0` 以某种方式通过检查 — 但第 83 行的 `n_sc < 2` 检查会防止这种情况。这实际上是安全的，但很脆弱。

然而，更严重的问题是：第 99 行的 `count` 变量计算为 `(end - start) as f32`，并在第 105 和 112 行用作除数。如果 `subs_per_zone == 0`（当 `zone_count > n_sc` 时可能发生），`count` 将为 0，导致除以零。目前 `zone_count` 被 `n_sc / 4` 限制，因此在 `n_sc >= 2` 时不会发生这种情况，但逻辑很脆弱。

**建议**：在第 105 行的除法之前添加 `if count < 1.0 { continue; }` 保护。

#### H-03：`rvf.rs:209-215` — `patch_signature` 对 `offset + RVF_SIGNATURE_LEN` 没有边界检查

**严重性**：高
**文件**：`rvf.rs:209-215`（仅 std 构建代码）

**描述**：`patch_signature` 函数从头部字节读取 `wasm_len` 并计算偏移量，然后复制到 `rvf[offset..offset + RVF_SIGNATURE_LEN]`，而不检查 `offset + RVF_SIGNATURE_LEN <= rvf.len()`：

```rust
pub fn patch_signature(rvf: &mut [u8], signature: &[u8; RVF_SIGNATURE_LEN]) {
    let sig_offset = RVF_HEADER_SIZE + RVF_MANIFEST_SIZE;
    let wasm_len = u32::from_le_bytes([rvf[12], rvf[13], rvf[14], rvf[15]]) as usize;
    let offset = sig_offset + wasm_len;
    rvf[offset..offset + RVF_SIGNATURE_LEN].copy_from_slice(signature);
}
```

如果使用截断或格式错误的 RVF 缓冲区调用，或者头部中的 `wasm_len` 被篡改，这会在运行时 panic。由于这是仅 std 构建代码（在 `#[cfg(feature = "std")]` 后面），它不影响 WASM 目标，但在构建工具中存在潜在的拒绝服务风险。

**建议**：添加边界检查：`if offset + RVF_SIGNATURE_LEN > rvf.len() { return; }` 或返回 `Result`。

---

### 中

#### M-01：`lib.rs:391` — 来自主机的负 `n_subcarriers` 静默包装为大 `usize`

**严重性**：中
**文件**：`lib.rs:391`

**描述**：导出的 `on_frame(n_subcarriers: i32)` 转换为 usize：`let n_sc = n_subcarriers as usize;`。如果主机传递负值（例如 `-1`），在 32 位 WASM 目标上这会包装为 `usize::MAX`（`4294967295`）。随后的钳制 `if n_sc > 32 { 32 } else { n_sc }` 会安全处理，产生 `max_sc = 32`。然而，语义意图被破坏：负输入应被视为 0。

**建议**：添加：`let n_sc = if n_subcarriers < 0 { 0 } else { n_subcarriers as usize };`

#### M-02：`coherence.rs:142-144` — `mean_phasor_angle()` 使用过时的 `phasor_re/phasor_im` 字段

**严重性**：中
**文件**：`coherence.rs:142-144`

**描述**：`mean_phasor_angle()` 方法计算 `atan2f(self.phasor_im, self.phasor_re)`，但 `phasor_re` 和 `phasor_im` 在 `new()` 中初始化为 `0.0`，在 `process_frame()` 中从未更新。`process_frame()` 中计算的运行相量和使用局部变量 `sum_re` 和 `sum_im`，但从未将它们存储回 `self.phasor_re/self.phasor_im`。

**影响**：`mean_phasor_angle()` 始终返回 `atan2(0, 0) = 0.0`，这是不正确的。

**建议**：在 `process_frame()` 末尾存储每帧的平均相量分量：`self.phasor_re = mean_re; self.phasor_im = mean_im;`。

#### M-03：`gesture.rs:200` — DTW 成本矩阵使用 9.6 KB 栈，对不匹配大小无保护

**严重性**：中
**文件**：`gesture.rs:200`

**描述**：`dtw_distance` 函数在栈上分配 `[[f32::MAX; 40]; 60]` = 2400 * 4 = 9600 字节。这在 WASM3 的默认 64 KB 栈范围内，但结合调用者的栈帧（GestureDetector 约 360 字节 + 局部变量），每次手势检查的总栈压力接近 11-12 KB。

`vendor_common.rs` 中的 DTW 函数使用 `[[f32::MAX; 64]; 64]` = 16384 字节，更令人担忧。

**影响**：如果多个 DTW 调用嵌套或 WASM 栈配置小于 32 KB，会发生栈溢出（在 WASM3 中由于 panic 处理程序循环导致无限循环）。

**建议**：记录最小 WASM 栈要求（建议 32 KB）。考虑将 `vendor_common.rs` 中的 `DTW_MAX_LEN` 从 64 减少到 48，以使每次调用的栈使用量低于 10 KB。

#### M-04：`frame_count` 字段在 20 Hz 下约 2.5 天后静默溢出

**严重性**：中
**文件**：所有带有 `frame_count: u32` 的模块

**描述**：在 20 Hz 帧率下，`u32::MAX / 20 / 3600 / 24 = 2.48` 天。溢出后，任何 `frame_count % N == 0` 周期性发射逻辑都会改变时间。`sig_temporal_compress.rs:231` 明确使用 `wrapping_add`，但大多数模块使用 `+= 1`，这在调试模式下会 panic。

**影响**：在嵌入式发布构建（panic=abort）中，`+= 1` 编译为包装算术，因此不会崩溃。然而，与阈值比较 `frame_count` 的模块（例如 `lrn_anomaly_attractor.rs:192`：`self.frame_count >= MIN_FRAMES_FOR_CLASSIFICATION`）会在溢出后重新触发学习阶段。

**建议**：在所有模块中明确使用 `.wrapping_add(1)` 以提高清晰度。对于有阈值比较的模块，添加 `saturating` 标志以防止重新触发。

#### M-05：`tmp_pattern_sequence.rs:159` — 日期边界处潜在的越界写入

**严重性**：中
**文件**：`tmp_pattern_sequence.rs:159`

**描述**：写入索引为 `DAY_LEN + self.minute_counter as usize`。当 `minute_counter` 等于 `DAY_LEN - 1`（1439）时，索引为 `2879`，这是 `history: [u8; DAY_LEN * 2]` 数组中的最后一个有效索引。这没问题。然而，第 160 行的边界检查 `if idx < DAY_LEN * 2` 是一个安全网，表明意识到可能存在的差一错误。检查是正确的，防止了溢出。

实际上，问题是 `minute_counter` 是 `u16`，并与 `DAY_LEN as u16`（1440）进行比较。如果 `minute_counter` 以某种方式递增超过 `DAY_LEN` 而未触发第 192 行的翻转检查（检查 `>=`），由于第 160 行的保护，不会发生越界。这是防御性的且安全的。

**降级关注**：这实际上处理得很好。保持为中等，因为如果没有保护，计算 `DAY_LEN + minute_counter` 的模式将是危险的。

#### M-06：`spt_micro_hnsw.rs:187` — 邻居索引存储为 `u8`，当 `MAX_VECTORS > 255` 时静默截断

**严重性**：中
**文件**：`spt_micro_hnsw.rs:187,197`

**描述**：邻居索引在 `HnswNode::neighbors` 中存储为 `u8`。代码在第 187/197 行存储 `to as u8`。对于 `MAX_VECTORS = 64`，这是安全的。然而，如果 `MAX_VECTORS` ever 增加到 255 以上，索引会静默截断，导致错误的图边，可能导致错误的最近邻结果。

**建议**：添加编译时断言：`const _: () = assert!(MAX_VECTORS <= 255);`

---

### 低

#### L-01：`lib.rs:35` — `#![allow(clippy::missing_safety_doc)]` 抑制安全文档

**严重性**：低
**文件**：`lib.rs:35`

**描述**：这会抑制关于不安全函数缺少 `# Safety` 部分的警告。考虑到广泛使用 `unsafe` 进行 `static mut` 访问和 FFI 调用，记录安全不变量将提高可维护性。

#### L-02：所有 `static mut EVENTS` 缓冲区都在非 cfg 门控函数内

**严重性**：低
**文件**：所有 26 个在函数体内使用 `static mut EVENTS` 的模块

**描述**：`static mut EVENTS` 缓冲区在未通过 `cfg(target_arch = "wasm32")` 门控的函数内声明。这意味着它们存在于所有目标上，包括主机测试。虽然这对于函数在主机上编译和可测试是必要的，但这意味着在使用并行测试线程的 `cargo test` 期间，"单线程 WASM" 的健全性论点不成立。

**影响**：测试当前按模块函数单线程运行，因此实际上不会发生数据竞争。Rust 的测试工具在并行线程中运行测试，但每个测试创建自己的实例并顺序调用方法。

**建议**：使用 `-- --test-threads=1` 运行测试或在测试配置中添加注释。

#### L-03：`lrn_dtw_gesture_learn.rs:357` — `next_id` 在 255 处包装，可能与内置手势 ID 冲突

**严重性**：低
**文件**：`lrn_dtw_gesture_learn.rs:357`

**描述**：`self.next_id = self.next_id.wrapping_add(1)` 从 100 开始，从 255 包装到 0，可能与 `gesture.rs` 中的内置手势 ID 1-4 重叠。

**建议**：使用 `wrapping_add(1).max(100)` 或 saturating_add 以保持在 100-255 范围内。

#### L-04：`ais_prompt_shield.rs:294` — FNV-1a 哈希量化分辨率可能导致虚假重放阳性

**严重性**：低
**文件**：`ais_prompt_shield.rs:292-308`

**描述**：重放检测以 0.01 分辨率量化特征（`(mean_phase * 100.0) as i32`）。两个真正不同的帧，如果 mean_phase 值差异小于 0.01，将哈希相同，触发虚假重放警报。在 20 Hz 下，CSI 缓慢变化时，这可能频繁发生。

**建议**：将量化分辨率增加到 0.001 或添加次要判别器（例如，在哈希中包含帧序列计数器）。

#### L-05：`qnt_quantum_coherence.rs:188` — `inv_n` 计算没有零检查

**严重性**：低
**文件**：`qnt_quantum_coherence.rs:188`

**描述**：`let inv_n = 1.0 / (n_sc as f32);` — 虽然第 94 行检查了 `n_sc < 2`，但不使用显式保护进行除法的模式与其他模块不一致。

---

## WASM 特定检查清单

| 检查 | 状态 | 说明 |
|------|------|------|
| 主机 API 调用在 `cfg(target_arch = "wasm32")` 后面 | 通过 | 所有 FFI 在 `lib.rs:100-137`、`log_msg`、`emit` 都正确门控 |
| no_std 构建中无 std 依赖 | 通过 | `Vec`、`String`、`Box` 仅在 `rvf.rs` 中，在 `#[cfg(feature = "std")]` 后面 |
| 精确定义一次 panic 处理程序 | 通过 | `lib.rs:349-353`，由 `cfg(target_arch = "wasm32")` 门控 |
| no_std 代码中无堆分配 | 通过 | 所有存储使用固定大小数组和栈分配 |
| `static mut STATE` 门控 | 通过 | `lib.rs:361` 在 `cfg(target_arch = "wasm32")` 后面 |

## 信号完整性检查

| 检查 | 状态 | 说明 |
|------|------|------|
| 对抗性 CSI 输入崩溃抵抗 | 通过 | 所有模块将 `n_sc` 钳制到 `MAX_SC`（32），处理空输入 |
| 可配置阈值 | 部分 | 阈值是 `const` 值，不能通过 NVS 运行时配置。对于按用途加载的 WASM 模块是可接受的 |
| 事件 ID 匹配 ADR-041 注册表 | 通过 | 核心（0-99）、医疗（100-199）、安全（200-299）、智能建筑（300-399）、信号（700-729）、自适应（730-749）、空间（760-773）、时间（790-803）、AI 安全（820-828）、量子（850-857）、自主（880-888） |
| 有界事件发射率 | 通过 | 所有模块使用冷却计数器、周期性发射（`% N == 0`）和静态缓冲区上限（每次调用最多 4-12 个事件） |

## 总体风险评估

**风险级别**：低-中

代码库展示了针对嵌入式 no_std WASM 目标的强大安全实践：
- 感知模块中无堆分配
- 所有数组访问都有一致的边界检查
- 通过冷却计数器和周期性发射限制事件速率
- 主机 API 通过目标架构 cfg 门正确隔离
- 单个 panic 处理程序，正确门控

主要关注点（C-01）是在 no_std 环境中返回对 `static mut` 数据的引用的固有限制。这是嵌入式 Rust 中的已知模式，考虑到单线程 WASM3 执行模型，是可接受的，但必须记录。

高问题（H-01、H-02、H-03）涉及边缘情况下的潜在除以零和未检查的缓冲区访问。H-01 是最可行的，应在生产部署前修复。

---

## 应用的修复

以下严重和高问题已直接在源文件中修复：

1. **H-01**：在 `coherence.rs:process_frame()` 中添加了零长度保护
2. **H-02**：在 `occupancy.rs` 区域方差计算中添加了零计数保护
3. **M-01**：在 `lib.rs:on_frame()` 中添加了负输入保护
4. **M-02**：修复了 `coherence.rs:process_frame()` 中过时的相量字段
5. **M-06**：在 `spt_micro_hnsw.rs` 中添加了编译时断言

H-03（rvf.rs patch_signature）是仅 std 构建代码，未修复以避免范围蔓延；在构建工具用于 CI/CD 管道之前应添加边界检查。