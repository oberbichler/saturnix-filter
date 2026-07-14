"""Human-readable descriptions for each filter.

The authoritative list of filter *names* comes from the Rust extension via
:func:`saturnix_filter.available_filters`. This module only adds a short
description for display in the CLI; any name without an entry here simply shows
no description.
"""

FILTER_DESCRIPTIONS = {
    "S-Gold": "Kodak Gold warm vintage style",
    "S-Vivid": "Kodak Ektar ultra-saturated style",
    "S-Natural": "Fujifilm organic greens style",
    "S-Saturnix": "Signature cel-animation glow style",
    "S-MonoX": "Kodak Tri-X 400 panchromatic S-curve B&W style",
    "S-Portra": "Kodak Portra 400 soft, warm skin-tone style",
    "S-Cinestill": "Cinestill 800T cool tungsten / teal-shadow style",
    "S-Cross": "Cross-processed E-6-in-C-41 high-contrast style",
    "S-Faded": "Sun-faded vintage print with milky lifted blacks",
    "S-Bleach": "Bleach-bypass desaturated high-contrast silver style",
    "S-Sepia": "Warm brown-toned B&W",
    "S-Cyano": "Cool blue Cyanotype-toned B&W",
    "S-Noir": "High-contrast neutral B&W with heavy vignette",
    "S-Teal": "Cinematic teal-and-orange style",
    "S-Lomo": "Lomography toy-camera oversaturated style",
    "S-Fuji": "Fujifilm Velvia high-saturation landscape style",
    "S-Selenium": "Cool selenium-toned B&W",
    "S-Platinum": "Warm, soft platinum-print-toned B&W",
    "S-Infrared": "Aerochrome-style false-colour infrared",
    "S-SplitTone": "Cinematic warm-highlight / cool-shadow split-toning",
    "S-Kodachrome": "Rich, warm vintage-slide style",
    "S-Polaroid": "Instant-film look with milky blacks and cyan cast",
    "S-Matrix": "Digital-dystopia green cast",
    "S-Cine": "Filmic S-curve digital-cinema grade",
    "S-Leak": "Warm light-leak flare from the corner",
    "S-Halation": "Warm red-orange glow blooming from highlights",
    "S-CA": "Lo-fi lens look with red/blue chromatic-aberration fringing",
    "VHS": "Vintage VHS tape simulation",
}


def describe(name: str) -> str:
    """Return a short description for ``name`` (empty string if unknown)."""
    return FILTER_DESCRIPTIONS.get(name, "")
