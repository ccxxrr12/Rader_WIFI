# 《基于WiFi信号的密集人体姿态估计》

Jiaqi Geng
[jiaqigen@andrew.cmu.edu](mailto:jiaqigen@andrew.cmu.edu)
美国宾夕法尼亚州匹兹堡市，卡内基梅隆大学

Dong Huang
[donghuang@cmu.edu](mailto:donghuang@cmu.edu)
美国宾夕法尼亚州匹兹堡市，卡内基梅隆大学

Fernando De la Torre
[ftorre@cs.cmu.edu](mailto:ftorre@cs.cmu.edu)
美国宾夕法尼亚州匹兹堡市，卡内基梅隆大学

---

## 摘要

计算机视觉与机器学习技术的进步，推动了基于 RGB 相机、激光雷达与雷达的二维与三维人体姿态估计技术取得显著发展。然而，基于图像的人体姿态估计方法易受遮挡与光照的负面影响，这在诸多目标场景中都十分常见。另一方面，雷达与激光雷达技术需要专用的硬件设备，这类设备不仅价格高昂，而且功耗较高。此外，在非公共区域部署这类传感器还会引发严重的隐私顾虑。

为解决这些局限，近期研究探索了将 WiFi 天线（一维传感器）用于人体分割与人体关键点检测的方案。本文进一步拓展了 WiFi 信号的应用，结合计算机视觉领域常用的深度学习架构，实现密集人体姿态对应关系的估计。我们构建了一个深度神经网络，可将 WiFi 信号的相位与幅度映射至 24 个人体区域内的 UV 坐标。研究结果表明，我们的模型仅以 WiFi 信号作为输入，即可估计多个目标的密集姿态，性能可与基于图像的方法相媲美。这为低成本、普适性且保护隐私的人体感知算法铺平了道路。

---

## CCS 分类概念

- **计算方法学** → 神经网络；人工智能；机器学习

- **硬件** → 通信硬件、接口与存储；鲁棒性

---

## 关键词

姿态估计，密集人体姿态估计，WiFi 信号，关键点估计，人体分割，目标检测，UV 坐标，相位与幅度，相位净化，信道状态信息，域转换，深度神经网络，Mask R-CNN

---

## 1 引言

过去几年间，得益于自动驾驶与增强现实领域的应用需求，基于二维 [7, 8, 12, 22, 28, 33] 与三维 [17, 32] 传感器（例如 RGB 传感器、激光雷达、雷达）的人体姿态估计技术已取得了长足进展。然而，这些传统传感器受限于技术与实际应用层面的诸多约束。激光雷达与雷达传感器价格高昂，普通家庭或小型商户往往难以负担。例如，最常见的商用现货激光雷达之一 Intel L515，均价约为 700 美元，而普通雷达探测器的价格也在 200 至 600 美元之间。此外，这类传感器功耗过高，并不适用于日常家庭使用。对于 RGB 相机而言，狭窄的视场与恶劣的光照条件（如眩光、黑暗环境）会严重影响基于相机的方法的效果。遮挡则是另一大阻碍，它会导致基于相机的模型无法在图像中生成合理的姿态预测，这一问题在室内场景中尤为突出 —— 室内的家具通常会遮挡人体。

更重要的是，隐私顾虑使得这些技术无法应用于非公共区域。例如，大多数人都不愿在家中被摄像头记录，而在某些区域（如浴室），安装摄像头更是完全不可行。这些问题在医疗应用中尤为关键：如今医疗服务正逐渐从诊所转向家庭，人们需要借助摄像头与其他传感器进行居家监测。为了更好地帮扶老龄人口 —— 这一群体最易受疾病影响（尤其是新冠疫情期间），且对居家独立生活的需求日益增长 —— 解决上述问题至关重要。

我们认为，在特定场景下，WiFi 信号可以作为 RGB 图像的通用替代方案，用于人体感知。用于室内监测的 WiFi 方案几乎不受光照与遮挡的影响，同时还能保护用户隐私，所需设备的价格也十分亲民。事实上，发达国家的大多数家庭已经部署了 WiFi，这项技术可以扩展应用于监测老年人的健康状况，或是识别家中的异常行为。

本文要解决的问题如图 1 第一行所示。给定 3 个 WiFi 发射天线与 3 个对齐的接收天线，我们能否在存在多个人体的复杂场景中，检测并恢复密集人体姿态对应关系（图 1 第四行）？值得注意的是，许多 WiFi 路由器（如 TP-Link AC1750）都配备了 3 根天线，因此我们的方法仅需要 2 台这类路由器即可。每台路由器的价格约为 30 美元，这意味着我们的整套设备仍远低于激光雷达与雷达系统的成本。

诸多因素使得这一任务极具挑战性。首先，基于 WiFi 的感知 [11, 30] 依赖于信道状态信息（CSI），该信息表征了发射信号波与接收信号波的比值。CSI 是复数值序列，与图像像素这类空间位置并不存在直接的空间对应关系。其次，传统技术依赖于对发射端与接收端之间信号的飞行时间与到达角的精确测量 [13, 26]。这类技术仅能定位目标的中心，而且受 IEEE 802.11n/ac WiFi 通信标准允许的随机相移，以及微波炉、手机等同频段电子设备的潜在干扰影响，其定位精度仅约 0.5 米。

为解决这些问题，我们从计算机视觉领域近期提出的深度学习架构中获取灵感，提出了一种可基于 WiFi 信号实现密集姿态估计的神经网络架构。图 1 的最后一行展示了我们的算法如何在存在遮挡与多个人体的场景中，仅使用 WiFi 信号估计密集姿态。

