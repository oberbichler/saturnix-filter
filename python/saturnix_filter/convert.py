"""Reusable image conversion helpers built on the core filter.

These helpers use Pillow for file I/O and are therefore part of the ``cli``
extra rather than the core runtime. Pillow is imported lazily so that importing
this module never fails on a minimal (Pi) install; the import error only
surfaces if a function that actually needs Pillow is called.
"""

from __future__ import annotations

from pathlib import Path

from . import apply_film_inplace


def _require_pillow():
    try:
        from PIL import Image
    except ImportError as exc:  # pragma: no cover - environment dependent
        raise RuntimeError(
            "Pillow is required for image file conversion. Install the CLI "
            'extra with:  pip install "saturnix-filter[cli]"'
        ) from exc
    return Image


def apply_to_image(image, filter_name: str):
    """Apply ``filter_name`` to a Pillow image, returning a new RGB image."""
    Image = _require_pillow()
    rgb = image.convert("RGB")
    width, height = rgb.size
    buffer = bytearray(rgb.tobytes())
    apply_film_inplace(buffer, width, height, filter_name)
    return Image.frombytes("RGB", (width, height), bytes(buffer))


def _resize_to_width(image, max_width: int):
    """Downscale ``image`` so its width is at most ``max_width`` (never upscale)."""
    Image = _require_pillow()
    width, height = image.size
    if max_width <= 0 or width <= max_width:
        return image
    new_height = round(height * max_width / width)
    return image.resize((max_width, new_height), Image.LANCZOS)


def convert_file(
    src: Path,
    dst: Path,
    filter_name: str,
    *,
    quality: int = 90,
    max_width: int = 0,
) -> Path:
    """Load ``src``, apply ``filter_name``, and save the result to ``dst``.

    ``max_width`` of 0 keeps the original size. Returns the output path.
    """
    Image = _require_pillow()
    with Image.open(src) as image:
        out = apply_to_image(image, filter_name)
    if max_width:
        out = _resize_to_width(out, max_width)
    dst.parent.mkdir(parents=True, exist_ok=True)
    save_kwargs = {}
    if dst.suffix.lower() in (".jpg", ".jpeg"):
        save_kwargs["quality"] = quality
    out.save(dst, **save_kwargs)
    return dst


def default_output_path(src: Path, filter_name: str) -> Path:
    """Build an automatic output name next to ``src``.

    e.g. ``photo.jpg`` + ``S-Gold`` -> ``photo_s-gold.jpg``.
    """
    slug = filter_name.lower()
    return src.with_name(f"{src.stem}_{slug}{src.suffix}")
