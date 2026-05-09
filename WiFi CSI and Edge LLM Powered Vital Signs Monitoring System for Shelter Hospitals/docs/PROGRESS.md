# 竞赛项目构建进度 ✅

> 原始构建: 2026-05-06 17:10 | 审计修正: 2026-05-06 17:15
> **阶段 2 修复 + MAT 集成**: 2026-05-09 08:30
> 状态: **阶段 2 Cargo 编译通过 + MAT Pipeline 完整集成 ✅**

---

## 审计修正记录

| # | 发现 | 修正 |
|---|------|------|
| 1 | `mat_pipeline.rs` 导入了未使用的 `Arc`/`parking_lot::RwLock` | 删除 (Cargo.toml 无此依赖) |
| 2 | `mat_pipeline.rs` 错误地重新实现了信号处理 (服务端已有 FFT VitalSignDetector) | 重写为纯分诊层 — 接受 VitalSigns 输入, 只做 START + 追踪 + 告警 |
| 3 | `triage.html` 连接不存在的 `/ws/triage` WebSocket | 改为连接现有 `/ws/sensing`,解析 `SensingUpdate` 格式 |
| 4 | `deploy.sh` 使用了不存在的 `--triage-ui` CLI 参数 | 改为 `cp triage.html → ui/` + 正确参数 `--ui-path --bind-addr --source` |
| 5 | 所有 sdkconfig 选项 | 逐项对照 Kconfig.projbuild — 全部真实存在 ✅ |

## 阶段 2 修复记录 (2026-05-09)

### 2.1 Cargo.toml 修复 — 目录评估发现的实际问题

| # | 发现 | 修正 | 文件 |
|---|------|------|------|
| 1 | **Workspace 成员缺失** — Cargo.toml 声明 16 个成员但只存在 9 个 (api/db/wasm/cli/train/wifiscan/desktop 无源码) | 删除 8 个不存在成员，保留 core/signal/nn/config/hardware/mat/sensing-server/vitals | `rust-server/Cargo.toml` |
| 2 | **sensing-server 依赖幽灵 crate** — `wifi-densepose-wifiscan` 不存在 | 替换为 `wifi-densepose-mat` + `signal` + `vitals` | `sensing-server/Cargo.toml` |
| 3 | **缺失 Cargo.lock** — 整个项目无 lockfile | `cargo check` 自动生成 | — |
| 4 | **4 个 crate 缺失 bench 文件** — hardware/nn/signal/mat 的 `[[bench]]` 引用不存在的基准文件 | 移除 `[[bench]]` 段 | 4× `Cargo.toml` |

### 2.2 Windows WiFi 代码剔除

| # | 删除内容 | 行数 |
|---|----------|:--:|
| 1 | `parse_netsh_interfaces_output()` — netsh 输出解析 | ~30 |
| 2 | `windows_wifi_task()` — 多BSSID扫描管道 | ~220 |
| 3 | `windows_wifi_fallback_tick()` — 单RSSI回退 | ~125 |
| 4 | `probe_windows_wifi()` — Windows WiFi 探测 | ~15 |
| 5 | `SensingUpdate` 中 7 个 BSSID 字段 | — |
| 6 | `main()` 中 `"wifi"` 源分支 | — |
| **合计** | **约 390 行删除** | |

**原因**: 竞赛目标平台为 RZ/V2H (ARM Linux)，不存在 Windows `netsh` 命令。原代码依赖 `wifi-densepose-wifiscan` crate 用于 Windows 笔记本 CSI 采集。

### 2.3 MAT Pipeline 完整集成 ⭐

| # | 修改 | 位置 |
|---|------|------|
| 1 | `AppStateInner` 新增 `triage_engine: TriageEngine` 字段 | `main.rs` struct |
| 2 | `main()` 初始化 `TriageEngine::new(TriageConfig::competition())` | `main.rs` state 构造 |
| 3 | `udp_receiver_task` — ESP32 帧处理后调用 `triage_engine.process()` → `TriageUpdate` 写入 `SensingUpdate.triage_update` | `main.rs:2480` |
| 4 | `simulated_data_task` — 模拟帧处理后同样集成 MAT | `main.rs:2590` |
| 5 | `SensingUpdate` 新增 `triage_update: Option<TriageUpdate>` 字段 (START分诊+伤员追踪+告警) | `main.rs` struct |
| 6 | `lib.rs` 新增 `pub mod mat_pipeline;` | `lib.rs` |

### 2.4 编译结果

```
cargo check  → ✅ 编译通过
  - 0 errors
  - 247 warnings (226 from mat crate 缺失文档 + 21 from sensing-server 未使用字段)
  - warnings 均为非阻断性（缺失文档注释、未使用变量），不影响功能
```

### 2.5 triage.html 重写 — 消费服务端 MAT 数据

