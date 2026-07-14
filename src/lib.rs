use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyByteArray;
use rayon::prelude::*;

struct FilmProfile {
    color_r: f32,
    color_g: f32,
    color_b: f32,
    saturation: f32,
    contrast: f32,
    brightness: f32,
    shadow_r: i32,
    shadow_g: i32,
    shadow_b: i32,
    // Split-toning highlight tint (additive, per channel). Applied by the tone
    // LUT weighted towards the highlights (v/255), the counterpart to the
    // shadow_* tint which is weighted towards the shadows. 0/0/0 == no highlight
    // tint. Ignored on the monochrome path.
    highlight_r: i32,
    highlight_g: i32,
    highlight_b: i32,
    lift_shadows: i32,
    compress_highlights: i32,
    grain: i32,
    vignette: f32,
    is_monochrome: bool,
    // Monochrome tint multipliers (around 1.0). Applied to the neutral
    // luminance in the monochrome path to produce toned B&W (sepia, cyanotype,
    // ...). Ignored on the colour path. 1.0/1.0/1.0 == neutral (no tint).
    tint_r: f32,
    tint_g: f32,
    tint_b: f32,
    // Optional 3x3 RGB channel-mix matrix applied *before* the per-channel
    // tone LUTs on the colour path (row-major: out_r, out_g, out_b rows).
    // Enables cross-channel effects such as false-colour infrared. The
    // identity matrix `IDENTITY_MIX` leaves the image unchanged. Ignored on the
    // monochrome path.
    mix: [f32; 9],
    // S-curve strength baked into the tone LUTs (0.0 == linear, no effect).
    // Positive values add a filmic toe/shoulder: mid-tone contrast rises while
    // pure black and white are preserved. Applies on both the colour and the
    // monochrome path (via the shared tone LUT).
    curve: f32,
    // Light-leak colour added towards one corner, falling off with distance.
    // 0/0/0 == no leak. `leak_corner` selects the origin corner: 0=top-left,
    // 1=top-right, 2=bottom-left, 3=bottom-right.
    leak_r: i32,
    leak_g: i32,
    leak_b: i32,
    leak_corner: u8,
    // Halation / bloom (a neighbourhood postpass, not a point operation).
    // Bright highlights above `halation_threshold` bleed a soft coloured glow
    // (halation_r/g/b tint) into their surroundings, screened back onto the
    // image with `halation_strength` (0 == disabled, 255 == full strength).
    halation_strength: i32,
    halation_threshold: i32,
    halation_r: i32,
    halation_g: i32,
    halation_b: i32,
    // Chromatic aberration (a neighbourhood postpass). Red and blue channels are
    // sampled with a radial offset that grows towards the image corners, mimicking
    // a lens's transverse colour fringing. `ca_strength` is the maximum channel
    // shift in pixels at the corner (0 == disabled).
    ca_strength: i32,
}

// Identity channel-mix matrix (no cross-channel mixing).
const IDENTITY_MIX: [f32; 9] = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];

// Neutral profile: a no-op filter. Every field is set to the value that leaves
// the image unchanged, so a concrete profile only needs to spell out the fields
// that actually deviate from neutral (via `..FilmProfile::default()`).
impl Default for FilmProfile {
    fn default() -> Self {
        FilmProfile {
            color_r: 1.0,
            color_g: 1.0,
            color_b: 1.0,
            saturation: 1.0,
            contrast: 1.0,
            brightness: 1.0,
            shadow_r: 0,
            shadow_g: 0,
            shadow_b: 0,
            highlight_r: 0,
            highlight_g: 0,
            highlight_b: 0,
            lift_shadows: 0,
            compress_highlights: 0,
            grain: 0,
            vignette: 0.0,
            is_monochrome: false,
            tint_r: 1.0,
            tint_g: 1.0,
            tint_b: 1.0,
            mix: IDENTITY_MIX,
            curve: 0.0,
            leak_r: 0,
            leak_g: 0,
            leak_b: 0,
            leak_corner: 0,
            halation_strength: 0,
            halation_threshold: 200,
            halation_r: 255,
            halation_g: 255,
            halation_b: 255,
            ca_strength: 0,
        }
    }
}

// Canonical list of every filter the library exposes, in display order. This
// is the single source of truth for filter names; the CLI and docs consume it
// via `available_filters`. "VHS" is a filter without a FilmProfile and is
// handled separately by the pipeline.
const FILTER_NAMES: &[&str] = &[
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
    "S-CA",
    "VHS",
];

