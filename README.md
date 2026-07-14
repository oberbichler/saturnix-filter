# saturnix-filter

High-performance camera film simulation filters in Rust, designed for the [SATURNIX](https://github.com/Yutani140x/saturnix-camera) open-source camera (Raspberry Pi Zero 2W).

Developed to accelerate on-device photo processing, `saturnix-filter` delivers a ~47x to ~134x speedup over traditional pure-Python image processing on the target Raspberry Pi Zero 2 W hardware by leveraging zero-copy in-memory pixel manipulation, 10-bit fixed-point integer math, and multicore scaling via Rayon.

## Features

- Zero-Copy Memory Model: Manipulates Pillow image bytes directly in memory space
- Parallel Execution: Distributes row-by-row pixel computations across all available CPU cores using `rayon`.
- 26 Complete Film Styles:
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
  - `S-Sepia` (Warm brown-toned B&W)
  - `S-Cyano` (Cool blue Cyanotype-toned B&W)
  - `S-Noir` (High-contrast neutral B&W with heavy vignette)
  - `S-Teal` (Cinematic teal-and-orange style)
  - `S-Lomo` (Lomography toy-camera oversaturated style)
  - `S-Fuji` (Fujifilm Velvia high-saturation landscape style)
  - `S-Selenium` (Cool selenium-toned B&W)
  - `S-Platinum` (Warm, soft platinum-print-toned B&W)
  - `S-Infrared` (Aerochrome-style false-colour infrared)
  - `S-SplitTone` (Cinematic warm-highlight / cool-shadow split-toning)
  - `S-Kodachrome` (Rich, warm vintage-slide style)
  - `S-Polaroid` (Instant-film look with milky blacks and cyan cast)
  - `S-Matrix` (Digital-dystopia green cast)
  - `S-Cine` (Filmic S-curve digital-cinema grade)
  - `S-Leak` (Warm light-leak flare from the corner)
  - `S-Halation` (Warm red-orange glow blooming from highlights)
  - `S-CA` (Lo-fi lens look with red/blue chromatic-aberration fringing)
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
| S-Sepia     | ![](docs/examples/s_01_s-sepia.jpg)   | ![](docs/examples/s_02_s-sepia.jpg)   | ![](docs/examples/s_03_s-sepia.jpg)   | ![](docs/examples/s_04_s-sepia.jpg)   |
| S-Cyano     | ![](docs/examples/s_01_s-cyano.jpg)   | ![](docs/examples/s_02_s-cyano.jpg)   | ![](docs/examples/s_03_s-cyano.jpg)   | ![](docs/examples/s_04_s-cyano.jpg)   |
| S-Noir      | ![](docs/examples/s_01_s-noir.jpg)    | ![](docs/examples/s_02_s-noir.jpg)    | ![](docs/examples/s_03_s-noir.jpg)    | ![](docs/examples/s_04_s-noir.jpg)    |
| S-Teal      | ![](docs/examples/s_01_s-teal.jpg)    | ![](docs/examples/s_02_s-teal.jpg)    | ![](docs/examples/s_03_s-teal.jpg)    | ![](docs/examples/s_04_s-teal.jpg)    |
| S-Lomo      | ![](docs/examples/s_01_s-lomo.jpg)    | ![](docs/examples/s_02_s-lomo.jpg)    | ![](docs/examples/s_03_s-lomo.jpg)    | ![](docs/examples/s_04_s-lomo.jpg)    |
| S-Fuji      | ![](docs/examples/s_01_s-fuji.jpg)    | ![](docs/examples/s_02_s-fuji.jpg)    | ![](docs/examples/s_03_s-fuji.jpg)    | ![](docs/examples/s_04_s-fuji.jpg)    |
| S-Selenium  | ![](docs/examples/s_01_s-selenium.jpg)| ![](docs/examples/s_02_s-selenium.jpg)| ![](docs/examples/s_03_s-selenium.jpg)| ![](docs/examples/s_04_s-selenium.jpg)|
| S-Platinum  | ![](docs/examples/s_01_s-platinum.jpg)| ![](docs/examples/s_02_s-platinum.jpg)| ![](docs/examples/s_03_s-platinum.jpg)| ![](docs/examples/s_04_s-platinum.jpg)|
| S-Infrared  | ![](docs/examples/s_01_s-infrared.jpg)| ![](docs/examples/s_02_s-infrared.jpg)| ![](docs/examples/s_03_s-infrared.jpg)| ![](docs/examples/s_04_s-infrared.jpg)|
| S-SplitTone | ![](docs/examples/s_01_s-splittone.jpg)| ![](docs/examples/s_02_s-splittone.jpg)| ![](docs/examples/s_03_s-splittone.jpg)| ![](docs/examples/s_04_s-splittone.jpg)|
| S-Kodachrome| ![](docs/examples/s_01_s-kodachrome.jpg)| ![](docs/examples/s_02_s-kodachrome.jpg)| ![](docs/examples/s_03_s-kodachrome.jpg)| ![](docs/examples/s_04_s-kodachrome.jpg)|
| S-Polaroid  | ![](docs/examples/s_01_s-polaroid.jpg)| ![](docs/examples/s_02_s-polaroid.jpg)| ![](docs/examples/s_03_s-polaroid.jpg)| ![](docs/examples/s_04_s-polaroid.jpg)|
| S-Matrix    | ![](docs/examples/s_01_s-matrix.jpg)  | ![](docs/examples/s_02_s-matrix.jpg)  | ![](docs/examples/s_03_s-matrix.jpg)  | ![](docs/examples/s_04_s-matrix.jpg)  |
| S-Cine      | ![](docs/examples/s_01_s-cine.jpg)    | ![](docs/examples/s_02_s-cine.jpg)    | ![](docs/examples/s_03_s-cine.jpg)    | ![](docs/examples/s_04_s-cine.jpg)    |
| S-Leak      | ![](docs/examples/s_01_s-leak.jpg)    | ![](docs/examples/s_02_s-leak.jpg)    | ![](docs/examples/s_03_s-leak.jpg)    | ![](docs/examples/s_04_s-leak.jpg)    |
| S-Halation  | ![](docs/examples/s_01_s-halation.jpg)| ![](docs/examples/s_02_s-halation.jpg)| ![](docs/examples/s_03_s-halation.jpg)| ![](docs/examples/s_04_s-halation.jpg)|
| S-CA        | ![](docs/examples/s_01_s-ca.jpg)      | ![](docs/examples/s_02_s-ca.jpg)      | ![](docs/examples/s_03_s-ca.jpg)      | ![](docs/examples/s_04_s-ca.jpg)      |
| VHS         | ![](docs/examples/s_01_vhs.jpg)       | ![](docs/examples/s_02_vhs.jpg)       | ![](docs/examples/s_03_vhs.jpg)       | ![](docs/examples/s_04_vhs.jpg)       |

## Installation

Install the precompiled binary wheel directly from PyPI (no compiler required on the Raspberry Pi!):

```bash
pip install saturnix-filter
```

The core install has no Python dependencies — ideal for the production
runtime on the Raspberry Pi. To also get the `saturnix-filter` command-line
tool for experimenting on a PC (image file conversion, gallery rendering),
install the optional `cli` extra:

```bash
pip install "saturnix-filter[cli]"
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
#                    "S-Portra", "S-Cinestill", "S-Cross", "S-Faded", "S-Bleach",
#                    "S-Sepia", "S-Cyano", "S-Noir", "S-Teal", "S-Lomo", "S-Fuji",
#                    "S-Selenium", "S-Platinum", "S-Infrared", "S-SplitTone",
#                    "S-Kodachrome", "S-Polaroid", "S-Matrix", "S-Cine",
#                    "S-Leak", "S-Halation", "S-CA", "VHS"
saturnix_filter.apply_film_inplace(buf, width, height, "S-Saturnix")

# 4. Re-construct the Pillow image from the modified buffer
filtered_img = Image.frombytes("RGB", (width, height), bytes(buf))
filtered_img.save("photo_filtered.jpg", "JPEG", quality=92)
```

## Command-line tool

With the `cli` extra installed, the `saturnix-filter` command converts image
files directly:

```bash
# Apply a filter (auto-names the output photo_s-gold.jpg next to the input)
saturnix-filter convert photo.jpg -f S-Gold

# Several filters and/or images at once, downscaled to 1200 px wide
saturnix-filter convert *.jpg -f S-Gold -f S-Halation --max-width 1200

# List every available filter with a description
saturnix-filter list

# Apply every filter to one image, rendering a gallery next to it
saturnix-filter gallery photo.jpg
```

## Performance Comparison

All numbers are pure in-memory processing times at full 16 MP camera resolution (4656 x 3496 pixels), measured with a warm-up pass and excluding file I/O. `saturnix-filter` uses the optimized 10-bit fixed-point integer implementation.

Hardware: Raspberry Pi Zero 2 W (ARM Cortex-A53 quad-core @ 1.0 GHz)

All profiles share the same base point operations (tone curve,
saturation, contrast, vignette, grain); the Pipeline column lists the extra stages that set a profile apart:

- **mono** – monochrome conversion: collapses the image to a tinted luminance channel for black-and-white looks.
- **mix** – 3×3 channel-mix matrix: blends the R/G/B channels into each other for cross-channel colour shifts.
- **s-curve** – filmic S-curve tone shaping for richer contrast in the mid-tones while protecting highlights and shadows.
- **split-tone** – tints highlights and shadows with separate colours (e.g. warm highlights, cool shadows).
- **leak** – light-leak: adds a coloured flare that falls off from one corner, applied in the main pixel pass.
- **halation** – bloom post-pass: bright highlights bleed a soft coloured glow into their surroundings (a second image pass, hence slower).
- **CA** – chromatic aberration post-pass: shifts the red and blue channels radially for lens-style colour fringing (a second image pass, hence slower).

| Filter       | Original Python | **`saturnix-filter`** | **Speedup** | Pipeline (extra stages) |
| :----------- | :-------------- | :-------------------- | :---------- | :---------------------- |
| S-Gold       | 16.729 s        | 0.241 s               | **~69.4x**  | –                       |
| S-Vivid      | 16.711 s        | 0.354 s               | **~47.2x**  | mix                     |
| S-Natural    | 16.702 s        | 0.240 s               | **~69.6x**  | –                       |
| S-Saturnix   | 32.083 s        | 0.240 s               | **~133.7x** | –                       |
| S-MonoX      | 31.841 s        | 0.249 s               | **~127.9x** | mono                    |
| S-Portra     | –               | 0.240 s               | –           | –                       |
| S-Cross      | –               | 0.354 s               | –           | mix                     |
| S-Faded      | –               | 0.240 s               | –           | –                       |
| S-Bleach     | –               | 0.240 s               | –           | –                       |
| S-Sepia      | –               | 0.249 s               | –           | mono                    |
| S-Cyano      | –               | 0.249 s               | –           | mono                    |
| S-Noir       | –               | 0.249 s               | –           | mono                    |
| S-Teal       | –               | 0.240 s               | –           | –                       |
| S-Lomo       | –               | 0.240 s               | –           | –                       |
| S-Fuji       | –               | 0.354 s               | –           | mix                     |
| S-Selenium   | –               | 0.249 s               | –           | mono                    |
| S-Platinum   | –               | 0.249 s               | –           | mono                    |
| S-Infrared   | –               | 0.354 s               | –           | mix                     |
| S-SplitTone  | –               | 0.240 s               | –           | split-tone              |
| S-Kodachrome | –               | 0.355 s               | –           | mix, split-tone         |
| S-Polaroid   | –               | 0.242 s               | –           | split-tone              |
| S-Matrix     | –               | 0.354 s               | –           | mix, split-tone         |
| S-Cine       | –               | 0.241 s               | –           | s-curve, split-tone     |
| S-Leak       | –               | 0.320 s               | –           | leak                    |
| S-CA         | –               | 0.637 s               | –           | CA                      |
| S-Cinestill  | –               | 1.503 s               | –           | mix, halation           |
| S-Halation   | –               | 1.388 s               | –           | halation                |
| VHS          | 18.452 s        | 0.351 s               | **~52.6x**  | dedicated VHS pipeline  |

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
