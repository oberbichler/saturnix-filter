# saturnix-filter

High-performance camera film simulation filters in Rust, designed for the [SATURNIX](https://github.com/Yutani140x/saturnix-camera) open-source camera (Raspberry Pi Zero 2W).

Developed to accelerate on-device photo processing, `saturnix-filter` delivers a ~41x to ~91x speedup over traditional pure-Python image processing on the target Raspberry Pi Zero 2 W hardware by leveraging zero-copy in-memory pixel manipulation, 10-bit fixed-point integer math, and multicore scaling via Rayon.

## Features

- Zero-Copy Memory Model: Manipulates Pillow image bytes directly in memory space
- Parallel Execution: Distributes row-by-row pixel computations across all available CPU cores using `rayon`.
- 11 Complete Film Styles:
  - `S-Gold` (Kodak Gold warm vintage style)
  - `S-Vivid` (Kodak Ektar ultra-saturated style)
  - `S-Natural` (Fujifilm organic greens style)
  - `S-Saturnix` (Signature cel-animation glow style)
  - `S-MonoX` (Kodak Tri-X 400 panchromatic S-curve B&W style)
  - `S-Portra` (Kodak Portra 400 soft, warm skin-tone style)
  - `S-Cinestill` (Cinestill 800T cool tungsten / teal-shadow style)
  - `S-Cross` (Cross-processed E-6-in-C-41 high-contrast style)
  - `S-Faded` (Sun-faded vintage print with milky lifted blacks)
  - `S-Bleach` (Bleach-bypass desaturated high-contrast silver style)
  - `VHS` (Vintage VHS tape simulation)

## Examples

Each column is one source photo; each row is the same image with a filter applied.

|             | Sample 01                             | Sample 02                             | Sample 03                             | Sample 04                             |
| :---------- | :------------------------------------ | :------------------------------------ | :------------------------------------ | :------------------------------------ |
| Original    | ![](docs/examples/s_01_original.jpg)  | ![](docs/examples/s_02_original.jpg)  | ![](docs/examples/s_03_original.jpg)  | ![](docs/examples/s_04_original.jpg)  |
| S-Gold      | ![](docs/examples/s_01_s-gold.jpg)    | ![](docs/examples/s_02_s-gold.jpg)    | ![](docs/examples/s_03_s-gold.jpg)    | ![](docs/examples/s_04_s-gold.jpg)    |
| S-Vivid     | ![](docs/examples/s_01_s-vivid.jpg)   | ![](docs/examples/s_02_s-vivid.jpg)   | ![](docs/examples/s_03_s-vivid.jpg)   | ![](docs/examples/s_04_s-vivid.jpg)   |
| S-Natural   | ![](docs/examples/s_01_s-natural.jpg) | ![](docs/examples/s_02_s-natural.jpg) | ![](docs/examples/s_03_s-natural.jpg) | ![](docs/examples/s_04_s-natural.jpg) |
| S-Saturnix  | ![](docs/examples/s_01_s-saturnix.jpg)| ![](docs/examples/s_02_s-saturnix.jpg)| ![](docs/examples/s_03_s-saturnix.jpg)| ![](docs/examples/s_04_s-saturnix.jpg)|
| S-MonoX     | ![](docs/examples/s_01_s-monox.jpg)   | ![](docs/examples/s_02_s-monox.jpg)   | ![](docs/examples/s_03_s-monox.jpg)   | ![](docs/examples/s_04_s-monox.jpg)   |
| S-Portra    | ![](docs/examples/s_01_s-portra.jpg)  | ![](docs/examples/s_02_s-portra.jpg)  | ![](docs/examples/s_03_s-portra.jpg)  | ![](docs/examples/s_04_s-portra.jpg)  |
| S-Cinestill | ![](docs/examples/s_01_s-cinestill.jpg)| ![](docs/examples/s_02_s-cinestill.jpg)| ![](docs/examples/s_03_s-cinestill.jpg)| ![](docs/examples/s_04_s-cinestill.jpg)|
| S-Cross     | ![](docs/examples/s_01_s-cross.jpg)   | ![](docs/examples/s_02_s-cross.jpg)   | ![](docs/examples/s_03_s-cross.jpg)   | ![](docs/examples/s_04_s-cross.jpg)   |
| S-Faded     | ![](docs/examples/s_01_s-faded.jpg)   | ![](docs/examples/s_02_s-faded.jpg)   | ![](docs/examples/s_03_s-faded.jpg)   | ![](docs/examples/s_04_s-faded.jpg)   |
| S-Bleach    | ![](docs/examples/s_01_s-bleach.jpg)  | ![](docs/examples/s_02_s-bleach.jpg)  | ![](docs/examples/s_03_s-bleach.jpg)  | ![](docs/examples/s_04_s-bleach.jpg)  |
| VHS         | ![](docs/examples/s_01_vhs.jpg)       | ![](docs/examples/s_02_vhs.jpg)       | ![](docs/examples/s_03_vhs.jpg)       | ![](docs/examples/s_04_vhs.jpg)       |

## Installation

Install the precompiled binary wheel directly from PyPI (no compiler required on the Raspberry Pi!):

```bash
pip install saturnix-filter
```

## Usage

```python
from PIL import Image
import saturnix_filter

# 1. Load an image as RGB
img = Image.open("photo.jpg").convert("RGB")
width, height = img.size

# 2. Extract mutable bytearray (Zero-Copy pointer reference)
buf = bytearray(img.tobytes())

# 3. Apply the filter instantly inside RAM
# Supported options: "S-Gold", "S-Vivid", "S-Natural", "S-Saturnix", "S-MonoX",
#                    "S-Portra", "S-Cinestill", "S-Cross", "S-Faded", "S-Bleach", "VHS"
saturnix_filter.apply_film_inplace(buf, width, height, "S-Saturnix")

# 4. Re-construct the Pillow image from the modified buffer
filtered_img = Image.frombytes("RGB", (width, height), bytes(buf))
filtered_img.save("photo_filtered.jpg", "JPEG", quality=92)
```

## Performance Comparison

All numbers are pure in-memory processing times at full 16 MP camera resolution (4656 x 3496 pixels), measured with a warm-up pass and excluding file I/O. `saturnix-filter` uses the optimized 10-bit fixed-point integer implementation.

Hardware: Raspberry Pi Zero 2 W (ARM Cortex-A53 quad-core @ 1.0 GHz)

| Filter     | Original Python | **`saturnix-filter`** | **Speedup** |
| :--------- | :-------------- | :-------------------- | :---------- |
| S-Gold     | 16.729 s        | 0.375 s               | **~44.6x**  |
| S-Vivid    | 16.711 s        | 0.405 s               | **~41.3x**  |
| S-Natural  | 16.702 s        | 0.374 s               | **~44.7x**  |
| S-Saturnix | 32.083 s        | 0.392 s               | **~81.9x**  |
| S-MonoX    | 31.841 s        | 0.350 s               | **~91.0x**  |
| VHS        | 18.452 s        | 0.351 s               | **~52.5x**  |

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