fn get_profile(name: &str) -> Option<FilmProfile> {
    match name {
        "S-Gold" => Some(FilmProfile {
            color_r: 1.12,
            color_b: 0.80,
            saturation: 1.30,
            contrast: 1.08,
            brightness: 1.03,
            shadow_r: 30,
            shadow_g: 20,
            shadow_b: 5,
            lift_shadows: 18,
            compress_highlights: -12,
            grain: 14,
            vignette: 0.45,
            ..Default::default()
        }),
        // Kodak Ektar: ultra-saturated. A slightly "unmixing" matrix pulls each
        // channel away from its neighbours to widen colour separation (Ektar's
        // signature clean, differentiated primaries) while keeping neutral grays
        // neutral (rows sum to ~1.0).
        "S-Vivid" => Some(FilmProfile {
            color_r: 0.98,
            color_b: 1.03,
            saturation: 1.85,
            contrast: 1.50,
            grain: 2,
            vignette: 0.05,
            #[rustfmt::skip]
            mix: [
                1.12, -0.07, -0.05, // out_r <- red, minus green/blue
                -0.06, 1.12, -0.06, // out_g <- green, minus red/blue
                -0.05, -0.07, 1.12, // out_b <- blue, minus red/green
            ],
            ..Default::default()
        }),
        "S-Natural" => Some(FilmProfile {
            color_r: 0.92,
            color_g: 1.02,
            color_b: 1.08,
            saturation: 1.20,
            contrast: 1.10,
            shadow_r: 5,
            shadow_g: 18,
            shadow_b: 14,
            lift_shadows: 12,
            compress_highlights: -8,
            grain: 10,
            vignette: 0.40,
            ..Default::default()
        }),
        "S-Saturnix" => Some(FilmProfile {
            color_r: 1.10,
            color_g: 0.97,
            saturation: 1.26,
            contrast: 1.04,
            brightness: 1.02,
            shadow_r: 12,
            shadow_g: 6,
            shadow_b: 40,
            lift_shadows: 20,
            compress_highlights: -6,
            grain: 6,
            vignette: 0.18,
            ..Default::default()
        }),
        "S-MonoX" => Some(FilmProfile {
            color_r: 0.25,
            color_g: 0.60,
            color_b: 0.15,
            saturation: 0.0,
            contrast: 1.45,
            brightness: 1.03,
            grain: 16,
            vignette: 0.30,
            is_monochrome: true,
            ..Default::default()
        }),
        // Kodak Portra 400: warm, restrained saturation, flat forgiving curve,
        // clean slightly-warm shadows, soft highlight roll-off, fine grain.
        "S-Portra" => Some(FilmProfile {
            color_r: 1.05,
            color_b: 0.94,
            saturation: 1.08,
            contrast: 0.95,
            brightness: 1.02,
            shadow_r: 8,
            shadow_g: 4,
            lift_shadows: 10,
            compress_highlights: -10,
            grain: 8,
            vignette: 0.15,
            ..Default::default()
        }),
        // Cinestill 800T: tungsten stock in daylight -> strong cool cast,
        // teal-leaning shadows, cinematic contrast, noticeable grain, plus the
        // signature red halation bloom around highlights (from the film's
        // removed anti-halation remjet layer), modelled via the halation postpass.
        "S-Cinestill" => Some(FilmProfile {
            color_r: 0.94,
            color_b: 1.12,
            saturation: 1.15,
            contrast: 1.15,
            shadow_g: 10,
            shadow_b: 24,
            lift_shadows: 6,
            compress_highlights: -4,
            grain: 12,
            vignette: 0.25,
            // A subtle mix bleeds green into blue (teal shadows) and a little
            // red into blue (cool magenta highlights) for a more authentic
            // tungsten/neon crossover than a per-channel curve alone.
            #[rustfmt::skip]
            mix: [
                0.98, 0.00, 0.02, // out_r <- red, faint blue
                0.00, 0.97, 0.03, // out_g <- green, faint blue
                0.05, 0.08, 0.90, // out_b <- blue + green/red bleed (teal)
            ],
            // Signature red halation: a strongly red-weighted glow blooms out of
            // bright highlights.
            halation_strength: 130,
            halation_threshold: 200,
            halation_r: 255,
            halation_g: 60,
            halation_b: 40,
            ..Default::default()
        }),
        // Cross-processing (E-6 in C-41): exaggerated saturation, high contrast,
        // yellow-green highlights and cyan-blue shadows, coarse grain.
        // A mild channel-mix reproduces the signature cross-channel dye
        // contamination that a per-channel curve alone cannot: green bleeds
        // into red (warm/yellow highlights) and blue bleeds into green
        // (cyan cast), while red is slightly pulled out of blue.
        "S-Cross" => Some(FilmProfile {
            color_r: 1.10,
            color_g: 1.05,
            color_b: 0.90,
            saturation: 1.60,
            contrast: 1.40,
            shadow_g: 10,
            shadow_b: 30,
            grain: 12,
            vignette: 0.20,
            #[rustfmt::skip]
            mix: [
                0.92, 0.14, -0.06, // out_r <- red + a little green (yellow highlights)
                0.00, 0.94, 0.10,  // out_g <- green + a little blue (cyan cast)
                0.06, 0.00, 0.98,  // out_b <- blue + a touch of red
            ],
            ..Default::default()
        }),
        // Faded / aged vintage print: milky lifted blacks, dulled highlights,
        // warm yellow-magenta cast, low saturation, flat compressed range.
        "S-Faded" => Some(FilmProfile {
            color_r: 1.06,
            color_b: 0.90,
            saturation: 0.82,
            contrast: 0.82,
            brightness: 1.02,
            shadow_r: 20,
            shadow_g: 14,
            shadow_b: 6,
            lift_shadows: 35,
            compress_highlights: -15,
            grain: 10,
            vignette: 0.30,
            ..Default::default()
        }),
        // Bleach bypass (silver retention): heavily desaturated, very high
        // contrast, near-neutral slightly-cool metallic look, gritty grain.
        "S-Bleach" => Some(FilmProfile {
            color_r: 0.98,
            color_b: 1.02,
            saturation: 0.45,
            contrast: 1.50,
            brightness: 1.02,
            shadow_b: 4,
            grain: 14,
            vignette: 0.20,
            ..Default::default()
        }),
        // Sepia: warm brown-toned B&W. Neutral luminance is tinted towards
        // red/orange and away from blue.
        "S-Sepia" => Some(FilmProfile {
            color_r: 0.30,
            color_g: 0.59,
            color_b: 0.11,
            saturation: 0.0,
            contrast: 1.10,
            brightness: 1.02,
            lift_shadows: 8,
            compress_highlights: -6,
            grain: 12,
            vignette: 0.30,
            is_monochrome: true,
            tint_r: 1.15,
            tint_b: 0.72,
            ..Default::default()
        }),
        // Cyanotype: cool blue-toned B&W. Neutral luminance is tinted towards
        // blue and away from red.
        "S-Cyano" => Some(FilmProfile {
            color_r: 0.30,
            color_g: 0.59,
            color_b: 0.11,
            saturation: 0.0,
            contrast: 1.15,
            lift_shadows: 6,
            grain: 8,
            vignette: 0.25,
            is_monochrome: true,
            tint_r: 0.62,
            tint_g: 0.90,
            tint_b: 1.25,
            ..Default::default()
        }),
        // Noir: high-contrast neutral B&W with a heavy vignette.
        "S-Noir" => Some(FilmProfile {
            color_r: 0.30,
            color_g: 0.59,
            color_b: 0.11,
            saturation: 0.0,
            contrast: 1.70,
            grain: 10,
            vignette: 0.45,
            is_monochrome: true,
            ..Default::default()
        }),
        // Teal & Orange: cinematic look with warm highlights (orange skin/light)
        // and teal-pushed shadows.
        "S-Teal" => Some(FilmProfile {
            color_r: 1.08,
            color_g: 0.99,
            color_b: 0.96,
            saturation: 1.20,
            contrast: 1.18,
            shadow_g: 14,
            shadow_b: 26,
            lift_shadows: 6,
            compress_highlights: -4,
            grain: 6,
            vignette: 0.22,
            ..Default::default()
        }),
        // Lomo / toy camera: oversaturated, punchy, heavy vignette and grain.
        "S-Lomo" => Some(FilmProfile {
            color_r: 1.06,
            color_g: 1.02,
            saturation: 1.70,
            contrast: 1.35,
            shadow_r: 6,
            shadow_g: 4,
            shadow_b: 18,
            lift_shadows: 4,
            compress_highlights: -6,
            grain: 16,
            vignette: 0.65,
            ..Default::default()
        }),
        // Fujifilm Velvia: high-saturation landscape stock with strong greens
        // and blues and a punchy contrast curve. The mix separates green from
        // blue (richer, deeper foliage and skies - Velvia's signature) while
        // keeping neutrals neutral.
        "S-Fuji" => Some(FilmProfile {
            color_r: 0.98,
            color_g: 1.04,
            color_b: 1.06,
            saturation: 1.55,
            contrast: 1.25,
            shadow_g: 8,
            shadow_b: 6,
            compress_highlights: -6,
            grain: 4,
            vignette: 0.12,
            #[rustfmt::skip]
            mix: [
                1.06, -0.03, -0.03, // out_r <- red, lightly cleaned
                -0.02, 1.10, -0.08, // out_g <- green, minus blue (warmer greens)
                -0.02, -0.06, 1.08, // out_b <- blue, minus green (deeper skies)
            ],
            ..Default::default()
        }),
        // Selenium-toned B&W: cool, slightly purple tone.
        "S-Selenium" => Some(FilmProfile {
            color_r: 0.30,
            color_g: 0.59,
            color_b: 0.11,
            saturation: 0.0,
            contrast: 1.30,
            grain: 8,
            vignette: 0.28,
            is_monochrome: true,
            tint_r: 0.94,
            tint_g: 0.96,
            tint_b: 1.10,
            ..Default::default()
        }),
        // Platinum / palladium print: warm-neutral, soft low-contrast tone with
        // a long tonal range.
        "S-Platinum" => Some(FilmProfile {
            color_r: 0.30,
            color_g: 0.59,
            color_b: 0.11,
            saturation: 0.0,
            contrast: 0.92,
            brightness: 1.02,
            lift_shadows: 14,
            compress_highlights: -8,
            grain: 6,
            vignette: 0.20,
            is_monochrome: true,
            tint_r: 1.08,
            tint_g: 1.02,
            tint_b: 0.90,
            ..Default::default()
        }),
        // False-colour infrared (Aerochrome-style): the channel-mix routes the
        // green (foliage/IR) signal into red so vegetation renders crimson,
        // shifts red into green, and keeps blue dark. High saturation and
        // contrast complete the surreal look.
        "S-Infrared" => Some(FilmProfile {
            saturation: 1.45,
            contrast: 1.20,
            compress_highlights: -6,
            grain: 8,
            vignette: 0.20,
            #[rustfmt::skip]
            mix: [
                0.10, 0.90, 0.10, // out_r <- mostly green (IR foliage -> red)
                0.85, 0.15, 0.05, // out_g <- mostly red
                0.05, 0.10, 0.70, // out_b <- attenuated blue
            ],
            ..Default::default()
        }),
        // Split-toning: independent tints for highlights and shadows. Warm
        // (orange) highlights and cool (teal/blue) shadows - the classic
        // cinematic split-tone grade.
        "S-SplitTone" => Some(FilmProfile {
            saturation: 1.10,
            contrast: 1.12,
            shadow_r: -14,
            shadow_g: 4,
            shadow_b: 24,
            highlight_r: 22,
            highlight_g: 8,
            highlight_b: -18,
            grain: 6,
            vignette: 0.18,
            ..Default::default()
        }),
        // Kodachrome: rich, warm reds, deep blues, punchy contrast. A mild mix
        // cleans the primaries; warm highlights and slightly cool shadows give
        // the signature vintage-slide look.
        "S-Kodachrome" => Some(FilmProfile {
            color_r: 1.08,
            color_g: 0.98,
            color_b: 0.96,
            saturation: 1.35,
            contrast: 1.22,
            shadow_b: 12,
            highlight_r: 14,
            highlight_g: 6,
            highlight_b: -8,
            compress_highlights: -6,
            grain: 8,
            vignette: 0.22,
            #[rustfmt::skip]
            mix: [
                1.08, -0.04, -0.04, // out_r <- clean red
                -0.03, 1.05, -0.02, // out_g
                -0.02, -0.04, 1.06, // out_b <- deeper blue
            ],
            ..Default::default()
        }),
        // Polaroid / instant film: milky lifted blacks, soft contrast, a cyan
        // cast and a heavy vignette.
        "S-Polaroid" => Some(FilmProfile {
            color_r: 0.96,
            color_g: 1.02,
            color_b: 1.04,
            saturation: 1.05,
            contrast: 0.88,
            brightness: 1.03,
            shadow_g: 14,
            shadow_b: 20,
            highlight_r: 12,
            highlight_g: 6,
            lift_shadows: 40,
            compress_highlights: -12,
            grain: 10,
            vignette: 0.45,
            ..Default::default()
        }),
        // The Matrix / digital-dystopia: dominant green cast driven by the
        // channel mix and green highlights.
        "S-Matrix" => Some(FilmProfile {
            color_r: 0.90,
            color_g: 1.10,
            color_b: 0.92,
            saturation: 1.10,
            contrast: 1.20,
            shadow_g: 12,
            highlight_g: 16,
            highlight_b: -6,
            lift_shadows: 4,
            compress_highlights: -4,
            grain: 8,
            vignette: 0.28,
            #[rustfmt::skip]
            mix: [
                0.85, 0.15, 0.00, // out_r <- some green
                0.10, 0.95, 0.10, // out_g <- boosted with red/blue bleed
                0.00, 0.18, 0.82, // out_b <- some green
            ],
            ..Default::default()
        }),
        // Cinematic tone curve: a pronounced filmic S-curve gives a rich toe and
        // shoulder (deep-but-detailed shadows, gentle highlight roll-off) with a
        // mild teal/orange split-tone for a modern digital-cinema grade.
        "S-Cine" => Some(FilmProfile {
            saturation: 1.08,
            shadow_r: -8,
            shadow_g: 2,
            shadow_b: 12,
            highlight_r: 10,
            highlight_g: 4,
            highlight_b: -8,
            grain: 6,
            vignette: 0.18,
            curve: 0.55,
            ..Default::default()
        }),
        // Light leak: a warm orange-red flare bleeds in from the top-right
        // corner, over a mildly faded, warm base - the classic accidental
        // film-exposure look.
        "S-Leak" => Some(FilmProfile {
            color_r: 1.05,
            color_b: 0.95,
            saturation: 1.10,
            contrast: 1.02,
            shadow_r: 6,
            shadow_g: 2,
            lift_shadows: 12,
            compress_highlights: -4,
            grain: 8,
            vignette: 0.15,
            leak_r: 160,
            leak_g: 70,
            leak_b: 30,
            leak_corner: 1,
            ..Default::default()
        }),
        // Halation / bloom: a warm red-orange glow blooms out of bright
        // highlights (the classic film halation look, as seen around neon and
        // bright light sources). Built on a mildly cool, contrasty base so the
        // warm glow stands out.
        "S-Halation" => Some(FilmProfile {
            color_r: 1.00,
            color_g: 1.00,
            color_b: 1.02,
            saturation: 1.12,
            contrast: 1.15,
            grain: 6,
            vignette: 0.18,
            halation_strength: 170,
            halation_threshold: 175,
            halation_r: 255,
            halation_g: 100,
            halation_b: 45,
            ..Default::default()
        }),
        // Chromatic aberration: a lo-fi lens look with pronounced red/blue
        // colour fringing towards the edges, over a punchy, slightly vignetted
        // base reminiscent of a cheap wide-angle or toy-camera lens.
        "S-CA" => Some(FilmProfile {
            color_r: 1.02,
            color_g: 1.00,
            color_b: 1.02,
            saturation: 1.20,
            contrast: 1.10,
            grain: 6,
            vignette: 0.30,
            ca_strength: 8,
            ..Default::default()
        }),
        _ => None,
    }
}

