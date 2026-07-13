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
}

// Identity channel-mix matrix (no cross-channel mixing).
const IDENTITY_MIX: [f32; 9] = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];

fn get_profile(name: &str) -> Option<FilmProfile> {
    match name {
        "S-Gold" => Some(FilmProfile {
            color_r: 1.12,
            color_g: 1.00,
            color_b: 0.80,
            saturation: 1.30,
            contrast: 1.08,
            brightness: 1.03,
            shadow_r: 30,
            shadow_g: 20,
            shadow_b: 5,
            highlight_r: 0,
            highlight_g: 0,
            highlight_b: 0,
            lift_shadows: 18,
            compress_highlights: -12,
            grain: 14,
            vignette: 0.45,
            is_monochrome: false,
            tint_r: 1.0,
            tint_g: 1.0,
            tint_b: 1.0,
            mix: IDENTITY_MIX,
            curve: 0.0,
        }),
        // Kodak Ektar: ultra-saturated. A slightly "unmixing" matrix pulls each
        // channel away from its neighbours to widen colour separation (Ektar's
        // signature clean, differentiated primaries) while keeping neutral grays
        // neutral (rows sum to ~1.0).
        "S-Vivid" => Some(FilmProfile {
            color_r: 0.98,
            color_g: 1.00,
            color_b: 1.03,
            saturation: 1.85,
            contrast: 1.50,
            brightness: 1.00,
            shadow_r: 0,
            shadow_g: 0,
            shadow_b: 0,
            highlight_r: 0,
            highlight_g: 0,
            highlight_b: 0,
            lift_shadows: 0,
            compress_highlights: 0,
            grain: 2,
            vignette: 0.05,
            is_monochrome: false,
            tint_r: 1.0,
            tint_g: 1.0,
            tint_b: 1.0,
            #[rustfmt::skip]
            mix: [
                1.12, -0.07, -0.05, // out_r <- red, minus green/blue
                -0.06, 1.12, -0.06, // out_g <- green, minus red/blue
                -0.05, -0.07, 1.12, // out_b <- blue, minus red/green
            ],
            curve: 0.0,
        }),
        "S-Natural" => Some(FilmProfile {
            color_r: 0.92,
            color_g: 1.02,
            color_b: 1.08,
            saturation: 1.20,
            contrast: 1.10,
            brightness: 1.00,
            shadow_r: 5,
            shadow_g: 18,
            shadow_b: 14,
            highlight_r: 0,
            highlight_g: 0,
            highlight_b: 0,
            lift_shadows: 12,
            compress_highlights: -8,
            grain: 10,
            vignette: 0.40,
            is_monochrome: false,
            tint_r: 1.0,
            tint_g: 1.0,
            tint_b: 1.0,
            mix: IDENTITY_MIX,
            curve: 0.0,
        }),
        "S-Saturnix" => Some(FilmProfile {
            color_r: 1.10,
            color_g: 0.97,
            color_b: 1.00,
            saturation: 1.26,
            contrast: 1.04,
            brightness: 1.02,
            shadow_r: 12,
            shadow_g: 6,
            shadow_b: 40,
            highlight_r: 0,
            highlight_g: 0,
            highlight_b: 0,
            lift_shadows: 20,
            compress_highlights: -6,
            grain: 6,
            vignette: 0.18,
            is_monochrome: false,
            tint_r: 1.0,
            tint_g: 1.0,
            tint_b: 1.0,
            mix: IDENTITY_MIX,
            curve: 0.0,
        }),
        "S-MonoX" => Some(FilmProfile {
            color_r: 0.25,
            color_g: 0.60,
            color_b: 0.15,
            saturation: 0.0,
            contrast: 1.45,
            brightness: 1.03,
            shadow_r: 0,
            shadow_g: 0,
            shadow_b: 0,
            highlight_r: 0,
            highlight_g: 0,
            highlight_b: 0,
            lift_shadows: 0,
            compress_highlights: 0,
            grain: 16,
            vignette: 0.30,
            is_monochrome: true,
            tint_r: 1.0,
            tint_g: 1.0,
            tint_b: 1.0,
            mix: IDENTITY_MIX,
            curve: 0.0,
        }),
        // Kodak Portra 400: warm, restrained saturation, flat forgiving curve,
        // clean slightly-warm shadows, soft highlight roll-off, fine grain.
        "S-Portra" => Some(FilmProfile {
            color_r: 1.05,
            color_g: 1.00,
            color_b: 0.94,
            saturation: 1.08,
            contrast: 0.95,
            brightness: 1.02,
            shadow_r: 8,
            shadow_g: 4,
            shadow_b: 0,
            highlight_r: 0,
            highlight_g: 0,
            highlight_b: 0,
            lift_shadows: 10,
            compress_highlights: -10,
            grain: 8,
            vignette: 0.15,
            is_monochrome: false,
            tint_r: 1.0,
            tint_g: 1.0,
            tint_b: 1.0,
            mix: IDENTITY_MIX,
            curve: 0.0,
        }),
        // Cinestill 800T: tungsten stock in daylight -> strong cool cast,
        // teal-leaning shadows, cinematic contrast, noticeable grain.
        // (Signature red halation bloom is a neighbourhood effect and not modelled here.)
        "S-Cinestill" => Some(FilmProfile {
            color_r: 0.94,
            color_g: 1.00,
            color_b: 1.12,
            saturation: 1.15,
            contrast: 1.15,
            brightness: 1.00,
            shadow_r: 0,
            shadow_g: 10,
            shadow_b: 24,
            highlight_r: 0,
            highlight_g: 0,
            highlight_b: 0,
            lift_shadows: 6,
            compress_highlights: -4,
            grain: 12,
            vignette: 0.25,
            is_monochrome: false,
            tint_r: 1.0,
            tint_g: 1.0,
            tint_b: 1.0,
            // A subtle mix bleeds green into blue (teal shadows) and a little
            // red into blue (cool magenta highlights) for a more authentic
            // tungsten/neon crossover than a per-channel curve alone.
            #[rustfmt::skip]
            mix: [
                0.98, 0.00, 0.02, // out_r <- red, faint blue
                0.00, 0.97, 0.03, // out_g <- green, faint blue
                0.05, 0.08, 0.90, // out_b <- blue + green/red bleed (teal)
            ],
            curve: 0.0,
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
            brightness: 1.00,
            shadow_r: 0,
            shadow_g: 10,
            shadow_b: 30,
            highlight_r: 0,
            highlight_g: 0,
            highlight_b: 0,
            lift_shadows: 0,
            compress_highlights: 0,
            grain: 12,
            vignette: 0.20,
            is_monochrome: false,
            tint_r: 1.0,
            tint_g: 1.0,
            tint_b: 1.0,
            #[rustfmt::skip]
            mix: [
                0.92, 0.14, -0.06, // out_r <- red + a little green (yellow highlights)
                0.00, 0.94, 0.10,  // out_g <- green + a little blue (cyan cast)
                0.06, 0.00, 0.98,  // out_b <- blue + a touch of red
            ],
            curve: 0.0,
        }),
        // Faded / aged vintage print: milky lifted blacks, dulled highlights,
        // warm yellow-magenta cast, low saturation, flat compressed range.
        "S-Faded" => Some(FilmProfile {
            color_r: 1.06,
            color_g: 1.00,
            color_b: 0.90,
            saturation: 0.82,
            contrast: 0.82,
            brightness: 1.02,
            shadow_r: 20,
            shadow_g: 14,
            shadow_b: 6,
            highlight_r: 0,
            highlight_g: 0,
            highlight_b: 0,
            lift_shadows: 35,
            compress_highlights: -15,
            grain: 10,
            vignette: 0.30,
            is_monochrome: false,
            tint_r: 1.0,
            tint_g: 1.0,
            tint_b: 1.0,
            mix: IDENTITY_MIX,
            curve: 0.0,
        }),
        // Bleach bypass (silver retention): heavily desaturated, very high
        // contrast, near-neutral slightly-cool metallic look, gritty grain.
        "S-Bleach" => Some(FilmProfile {
            color_r: 0.98,
            color_g: 1.00,
            color_b: 1.02,
            saturation: 0.45,
            contrast: 1.50,
            brightness: 1.02,
            shadow_r: 0,
            shadow_g: 0,
            shadow_b: 4,
            highlight_r: 0,
            highlight_g: 0,
            highlight_b: 0,
            lift_shadows: 0,
            compress_highlights: 0,
            grain: 14,
            vignette: 0.20,
            is_monochrome: false,
            tint_r: 1.0,
            tint_g: 1.0,
            tint_b: 1.0,
            mix: IDENTITY_MIX,
            curve: 0.0,
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
            shadow_r: 0,
            shadow_g: 0,
            shadow_b: 0,
            highlight_r: 0,
            highlight_g: 0,
            highlight_b: 0,
            lift_shadows: 8,
            compress_highlights: -6,
            grain: 12,
            vignette: 0.30,
            is_monochrome: true,
            tint_r: 1.15,
            tint_g: 1.00,
            tint_b: 0.72,
            mix: IDENTITY_MIX,
            curve: 0.0,
        }),
        // Cyanotype: cool blue-toned B&W. Neutral luminance is tinted towards
        // blue and away from red.
        "S-Cyano" => Some(FilmProfile {
            color_r: 0.30,
            color_g: 0.59,
            color_b: 0.11,
            saturation: 0.0,
            contrast: 1.15,
            brightness: 1.00,
            shadow_r: 0,
            shadow_g: 0,
            shadow_b: 0,
            highlight_r: 0,
            highlight_g: 0,
            highlight_b: 0,
            lift_shadows: 6,
            compress_highlights: 0,
            grain: 8,
            vignette: 0.25,
            is_monochrome: true,
            tint_r: 0.62,
            tint_g: 0.90,
            tint_b: 1.25,
            mix: IDENTITY_MIX,
            curve: 0.0,
        }),
        // Noir: high-contrast neutral B&W with a heavy vignette.
        "S-Noir" => Some(FilmProfile {
            color_r: 0.30,
            color_g: 0.59,
            color_b: 0.11,
            saturation: 0.0,
            contrast: 1.70,
            brightness: 1.00,
            shadow_r: 0,
            shadow_g: 0,
            shadow_b: 0,
            highlight_r: 0,
            highlight_g: 0,
            highlight_b: 0,
            lift_shadows: 0,
            compress_highlights: 0,
            grain: 10,
            vignette: 0.45,
            is_monochrome: true,
            tint_r: 1.0,
            tint_g: 1.0,
            tint_b: 1.0,
            mix: IDENTITY_MIX,
            curve: 0.0,
        }),
        // Teal & Orange: cinematic look with warm highlights (orange skin/light)
        // and teal-pushed shadows.
        "S-Teal" => Some(FilmProfile {
            color_r: 1.08,
            color_g: 0.99,
            color_b: 0.96,
            saturation: 1.20,
            contrast: 1.18,
            brightness: 1.00,
            shadow_r: 0,
            shadow_g: 14,
            shadow_b: 26,
            highlight_r: 0,
            highlight_g: 0,
            highlight_b: 0,
            lift_shadows: 6,
            compress_highlights: -4,
            grain: 6,
            vignette: 0.22,
            is_monochrome: false,
            tint_r: 1.0,
            tint_g: 1.0,
            tint_b: 1.0,
            mix: IDENTITY_MIX,
            curve: 0.0,
        }),
        // Lomo / toy camera: oversaturated, punchy, heavy vignette and grain.
        "S-Lomo" => Some(FilmProfile {
            color_r: 1.06,
            color_g: 1.02,
            color_b: 1.00,
            saturation: 1.70,
            contrast: 1.35,
            brightness: 1.00,
            shadow_r: 6,
            shadow_g: 4,
            shadow_b: 18,
            highlight_r: 0,
            highlight_g: 0,
            highlight_b: 0,
            lift_shadows: 4,
            compress_highlights: -6,
            grain: 16,
            vignette: 0.65,
            is_monochrome: false,
            tint_r: 1.0,
            tint_g: 1.0,
            tint_b: 1.0,
            mix: IDENTITY_MIX,
            curve: 0.0,
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
            brightness: 1.00,
            shadow_r: 0,
            shadow_g: 8,
            shadow_b: 6,
            highlight_r: 0,
            highlight_g: 0,
            highlight_b: 0,
            lift_shadows: 0,
            compress_highlights: -6,
            grain: 4,
            vignette: 0.12,
            is_monochrome: false,
            tint_r: 1.0,
            tint_g: 1.0,
            tint_b: 1.0,
            #[rustfmt::skip]
            mix: [
                1.06, -0.03, -0.03, // out_r <- red, lightly cleaned
                -0.02, 1.10, -0.08, // out_g <- green, minus blue (warmer greens)
                -0.02, -0.06, 1.08, // out_b <- blue, minus green (deeper skies)
            ],
            curve: 0.0,
        }),
        // Selenium-toned B&W: cool, slightly purple tone.
        "S-Selenium" => Some(FilmProfile {
            color_r: 0.30,
            color_g: 0.59,
            color_b: 0.11,
            saturation: 0.0,
            contrast: 1.30,
            brightness: 1.00,
            shadow_r: 0,
            shadow_g: 0,
            shadow_b: 0,
            highlight_r: 0,
            highlight_g: 0,
            highlight_b: 0,
            lift_shadows: 0,
            compress_highlights: 0,
            grain: 8,
            vignette: 0.28,
            is_monochrome: true,
            tint_r: 0.94,
            tint_g: 0.96,
            tint_b: 1.10,
            mix: IDENTITY_MIX,
            curve: 0.0,
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
            shadow_r: 0,
            shadow_g: 0,
            shadow_b: 0,
            highlight_r: 0,
            highlight_g: 0,
            highlight_b: 0,
            lift_shadows: 14,
            compress_highlights: -8,
            grain: 6,
            vignette: 0.20,
            is_monochrome: true,
            tint_r: 1.08,
            tint_g: 1.02,
            tint_b: 0.90,
            mix: IDENTITY_MIX,
            curve: 0.0,
        }),
        // False-colour infrared (Aerochrome-style): the channel-mix routes the
        // green (foliage/IR) signal into red so vegetation renders crimson,
        // shifts red into green, and keeps blue dark. High saturation and
        // contrast complete the surreal look.
        "S-Infrared" => Some(FilmProfile {
            color_r: 1.00,
            color_g: 1.00,
            color_b: 1.00,
            saturation: 1.45,
            contrast: 1.20,
            brightness: 1.00,
            shadow_r: 0,
            shadow_g: 0,
            shadow_b: 0,
            highlight_r: 0,
            highlight_g: 0,
            highlight_b: 0,
            lift_shadows: 0,
            compress_highlights: -6,
            grain: 8,
            vignette: 0.20,
            is_monochrome: false,
            tint_r: 1.0,
            tint_g: 1.0,
            tint_b: 1.0,
            #[rustfmt::skip]
            mix: [
                0.10, 0.90, 0.10, // out_r <- mostly green (IR foliage -> red)
                0.85, 0.15, 0.05, // out_g <- mostly red
                0.05, 0.10, 0.70, // out_b <- attenuated blue
            ],
            curve: 0.0,
        }),
        // Split-toning: independent tints for highlights and shadows. Warm
        // (orange) highlights and cool (teal/blue) shadows - the classic
        // cinematic split-tone grade.
        "S-SplitTone" => Some(FilmProfile {
            color_r: 1.00,
            color_g: 1.00,
            color_b: 1.00,
            saturation: 1.10,
            contrast: 1.12,
            brightness: 1.00,
            shadow_r: -14,
            shadow_g: 4,
            shadow_b: 24,
            highlight_r: 22,
            highlight_g: 8,
            highlight_b: -18,
            lift_shadows: 0,
            compress_highlights: 0,
            grain: 6,
            vignette: 0.18,
            is_monochrome: false,
            tint_r: 1.0,
            tint_g: 1.0,
            tint_b: 1.0,
            mix: IDENTITY_MIX,
            curve: 0.0,
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
            brightness: 1.00,
            shadow_r: 0,
            shadow_g: 0,
            shadow_b: 12,
            highlight_r: 14,
            highlight_g: 6,
            highlight_b: -8,
            lift_shadows: 0,
            compress_highlights: -6,
            grain: 8,
            vignette: 0.22,
            is_monochrome: false,
            tint_r: 1.0,
            tint_g: 1.0,
            tint_b: 1.0,
            #[rustfmt::skip]
            mix: [
                1.08, -0.04, -0.04, // out_r <- clean red
                -0.03, 1.05, -0.02, // out_g
                -0.02, -0.04, 1.06, // out_b <- deeper blue
            ],
            curve: 0.0,
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
            shadow_r: 0,
            shadow_g: 14,
            shadow_b: 20,
            highlight_r: 12,
            highlight_g: 6,
            highlight_b: 0,
            lift_shadows: 40,
            compress_highlights: -12,
            grain: 10,
            vignette: 0.45,
            is_monochrome: false,
            tint_r: 1.0,
            tint_g: 1.0,
            tint_b: 1.0,
            mix: IDENTITY_MIX,
            curve: 0.0,
        }),
        // The Matrix / digital-dystopia: dominant green cast driven by the
        // channel mix and green highlights.
        "S-Matrix" => Some(FilmProfile {
            color_r: 0.90,
            color_g: 1.10,
            color_b: 0.92,
            saturation: 1.10,
            contrast: 1.20,
            brightness: 1.00,
            shadow_r: 0,
            shadow_g: 12,
            shadow_b: 0,
            highlight_r: 0,
            highlight_g: 16,
            highlight_b: -6,
            lift_shadows: 4,
            compress_highlights: -4,
            grain: 8,
            vignette: 0.28,
            is_monochrome: false,
            tint_r: 1.0,
            tint_g: 1.0,
            tint_b: 1.0,
            #[rustfmt::skip]
            mix: [
                0.85, 0.15, 0.00, // out_r <- some green
                0.10, 0.95, 0.10, // out_g <- boosted with red/blue bleed
                0.00, 0.18, 0.82, // out_b <- some green
            ],
            curve: 0.0,
        }),
        // Cinematic tone curve: a pronounced filmic S-curve gives a rich toe and
        // shoulder (deep-but-detailed shadows, gentle highlight roll-off) with a
        // mild teal/orange split-tone for a modern digital-cinema grade.
        "S-Cine" => Some(FilmProfile {
            color_r: 1.00,
            color_g: 1.00,
            color_b: 1.00,
            saturation: 1.08,
            contrast: 1.00,
            brightness: 1.00,
            shadow_r: -8,
            shadow_g: 2,
            shadow_b: 12,
            highlight_r: 10,
            highlight_g: 4,
            highlight_b: -8,
            lift_shadows: 0,
            compress_highlights: 0,
            grain: 6,
            vignette: 0.18,
            is_monochrome: false,
            tint_r: 1.0,
            tint_g: 1.0,
            tint_b: 1.0,
            mix: IDENTITY_MIX,
            curve: 0.55,
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

// Compile-Time specialize loop iterations using Rust Const Generics!
// This completely removes branch 'if' statements from the inner loops, allowing
// LLVM's auto-vectorizer to generate highly efficient SIMD (NEON/SSE/AVX) assembly instructions.
fn process_filter_generic<const MONO: bool, const VIG: bool, const GRAIN: bool, const MIX: bool>(
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

    // Parallel process image rows
    slice
        .par_chunks_mut((width * 3) as usize)
        .enumerate()
        .for_each(|(y, row)| {
            let y_f = y as f32;
            let dy = y_f - cy;
            let dy_sq = (dy * dy) as u64;

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

    // Dispatch compile-time specialized loop branches.
    match (mono, vig, grain, mix) {
        (true, true, true, _) => {
            process_filter_generic::<true, true, true, false>(slice, width, height, p)
        }
        (true, true, false, _) => {
            process_filter_generic::<true, true, false, false>(slice, width, height, p)
        }
        (true, false, true, _) => {
            process_filter_generic::<true, false, true, false>(slice, width, height, p)
        }
        (true, false, false, _) => {
            process_filter_generic::<true, false, false, false>(slice, width, height, p)
        }
        (false, true, true, false) => {
            process_filter_generic::<false, true, true, false>(slice, width, height, p)
        }
        (false, true, true, true) => {
            process_filter_generic::<false, true, true, true>(slice, width, height, p)
        }
        (false, true, false, false) => {
            process_filter_generic::<false, true, false, false>(slice, width, height, p)
        }
        (false, true, false, true) => {
            process_filter_generic::<false, true, false, true>(slice, width, height, p)
        }
        (false, false, true, false) => {
            process_filter_generic::<false, false, true, false>(slice, width, height, p)
        }
        (false, false, true, true) => {
            process_filter_generic::<false, false, true, true>(slice, width, height, p)
        }
        (false, false, false, false) => {
            process_filter_generic::<false, false, false, false>(slice, width, height, p)
        }
        (false, false, false, true) => {
            process_filter_generic::<false, false, false, true>(slice, width, height, p)
        }
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
        Some(p) => process_filter(slice, width, height, &p),
        None => process_vhs(slice, width, height),
    }
    Ok(())
}

#[pymodule]
fn saturnix_filter(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(apply_film_inplace, m)?)?;
    Ok(())
}
