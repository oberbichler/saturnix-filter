# saturnix-filter

High-performance camera film simulation filters in Rust, designed for the [SATURNIX](https://github.com/Yutani140x/saturnix-camera) open-source camera (Raspberry Pi Zero 2W).

Developed to accelerate on-device photo processing, `saturnix-filter` delivers a ~41x to ~91x speedup over traditional pure-Python image processing on the target Raspberry Pi Zero 2 W hardware by leveraging zero-copy in-memory pixel manipulation, 10-bit fixed-point integer math, and multicore scaling via Rayon.

## Features

- Zero-Copy Memory Model: Manipulates Pillow image bytes directly in memory space
- Parallel Execution: Distributes row-by-row pixel computations across all available CPU cores using `rayon`.
- 6 Complete Film Styles:
  - `S-Gold` (Kodak Gold warm vintage style)
  - `S-Vivid` (Kodak Ektar ultra-saturated style)
  - `S-Natural` (Fujifilm organic greens style)
  - `S-Saturnix` (Signature cel-animation glow style)
  - `S-MonoX` (Kodak Tri-X 400 panchromatic S-curve B&W style)
  - `VHS` (Vintage VHS tape simulation)

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
# Supported options: "S-Gold", "S-Vivid", "S-Natural", "S-Saturnix", "S-MonoX", "VHS"
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