fn make_lut(cm: f32, lift: f32, comp: f32, sh: f32, hi: f32, curve: f32) -> [u8; 256] {
    let mut lut = [0u8; 256];
    for (i, slot) in lut.iter_mut().enumerate() {
        let mut v = (i as f32 * cm).min(255.0);
        let frac = v / 255.0;
        v = v + lift * (1.0 - frac) + comp * frac;
        v = v.clamp(0.0, 255.0);

        // Filmic S-curve (0.0 == linear). smootherstep preserves the 0/255
        // endpoints while adding a toe/shoulder and raising mid-tone contrast.
        if curve != 0.0 {
            let x = (v / 255.0).clamp(0.0, 1.0);
            let s = x * x * x * (x * (x * 6.0 - 15.0) + 10.0);
            v = (v + curve * (s * 255.0 - v)).clamp(0.0, 255.0);
        }

        let vf = v / 255.0;
        // Shadow tint weighted towards shadows, highlight tint towards highlights.
        v = (v + sh * (1.0 - vf) + hi * vf).clamp(0.0, 255.0);
        *slot = v as u8;
    }
    lut
}

fn make_trix_lut() -> [u8; 256] {
    let mut lut = [0u8; 256];
    for (i, slot) in lut.iter_mut().enumerate() {
        let x = i as f32 / 255.0;
        let v = if x < 0.08 {
            x * 0.3
        } else if x > 0.92 {
            0.92 + (x - 0.92) * 1.2
        } else {
            let t = (x - 0.08) / 0.84;
            0.024 + 0.976 * (t * t * (3.0 - 2.0 * t))
        };
        *slot = (v * 255.0).clamp(0.0, 255.0) as u8;
    }
    lut
}