![Image](https://p11-flow-imagex-sign.byteimg.com/tos-cn-i-a9rns2rl98/rc/online_export/491018865e8149ce9190c8f53b8e0b6b~tplv-noop.jpeg?rk3s=49177a0b&x-expires=1775009845&x-signature=fxBw%2BmKGNSkC9Atkir0CREIRaKs%3D&resource_key=c2ee1c27-11fe-4ef1-9dfd-aba7791d3aec&resource_key=c2ee1c27-11fe-4ef1-9dfd-aba7791d3aec)

---

## 2 相关工作

本节简要介绍现有的图像密集估计与 WiFi 人体感知相关研究。

我们的研究目标是通过 WiFi 实现密集人体姿态估计。在计算机视觉领域，基于图像与视频的密集姿态估计已得到广泛关注 [6, 8, 18, 40]。该任务的核心是找到图像像素与三维人体模型密集顶点索引之间的密集对应关系。Güler 等人 [8] 的开创性工作使用深度网络，将人体图像映射至人体网格模型的密集对应关系。DensePose 基于 Mask R-CNN [9] 这类实例分割架构，为每个像素预测分人体的 UV 图 ——UV 图是三维几何的扁平化表示，其坐标点通常对应三维物体的顶点。在本文中，我们沿用了 DensePose [8] 的架构，但我们的输入不再是图像或视频，而是使用一维 WiFi 信号来恢复密集对应关系。

近期，研究者提出了诸多 DensePose 的扩展工作，尤其是结合密集身体部位的三维人体重建方向 [3, 35, 37, 38]。Shapovalov 等人 [24] 的工作聚焦于将密集姿态表面映射提升至无三维监督的三维人体模型。他们的网络证明，仅密集对应关系（无需使用完整的二维 RGB 图像）就已包含足够的信息，可生成带姿态的三维人体。与以往使用稀疏二维关键点重建三维人体的工作相比，DensePose 标注的密度要高得多，且能提供三维表面的信息，而非仅二维人体关节的信息。

尽管基于图像与视频的检测 [19, 20]、跟踪 [4, 34] 与密集姿态估计 [8, 18] 已有大量研究，但基于 WiFi 或雷达的人体姿态估计仍是一个相对未被探索的问题。在此，有必要区分当前基于雷达的系统与 WiFi 系统的差异。Adib 等人 [2] 提出了一种调频连续波（FMCW）雷达系统（带宽为 5.56GHz 至 7.25GHz），用于室内人体定位。该系统的局限在于，它需要专用硬件来同步发射、折射与反射过程，以计算飞行时间（ToF），该系统的人体定位分辨率可达 8.8 厘米。在后续工作 [1] 中，他们对系统进行了改进，聚焦于移动的人体，并结合深度图生成了粗略的单人轮廓。近期，他们将深度学习方法应用于类似系统，实现了细粒度的人体姿态估计，该系统名为 RF-Pose [39]。这些系统并不兼容 IEEE 802.11n/ac WiFi 通信标准（中心频率 2.4GHz、40MHz 带宽），它们依赖额外的高频、高带宽电磁场，需要普通公众无法获取的专用技术。

近期，基于雷达的人体感知系统已取得显著进展。mmMesh [36] 可基于商用便携式毫米波设备生成三维人体网格。该系统能够精准定位人体网格上的顶点，平均误差为 2.47 厘米。然而，mmMesh 在遮挡场景下表现不佳，因为高频无线电波无法穿透物体。

与上述雷达系统不同，基于 WiFi 的方案 [11, 30] 使用商用现货 WiFi 适配器与 3dB 全向天线。信号以 IEEE 802.11n/ac WiFi 数据包的形式在天线之间传输，不会引入额外干扰。但传统飞行时间（ToF）方法的 WiFi 人体定位，受波长与信噪比的限制。大多数现有方法仅能实现质心定位 [5, 27] 与单人动作分类 [25, 29]。近期，Fei Wang 等人 [31] 证明，仅使用 WiFi 信号就可以检测 17 个二维人体关节，并实现二维语义人体分割掩码。在本文中，我们在 [31] 的基础上更进一步，实现了密集人体姿态估计，精度远超 WiFi 信号理论上可提供的 0.5 米定位能力。我们的密集姿态输出突破了 WiFi 信号在人体定位上的约束，为通过 WiFi 实现完整的密集二维乃至三维人体感知铺平了道路。为实现这一目标，我们没有直接训练随机初始化的 WiFi 模型，而是探索了丰富的监督信息来提升模型性能与训练效率，例如利用 CSI 相位、添加关键点检测分支，以及从基于图像的模型进行迁移学习。

---

## 3 方法

我们的方法通过三个组件，从 WiFi 信号中得到人体表面的 UV 坐标：首先，通过幅度与相位净化对原始 CSI 信号进行清洗；随后，双分支编码器 - 解码器网络执行域转换，将净化后的 CSI 样本转换为类似图像的二维特征图；最后，将二维特征输入改进的 DensePose-RCNN 架构 [8]，以估计 UV 图 —— 这是二维与三维人体之间密集对应关系的表征。为优化我们的 WiFi 输入网络的训练过程，我们采用了迁移学习，在训练主网络之前，最小化图像与 WiFi 信号生成的多级特征图之间的差异。

原始 CSI 数据以 100Hz 的频率采样，覆盖 30 个子载波频率（在 2.4GHz±20MHz 范围内线性分布），在 3 个发射天线与 3 个接收天线之间传输（见图 2）。每个 CSI 样本包含一个 3×3 的实整数矩阵与一个 3×3 的虚整数矩阵。我们网络的输入包含 30 个频率下的 5 个连续 CSI 样本，分别组织为 150×3×3 的幅度张量与 150×3×3 的相位张量。我们的网络输出包括：17×56×56 的关键点热图张量（17 个关键点各对应一个 56×56 的热图），以及 25×112×112 的 UV 图张量（24 个人体部位各对应一个 112×112 的图，额外一个图用于背景）。

![Image](https://p11-flow-imagex-sign.byteimg.com/tos-cn-i-a9rns2rl98/rc/online_export/8a93e6d352b14fbdb7a17f9ea2ef82fd~tplv-noop.jpeg?rk3s=49177a0b&x-expires=1775009845&x-signature=gwvRVUI24HYe21riqq4fkPqcF6Y%3D&resource_key=cfbbbaed-1f21-4000-81a9-e277c6b21b26&resource_key=cfbbbaed-1f21-4000-81a9-e277c6b21b26)

### 3.1 相位净化

原始 CSI 样本存在噪声，伴随随机的相位漂移与翻转（见图 3 (b)）。大多数 WiFi 方案都会忽略 CSI 的相位信息，仅依赖其幅度（见图 3 (a)）。正如我们的实验验证所示，丢弃相位信息会对模型性能产生负面影响。在本节中，我们通过净化操作得到稳定的相位值，以充分利用 CSI 的全部信息。

在原始 CSI 样本中（图 3 (a-b) 可视化了 5 个连续样本），每个复元素 $z=a+bi$ 的幅度 $A$ 与相位 $\Phi$ 通过公式 $A=\sqrt{(a^{2}+b^{2})}$ 与 $\Phi=\arctan (b / a)$ 计算得到。注意，反正切函数的取值范围为 $-\pi$ 到 $\pi$ ，超出该范围的相位值会发生卷绕，导致相位值出现不连续。我们的第一步净化操作是按照 [10] 的方法对相位进行解卷绕：
$\begin{array}{rlrl}{\Delta \phi _{i,j}}&{=\Phi _{i,j+1}-\Phi _{i,j}}\\
{if \Delta \phi _{i,j}>\pi ,\Phi _{i,j+1}}&{=\Phi _{i,j}+\Delta \phi _{i,j}-2\pi }\\
{if \Delta \phi _{i,j}<-\pi ,\Phi _{i,j+1}}&{=\Phi _{i,j}+\Delta \phi _{i,j}+2\pi ,}
\end{array}$
其中 $i$ 表示 5 个连续样本中的测量索引， $j$ 表示子载波（频率）的索引。解卷绕后，图 3 (b) 中原本翻转的相位曲线被恢复为图 3 (c) 中的连续曲线。

可以观察到，在图 3 (c) 的 5 个连续样本对应的 5 条相位曲线中，存在随机抖动，破坏了样本之间的时间顺序。为了保持信号的时间顺序，之前的工作 [23] 提到线性拟合是一种常用方法。但直接对图 3 (c) 应用线性拟合，反而会放大抖动，而非修复它（失败结果见图 3 (d)）。

针对图 3 (c)，我们使用中值滤波与均匀滤波，消除时域与频域的异常值，得到图 3 (e) 的结果。最后，我们通过如下公式的线性拟合方法，得到完全净化后的相位值：
$\begin{aligned}
& \alpha_{1}=\frac{\Phi_{F}-\Phi_{1}}{2 \pi F} \\
& \alpha_{0}=\frac{1}{F} \sum_{1 \leq f \leq F} \phi_{f} \\
& \hat{\phi_{f}}=\phi_{f}-\left(\alpha_{1} f+a_{0}\right),
\end{aligned}$
其中 $F$ 表示最大子载波索引（在我们的设置中为 30）， $\hat{\phi_{f}}$ 是子载波 $f$ （即第 $f$ 个频率）处的净化后相位值。在图 3 (f) 中，最终的相位曲线在时间上保持了一致性。

![Image](https://p11-flow-imagex-sign.byteimg.com/tos-cn-i-a9rns2rl98/rc/online_export/16e4ded2d239439ea3063df27cb5ab53~tplv-noop.jpeg?rk3s=49177a0b&x-expires=1775009845&x-signature=W0aU3YeiC%2Bdfo4%2BwrT5Hrin30d4%3D&resource_key=11f3ca3e-69fa-4eb9-8405-b65482a23b81&resource_key=11f3ca3e-69fa-4eb9-8405-b65482a23b81)

### 3.2 模态转换网络

为了从一维 CSI 信号中估计空间域的 UV 图，我们首先将网络输入从 CSI 域转换至空间域，这一过程通过模态转换网络实现（见图 4）。我们首先使用两个编码器提取 CSI 隐空间特征，一个用于幅度张量，另一个用于相位张量，两个张量的尺寸均为 150×3×3（5 个连续样本、30 个频率、3 个发射端与 3 个接收端）。

之前的 WiFi 人体感知工作 [30] 指出，卷积神经网络（CNN）可以从输入张量的最后两个维度（即 3×3 的发射传感器对）中提取空间特征。但我们认为，3×3 特征图中的位置与二维场景中的位置并不相关。具体来说，正如图 2 (b) 所示，蓝色标记的元素代表发射端 1 与接收端 3（E1-R3）捕获的整个场景的一维汇总，而非二维场景右上角的局部空间信息。因此，我们认为两个张量中的 1350 个元素，每个都捕获了整个场景的唯一一维汇总。基于这一思路，我们将幅度与相位张量展平，输入两个独立的多层感知机（MLP），以得到它们在 CSI 隐空间中的特征。我们将两个编码分支的一维特征拼接，随后将合并后的张量输入另一个 MLP 进行特征融合。

下一步是将 CSI 隐空间特征转换为空间域的特征图。如图 4 所示，融合后的一维特征被重塑为 24×24 的二维特征图。随后，我们通过两个卷积块提取空间信息，得到空间维度为 6×6 的更紧凑的特征图。最后，我们使用 4 个反卷积层，将低维的编码特征图上采样至 3×720×1280 的尺寸。我们设置这一输出张量尺寸，以匹配 RGB 图像输入网络的常用维度。至此，我们得到了由 WiFi 信号生成的、图像域的场景表征。

![Image](https://p11-flow-imagex-sign.byteimg.com/tos-cn-i-a9rns2rl98/rc/online_export/7b48b0ce2a0049dd82e2baa7baf6e00f~tplv-noop.jpeg?rk3s=49177a0b&x-expires=1775009845&x-signature=Ih8Yv3NRXVQco%2BWZOElLYFTxOnU%3D&resource_key=db96bee6-94d9-4c5b-86ce-aa62a36065c7&resource_key=db96bee6-94d9-4c5b-86ce-aa62a36065c7)

### 3.3 WiFi-DensePose RCNN

在得到图像域的 3×720×1280 场景表征后，我们就可以利用基于图像的方法来预测人体的 UV 图。当前最先进的姿态估计算法都是两阶段的：首先运行独立的人体检测器估计边界框，再基于分人体的图像块进行姿态估计。但正如之前所述，我们的 CSI 输入张量中的每个元素都是整个场景的汇总，无法从场景中的多个人体中提取对应单个人体的信号。因此，我们采用了与 DensePose-RCNN [8] 类似的网络结构，因为它可以端到端地预测多个人体的密集对应关系。

具体来说，在 WiFi-DensePose RCNN（图 5）中，我们使用 ResNet-FPN 骨干网络 [14]，从得到的 3×720×1280 类图像特征图中提取空间特征。随后，输出会经过区域候选网络 [20]。为了更好地利用不同来源的互补信息，网络的后续部分包含两个分支：密集姿态头与关键点头。关键点位置的估计比密集对应关系的估计更可靠，因此我们可以训练网络，利用关键点来约束密集姿态的预测，避免其偏离人体关节过远。

密集姿态头使用全卷积网络（FCN）[16]，密集预测人体部位标签，以及每个部位内的表面坐标（UV 坐标）；而关键点头则使用 FCN 估计关键点热图。结果被合并后输入每个分支的细化单元，每个细化单元包含两个卷积块，后接一个全卷积网络。网络输出 17×56×56 的关键点掩码，以及 25×112×112 的 IUV 图。这一过程如图 5 所示。需要注意的是，模态转换网络与 WiFi-DensePose RCNN 是联合训练的。

![Image](https://p11-flow-imagex-sign.byteimg.com/tos-cn-i-a9rns2rl98/rc/online_export/6e29bb33a3764c9c92a7fe983c416dce~tplv-noop.jpeg?rk3s=49177a0b&x-expires=1775009845&x-signature=vh2IVRJVOdZyrKvQWNx%2BExTS%2Bv8%3D&resource_key=0c2b44a8-4427-4b5c-a44b-87ed88ed4e8e&resource_key=0c2b44a8-4427-4b5c-a44b-87ed88ed4e8e)

### 3.4 迁移学习

从随机初始化开始训练模态转换网络与 WiFi-DensePose RCNN 网络需要大量时间（约 80 小时）。为了提升训练效率，我们从基于图像的 DensePose 网络向我们的 WiFi 网络进行迁移学习（细节见图 6）。

核心思路是用预训练的基于图像的网络，监督 WiFi 网络的训练。直接用图像网络的权重初始化 WiFi 网络是行不通的，因为两个网络的输入来自不同的域（图像与信道状态信息）。因此，我们首先训练一个基于图像的 DensePose-RCNN 模型作为教师网络。我们的学生网络则由模态转换网络与 WiFi-DensePose RCNN 组成。我们固定教师网络的权重，分别向教师与学生网络输入同步的图像与 CSI 张量，以此训练学生网络。我们更新学生网络，使其骨干网络（ResNet）的特征模仿教师网络的特征。我们的迁移学习目标是最小化学生模型与教师模型生成的多级特征图之间的差异，因此我们计算特征图之间的均方误差。从教师网络到学生网络的迁移学习损失为：
 $L_{t r}=MSE\left(P_{2}, P_{2}^{*}\right)+MSE\left(P_{3}, P_{3}^{*}\right)+MSE\left(P_{4}, P_{4}^{*}\right)+MSE\left(P_{5}, P_{5}^{*}\right),(3)$ 
其中 $MSE(·)$ 计算两个特征图之间的均方误差， ${P_{2}, P_{3}, P_{4}, P_{5}}$ 是教师网络 [14] 生成的特征图集合， ${P_{2}^{*}, P_{3}^{*}, P_{4}^{*}, P_{5}^{*}}$ 是学生网络 [14] 生成的特征图集合。

得益于图像模型的额外监督，学生网络的性能更高，且收敛所需的迭代次数更少（结果见表 5）。

![Image](https://p11-flow-imagex-sign.byteimg.com/tos-cn-i-a9rns2rl98/rc/online_export/2d918d00ce3944ccbf31454a1bab453a~tplv-noop.jpeg?rk3s=49177a0b&x-expires=1775009845&x-signature=zjLGMIhcxsAPbqrCoGHldKvMuhU%3D&resource_key=c9ee92cc-cbb9-4d2d-b933-76b4b57b763e&resource_key=c9ee92cc-cbb9-4d2d-b933-76b4b57b763e)

### 3.5 损失函数

我们方法的总损失计算如下：
$\begin{array}{rlr}{L}&{=}&{L_{cls}+L_{box}+\lambda *{dp}L*{dp}+\lambda *{kp}L*{kp}+\lambda *{tr}L*{tr},}
\end{array}$
其中 $L_{cls}$ 、 $L_{box}$ 、 $L_{dp}$ 、 $L_{kp}$ 、 $L_{tr}$ 分别是人体分类、边界框回归、密集姿态、关键点与迁移学习的损失。分类损失 $L_{cls}$ 与边界框回归损失 $L_{box}$ 是标准的 RCNN 损失 [9, 21]。密集姿态损失 $L_{dp}$ [8] 包含多个子部分：(1) 粗分割任务的交叉熵损失，每个像素被分类为背景或 24 个人体区域之一；(2) 身体部位分类的交叉熵损失，以及 UV 坐标回归的平滑 L1 损失。这些损失用于确定像素的精确坐标，即我们构建 24 个回归器，将整个人体划分为小的部位，每个部位使用局部的二维 UV 坐标系进行参数化，该坐标系用于标识该表面部位上 UV 节点的位置。

我们添加 $L_{kp}$ 来帮助密集姿态任务平衡 UV 节点更多的躯干与 UV 节点更少的四肢。受 Keypoint RCNN [9] 的启发，我们将 17 个真值关键点分别独热编码到一个 56×56 的热图中，生成 17×56×56 的关键点热图，并使用交叉熵损失监督输出。为了对密集姿态回归进行紧密正则化，关键点热图回归器使用与密集姿态 UV 图相同的输入特征。

---

## 4 实验

本节展示我们的 WiFi 密集姿态方法的实验验证。

### 4.1 数据集

我们使用了 [31] 中描述的数据集 1，该数据集包含接收天线以 100Hz 采集的 CSI 样本，以及 20FPS 录制的视频。我们使用时间戳同步 CSI 与视频帧，使得 5 个 CSI 样本对应 1 个视频帧。该数据集在 16 种空间布局下采集：6 次在实验室办公室，10 次在教室。每次采集时长约 13 分钟，包含 1 到 5 个目标（整个数据集共 8 个目标），在图 2 (a) 所述的布局下进行日常活动。这 16 种空间布局的差异在于 WiFi 发射天线、人体、家具与 WiFi 接收天线的相对位置 / 朝向。

该数据集没有人工标注。我们使用 MS-COCO 预训练的密集模型`R_101_FPN_s1x_legacy`2 与 MS-COCO 预训练的 Keypoint R-CNN`R101-FPN`3 来生成伪真值。我们将该真值记为 “R101 伪真值”（标注示例如图 7 所示）。R101 伪真值包含人体边界框、人体实例分割掩码、身体部位 UV 图，以及分人体的关键点坐标。

> 1 为保护隐私，该数据集中的可识别信息已被移除。
> 2 [https://github.com/facebookresearch/detectron2/blob/main/projects/DensePose/doc/DENSEPOSE_IUV.md#ModelZoo](https://github.com/facebookresearch/detectron2/blob/main/projects/DensePose/doc/DENSEPOSE_IUV.md#ModelZoo)
> 3 [https://github.com/facebookresearch/detectron2/blob/main/MODEL_ZOO.md#coco-person-keypoint-detection-baselines-with-keypoint-r-cnn](https://github.com/facebookresearch/detectron2/blob/main/MODEL_ZOO.md#coco-person-keypoint-detection-baselines-with-keypoint-r-cnn)
> 
> 

在本节中，我们使用 R101 伪真值来训练我们的 WiFi 密集姿态模型，同时微调基于图像的基线模型`R_50_FPN_s1x_legacy`。

![Image](https://p11-flow-imagex-sign.byteimg.com/tos-cn-i-a9rns2rl98/rc/online_export/53b94aa7b68943c6bc9c80e992d26498~tplv-noop.jpeg?rk3s=49177a0b&x-expires=1775009845&x-signature=yuJCVH%2BHMyVM41Ez3fO%2BnQKF6PU%3D&resource_key=07d0720b-7045-4a09-bc36-384660ea825e&resource_key=07d0720b-7045-4a09-bc36-384660ea825e)

### 4.2 训练 / 测试协议与评估指标

我们在两种协议下报告结果：

1. **同布局**：我们在全部 16 种空间布局的训练集上训练，在剩余帧上测试。遵循 [31] 的设置，我们随机选择 80% 的样本作为训练集，其余作为测试集。训练与测试样本的人体位置与姿态不同，但共享相同的人体身份与背景。这是一个合理的假设，因为 WiFi 设备通常安装在固定位置。

2. **跨布局**：我们在 15 种空间布局上训练，在 1 个未见过的空间布局上测试。该未见过的布局属于教室场景。

我们从两个方面评估算法的性能：检测人体的能力（边界框），以及密集姿态估计的精度。

为评估模型检测人体的性能，我们计算人体边界框在 0.5 到 0.95 的多个 IOU 阈值下的标准平均精度（AP）。

此外，根据 MS-COCO [15] 的定义，我们还计算了 AP-m（中等人体的 AP，对应在归一化 640×480 像素图像空间中，边界框尺寸在 32×32 到 96×96 像素之间的人体），以及 AP-l（大人体的 AP，对应边界框尺寸大于 96×96 像素的人体）。

为衡量密集姿态检测的性能，我们遵循原始 DensePose 论文 [8] 的方法。首先计算测地点相似度（GPS）作为密集对应关系的匹配分数：
 $G P S_{j}=\frac{1}{\left|P_{j}\right|} \sum_{p \in P_{j}} exp \left(\frac{-g\left(i_{p}, \hat{i}_{p}\right)^{2}}{2 \kappa^{2}}\right),$ 
其中 $g$ 计算测地距离， $P_{j}$ 表示人体 $j$ 的真值点标注， $i_{p}$ 与 $\hat{i}_{p}$ 分别是点 $p$ 处的估计顶点与真值顶点， $\kappa$ 是归一化参数，根据 [8] 设置为 0.255。

GPS 的一个问题是它不会对虚假预测进行惩罚，因此所有像素都被分类为前景的估计会得到更高的分数。为缓解这一问题，[8] 中引入了掩码测地点相似度（GPSm），它结合了 GPS 与分割掩码，公式如下：
 $G P S m=\sqrt{G P S \cdot I}, I=\frac{M \cap \hat{M}}{M \cup \hat{M}},$ 
其中 $M$ 与 $\hat{M}$ 分别是预测与真值的前景分割掩码。

接下来，我们可以遵循边界框 AP 的计算逻辑，分别以 GPS（记为 dpAP・GPS）与 GPSm（记为 dpAP・GPSm）为阈值，计算密集姿态的平均精度。

### 4.3 实现细节

我们在 PyTorch 中实现了我们的方法。在配备 4 块 Titan X GPU 的服务器上，我们将训练批次大小设置为 16。我们根据经验设置 $\lambda_{dp}=0.6$ 、 $\lambda_{kp}=0.3$ 、 $\lambda_{tr}=0.1$ 。我们使用热身多步学习率调度器，初始学习率设置为 $1e-5$ 。在前 2000 次迭代中，学习率提升至 $1e-3$ ，随后每 48000 次迭代，学习率降为当前值的 1/10。我们的最终模型共训练了 145000 次迭代。

### 4.4 同布局下的 WiFi 密集姿态

在同布局协议下，我们计算人体边界框检测的 AP，以及密集对应预测的 dpAP・GPS 与 dpAP・GPSm，结果分别如表 1 与表 2 所示。

|方法|AP|AP@50|AP@75|AP-m|AP-l|
|---|---|---|---|---|---|
|WiFi|43.5|87.2|44.6|38.1|46.4|
表 1: 同布局协议下，基于 WiFi 的密集姿态的平均精度（AP）。所有指标均为越高越好。

从表 1 可以观察到，AP@50 达到了 87.2 的高值，说明我们的模型可以有效检测人体边界框的大致位置。而 AP@75 的数值相对较低（35.6），说明人体的细节尚未被完美估计。

密集姿态估计的结果也呈现出类似的规律（详见表 2）。实验显示，dpAP・GPS@50 与 dpAP・GPSm@50 的数值较高，但 dpAP・GPS@75 与 dpAP・GPSm@75 的数值较低。这说明我们的模型在估计人体躯干的姿态时表现良好，但在检测四肢这类细节时仍存在困难。

|方法|dpAP·GPS|dpAP·GPS@50|dpAP·GPS@75|dpAP·GPSm|dpAP·GPSm@50|dpAP·GPSm@75|
|---|---|---|---|---|---|---|
|WiFi|45.3|76.7|47.7|44.8|73.6|44.9|
表 2: 同布局协议下，基于 WiFi 的密集姿态的密集姿态平均精度（dpAP・GPS、dpAP・GPSm）。所有指标均为越高越好。

### 4.5 与基于图像的密集姿态的对比

正如 4.1 节所述，由于 WiFi 数据集没有人工标注，我们很难直接对比 WiFi 密集姿态与图像密集姿态的性能，这也是包括 [31] 在内的诸多 WiFi 感知工作的共同局限。

尽管如此，进行这一对比对于评估 WiFi 感知的当前极限仍有价值。我们测试了一个基于图像的 DensePose 基线模型`R_50_FPN_s1x_legacy`，它在同布局协议下，基于 R101 伪真值进行微调。此外，如图 9 与图 10 所示，尽管仍存在部分缺陷，但我们的 WiFi 模型的估计结果，与基于图像的 DensePose 的结果相比，已经达到了相当不错的水平。

在表 3 与表 4 的定量结果中，由于图像基线的 ResNet50 骨干网络，与生成 R101 伪真值的 ResNet101 骨干网络差异很小，因此图像基线得到了非常高的 AP 值，这是符合预期的。我们的 WiFi 模型的绝对指标要低得多，但从表 3 可以观察到，WiFi 模型的 AP-m 与 AP-l 的差异相对较小。我们认为这是因为，远离相机的人体在图像中占据的空间更小，导致这些目标的信息更少；而 WiFi 信号会整合整个场景的全部信息，与目标的位置无关。

|方法|AP|AP@50|AP@75|AP-m|AP-l|
|---|---|---|---|---|---|
|WiFi|43.5|87.2|44.6|38.1|46.4|
|Image|84.7|94.4|77.1|70.3|83.8|
表 3: 同布局协议下，基于 WiFi 与基于图像的密集姿态的平均精度（AP）。所有指标均为越高越好。

|方法|dpAP·GPS|dpAP·GPS@50|dpAP·GPS@75|dpAP·GPSm|dpAP·GPSm@50|dpAP·GPSm@75|
|---|---|---|---|---|---|---|
|Image|81.8|93.7|86.2|84.0|94.9|86.8|
|WiFi|45.3|79.3|47.7|43.2|77.4|45.5|
表 4: 同布局协议下，基于 WiFi 与基于图像的密集姿态的密集姿态平均精度（dpAP・GPS、dpAP・GPSm）。所有指标均为越高越好。

### 4.6 消融实验

本节通过消融实验，理解相位信息、关键点监督与迁移学习对估计密集对应关系的影响。与 4.4 节类似，本节分析的所有模型均在 4.2 节所述的同布局下训练。

我们首先训练了一个基线 WiFi 模型，该模型不包含相位编码器、关键点检测分支与迁移学习，结果在表 5 与表 6 的第一行展示，作为参考。

**添加相位信息**：我们首先验证相位信息是否能提升基线性能。如表 5 与表 6 的第二行所示，所有指标的结果都相比基线有小幅提升，这验证了我们的假设：相位可以揭示与密集人体姿态相关的有效信息。

**添加关键点检测分支**：在验证了加入相位信息的优势后，我们评估为模型添加关键点分支的效果。定量结果总结在表 5 与表 6 的第三行。

与第二行的数值对比可以观察到，dpAP・GPS@50（从 77.4 提升至 78.8）与 dpAP・GPSm@50（从 75.7 提升至 76.8）有小幅提升，而 dpAP・GPS@75（从 42.3 提升至 46.9）与 dpAP・GPSm@75（从 40.5 提升至 44.9）的提升更为显著。这说明关键点分支为密集姿态估计提供了有效的参考，我们的模型在检测细微细节（如四肢）上的能力得到了明显提升。

**迁移学习的效果**：我们的目标是借助迁移学习减少模型的训练时间。对于表 5 中的每个模型，我们持续训练模型，直到性能不再有显著变化。表 5 与表 6 的最后一行代表我们加入迁移学习的最终模型。尽管与（加入了相位信息与关键点的）无迁移学习的模型相比，最终性能的提升并不多，但需要注意的是，训练迭代次数从 186000 显著减少至 145000（该数值已包含迁移学习与主模型训练的时间）。

|方法|AP|AP@50|AP@75|AP-m|AP-l|训练迭代数|
|---|---|---|---|---|---|---|
|仅幅度模型|39.5|85.4|41.3|34.4|43.7|174000|
|+ 净化后相位输入|40.3|85.9|41.9|34.6|44.5|180000|
|+ 关键点监督|42.9|86.8|44.1|38.0|45.8|186000|
|+ 迁移学习|43.5|87.2|44.6|38.1|46.4|145000|
表 5: 同布局协议下，人体检测的消融实验。所有指标均为越高越好。

|方法|dpAP·GPS|dpAP·GPS@50|dpAP·GPS@75|dpAP·GPSm|dpAP·GPSm@50|dpAP·GPSm@75|
|---|---|---|---|---|---|---|
|仅幅度模型|40.6|76.6|41.5|39.7|75.1|40.3|
|+ 净化后相位输入|41.2|77.4|42.3|40.1|75.7|40.5|
|+ 关键点监督|44.6|78.8|46.9|42.9|76.8|44.9|
|+ 迁移学习|45.3|79.3|47.7|43.2|77.4|45.5|
表 6: 同布局协议下，密集姿态估计的消融实验。所有指标均为越高越好。

### 4.7 不同布局下的性能

上述所有结果都是在训练与测试使用相同布局的情况下得到的。但不同环境下的 WiFi 信号，传播模式存在显著差异，因此将我们的模型部署到未训练过的布局的数据上，仍是一个极具挑战性的问题。

为测试模型的鲁棒性，我们在跨布局协议下重复了之前的实验，即使用 15 个训练布局，1 个测试布局。实验结果记录在表 7 与表 8 中。

可以观察到，我们的最终模型在未见过的域上，性能优于基线模型，但相比同布局协议，性能出现了显著下降：AP 性能从 43.5 降至 27.3，dpAP・GPS 从 45.3 降至 25.4。但同样需要注意的是，基于图像的模型也存在同样的域泛化问题。我们认为，收集来自广泛场景的更全面的数据集，可以缓解这一问题。

|方法|AP|AP@50|AP@75|AP-m|AP-l|
|---|---|---|---|---|---|
|WiFi（基线）|23.5|48.1|20.3|19.4|24.5|
|WiFi（最终）|27.3|51.8|24.2|22.1|28.6|
|Image|60.6|80.4|52.1|48.3|65.8|
表 7: 跨布局协议下，基于 WiFi 与基于图像的密集姿态的平均精度（AP）。所有指标均为越高越好。

|方法|dpAP·GPS|dpAP·GPS@50|dpAP·GPS@75|dpAP·GPSm|dpAP·GPSm@50|dpAP·GPSm@75|
|---|---|---|---|---|---|---|
|WiFi（基线）|22.3|47.3|21.5|20.9|44.6|21.8|
|WiFi（最终）|25.4|50.2|24.7|23.2|47.4|26.5|
|Image|60.2|70.1|62.3|54.0|72.7|58.8|
表 8: 跨布局协议下，基于 WiFi 与基于图像的密集姿态的密集姿态平均精度（dpAP・GPS、dpAP・GPSm）。所有指标均为越高越好。

### 4.8 失败案例

我们观察到两种主要的失败案例：

1. 当训练集中很少出现的人体姿态出现时，WiFi 模型会产生偏差，很可能会错误预测身体部位（见图 8 的 (a-b) 示例）。

2. 当一次采集中同时存在 3 个或更多目标时，WiFi 模型很难从整个采集的幅度与相位张量中，提取每个个体的详细信息（见图 8 的 (c-d) 示例）。

我们认为这两个问题都可以通过获取更全面的训练数据来解决。

![Image](https://p11-flow-imagex-sign.byteimg.com/tos-cn-i-a9rns2rl98/rc/online_export/c24608a5e0004d2593b2b5ae539e6fab~tplv-noop.jpeg?rk3s=49177a0b&x-expires=1775009845&x-signature=Uorq9No8fTMmQU%2Bwt4nX4pgGaeo%3D&resource_key=4087e4b9-411d-4dcf-ad09-5e95aac94eb7&resource_key=4087e4b9-411d-4dcf-ad09-5e95aac94eb7)

---

## 5 结论与未来工作

在本文中，我们证明了借助计算机视觉领域常用的深度学习架构，从 WiFi 信号中获取密集人体姿态是可行的。我们没有直接训练随机初始化的 WiFi 模型，而是探索了丰富的监督信息来提升模型性能与训练效率，例如利用 CSI 相位、添加关键点检测分支，以及从基于图像的模型进行迁移学习。我们的工作性能目前仍受限于 WiFi 感知领域的公开训练数据，尤其是跨布局场景下的性能。在未来工作中，我们还计划采集多布局数据，并将我们的工作扩展为从 WiFi 信号预测三维人体形状。我们认为，与 RGB 相机和激光雷达相比，这种先进的密集感知能力，能够让 WiFi 设备成为一种保护隐私、不受光照影响且低成本的人体传感器。

---

## 参考文献

[1] Fadel Adib, Chen-Yu Hsu, Hongzi Mao, Dina Katabi, Frédo Durand. 2015. 《穿墙捕捉人体轮廓》. ACM Trans. Graph. 34, 6, 文章 219 (2015 年 10 月), 13 页. [https://doi.org/10.1145/2816795.2818072](https://doi.org/10.1145/2816795.2818072)

[2] Fadel Adib, Zach Kabelac, Dina Katabi, Robert C. Miller. 2014. 《基于人体无线电反射的三维跟踪》. 第 11 届 USENIX 网络系统设计与实现研讨会（NSDI 14）. USENIX 协会，华盛顿州西雅图，317–329. [https://www.usenix.org/conference/nsdi14/technical-sessions/presentation/adib](https://www.usenix.org/conference/nsdi14/technical-sessions/presentation/adib)

[3] Thiemo Alldieck, Gerard Pons-Moll, Christian Theobalt, Marcus Magnor. 2019. 《Tex2Shape: 从单张图像得到完整的精细人体几何》. arXiv:1904.08645 [[cs.CV](cs.CV)]

[4] Mykhaylo Andriluka, Umar Iqbal, Eldar Insafutdinov, Leonid Pishchulin, Anton Milan, Juergen Gall, Bernt Schiele. 2018. 《PoseTrack: 人体姿态估计与跟踪的基准》. arXiv:1710.10000 [[cs.CV](cs.CV)]

[5] Chaimaa BASRI, Ahmed El Khadimi. 2016. 《室内定位系统综述与 WiFi 指纹技术的最新进展》. 多媒体计算与系统国际会议.

[6] Hilton Bristow, Jack Valmadre, Simon Lucey. 2015. 《每个像素都是分类器的密集语义对应》. arXiv:1505.04143 [[cs.CV](cs.CV)]

[7] Zhe Cao, Gines Hidalgo, Tomas Simon, Shih-En Wei, Yaser Sheikh. 2019. 《OpenPose: 使用部分亲和场的实时多人二维姿态估计》. arXiv:1812.08008 [[cs.CV](cs.CV)]

[8] Riza Alp Güler, Natalia Neverova, Iasonas Kokkinos. 2018. 《DensePose: 野外的密集人体姿态估计》. CoRR abs/1802.00434 (2018). arXiv:1802.00434 [http://arxiv.org/abs/1802.00434](http://arxiv.org/abs/1802.00434)

[9] Kaiming He, Georgia Gkioxari, Piotr Dollár, Ross B. Girshick. 2017. 《Mask R-CNN》. CoRR abs/1703.06870 (2017). arXiv:1703.06870 [http://arxiv.org/abs/1703.06870](http://arxiv.org/abs/1703.06870)

[10] Weipeng Jiang, Yongjun Liu, Yun Lei, Kaiyao Wang, Hui Yang, Zhihao Xing. 2017. 《为更好的基于 CSI 指纹的定位：一种新型相位净化方法与距离度量》. 2017 IEEE 第 85 届车用技术会议（VTC Spring）. 1–7. [https://doi.org/10.1109/VTCSpring.2017.8108351](https://doi.org/10.1109/VTCSpring.2017.8108351)

[11] Mohammad Hadi Kefayati, Vahid Pourahmadi, Hassan Aghaeinia. 2020. 《Wi2Vi: 从 WiFi CSI 样本生成视频帧》. CoRR abs/2001.05842 (2020). arXiv:2001.05842 [https://arxiv.org/abs/2001.05842](https://arxiv.org/abs/2001.05842)

[12] Muhammed Kocabas, Nikos Athanasiou, Michael J. Black. 2020. 《VIBE: 人体姿态与形状估计的视频推理》. arXiv:1912.05656 [[cs.CV](cs.CV)]

[13] Anton Ledergerber, Raffaello D’Andrea. 2019. 《基于角度相关天线传递函数的超宽带到达角估计》. Sensors 19, 20 (2019). [https://doi.org/10.3390/s19204466](https://doi.org/10.3390/s19204466)

[14] Tsung-Yi Lin, Piotr Dollár, Ross B. Girshick, Kaiming He, Bharath Hariharan, Serge J. Belongie. 2016. 《用于目标检测的特征金字塔网络》. CoRR abs/1612.03144 (2016). arXiv:1612.03144 [http://arxiv.org/abs/1612.03144](http://arxiv.org/abs/1612.03144)

[15] Tsung-Yi Lin, Michael Maire, Serge J. Belongie, Lubomir D. Bourdev, Ross B. Girshick, James Hays, Pietro Perona, Deva Ramanan, Piotr Dollár, C. Lawrence Zitnick. 2014. 《Microsoft COCO: 通用对象上下文》. CoRR abs/1405.0312 (2014). arXiv:1405.0312 [http://arxiv.org/abs/1405.0312](http://arxiv.org/abs/1405.0312)

[16] Jonathan Long, Evan Shelhamer, Trevor Darrell. 2014. 《用于语义分割的全卷积网络》. CoRR abs/1411.4038 (2014). arXiv:1411.4038 [http://arxiv.org/abs/1411.4038](http://arxiv.org/abs/1411.4038)

[17] Daniel Maturana, Sebastian Scherer. 2015. 《VoxNet: 用于实时目标识别的三维卷积神经网络》. 2015 IEEE/RSJ 智能机器人与系统国际会议（IROS）. 922–928. [https://doi.org/10.1109/IROS.2015.7353481](https://doi.org/10.1109/IROS.2015.7353481)

[18] Natalia Neverova, David Novotny, Vasil Khalidov, Marc Szafraniec, Patrick Labatut, Andrea Vedaldi. 2020. 《连续表面嵌入》. 神经信息处理系统进展.

[19] Joseph Redmon, Ali Farhadi. 2018. 《YOLOv3: 增量改进》. arXiv:1804.02767 [[cs.CV](cs.CV)]

[20] Shaoqing Ren, Kaiming He, Ross B. Girshick, Jian Sun. 2015. 《Faster R-CNN: 面向实时目标检测的区域候选网络》. CoRR abs/1506.01497 (2015). arXiv:1506.01497 [http://arxiv.org/abs/1506.01497](http://arxiv.org/abs/1506.01497)

[21] Shaoqing Ren, Kaiming He, Ross B. Girshick, Jian Sun. 2015. 《Faster R-CNN: 面向实时目标检测的区域候选网络》. CoRR abs/1506.01497 (2015). arXiv:1506.01497 [http://arxiv.org/abs/1506.01497](http://arxiv.org/abs/1506.01497)

[22] Shunsuke Saito, Zeng Huang, Ryota Natsume, Shigeo Morishima, Angjoo Kanazawa, Hao Li. 2019. 《PIFu: 用于高分辨率穿衣人体数字化的像素对齐隐函数》. arXiv 预印本 arXiv:1905.05172 (2019).

[23] Souvik Sen, Božidar Radunovic, Romit Roy Choudhury, Tom Minka. 2012. 《你正对着蒙娜丽莎：使用物理层信息的点定位》. 第 10 届移动系统、应用与服务国际会议论文集（英国湖区洛伍德湾）(MobiSys ’12). 计算机协会，纽约，183–196. [https://doi.org/10.1145/2307636.2307654](https://doi.org/10.1145/2307636.2307654)

[24] Roman Shapovalov, David Novotný, Benjamin Graham, Patrick Labatut, Andrea Vedaldi. 2021. 《DensePose 3D: 将关节对象的规范表面映射提升至三维》. CoRR abs/2109.00033 (2021). arXiv:2109.00033 [https://arxiv.org/abs/2109.00033](https://arxiv.org/abs/2109.00033)

[25] B. Sheng, F. Xiao, L. Sha, L. Sun. 2020. 《基于深度时空模型的跨场景动作识别，使用商用 WiFi》. IEEE Internet of Things Journal 7 (2020).

[26] Elahe Soltanaghaei, Avinash Kalyanaraman, Kamin Whitehouse. 2018. 《多径三角测量：使用单个无辅助接收器的分米级 WiFi 定位与朝向》. 第 16 届移动系统、应用与服务年度国际会议论文集（德国慕尼黑）(MobiSys ’18). 计算机协会，纽约，376–388. [https://doi.org/10.1145/3210240.3210347](https://doi.org/10.1145/3210240.3210347)

[27] Elahe Soltanaghaei, Avinash Kalyanaraman, Kamin Whitehouse. 2018. 《多径三角测量：使用单个无辅助接收器的分米级 WiFi 定位与朝向》. 第 16 届移动系统、应用与服务年度国际会议论文集.

[28] Yu Sun, Qian Bao, Wu Liu, Yili Fu, Michael J. Black, Tao Mei. 2021. 《单目、单阶段的多三维人体回归》. arXiv:2008.12272 [[cs.CV](cs.CV)]

[29] Fei Wang, Yunpeng Song, Jimuyang Zhang, Jinsong Han, Dong Huang. 2019. 《时间 Unet: 使用 WiFi 的样本级人体动作识别》. arXiv 预印本 arXiv:1904.11953 (2019).

[30] Fei Wang, Sanping Zhou, Stanislav Panev, Jinsong Han, Dong Huang. 2019. 《Person-in-WiFi: 使用 WiFi 的细粒度人体感知》. CoRR abs/1904.00276 (2019). arXiv:1904.00276 [http://arxiv.org/abs/1904.00276](http://arxiv.org/abs/1904.00276)

[31] Fei Wang, Sanping Zhou, Stanislav Panev, Jinsong Han, Dong Huang. 2019. 《Person-in-WiFi: 使用 WiFi 的细粒度人体感知》. ICCV.

[32] Zhe Wang, Yang Liu, Qinghai Liao, Haoyang Ye, Ming Liu, Lujia Wang. 2018. 《用于三维感知的 RS-LiDAR 特性》. 2018 IEEE 第 8 届网络技术自动化、控制与智能系统国际会议（CYBER）. 564–569. [https://doi.org/10.1109/CYBER.2018.8688235](https://doi.org/10.1109/CYBER.2018.8688235)

[33] Shih-En Wei, Varun Ramakrishna, Takeo Kanade, Yaser Sheikh. 2016. 《卷积姿态机》. arXiv:1602.00134 [[cs.CV](cs.CV)]

[34] Bin Xiao, Haiping Wu, Yichen Wei. 2018. 《人体姿态估计与跟踪的简单基线》. arXiv:1804.06208 [[cs.CV](cs.CV)]

[35] Yuanlu Xu, Song-Chun Zhu, Tony Tung. 2019. 《DenseRaC: 通过密集渲染与对比的联合三维姿态与形状估计》. CoRR abs/1910.00116 (2019). arXiv:1910.00116 [http://arxiv.org/abs/1910.00116](http://arxiv.org/abs/1910.00116)

[36] Hongfei Xue, Yan Ju, Chenglin Miao, Yijiang Wang, Shiyang Wang, Aidong Zhang, Lu Su. 2021. 《mmMesh: 面向使用毫米波的三维实时动态人体网格构建》. 第 19 届移动系统、应用与服务年度国际会议论文集. 269–282.

[37] Pengfei Yao, Zheng Fang, Fan Wu, Yao Feng, Jiwei Li. 2019. 《DenseBody: 从单张彩色图像直接回归密集三维人体姿态与形状》. arXiv:1903.10153 [[cs.CV](cs.CV)]

[38] Hongwen Zhang, Jie Cao, Guo Lu, Wanli Ouyang, Zhenan Sun. 2020. 《从密集身体部位学习三维人体形状与姿态》. arXiv:1912.13344 [[cs.CV](cs.CV)]

[39] Mingmin Zhao, Tianhong Li, Mohammad Abu Alsheikh, Yonglong Tian, Hang Zhao, Antonio Torralba, Dina Katabi. 2018. 《使用无线电信号的穿墙人体姿态估计》. 2018 IEEE/CVF 计算机视觉与模式识别会议. 7356–7365. [https://doi.org/10.1109/CVPR.2018.00768](https://doi.org/10.1109/CVPR.2018.00768)

[40] Tinghui Zhou, Philipp Krähenbühl, Mathieu Aubry, Qixing Huang, Alexei A. Efros. 2016. 《通过三维引导循环一致性学习密集对应》. arXiv:1604.05383 [[cs.CV](cs.CV)]

---

![Image](https://p11-flow-imagex-sign.byteimg.com/tos-cn-i-a9rns2rl98/rc/online_export/56cf48589d9a4839ac85069874b3b462~tplv-noop.jpeg?rk3s=49177a0b&x-expires=1775009845&x-signature=AFZ4iRimo8h9VSdyQafCMJbxfYQ%3D&resource_key=135cd486-01c1-4c22-8b51-052b1290a38b&resource_key=135cd486-01c1-4c22-8b51-052b1290a38b)
