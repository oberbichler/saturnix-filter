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
    "S-Infrared",
    "S-SplitTone",
    "S-Kodachrome",
    "S-Polaroid",
    "S-Matrix",
    "S-Cine",
    "S-Leak",
    "S-Halation",
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


def solid_green_buffer():
    # Uniform pure green: every pixel is (0, 200, 0).
    buf = bytearray(WIDTH * HEIGHT * 3)
    for i in range(1, len(buf), 3):
        buf[i] = 200
    return buf


def solid_gray_buffer(value):
    # Uniform neutral gray at the given level.
    return bytearray([value] * (WIDTH * HEIGHT * 3))


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


def test_infrared_turns_green_red():
    # False-colour infrared renders green foliage as red-dominant (r > g).
    buf = solid_green_buffer()
    saturnix_filter.apply_film_inplace(buf, WIDTH, HEIGHT, "S-Infrared")
    r, g, b = first_pixel(buf)
    assert r > g


def test_splittone_warms_highlights_cools_shadows():
    # Split-toning tints highlights and shadows independently:
    # a bright gray leans warm (r > b), a dark gray leans cool (b > r).
    hi = solid_gray_buffer(220)
    saturnix_filter.apply_film_inplace(hi, WIDTH, HEIGHT, "S-SplitTone")
    hr, hg, hb = first_pixel(hi)

    lo = solid_gray_buffer(40)
    saturnix_filter.apply_film_inplace(lo, WIDTH, HEIGHT, "S-SplitTone")
    lr, lg, lb = first_pixel(lo)

    assert hr > hb  # warm highlights
    assert lb > lr  # cool shadows


def test_matrix_casts_green():
    # The Matrix look pushes a neutral gray towards green.
    buf = mid_gray_buffer()
    saturnix_filter.apply_film_inplace(buf, WIDTH, HEIGHT, "S-Matrix")
    r, g, b = first_pixel(buf)
    assert g > r
    assert g > b


def test_leak_brightens_target_corner():
    # A light leak adds coloured light towards its origin corner: the top-right
    # pixel gains more red than the opposite (bottom-left) pixel.
    buf = mid_gray_buffer()
    saturnix_filter.apply_film_inplace(buf, WIDTH, HEIGHT, "S-Leak")
    # top-right pixel
    tr = (WIDTH - 1) * 3
    r_tr = buf[tr]
    # bottom-left pixel
    bl = (WIDTH * (HEIGHT - 1)) * 3
    r_bl = buf[bl]
    assert r_tr > r_bl


def test_cine_scurve_increases_contrast():
    # A positive S-curve pushes dark tones darker and bright tones brighter
    # (higher mid-tone contrast) while preserving the endpoints.
    dark = solid_gray_buffer(64)
    saturnix_filter.apply_film_inplace(dark, WIDTH, HEIGHT, "S-Cine")
    dr, _, _ = first_pixel(dark)

    bright = solid_gray_buffer(192)
    saturnix_filter.apply_film_inplace(bright, WIDTH, HEIGHT, "S-Cine")
    br, _, _ = first_pixel(bright)

    assert dr < 64  # shadows pushed down
    assert br > 192  # highlights pushed up


def test_halation_bleeds_glow_from_highlight():
    # Halation is a neighbourhood effect: a bright block bleeds a warm glow into
    # surrounding dark pixels that a pure point operation could never brighten.
    w, h = 64, 64
    buf = bytearray(w * h * 3)  # all black
    # Bright white block in the centre.
    for y in range(28, 36):
        for x in range(28, 36):
            idx = (y * w + x) * 3
            buf[idx] = buf[idx + 1] = buf[idx + 2] = 255

    # A dark pixel a few pixels away from the block.
    probe = (24 * w + 24) * 3
    assert buf[probe] == 0

    saturnix_filter.apply_film_inplace(buf, w, h, "S-Halation")

    # The probe gained glow, and the warm tint means red exceeds blue.
    assert buf[probe] > 0
    assert buf[probe] > buf[probe + 2]


def test_cinestill_has_red_halation():
    # Cinestill's signature is a red halation bloom around highlights. A dark
    # pixel next to a bright block must end up redder than an identical dark
    # pixel far from any highlight (isolating the neighbourhood bleed from the
    # profile's uniform colour cast).
    w, h = 96, 96
    buf = bytearray(w * h * 3)  # all black
    # Bright white block near one edge.
    for y in range(10, 26):
        for x in range(10, 26):
            idx = (y * w + x) * 3
            buf[idx] = buf[idx + 1] = buf[idx + 2] = 255

    near = (18 * w + 28) * 3  # just right of the block
    far = (80 * w + 80) * 3  # opposite corner, no highlight nearby

    saturnix_filter.apply_film_inplace(buf, w, h, "S-Cinestill")

    # The near pixel picks up the red halation glow; the far one does not.
    assert buf[near] > buf[far], (
        f"near red {buf[near]} should exceed far red {buf[far]}"
    )
    # The glow is red-dominant (warm halation).
    assert buf[near] > buf[near + 2]