// Simple LCG pseudo-random generator
struct SimpleRng {
    state: u32,
}

impl SimpleRng {
    fn new(seed: u32) -> Self {
        SimpleRng { state: seed }
    }

    fn next_u32(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(1664525).wrapping_add(1013904223);
        self.state
    }

    fn next_byte(&mut self) -> u32 {
        self.next_u32() >> 24
    }

    fn range(&mut self, min: i32, max: i32) -> i32 {
        if min >= max {
            return min;
        }
        min + (self.next_u32() % (max - min) as u32) as i32
    }
}

// Compile-time specialize the inner loop with const generics so LLVM can
// auto-vectorize each variant (NEON/SSE/AVX) with dead effect-branches removed.
//
// Benchmarks showed that turning the per-pixel effects (vignette, grain,
// light-leak) into runtime `if`s breaks vectorization and slows the hot path
// noticeably (e.g. S-Leak 9.5 -> 16.5 ms), so they stay const-generic. The
// dispatch table below is generated by a macro instead of being hand-written,
// keeping it maintainable without the previous 24 hand-written arms.
fn process_filter_generic<
    const MONO: bool,
    const VIG: bool,
    const GRAIN: bool,
    const MIX: bool,
    const LEAK: bool,
>(
    slice: &mut [u8],
    width: u32,
    height: u32,
    p: &FilmProfile,
) {
    let trix_lut = if MONO { make_trix_lut() } else { [0u8; 256] };
    let r_lut = make_lut(
        p.color_r,
        p.lift_shadows as f32,
        p.compress_highlights as f32,
        p.shadow_r as f32,
        p.highlight_r as f32,
        p.curve,
    );
    let g_lut = make_lut(
        p.color_g,
        p.lift_shadows as f32,
        p.compress_highlights as f32,
        p.shadow_g as f32,
        p.highlight_g as f32,
        p.curve,
    );
    let b_lut = make_lut(
        p.color_b,
        p.lift_shadows as f32,
        p.compress_highlights as f32,
        p.shadow_b as f32,
        p.highlight_b as f32,
        p.curve,
    );

    let mean = 128.0;
    let mut cb_lut = [0u8; 256];
    for (i, slot) in cb_lut.iter_mut().enumerate() {
        let v = (mean + (i as f32 - mean) * p.contrast) * p.brightness;
        *slot = v.clamp(0.0, 255.0) as u8;
    }

    let cx = width as f32 / 2.0;
    let cy = height as f32 / 2.0;
    let max_dist_sq = cx * cx + cy * cy;

    // Fixed-Point conversion constants (10-bit scale)
    let mono_r_w = (p.color_r * 1024.0) as u32;
    let mono_g_w = (p.color_g * 1024.0) as u32;
    let mono_b_w = (p.color_b * 1024.0) as u32;

    let sat_inv_w = ((1.0 - p.saturation) * 1024.0) as i32;
    let sat_w = (p.saturation * 1024.0) as i32;

    // Monochrome tint weights (10-bit Fixed-Point). Only used on the MONO path.
    let tint_r_w = (p.tint_r * 1024.0) as u32;
    let tint_g_w = (p.tint_g * 1024.0) as u32;
    let tint_b_w = (p.tint_b * 1024.0) as u32;

    // Channel-mix weights (10-bit Fixed-Point). Only used on the colour path
    // when MIX is enabled.
    let mix_w = {
        let mut w = [0i32; 9];
        for (dst, &src) in w.iter_mut().zip(p.mix.iter()) {
            *dst = (src * 1024.0) as i32;
        }
        w
    };

    // Vignette factor (scaled by 2^24 to prevent division underflows)
    let vig_scale = if VIG {
        (p.vignette * 16777216.0 / max_dist_sq) as u64
    } else {
        0u64
    };

    let grain_w = p.grain as u32;

    // Light-leak setup: origin corner and inverse falloff scale. The leak is a
    // coloured additive term that is strongest at the chosen corner and decays
    // with squared distance. Only used when LEAK is enabled.
    let (leak_ox, leak_oy) = match p.leak_corner {
        1 => (width as f32, 0.0),
        2 => (0.0, height as f32),
        3 => (width as f32, height as f32),
        _ => (0.0, 0.0),
    };
    // Reach ~ half the diagonal; scaled to 2^16 for fixed-point falloff.
    let leak_max_sq = (max_dist_sq).max(1.0);
    let leak_scale = if LEAK { 65536.0 / leak_max_sq } else { 0.0 };

    // Parallel process image rows
    slice
        .par_chunks_mut((width * 3) as usize)
        .enumerate()
        .for_each(|(y, row)| {
            let y_f = y as f32;
            let dy = y_f - cy;
            let dy_sq = (dy * dy) as u64;

            let leak_dy = y_f - leak_oy;
            let leak_dy_sq = leak_dy * leak_dy;

            let mut rng = SimpleRng::new((y as u32).wrapping_add(1) ^ 123456789);

            for x in 0..(width as usize) {
                let idx = x * 3;
                if idx + 2 >= row.len() {
                    break;
                }

                let mut r: u32;
                let mut g: u32;
                let mut b: u32;

                if MONO {
                    // 1. Panchromatic B&W conversion (Fixed-Point)
                    let lum = (mono_r_w * row[idx] as u32
                        + mono_g_w * row[idx + 1] as u32
                        + mono_b_w * row[idx + 2] as u32)
                        >> 10;
                    let t_lum = trix_lut[lum.min(255) as usize];
                    let f_lum = cb_lut[t_lum as usize] as u32;
                    // Tint the neutral luminance (Fixed-Point). Neutral tint
                    // (1024/1024/1024) leaves f_lum unchanged.
                    r = ((f_lum * tint_r_w) >> 10).min(255);
                    g = ((f_lum * tint_g_w) >> 10).min(255);
                    b = ((f_lum * tint_b_w) >> 10).min(255);
                } else {
                    let in_r = row[idx] as i32;
                    let in_g = row[idx + 1] as i32;
                    let in_b = row[idx + 2] as i32;

                    // 0. Optional channel mix (Fixed-Point). Identity mix is
                    // compiled out via the MIX const generic.
                    let (mr, mg, mb) = if MIX {
                        let mr = ((mix_w[0] * in_r + mix_w[1] * in_g + mix_w[2] * in_b) >> 10)
                            .clamp(0, 255);
                        let mg = ((mix_w[3] * in_r + mix_w[4] * in_g + mix_w[5] * in_b) >> 10)
                            .clamp(0, 255);
                        let mb = ((mix_w[6] * in_r + mix_w[7] * in_g + mix_w[8] * in_b) >> 10)
                            .clamp(0, 255);
                        (mr as usize, mg as usize, mb as usize)
                    } else {
                        (in_r as usize, in_g as usize, in_b as usize)
                    };

                    // 1. Point LUT transform
                    let r_tone = r_lut[mr] as i32;
                    let g_tone = g_lut[mg] as i32;
                    let b_tone = b_lut[mb] as i32;

                    // 2. Saturation (Fixed-Point)
                    let lum = (306 * r_tone + 601 * g_tone + 117 * b_tone) >> 10;
                    let r_sat = ((lum * sat_inv_w + r_tone * sat_w) >> 10).clamp(0, 255) as usize;
                    let g_sat = ((lum * sat_inv_w + g_tone * sat_w) >> 10).clamp(0, 255) as usize;
                    let b_sat = ((lum * sat_inv_w + b_tone * sat_w) >> 10).clamp(0, 255) as usize;

                    // 3. Contrast & Brightness Lookups
                    r = cb_lut[r_sat] as u32;
                    g = cb_lut[g_sat] as u32;
                    b = cb_lut[b_sat] as u32;
                }

                // 4. Vignette (Fixed-Point, no floats!)
                if VIG {
                    let dx = x as f32 - cx;
                    let dist_sq = (dx * dx) as u64 + dy_sq;
                    let vig_reduce = (dist_sq * vig_scale) >> 24;
                    let factor = if vig_reduce >= 1024 {
                        0
                    } else {
                        1024 - vig_reduce as u32
                    };
                    r = (r * factor) >> 10;
                    g = (g * factor) >> 10;
                    b = (b * factor) >> 10;
                }

                // 5. Grain (Fixed-Point, no division!)
                if GRAIN {
                    let noise = rng.next_byte();
                    r = (((r * (255 - grain_w) + noise * grain_w) * 257) >> 16).min(255);
                    g = (((g * (255 - grain_w) + noise * grain_w) * 257) >> 16).min(255);
                    b = (((b * (255 - grain_w) + noise * grain_w) * 257) >> 16).min(255);
                }

                // 6. Light leak: coloured additive term strongest at the chosen
                // corner, decaying with squared distance (screen-like add).
                if LEAK {
                    let leak_dx = x as f32 - leak_ox;
                    let dist_sq = leak_dx * leak_dx + leak_dy_sq;
                    // falloff in [0, 1024]; 1024 at the corner, 0 past the reach.
                    let f = 1024.0 - (dist_sq * leak_scale) * (1024.0 / 65536.0);
                    if f > 0.0 {
                        let fi = f as u32;
                        r = (r + ((p.leak_r as u32 * fi) >> 10)).min(255);
                        g = (g + ((p.leak_g as u32 * fi) >> 10)).min(255);
                        b = (b + ((p.leak_b as u32 * fi) >> 10)).min(255);
                    }
                }

                row[idx] = r as u8;
                row[idx + 1] = g as u8;
                row[idx + 2] = b as u8;
            }
        });
}

