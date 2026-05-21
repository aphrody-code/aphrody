# Google · Vision, Media & Graphics

Google's open-source vision and media Python repositories span neural rendering (NeRF variants), generative image models (GANs, diffusion), 3D graphics and depth estimation, video analysis and synthesis, audio/speech processing, 360-degree spatial media, and CJK typography tooling. Most repositories are research code accompanying published papers at CVPR, SIGGRAPH, NeurIPS, and ICCV.

> Part of [`docs/python/google/`](../README.md). Full mechanical listing: [`all-repos.md`](../all-repos.md). This page covers 44 repos (13 active / 31 archived).

## 360-Degree & Spatial Media

### [spatial-media](https://github.com/google/spatial-media)
**★ 2073 · `active` · pushed 2026-04 · other**

Specifications and Python tooling for 360-degree video and spatial audio metadata injection. Covers the Spatial Audio RFC, Spherical Video V1 and V2 RFCs, and the VR180 video format. Provides a Python CLI and library (`spatialmedia`) for reading and writing spatial metadata into MP4 and MKV container files; widely used by 360-degree video creators and platforms.

---

## Neural Radiance Fields (NeRF)

### [nerfies](https://github.com/google/nerfies)
**★ 1957 · `archived` · pushed 2024-04 · Apache-2.0**
Topics: `3d` `machine-learning` `nerf` `neural-network` `neural-rendering`

Implementation of Nerfies (Deformable Neural Radiance Fields), which extends NeRF to capture free-viewpoint video of casually captured subjects with non-rigid deformations. Built on JAX/JaxNeRF; includes Colab notebooks for dataset processing, training, and video rendering. Presented at ICCV 2021.

### [hypernerf](https://github.com/google/hypernerf)
**★ 960 · `archived` · pushed 2024-05 · Apache-2.0**
Topics: `3d` `machine-learning` `nerf` `neural-network` `neural-rendering` `novel-view-synthesis`

HyperNeRF extends NeRF to handle scenes with topological changes (e.g., objects appearing or disappearing) by embedding them in a higher-dimensional ambient space. Implemented in JAX; provides Colab notebooks for training and rendering. Published as a SIGGRAPH Asia 2021 paper.

### [nerfactor](https://github.com/google/nerfactor)
**★ 450 · `archived` · pushed 2023-04 · Apache-2.0**
Topics: `illumination` `nerf` `neural-rendering` `reflectance` `relighting` `shape` `view-synthesis`

NeRFactor performs neural factorization of shape, reflectance (BRDF), and illumination from multi-view images of objects under unknown natural lighting. Enables relighting of captured scenes with new environment maps. Published at SIGGRAPH Asia 2021.

### [dynibar](https://github.com/google/dynibar)
**★ 820 · `archived` · pushed 2023-10 · Apache-2.0**
Topics: `3d-vision` `dynamic-reconstruction` `view-synthesis`

DynIBaR (Neural Dynamic Image-Based Rendering) synthesizes novel views of complex dynamic scenes from monocular video by modeling per-frame scene structure and appearance. Received a Best Paper Honorable Mention at CVPR 2023.

---

## Generative Image Models

### [compare_gan](https://github.com/google/compare_gan)
**★ 1820 · `archived` · pushed 2021-01 · Apache-2.0**

Comprehensive TensorFlow framework for GAN research, implementing a wide range of GAN losses (non-saturating, WGAN, least-squares), regularization penalties (gradient penalty, spectral normalization), neural architectures (BigGAN, ResNet, DCGAN), and evaluation metrics (FID, Inception Score, precision-recall, KID). Configured via Gin; supported several landmark NeurIPS/ICML papers on large-scale GAN evaluation.

### [sg2im](https://github.com/google/sg2im)
**★ 1324 · `archived` · pushed 2024-07 · Apache-2.0**

Image Generation from Scene Graphs (CVPR 2018): an end-to-end neural network that maps structured scene graph inputs (objects and relationships) to photorealistic images via graph convolution and a cascaded refinement network. Enables fine-grained image manipulation by editing the scene graph.

### [style-aligned](https://github.com/google/style-aligned)
**★ 1320 · `archived` · pushed 2023-12 · Apache-2.0**

Style Aligned Image Generation (CVPR 2024): a method that achieves style consistency across a set of diffusion-generated images by sharing attention features between all images in a batch during the diffusion process, using a minimal set of operations without retraining.

