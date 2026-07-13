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
    "S-Sepia",
    "S-Cyano",
    "S-Noir",
    "S-Teal",
    "S-Lomo",
    "S-Fuji",
    "S-Selenium",
    "S-Platinum",
    "VHS",
]


def make_buffer():
    img = Image.linear_gradient("L").resize((WIDTH, HEIGHT)).convert("RGB")
    return bytearray(img.tobytes())


def mid_gray_buffer():
    # Uniform neutral mid-gray: every pixel is (128, 128, 128).
    return bytearray([128] * (WIDTH * HEIGHT * 3))


def first_pixel(buf):
    return buf[0], buf[1], buf[2]


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


def test_monox_keeps_neutral_gray_neutral():
    # An untinted monochrome profile must keep R == G == B on neutral input.
    buf = mid_gray_buffer()
    saturnix_filter.apply_film_inplace(buf, WIDTH, HEIGHT, "S-MonoX")
    r, g, b = first_pixel(buf)
    assert r == g == b


def test_sepia_tints_neutral_gray_warm():
    # Sepia must push a neutral gray towards warm (red > blue).
    buf = mid_gray_buffer()
    saturnix_filter.apply_film_inplace(buf, WIDTH, HEIGHT, "S-Sepia")
    r, g, b = first_pixel(buf)
    assert r > b


def test_cyano_tints_neutral_gray_cool():
    # Cyanotype must push a neutral gray towards cool (blue > red).
    buf = mid_gray_buffer()
    saturnix_filter.apply_film_inplace(buf, WIDTH, HEIGHT, "S-Cyano")
    r, g, b = first_pixel(buf)
    assert b > r


def test_selenium_tints_neutral_gray_cool():
    # Selenium toning leans cool/purple: blue > red on neutral input.
    buf = mid_gray_buffer()
    saturnix_filter.apply_film_inplace(buf, WIDTH, HEIGHT, "S-Selenium")
    r, g, b = first_pixel(buf)
    assert b > r


def test_platinum_tints_neutral_gray_warm():
    # Platinum toning leans warm-neutral: red > blue on neutral input.
    buf = mid_gray_buffer()
    saturnix_filter.apply_film_inplace(buf, WIDTH, HEIGHT, "S-Platinum")
    r, g, b = first_pixel(buf)
    assert r > b