fn process_filter(slice: &mut [u8], width: u32, height: u32, p: &FilmProfile) {
    let mono = p.is_monochrome;
    let vig = p.vignette > 0.0;
    let grain = p.grain > 0;
    // The channel mix only affects the colour path; ignore it for monochrome.
    let mix = !mono && p.mix != IDENTITY_MIX;
    let leak = p.leak_r != 0 || p.leak_g != 0 || p.leak_b != 0;

    // Dispatch to the compile-time specialized variant. Each runtime bool is
    // "lifted" to a const-generic bool by `specialize!`, which recursively
    // expands into a binary decision tree covering every combination. This
    // generates the full 2^n specialization table (so LLVM can vectorize each
    // variant) without hand-writing every arm.
    //
    // `$fixed` accumulates the already-decided const bools; `$flag` is the next
    // runtime bool to lift; the trailing `=>` marks the final call.
    macro_rules! specialize {
        // Base case: no flags left to lift -> emit the specialized call.
        ( [$($fixed:literal),*] => ) => {
            process_filter_generic::<$($fixed),*>(slice, width, height, p)
        };
        // Recursive case: split the next runtime flag into true/false branches.
        ( [$($fixed:literal),*] => $flag:ident $(, $rest:ident)* ) => {
            if $flag {
                specialize!([$($fixed,)* true] => $($rest),*)
            } else {
                specialize!([$($fixed,)* false] => $($rest),*)
            }
        };
    }

    specialize!([] => mono, vig, grain, mix, leak)
}

// Whether a profile requests any neighbourhood-based pass (blur/halation/etc).
// Neighbourhood passes cannot run inside the vectorised point-operation loop
// without slowing every profile down, so they live in separate passes wrapped
// around the main `process_filter` call. Point-only profiles report `false`
// here and take the exact legacy code path with zero extra work.
fn needs_prepass(_p: &FilmProfile) -> bool {
    false
}

fn needs_postpass(p: &FilmProfile) -> bool {
    p.halation_strength > 0 || p.ca_strength > 0
}

// Neighbourhood pass run BEFORE the point-operation filter (e.g. diffusion /
// blur that should be toned afterwards). No effect is wired up yet.
fn run_prepass(_slice: &mut [u8], _width: u32, _height: u32, _p: &FilmProfile) {}

// Downscale factor for the halation glow buffer. The glow is soft and
// low-frequency, so computing it at 1/4 resolution (1/16 the pixels) is
// visually indistinguishable while being far cheaper on the Pi.
const HALATION_DOWNSCALE: u32 = 4;
// Number of separable box-blur passes over the small buffer. Repeated box
// blurs approximate a Gaussian; 3 passes give a smooth glow.
const HALATION_BLUR_PASSES: u32 = 3;
// Half-width of the box blur kernel (radius) on the downscaled buffer.
const HALATION_BLUR_RADIUS: u32 = 2;

// One-dimensional box blur along rows, writing into `dst`. Uses a sliding
// window sum (O(w) per row instead of O(w*radius)) and clamps the window at the
// edges. Rows are independent, so they run in parallel. The clamped variable
// window width and integer division match the naive version exactly, keeping
// the output bit-for-bit identical.
fn box_blur_h(src: &[u16], dst: &mut [u16], w: usize, radius: usize) {
    dst.par_chunks_mut(w)
        .zip(src.par_chunks(w))
        .for_each(|(dst_row, src_row)| {
            // Initial window [0, min(radius, w-1)].
            let mut hi = radius.min(w - 1);
            let mut sum: u32 = 0;
            for &v in &src_row[0..=hi] {
                sum += v as u32;
            }
            let mut lo = 0usize;
            // The index `x` is needed for the sliding-window arithmetic below,
            // so this is not a simple element-wise map.
            #[allow(clippy::needless_range_loop)]
            for x in 0..w {
                dst_row[x] = (sum / (hi - lo + 1) as u32) as u16;
                // Advance the window for x+1: add the new right edge, drop the
                // left edge once it falls outside [x+1-radius, x+1+radius].
                let next_hi = (x + 1 + radius).min(w - 1);
                if next_hi > hi {
                    sum += src_row[next_hi] as u32;
                    hi = next_hi;
                }
                let next_lo = (x + 1).saturating_sub(radius);
                if next_lo > lo {
                    sum -= src_row[lo] as u32;
                    lo = next_lo;
                }
            }
        });
}

