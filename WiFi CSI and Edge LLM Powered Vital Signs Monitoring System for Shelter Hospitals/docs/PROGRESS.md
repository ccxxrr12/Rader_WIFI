# 竞赛项目构建进度 ✅

> 构建完成: 2026-05-06 17:10 | 审计修正: 2026-05-06 17:15
> 状态: **阶段 1 代码构建完成 + 通过自审计**

---

## 审计修正记录

| # | 发现 | 修正 |
|---|------|------|
| 1 | `mat_pipeline.rs` 导入了未使用的 `Arc`/`parking_lot::RwLock` | 删除 (Cargo.toml 无此依赖) |
| 2 | `mat_pipeline.rs` 错误地重新实现了信号处理 (服务端已有 FFT VitalSignDetector) | 重写为纯分诊层 — 接受 VitalSigns 输入, 只做 START + 追踪 + 告警 |
| 3 | `triage.html` 连接不存在的 `/ws/triage` WebSocket | 改为连接现有 `/ws/sensing`,解析 `SensingUpdate` 格式 |
| 4 | `deploy.sh` 使用了不存在的 `--triage-ui` CLI 参数 | 改为 `cp triage.html → ui/` + 正确参数 `--ui-path --bind-addr --source` |
| 5 | 所有 sdkconfig 选项 | 逐项对照 Kconfig.projbuild — 全部真实存在 ✅ |

---

## 进度总览

| 阶段 | 模块 | 状态 |
|:----:|------|:----:|
| P0 | 进度文档 + 竞赛 README | ✅ |
| P1 | MAT Pipeline (mat_pipeline.rs) | ✅ 已自审计 |
| P2 | 分诊仪表盘 (triage.html) | ✅ 已自审计 |
| P3 | 竞赛固件配置 | ✅ Kconfig 已验证 |
| P4 | 部署脚本 (deploy.sh) | ✅ CLI 参数已验证 |
| P5 | WASM 模块清单 | ✅ |
| P6 | 最终检查 | ✅ |

## 新建/修改文件 (11个)

| 文件 | 大小 | 审计状态 |
|------|------|:--:|
| `competition/PROGRESS.md` | 2.4KB | ✅ |
| `competition/README_COMPETITION.md` | 5.1KB | ✅ |
| `competition/ML架构详解.md` | 12.7KB | ✅ |
| `competition/竞赛改造方案.md` | 16.8KB | ✅ |
| `competition/竞赛差距分析.md` | 7.7KB | ✅ |
| `competition/竞赛准备清单.md` | 14.9KB | ✅ |
| `competition/triage-ui/triage.html` | 13.7KB | ✅ 连接 `/ws/sensing` |
| `competition/deploy.sh` | 4.2KB | ✅ CLI 参数正确 |
| `competition/wasm-modules-competition.toml` | 2.5KB | ✅ |
| `firmware/*/sdkconfig.defaults.competition` | 1.5KB | ✅ Kconfig 验证 |
| `rust-port/*/mat_pipeline.rs` | 15.6KB | ✅ 纯分诊层 |

## 数据流架构

```
ESP32-C5 ×3                  RZ/V2H                             Browser
─────────────    ────────────────────────────    ─────────────────────────
CSI 采集        UDP:5005 → sensing-server
                            │
                            ├─ esp32_parser → amplitudes/phases
                            ├─ VitalSignDetector (FFT) → VitalSigns
                            ├─ 现有 /ws/sensing → SensingUpdate JSON
                            │                         │
                            │    ┌────────────────────┘
                            │    ▼
                            │  triage.html (JS)
                            │    ├─ START 分诊 (VitalSigns → 红/黄/绿/黑)
                            │    ├─ 伤员追踪 (NodeID 匹配)
                            │    ├─ 位置估算 (RSSI → 米)
                            │    └─ Canvas 2D 地图渲染
                            │
                            └─ [可选] ONNX DensePose → 3D骨架按钮
```

## 待完成 (需硬件)

| 任务 | 依赖 |
|------|------|
| C5 固件编译 | ESP-IDF v5.5+ |
| Rust aarch64 交叉编译 | RZ/V2H SDK |
| 3 节点 烧录+联调 | 硬件 |
| 设计报告 + 视频 + PPT | 联调通过 |

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