### [break-a-scene](https://github.com/google/break-a-scene)
**★ 525 · `archived` · pushed 2024-01 · Apache-2.0**
Topics: `deep-learning` `diffusion-models` `generative-ai` `multimodal` `text-to-image`

Break-A-Scene (SIGGRAPH Asia 2023): extracts multiple distinct visual concepts from a single image by learning a separate text token per concept, given loose segmentation masks. Enables per-concept re-synthesis and local editing using Stable Diffusion as the backbone.

### [hdrnet](https://github.com/google/hdrnet)
**★ 875 · `archived` · pushed 2023-04 · Apache-2.0**

HDRNet implements Deep Bilateral Learning for real-time image enhancement (SIGGRAPH 2017). It learns to predict bilateral grid coefficients from a low-resolution input image and applies learned local affine color transforms at full resolution via a custom TensorFlow operator, achieving real-time performance on GPU.

---

## Computer Vision — Depth, Geometry & Video

### [mannequinchallenge](https://github.com/google/mannequinchallenge)
**★ 491 · `archived` · pushed 2021-01 · Apache-2.0**

Research code for learning depth of moving people by observing frozen people (CVPR 2019). Uses the Mannequin Challenge video dataset where people hold still to supervise a monocular depth estimation network that handles dynamic scenes.

### [stereo-magnification](https://github.com/google/stereo-magnification)
**★ 416 · `archived` · pushed 2019-07 · Apache-2.0**
Topics: `computer-graphics` `computer-vision` `deep-learning` `multiplane-images` `stereo-magnification` `view-synthesis`

Stereo Magnification (SIGGRAPH 2018): learns view synthesis from stereo image pairs using Multiplane Image (MPI) representations, enabling rendering of nearby novel views with correct parallax. Introduced the MPI representation that became widely adopted for view synthesis.

### [dynamic-video-depth](https://github.com/google/dynamic-video-depth)
**★ 274 · `archived` · pushed 2022-02 · Apache-2.0**

Code for Consistent Depth of Moving Objects in Video (SIGGRAPH 2021). Produces geometrically consistent depth maps across video frames for both static background and dynamic foreground objects, enabling stable 3D video effects.

### [next-prediction](https://github.com/google/next-prediction)
**★ 355 · `archived` · pushed 2023-03 · Apache-2.0**

Research code for predicting future person activities and locations in videos (CVPR 2019). Models joint prediction of bounding box trajectories and action labels for people in unconstrained video.

### [tirg](https://github.com/google/tirg)
**★ 305 · `archived` · pushed 2021-04 · Apache-2.0**

Text-Image Residual Gating (TIRG): a method for image retrieval guided by relative natural language descriptions ("same style but more formal"). Combines image and text features via a learned residual gating mechanism trained on fashion and scene datasets.

### [cameratrapai](https://github.com/google/cameratrapai)
**★ 517 · `active` · pushed 2026-04 · Apache-2.0**

SpeciesNet: an ensemble of AI models (MegaDetector object detector + EfficientNet V2 M classifier) for classifying wildlife species in motion-triggered camera trap images. Trained on 65M+ geographically diverse images, covering 2000+ species and taxa. Powers the Wildlife Insights platform; can be run locally or via cloud-based systems.

### [practical-inverse-rendering-of-textured-and-translucent-appearance](https://github.com/google/practical-inverse-rendering-of-textured-and-translucent-appearance)
**★ 122 · `active` · pushed 2025-12 · Apache-2.0**

SIGGRAPH 2025 research code for practical inverse rendering of objects with complex textured and translucent BSSRDF material appearance from multi-view image captures, enabling physically-based relighting and material editing.

---

## Audio & Speech

### [speaker-id](https://github.com/google/speaker-id)
**★ 449 · `active` · pushed 2025-08 · Apache-2.0**
Topics: `source-separation` `speaker-diarization` `speaker-identification` `speaker-recognition` `speaker-verification`

Repository of audio samples, supplementary materials, and research code from Google's Speaker, Voice and Language team. Contains open-source components including Lingvo-based speaker libraries and DiarizationLM (LLM-based post-processing for speaker diarization transcripts). Accompanies publications on VoiceFilter, GE2E loss, PersonalVAD, and related speaker recognition work.

---

## CJK Typography & Language Rendering

### [budoux](https://github.com/google/budoux)
**★ 1626 · `active` · pushed 2026-05 · Apache-2.0**
Topics: `javascript` `machine-learning` `nlp` `python`