// One-dimensional box blur along columns. Same sliding-window approach as the
// horizontal pass. Columns are processed with a single sequential sweep per
// column; the sliding sum makes this O(w*h) regardless of radius. Kept
// sequential (safe, no aliasing tricks) since the vertical pass runs on the
// small downscaled buffer.
fn box_blur_v(src: &[u16], dst: &mut [u16], w: usize, h: usize, radius: usize) {
    for x in 0..w {
        let mut hi = radius.min(h - 1);
        let mut sum: u32 = 0;
        for s in 0..=hi {
            sum += src[s * w + x] as u32;
        }
        let mut lo = 0usize;
        for y in 0..h {
            dst[y * w + x] = (sum / (hi - lo + 1) as u32) as u16;
            let next_hi = (y + 1 + radius).min(h - 1);
            if next_hi > hi {
                sum += src[next_hi * w + x] as u32;
                hi = next_hi;
            }
            let next_lo = (y + 1).saturating_sub(radius);
            if next_lo > lo {
                sum -= src[lo * w + x] as u32;
                lo = next_lo;
            }
        }
    }
}

// Halation / bloom postpass. Extracts highlights above the threshold into a
// small downscaled single-channel mask, blurs it into a soft glow, then screens
// a coloured version of that glow back onto the full-resolution image.
//
// Allocations (three small 1/16-size buffers) happen only when this runs, i.e.
// only for profiles that opt into halation. Point-only profiles never reach it.
fn apply_halation(slice: &mut [u8], width: u32, height: u32, p: &FilmProfile) {
    if p.halation_strength <= 0 {
        return;
    }
    let w = width as usize;
    let h = height as usize;
    if w == 0 || h == 0 {
        return;
    }

    let sw = (width / HALATION_DOWNSCALE).max(1) as usize;
    let sh = (height / HALATION_DOWNSCALE).max(1) as usize;
    let thr = p.halation_threshold.clamp(0, 254) as u32;
    // Luminance range above the threshold, used to normalise the highlight
    // excess to the full 0..255 glow range. Without this, a threshold of 175
    // would cap even a pure-white (255) highlight's glow at only 80, producing
    // a barely-visible bloom. Normalising means a maxed-out highlight yields a
    // full-strength glow while `halation_strength` stays the master intensity.
    let hl_range = (255 - thr).max(1);

    // 1. Extract highlights into the small buffer. Each small cell aggregates
    // the luminance excess above the threshold (normalised to 0..255) over the
    // whole full-res region that maps to it, taking the maximum so a small
    // bright feature is not missed by point sampling. Every full-res pixel is
    // read exactly once.
    // Parallel over full-res rows. Each row reduces into a per-row small buffer
    // (max of the highlight excess per small cell); the rows are then merged by
    // taking the element-wise maximum.
    //
    // This runs sequentially on purpose: a rayon fold/reduce version was tried
    // and measured *slower* on the target Pi Zero 2 W (per-task 2 MB buffer
    // allocations plus a large merge thrash the small cache / 512 MB RAM, and
    // contend with the parallel screen pass for the 4 weak cores). A single
    // linear sweep with one read per full-res pixel is the better fit here.
    let mut glow = vec![0u16; sw * sh];
    for y in 0..h {
        let sy = (y * sh / h).min(sh - 1);
        let base = sy * sw;
        let row = &slice[y * w * 3..(y + 1) * w * 3];
        for x in 0..w {
            let idx = x * 3;
            // Rec.601-ish luminance (fixed point, >> 10).
            let lum =
                (306 * row[idx] as u32 + 601 * row[idx + 1] as u32 + 117 * row[idx + 2] as u32)
                    >> 10;
            let excess = (lum.saturating_sub(thr) * 255 / hl_range).min(255) as u16;
            let sx = (x * sw / w).min(sw - 1);
            let cell = &mut glow[base + sx];
            if excess > *cell {
                *cell = excess;
            }
        }
    }

    // 2. Separable box blur, several passes, into a soft low-frequency glow.
    let mut tmp = vec![0u16; sw * sh];
    let radius = HALATION_BLUR_RADIUS as usize;
    for _ in 0..HALATION_BLUR_PASSES {
        box_blur_h(&glow, &mut tmp, sw, radius);
        box_blur_v(&tmp, &mut glow, sw, sh, radius);
    }

    // 3. Screen the coloured glow back onto the full image. For each full-res
    // pixel, bilinear-free nearest lookup into the small glow buffer keeps it
    // cheap; the glow is smooth so blockiness is invisible.
    //
    // add = glow * tint * strength, all in fixed point. `screen` blend
    // (255 - (255-a)*(255-b)/255) avoids harsh clipping in bright areas.
    let str_w = p.halation_strength.clamp(0, 255) as u32;
    let tint = [
        p.halation_r.clamp(0, 255) as u32,
        p.halation_g.clamp(0, 255) as u32,
        p.halation_b.clamp(0, 255) as u32,
    ];

    slice
        .par_chunks_mut(w * 3)
        .enumerate()
        .for_each(|(y, row)| {
            let sy = (y * sh / h).min(sh - 1);
            for x in 0..w {
                let sx = (x * sw / w).min(sw - 1);
                let g = glow[sy * sw + sx] as u32; // 0..255
                if g == 0 {
                    continue;
                }
                let idx = x * 3;
                for c in 0..3 {
                    // Coloured glow contribution for this channel, 0..255.
                    let add = (g * tint[c] * str_w) / (255 * 255);
                    let base = row[idx + c] as u32;
                    // Screen blend.
                    let blended = 255 - ((255 - base) * (255 - add.min(255)) / 255);
                    row[idx + c] = blended.min(255) as u8;
                }
            }
        });
}

// Chromatic aberration postpass. Samples the red and blue channels with equal
// and opposite radial offsets that grow linearly from 0 at the centre to
// `ca_strength` pixels at the corners, reproducing a lens's transverse colour
// fringing. Green is left in place. Reads from a copy so the offset lookups see
// the un-shifted image; writes back into `slice`. Parallel over rows.
fn apply_chromatic_aberration(slice: &mut [u8], width: u32, height: u32, p: &FilmProfile) {
    if p.ca_strength <= 0 {
        return;
    }
    let w = width as usize;
    let h = height as usize;
    if w == 0 || h == 0 {
        return;
    }

    let src = slice.to_vec();
    let cx = (w as f32 - 1.0) / 2.0;
    let cy = (h as f32 - 1.0) / 2.0;
    // Normalise the offset so it reaches `ca_strength` px at the corner.
    let max_dist = (cx * cx + cy * cy).sqrt().max(1.0);
    let strength = p.ca_strength as f32;

    slice
        .par_chunks_mut(w * 3)
        .enumerate()
        .for_each(|(y, row)| {
            let dy = y as f32 - cy;
            for x in 0..w {
                let dx = x as f32 - cx;
                let dist = (dx * dx + dy * dy).sqrt();
                // Per-pixel radial unit vector scaled by the distance-dependent
                // shift. `shift` px at the corner, 0 at the centre.
                let shift = strength * dist / max_dist;
                let (ux, uy) = if dist > 0.0 {
                    (dx / dist, dy / dist)
                } else {
                    (0.0, 0.0)
                };
                let off_x = ux * shift;
                let off_y = uy * shift;

                // Red sampled from further out, blue from further in.
                let rx = (x as f32 + off_x).round().clamp(0.0, (w - 1) as f32) as usize;
                let ry = (y as f32 + off_y).round().clamp(0.0, (h - 1) as f32) as usize;
                let bx = (x as f32 - off_x).round().clamp(0.0, (w - 1) as f32) as usize;
                let by = (y as f32 - off_y).round().clamp(0.0, (h - 1) as f32) as usize;

                let idx = x * 3;
                row[idx] = src[(ry * w + rx) * 3];
                // green unchanged (row[idx + 1] already correct)
                row[idx + 2] = src[(by * w + bx) * 3 + 2];
            }
        });
}