| # | 修改 | 说明 |
|---|------|------|
| 1 | `handleUpdate()` 优先读取 `data.triage_update` | 直接消费服务端 MAT 引擎输出 |
| 2 | 新增 `renderFromServer()` | 从 `TriageUpdate.assessment` 渲染统计栏、从 `TriageUpdate.survivors` 渲染伤员卡片、从 `TriageUpdate.alerts` 渲染告警列表 |
| 3 | 保留 `renderFromLocal()` 备用 | 兼容旧版服务器/无 MAT 场景 (JS 端 START 规则) |
| 4 | Canvas `draw()` 改为服务端位置 `s.position` | 不再用 JS 端 RSSI 三角估算 |
| 5 | 伤员卡片新增 `estimated_age` 和 `tracked_seconds` | 服务端追踪信息 |
| 6 | `deploy.sh` 路径修正 | `./competition/triage-ui/` → `./docs/triage-ui/` |

**效果**: triage.html 不再在浏览器端重复计算 START 分诊，完全消费服务端 Rust MAT pipeline 的 `TriageUpdate` 输出。

### 2.6 全目录文档审计修复 (2026-05-09 10:44)

对全部 13 个文档/配置文件逐行审计，修复过时路径和虚假引用：

| # | 文件 | 修复 |
|---|------|------|
| 1 | `docs/README_COMPETITION.md` | `competition/`→`docs/`、URL修正、目录树重写、架构图更新 |
| 2 | `docs/ESP32-C5 移植指南.md` | 矛盾结论统一、固件路径修正、删除不存在文件引用、C5改为推荐 |
| 3 | `README.md` | 删除不存在文件引用、文档表补全 |
| 4 | `docs/PROGRESS.md` | 阶段1文件表路径修正 (`competition/`→`docs/`、`rust-port/`→`rust-server/`) |
| 5 | `docs/竞赛改造方案.md` | N1代码示例重写(TriageEngine)、`competition/` 目录结构更新 |
| 6 | `docs/竞赛准备清单.md` | WASM数量(65→10)、`competition/`→`docs/` |
| 7 | `docs/竞赛差距分析.md` | 全局 `competition/`→`docs/` |
| 8 | `docs/ML架构详解.md` | 删除不存在crate引用 |
| 9 | `docs/ESP32-C5 移植审计报告.md` | `rust-port/`→`rust-server/` |
| 10 | `docs/瑞萨 RZV2H 移植计划.md` | `rust-port/`→`rust-server/` |
| 11 | `docs/目录审计报告.md` | Cargo.lock状态更新 |
| 12 | `docs/端侧LLM方案设计.md` | candle版本号(0.8→0.4) |
| 13 | `rust-server/Cargo.toml` | 删除4个幽灵workspace依赖(api/db/wasm/ruvector) |

**审计结论**: 13个文件修复完成，`cargo check` 仍然通过 ✅

---

## 进度总览

| 阶段 | 模块 | 状态 |
|:----:|------|:----:|
| P0 | 进度文档 + 竞赛 README | ✅ |
| P1 | MAT Pipeline (mat_pipeline.rs) | ✅ 已自审计 |
| P2 | 分诊仪表盘 (triage.html) | ✅ 已重写 (消费服务端 TriageUpdate) |
| P3 | 竞赛固件配置 | ✅ Kconfig 已验证 |
| P4 | 部署脚本 (deploy.sh) | ✅ CLI 参数已验证 |
| P5 | WASM 模块清单 | ✅ |
| P6 | 最终检查 (阶段1) | ✅ |
| **P7** | **Cargo.toml 修复 + 编译通过** | ✅ **2026-05-09** |
| **P8** | **MAT Pipeline 完整集成** | ✅ **2026-05-09** |
| P9 | 端侧 LLM 代码实现 | ❌ 待开发 |
| P10 | 竞赛申报材料 | ❌ 待准备 |
| P11 | 硬件联调 | ❌ 需硬件 |

## 新建/修改文件 (阶段1: 11个 + 阶段2: 17个)

### 阶段1 (2026-05-06)

| 文件 | 大小 | 审计状态 |
|------|------|:--:|
| `docs/PROGRESS.md` | — | ✅ |
| `docs/README_COMPETITION.md` | 5.1KB | ✅ |
| `docs/ML架构详解.md` | 12.7KB | ✅ |
| `docs/竞赛改造方案.md` | 16.8KB | ✅ |
| `docs/竞赛差距分析.md` | 7.7KB | ✅ |
| `docs/竞赛准备清单.md` | 14.9KB | ✅ |
| `docs/triage-ui/triage.html` | 14KB | ✅ 连接 `/ws/sensing` |
| `deploy.sh` | 4.2KB | ✅ CLI 参数正确 |
| `firmware/*/sdkconfig.defaults.competition` | 1.5KB | ✅ Kconfig 验证 |
| `rust-server/crates/wifi-densepose-sensing-server/src/mat_pipeline.rs` | 15.6KB | ✅ 纯分诊层 |

