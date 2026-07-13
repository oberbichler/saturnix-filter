import pytest
from PIL import Image

import saturnix_filter

WIDTH, HEIGHT = 32, 24
FILTERS = [
    "S-Gold",
    "S-Vivid",
    "S-Natural",
    "S-Saturnix",
    "S-MonoX",
    "S-Portra",
    "S-Cinestill",
    "S-Cross",
    "S-Faded",
    "S-Bleach",
    "VHS",
]


def make_buffer():
    img = Image.linear_gradient("L").resize((WIDTH, HEIGHT)).convert("RGB")
    return bytearray(img.tobytes())


@pytest.mark.parametrize("film_name", FILTERS)
def test_filter_modifies_buffer_in_place(film_name):
    buf = make_buffer()
    original = bytes(buf)

    saturnix_filter.apply_film_inplace(buf, WIDTH, HEIGHT, film_name)

    assert bytes(buf) != original
    assert len(buf) == len(original)


def test_unknown_filter_raises_value_error():
    buf = make_buffer()
    with pytest.raises(ValueError):
        saturnix_filter.apply_film_inplace(buf, WIDTH, HEIGHT, "Not-A-Real-Filter")


def test_buffer_size_mismatch_raises_value_error():
    buf = make_buffer()
    with pytest.raises(ValueError):
        saturnix_filter.apply_film_inplace(buf, WIDTH + 1, HEIGHT, "S-Gold")