// Neighbourhood pass run AFTER the point-operation filter (e.g. halation /
// bloom / chromatic aberration that act on the finished image).
fn run_postpass(slice: &mut [u8], width: u32, height: u32, p: &FilmProfile) {
    apply_halation(slice, width, height, p);
    apply_chromatic_aberration(slice, width, height, p);
}

// Image pipeline entry point for film profiles. When a profile only uses point
// operations (the common case, and all current profiles) this reduces to a
// single `process_filter` call — byte-for-byte identical to the legacy path,
// with no extra allocation or copying. Neighbourhood passes only run when a
// profile explicitly opts in.
fn process_image(slice: &mut [u8], width: u32, height: u32, p: &FilmProfile) {
    if needs_prepass(p) {
        run_prepass(slice, width, height, p);
    }

    process_filter(slice, width, height, p);

    if needs_postpass(p) {
        run_postpass(slice, width, height, p);
    }
}

fn process_vhs(slice: &mut [u8], width: u32, height: u32) {
    let mut rng_global = SimpleRng::new(123456789);

    // VHS LUT points
    let mut lut_r = [0u8; 256];
    let mut lut_g = [0u8; 256];
    let mut lut_b = [0u8; 256];
    for i in 0..256 {
        lut_r[i] = (20.0 + (i as f32 * 1.10) * 0.88).clamp(0.0, 255.0) as u8;
        lut_g[i] = (20.0 + (i as f32 * 0.92) * 0.88).clamp(0.0, 255.0) as u8;
        lut_b[i] = (20.0 + (i as f32 * 1.05) * 0.88).clamp(0.0, 255.0) as u8;
    }

    // Chromatic aberration shift value
    let shift = (width as i32 / 300).max(2);

    // Tracking glitch bands
    let num_bands = rng_global.range(2, 5) as usize;
    let mut bands = Vec::with_capacity(num_bands);
    let max_bh = (height / 60).max(4) as i32;
    for _ in 0..num_bands {
        let bh = rng_global.range(3, max_bh);
        let y_start = rng_global.range(0, height as i32 - bh);
        let dx = rng_global.range(4, (width / 60).max(5) as i32)
            * (if rng_global.next_u32() % 2 == 0 {
                1
            } else {
                -1
            });
        bands.push((y_start, y_start + bh, dx));
    }

    // Head switching noise bar height at the bottom
    let hs = (height / 80).max(2) as i32;
    let hs_start = height as i32 - hs;
    let hs_roll = rng_global.range((width / 40) as i32, (width / 12) as i32);

    // Parallel process image rows
    slice
        .par_chunks_mut((width * 3) as usize)
        .enumerate()
        .for_each(|(y, row)| {
            let mut rng = SimpleRng::new((y as u32).wrapping_add(1) ^ 987654321);
            let y_i = y as i32;

            let mut row_dx = 0;
            let mut row_glitch = false;
            for &(start, end, dx) in &bands {
                if y_i >= start && y_i < end {
                    row_dx = dx;
                    row_glitch = true;
                    break;
                }
            }

            if y_i >= hs_start {
                row_dx = hs_roll;
            }

            let is_scanline = y % 4 < 2;
            let row_copy = row.to_vec();

            for x in 0..(width as usize) {
                let idx = x * 3;
                if idx + 2 >= row.len() {
                    break;
                }

                let shifted_x = if row_dx != 0 {
                    let mut sx = x as i32 - row_dx;
                    while sx < 0 {
                        sx += width as i32;
                    }
                    (sx % width as i32) as usize
                } else {
                    x
                };

                let r_x = ((shifted_x as i32 + shift).clamp(0, width as i32 - 1)) as usize * 3;
                let b_x = ((shifted_x as i32 - shift).clamp(0, width as i32 - 1)) as usize * 3;
                let g_x = shifted_x * 3;

                let r_raw = row_copy[r_x] as i32;
                let g_raw = row_copy[g_x + 1] as i32;
                let b_raw = row_copy[b_x + 2] as i32;

                // 1. Tint point LUT
                let mut r = lut_r[r_raw as usize] as i32;
                let mut g = lut_g[g_raw as usize] as i32;
                let mut b = lut_b[b_raw as usize] as i32;

                // 2. Low saturation (0.65) in Fixed-Point (weights: 306, 601, 117)
                let lum = (306 * r + 601 * g + 117 * b) >> 10;
                r = (lum * 358 + r * 666) >> 10; // (1.0 - 0.65)*1024 = 358.4, 0.65*1024 = 665.6
                g = (lum * 358 + g * 666) >> 10;
                b = (lum * 358 + b * 666) >> 10;

                // 3. Low contrast (0.80, mean 128) in Fixed-Point
                r = 128 + (((r - 128) * 819) >> 10); // 0.80*1024 = 819.2
                g = 128 + (((g - 128) * 819) >> 10);
                b = 128 + (((b - 128) * 819) >> 10);

                // 4. Glitch band brightness boost
                if row_glitch {
                    r += 12;
                    g += 12;
                    b += 12;
                }

                // 5. Scanline darkening
                if is_scanline {
                    r = (r * 154) >> 10; // 0.15 * 1024 = 153.6
                    g = (g * 154) >> 10;
                    b = (b * 154) >> 10;
                }

                // 6. Head switching noise at bottom
                if y_i >= hs_start {
                    let noise_amt = (rng.next_byte() % 90) as i32;
                    r = (r >> 1) + noise_amt;
                    g = (g >> 1) + noise_amt;
                    b = (b >> 1) + noise_amt;
                }

                // 7. Tape grain blending (8% blending -> grain_w = 20)
                let noise_grain = rng.next_byte() as i32;
                r = ((r * 942 + noise_grain * 82) >> 10).clamp(0, 255); // (1 - 0.08)*1024 = 942, 0.08*1024 = 82
                g = ((g * 942 + noise_grain * 82) >> 10).clamp(0, 255);
                b = ((b * 942 + noise_grain * 82) >> 10).clamp(0, 255);

                row[idx] = r as u8;
                row[idx + 1] = g as u8;
                row[idx + 2] = b as u8;
            }
        });
}