### 阶段2 (2026-05-09) — Cargo修复 + MAT集成

| 文件 | 修改类型 | 说明 |
|------|:--:|------|
| `rust-server/Cargo.toml` | 修改 | workspace 成员 16→8 |
| `sensing-server/Cargo.toml` | 修改 | wifiscan→mat+signal+vitals |
| `sensing-server/src/lib.rs` | 修改 | 添加 mat_pipeline 模块 |
| `sensing-server/src/main.rs` | 重大修改 | 删390行wifiscan + 集成MAT pipeline |
| `sensing-server/src/mat_pipeline.rs` | 修复 | breathing_rate/heart_rate 字段名 |
| `hardware/Cargo.toml` | 修改 | 移除缺失 bench |
| `nn/Cargo.toml` | 修改 | 移除缺失 bench |
| `signal/Cargo.toml` | 修改 | 移除缺失 bench |
| `mat/Cargo.toml` | 修改 | 移除缺失 bench |
| `Cargo.lock` | 新建 | `cargo check` 自动生成 |

## 数据流架构 (更新: MAT 已集成)

```
ESP32-C5 ×3                  RZ/V2H                             Browser
─────────────    ────────────────────────────    ─────────────────────────
CSI 采集        UDP:5005 → sensing-server
                            │
                            ├─ parse_esp32_frame()     → amplitudes/phases
                            ├─ VitalSignDetector        → VitalSigns (呼吸率/心率)
                            ├─ TriageEngine.process()   → TriageUpdate ⭐ NEW
                            │    ├─ START 分诊 (红/黄/绿/黑)
                            │    ├─ 伤员追踪 (创建/匹配/更新)
                            │    ├─ 恶化检测 + 告警生成
                            │    └─ 群体伤情评估
                            │
                            ├─ SensingUpdate 构造
                            │  ├─ vital_signs: VitalSigns
                            │  └─ triage_update: TriageUpdate ⭐ NEW
                            │
                            └─ WebSocket /ws/sensing ──→ triage.html
                                                         ├─ 伤员地图 (Canvas 2D)
                                                         ├─ 生命体征卡片
                                                         ├─ START 分诊状态
                                                         └─ 告警列表
```

## 待完成

### 代码层面

| 任务 | 优先级 | 依赖 |
|------|:--:|------|
| **端侧 LLM 代码实现** | 🔴 必须 | candle + Qwen2.5-0.5B GGUF |
| C5 固件编译 | 🔴 必须 | ESP-IDF v5.5+ |
| Rust aarch64 交叉编译 | 🔴 必须 | RZ/V2H SDK |
| 3 节点 烧录+联调 | 🔴 必须 | 硬件 |

### 竞赛材料

| 任务 | 优先级 |
|------|:--:|
| 竞赛申报书/项目简介 | 🔴 必须 |
| 答辩 PPT (12-15页) | 🔴 必须 |
| 演示脚本 (5分钟) | 🔴 必须 |
| 设计报告 | 🔴 必须 |
| 系统架构图 (展板) | 🟡 重要 |
| 性能测试数据 | 🟡 重要 |
| 评委快速卡片 | 🟡 重要 |
| 现场故障预案 | 🟡 重要 |
| 项目视频 (3分钟) | 🟢 加分 |

---

## 第二轮深度审计追加发现 (17:37)

| # | 发现 | 修正 |
|---|------|------|
| 6 | **main.rs ADR-018 解析器全部字节偏移错误** — n_subcarriers 读1字节(u8)应为2字节(u16), freq_mhz 读2字节(u16)应为4字节(u32), rssi/noise偏移全错 | 修正全部偏移 + Esp32Frame结构体类型 |
| 7 | csi_collector.c C5 条件编译 (acquire_csi_*, first_word_invalid, 6GHz) | ✅ 审计通过 |
| 8 | main.c C5 WiFi 双频配置 (set_band_mode/protocols/bandwidths) | ✅ 审计通过 |
| 9 | edge_processing.h C5 子载波常量 (512/2068) | ✅ 审计通过 |
| 10 | hardware/esp32_parser.rs ADR-018 格式 | ✅ 审计通过 (独立实现, 是正确的) |
| 11 | ESP32 UDP接收器 (main.rs:2785) — 正确处理3种magic, VitalSigns提取, WebSocket广播 | ✅ 审计通过 |
| 12 | VitalSignDetector (vital_signs.rs) — 完整FFT呼吸/心率管道 | ✅ 审计通过 |

**审计结论: 发现1个阻断性Bug (ADR-018解析器) 已修复。其余竞赛关键代码路径全部审计通过。**

