# 示例

基于 RuView 平台构建的实时感知应用。

## 统一仪表板（从这里开始）

```bash
pip install pyserial numpy
python examples/ruview_live.py --csi COM7 --mmwave COM4
```

实时仪表板会自动检测可用传感器，并实时显示融合的生命体征、环境数据和事件。支持任何传感器组合。

## 单个示例

| 示例 | 传感器 | 功能 |
|------|--------|------|
| [**ruview_live.py**](ruview_live.py) | CSI + 毫米波 + 光线 | 统一仪表板：心率、呼吸率、血压、压力、存在、光线、RSSI |
| [医疗：血压](medical/) | 毫米波 | 基于 HRV 的非接触式血压估计 |
| [医疗：生命体征套件](medical/vitals_suite.py) | 毫米波 | 十合一：心率、呼吸率、血压、心率变异性、睡眠阶段、呼吸暂停、咳嗽、打鼾、活动、冥想 |
| [睡眠：呼吸暂停筛查](sleep/) | 毫米波 | 检测呼吸停止事件，计算 AHI |
| [压力：心率变异性监测](stress/) | 毫米波 | 基于心率变异性的实时压力水平 |
| [环境：房间监测](environment/) | CSI + 毫米波 | 占用、光线、射频指纹、活动事件 |

## 硬件

| 端口 | 设备 | 成本 | 提供功能 |
|------|------|------|----------|
| COM7 | ESP32-S3 (WiFi CSI) | ~$9 | 存在、运动、呼吸、心率（穿墙） |
| COM4 | ESP32-C6 + Seeed MR60BHA2 | ~$15 | 精确心率/呼吸率、存在、距离、环境光 |

任一传感器可单独工作。两者一起使用可实现融合（毫米波 80% + CSI 20%）。

## 快速开始

```bash
pip install pyserial numpy

# 统一仪表板（推荐）
python examples/ruview_live.py --csi COM7 --mmwave COM4

# 血压估计
python examples/medical/bp_estimator.py --port COM4

# 睡眠呼吸暂停筛查（通宵运行）
python examples/sleep/apnea_screener.py --port COM4 --duration 28800

# 压力监测（工作日会话）
python examples/stress/hrv_stress_monitor.py --port COM4 --duration 3600

# 房间环境监测
python examples/environment/room_monitor.py --csi-port COM7 --mmwave-port COM4

# 仅使用 CSI（无毫米波）
python examples/ruview_live.py --csi COM7 --mmwave none
```