#[pyfunction]
fn apply_film_inplace(
    data: &Bound<'_, PyByteArray>,
    width: u32,
    height: u32,
    film_name: &str,
) -> PyResult<()> {
    // Validate the filter name before touching any memory.
    let profile = if film_name == "VHS" {
        None
    } else {
        Some(
            get_profile(film_name)
                .ok_or_else(|| PyValueError::new_err(format!("unknown filter: {film_name}")))?,
        )
    };

    // Validate the buffer size matches the declared dimensions (RGB = 3 bytes/pixel).
    let expected = (width as usize)
        .checked_mul(height as usize)
        .and_then(|px| px.checked_mul(3))
        .ok_or_else(|| PyValueError::new_err("width * height * 3 overflows usize"))?;
    if data.len() != expected {
        return Err(PyValueError::new_err(format!(
            "buffer size {} does not match width*height*3 = {}",
            data.len(),
            expected
        )));
    }

    let slice = unsafe { data.as_bytes_mut() };
    match profile {
        Some(p) => process_image(slice, width, height, &p),
        None => process_vhs(slice, width, height),
    }
    Ok(())
}

/// Return the list of every available filter name, in display order.
#[pyfunction]
fn available_filters() -> Vec<String> {
    FILTER_NAMES.iter().map(|s| s.to_string()).collect()
}

#[pymodule]
fn saturnix_filter(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(apply_film_inplace, m)?)?;
    m.add_function(wrap_pyfunction!(available_filters, m)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Every profile name that `get_profile` knows about, so the passthrough
    // invariant can be checked exhaustively.
    const PROFILE_NAMES: &[&str] = &[
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
        "S-CA",
    ];

    // A small deterministic RGB gradient buffer for pixel-exact comparisons.
    fn gradient_buffer(width: u32, height: u32) -> Vec<u8> {
        let mut buf = vec![0u8; (width * height * 3) as usize];
        for y in 0..height {
            for x in 0..width {
                let idx = ((y * width + x) * 3) as usize;
                buf[idx] = (x * 255 / width.max(1)) as u8;
                buf[idx + 1] = (y * 255 / height.max(1)) as u8;
                buf[idx + 2] = ((x + y) * 255 / (width + height).max(1)) as u8;
            }
        }
        buf
    }

    // Every name reported by `available_filters` must resolve to a real
    // profile, and the list must include the known profiles plus VHS. This
    // keeps the exported list in sync with `get_profile`.
    #[test]
    fn available_filters_are_all_valid() {
        let filters = available_filters();

        // VHS is a filter but not a FilmProfile, so it is reported yet handled
        // separately by the pipeline.
        assert!(
            filters.iter().any(|f| f == "VHS"),
            "available_filters must include VHS"
        );

        for name in &filters {
            if name == "VHS" {
                continue;
            }
            assert!(
                get_profile(name).is_some(),
                "available_filters reported unknown profile: {name}"
            );
        }

        // Every profile the tests know about must be advertised.
        for name in PROFILE_NAMES {
            assert!(
                filters.iter().any(|f| f == name),
                "available_filters is missing profile: {name}"
            );
        }
    }

    // Pipeline passthrough invariant: for every existing profile the new
    // `process_image` dispatcher must produce a byte-for-byte identical result
    // to the legacy `process_filter` point-operation path, because none of the
    // current profiles enable a neighbourhood pass.
    #[test]
    fn process_image_matches_process_filter_for_all_profiles() {
        let (w, h) = (16u32, 12u32);
        for name in PROFILE_NAMES {
            let p = get_profile(name).expect("known profile");
            // Profiles with a neighbourhood pass legitimately differ from the
            // pure point-operation path; the invariant only covers point-only
            // profiles.
            if needs_prepass(&p) || needs_postpass(&p) {
                continue;
            }

            let mut via_pipeline = gradient_buffer(w, h);
            process_image(&mut via_pipeline, w, h, &p);

            let mut via_legacy = gradient_buffer(w, h);
            process_filter(&mut via_legacy, w, h, &p);

            assert_eq!(
                via_pipeline, via_legacy,
                "profile {name}: process_image diverged from process_filter"
            );
        }
    }

    // Halation is a neighbourhood effect: a single bright highlight on a dark
    // field must bleed a warm glow into neighbouring pixels that were dark
    // before. This is impossible for a pure point operation, so it exercises
    // the postpass path specifically.
    #[test]
    fn halation_bleeds_glow_into_dark_neighbours() {
        let (w, h) = (64u32, 64u32);
        let mut buf = vec![0u8; (w * h * 3) as usize];
        // A large bright white block; everything else pitch black.
        for y in 24..40 {
            for x in 24..40 {
                let idx = ((y * w + x) * 3) as usize;
                buf[idx] = 255;
                buf[idx + 1] = 255;
                buf[idx + 2] = 255;
            }
        }

        // A neighbour immediately outside the block edge, initially fully black.
        let probe = ((32 * w + 40) * 3) as usize;
        assert_eq!(buf[probe], 0, "probe must start black");

        // Strong warm halation. A full-brightness (255) highlight sits just
        // above threshold, so it must produce a strong glow, not a faint one.
        let p = FilmProfile {
            halation_strength: 200,
            halation_threshold: 180,
            halation_r: 255,
            halation_g: 120,
            halation_b: 60,
            ..Default::default()
        };
        apply_halation(&mut buf, w, h, &p);

        // The glow near a maxed-out highlight must be substantial, not a token
        // few levels. Before threshold normalisation this pixel only reached
        // ~11; a pure-white source above threshold must now bloom much stronger.
        assert!(
            buf[probe] > 35,
            "halation glow next to a white highlight too weak: red = {} (expected > 35)",
            buf[probe]
        );
        // Warm tint: red grows more than blue.
        assert!(
            buf[probe] > buf[probe + 2],
            "warm halation: red glow ({}) should exceed blue glow ({})",
            buf[probe],
            buf[probe + 2]
        );
    }

    // Chromatic aberration is a neighbourhood effect: red and blue channels are
    // sampled with opposite radial offsets, so a sharp grey edge (R == B
    // everywhere) develops a colour fringe (R != B) that a point operation
    // could never create.
    #[test]
    fn chromatic_aberration_splits_channels_at_edge() {
        let (w, h) = (64u32, 64u32);
        // A vertical grey edge far to the right of centre (x == 50). The radial
        // CA offset there is almost purely horizontal, so red and blue separate
        // across the edge. Every pixel starts neutral (R == G == B).
        let mut buf = vec![0u8; (w * h * 3) as usize];
        for y in 0..h {
            for x in 50..w {
                let idx = ((y * w + x) * 3) as usize;
                buf[idx] = 180;
                buf[idx + 1] = 180;
                buf[idx + 2] = 180;
            }
        }

        let p = FilmProfile {
            ca_strength: 6,
            ..Default::default()
        };
        apply_chromatic_aberration(&mut buf, w, h, &p);

        // On the centre row (y ~ 32) near the edge, look for a pixel where red
        // and blue diverge — the colour fringe. No such split can exist without
        // a neighbourhood shift.
        let mut max_split = 0i32;
        let y = 32u32;
        for x in 44..56u32 {
            let idx = ((y * w + x) * 3) as usize;
            let split = (buf[idx] as i32 - buf[idx + 2] as i32).abs();
            max_split = max_split.max(split);
        }
        assert!(
            max_split > 30,
            "expected a red/blue fringe at the edge, max split was {max_split}"
        );
    }
}