BudouX is the successor to Budou: a standalone, small (~15 KB) machine-learning-based line-break optimizer for CJK text (Japanese, Simplified Chinese, Traditional Chinese, Thai). It works without dependency on cloud APIs, supports HTML inputs, and is available as Python, JavaScript, and Java packages.

### [budou](https://github.com/google/budou)
**★ 1183 · `archived` · pushed 2023-04 · Apache-2.0**
Topics: `cjk` `natural-language-processing` `python` `web-development`

Original Budou line-break tool for CJK text, using Google Cloud Natural Language API for word segmentation and inserting non-breaking spaces to prevent awkward line breaks. Superseded by BudouX, which removes the cloud API dependency.

---

## Other repos in this category

| Repo | ★ | Status | Description |
|------|--:|--------|-------------|
| [in-silico-labeling](https://github.com/google/in-silico-labeling) | 264 | archived | Predicting fluorescent labels in unlabeled microscopy images |
| [burst-denoising](https://github.com/google/burst-denoising) | 247 | archived | Burst photography denoising with deep learning |
| [neural_rerendering_in_the_wild](https://github.com/google/neural_rerendering_in_the_wild) | 208 | archived | Neural re-rendering of in-the-wild scenes |
| [e3d_lstm](https://github.com/google/e3d_lstm) | 207 | archived | Eidetic 3D LSTM for video prediction |
| [retiming](https://github.com/google/retiming) | 178 | archived | Layered neural rendering for retiming people in video |
| [lasr](https://github.com/google/lasr) | 172 | archived | Learning articulated shape reconstruction from monocular video (CVPR 2021) |
| [tf_mesh_renderer](https://github.com/google/tf_mesh_renderer) | 496 | archived | Differentiable 3D mesh renderer using TensorFlow |
| [layered-scene-inference](https://github.com/google/layered-scene-inference) | 87 | archived | Layer-structured 3D scene inference via view synthesis (ECCV 2018) |
| [samurai](https://github.com/google/samurai) | 121 | archived | Shape and material reconstruction from unconstrained image collections (NeurIPS 2022) |
| [ceviche-challenges](https://github.com/google/ceviche-challenges) | 122 | archived | Photonic inverse design challenge problems for topology optimization |
| [rtc-video-quality](https://github.com/google/rtc-video-quality) | 115 | archived | Real-time video codec performance comparison tools |
| [audio-sync-kit](https://github.com/google/audio-sync-kit) | 112 | archived | Audio-video synchronization measurement toolkit |
| [tensorflow-recorder](https://github.com/google/tensorflow-recorder) | 179 | archived | Easy TFRecords creation from Pandas DataFrames and CSVs |
| [automl-video-ondevice](https://github.com/google/automl-video-ondevice) | 55 | archived | AutoML video classification for on-device deployment |
| [ai_video_dubbing](https://github.com/google/ai_video_dubbing) | 51 | archived | AI video dubbing research code |
| [volux-gan](https://github.com/google/volux-gan) | 29 | archived | Volumetric GAN for 3D-aware image synthesis |
| [autocjk](https://github.com/google/autocjk) | 13 | archived | Generating predictions for uncommon CJK characters from component images |
| [tim-gan](https://github.com/google/tim-gan) | 12 | archived | Text-guided image manipulation GAN |
| [Stereoscopic-Video-Generation-via-Denoising-Frame-Matrix](https://github.com/google/Stereoscopic-Video-Generation-via-Denoising-Frame-Matrix) | 11 | active | Stereoscopic video generation via denoising frame matrix |
| [image_mix](https://github.com/google/image_mix) | 16 | active | Image mixing / compositing research code |
| [learn-oss-with-google](https://github.com/google/learn-oss-with-google) | 46 | active | Code samples from "Learn Kubernetes with Google" video series |
| [dcm-video-uploader](https://github.com/google/dcm-video-uploader) | 9 | archived | Python script to upload geo-targeted video creatives to DCM |
| [it-cert-automation](https://github.com/google/it-cert-automation) | 18 | archived | Code from Google IT Automation with Python Professional Certificate |
| [mobly-bluetooth-ref-validation](https://github.com/google/mobly-bluetooth-ref-validation) | 8 | active | Bluetooth classic/BLE/LE Audio validation test suite using Mobly |
| [image-supplemental-feed-creator](https://github.com/google/image-supplemental-feed-creator) | 3 | archived | Supplemental image feed creator for Google Ads |
