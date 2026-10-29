//! Цвет символа и фона: схемы (спокойные / яркие), переключение отдельно от палитры символов.

use crate::analysis::music_noise;
use crate::features::AudioFeatures;

/// Схема окраски (клавиша `c` — цикл).
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum ColorScheme {
    /// Спокойная сине-бирюзовая, низкая насыщенность (по умолчанию).
    #[default]
    CalmOcean,
    /// Прежняя насыщенная раскраска.
    Vivid,
    /// Почти монохром, лёгкий оттенок стали.
    MonoSteel,
    /// Приглушённые фиолетово-синие сумерки.
    DuskViolet,
}

impl ColorScheme {
    pub const COUNT: usize = 4;

    pub fn cycle(self) -> Self {
        match self {
            ColorScheme::CalmOcean => ColorScheme::Vivid,
            ColorScheme::Vivid => ColorScheme::MonoSteel,
            ColorScheme::MonoSteel => ColorScheme::DuskViolet,
            ColorScheme::DuskViolet => ColorScheme::CalmOcean,
        }
    }
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let h = h.fract().rem_euclid(1.0);
    let s = s.clamp(0.0, 1.0);
    let v = v.clamp(0.0, 1.0);
    let i = (h * 6.0).floor() as i32;
    let f = h * 6.0 - i as f32;
    let p = v * (1.0 - s);
    let q = v * (1.0 - f * s);
    let t = v * (1.0 - (1.0 - f) * s);
    let (r, g, b) = match i.rem_euclid(6) {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    ((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
}

/// Базовые h,s,v до применения схемы (как раньше, но с меньшим «дёрганьем» hue).
fn base_hsv(
    f: &AudioFeatures,
    t: f32,
    col: u32,
    row: u32,
    n_rows: u32,
    ch: char,
    wobble: f32,
) -> (f32, f32, f32) {
    let nr = n_rows.max(1) as f32;
    let y = row as f32 / nr;
    let x = col as f32 * 0.035;

    let bass_w = f.bass_smooth * (1.0 - y).powf(1.12);
    let high_w = f.high_energy * (0.2 + y).powf(1.0);
    let mid_w = f.mid_energy * (1.0 - (y - 0.45).abs() * 2.0).max(0.0);

    let h_bass = 0.02 + f.beat_pulse * 0.025;
    let h_mid = 0.28 + f.spectral_centroid * 0.18 + x * 0.025;
    let h_high = 0.52 + t * 0.08 + x * 0.04;

    let sum = bass_w + mid_w + high_w + 0.025;
    let mut h = (h_bass * bass_w + h_mid * mid_w + h_high * high_w) / sum;
    h += t * 0.035 * f.transient_strength * wobble;
    h += (t * 0.9 + col as f32 * 0.09 + row as f32 * 0.055).sin()
        * 0.018
        * wobble
        * (0.45 + f.volume_smooth);
    h = h.fract();

    let s = (0.38 + f.mid_energy * 0.32 + f.beat_pulse * 0.16).clamp(0.2, 1.0);
    let mut v = if ch == ' ' {
        0.05 + f.volume_smooth * 0.06
    } else {
        0.34 + f.volume_smooth * 0.38 + bass_w * 0.14 + high_w * 0.1
    };
    v = (v + f.stereo_width * 0.04 * (col as f32 * 0.018).sin()).clamp(0.07, 1.0);

    (h, s, v)
}

fn scheme_adjust(scheme: ColorScheme, h: f32, s: f32, v: f32, ch: char) -> (f32, f32, f32) {
    match scheme {
        ColorScheme::CalmOcean => {
            let h = 0.48 + (h - 0.5) * 0.22;
            let s = s * 0.42;
            let v = if ch == ' ' {
                v * 0.75
            } else {
                v * 0.72 + 0.08
            };
            (h.fract(), s.clamp(0.08, 0.42), v.clamp(0.06, 0.62))
        }
        ColorScheme::Vivid => (h, s, v),
        ColorScheme::MonoSteel => {
            let h = 0.52;
            let s = (0.06 + s * 0.18).clamp(0.04, 0.22);
            let v = v * 0.88;
            (h, s, v.clamp(0.08, 0.92))
        }
        ColorScheme::DuskViolet => {
            let h = 0.72 + (h - 0.5) * 0.12;
            let s = s * 0.55;
            let v = if ch == ' ' {
                v * 0.7
            } else {
                v * 0.68 + 0.06
            };
            (h.fract(), s.clamp(0.12, 0.48), v.clamp(0.08, 0.58))
        }
    }
}

/// RGB для символа с учётом схемы.
pub fn rgb_for_cell(
    f: &AudioFeatures,
    t: f32,
    col: u32,
    row: u32,
    n_rows: u32,
    ch: char,
    scheme: ColorScheme,
) -> (u8, u8, u8) {
    let wobble = match scheme {
        ColorScheme::Vivid => 1.0,
        _ => 0.55,
    };
    let (h, s, v) = base_hsv(f, t, col, row, n_rows, ch, wobble);
    let (h, s, v) = scheme_adjust(scheme, h, s, v, ch);
    hsv_to_rgb(h, s, v)
}

/// Тёмный переливающийся фон (ячейка).
pub fn rgb_background_cell(
    f: &AudioFeatures,
    t: f32,
    col: u32,
    row: u32,
    scheme: ColorScheme,
) -> (u8, u8, u8) {
    let n = music_noise(col, row, t * 0.25, f.spectral_centroid);
    let w = (t * 0.35 + col as f32 * 0.07 + row as f32 * 0.055 + f.bass_smooth * 0.4).sin() * 0.5
        + 0.5;
    let pulse = f.beat_pulse * 0.35 + f.volume_smooth * 0.25;

    let mut r = 4.0 + w * 7.0 + n * 5.0 + pulse * 6.0;
    let mut g = 5.0 + w * 9.0 + f.mid_energy * 10.0 + pulse * 5.0;
    let mut b = 10.0 + w * 16.0 + f.high_energy * 12.0 + pulse * 8.0;

    match scheme {
        ColorScheme::CalmOcean => {
            r *= 0.85;
            g += 2.0;
            b += 4.0;
        }
        ColorScheme::MonoSteel => {
            r = r * 0.9 + 2.0;
            g = g * 0.95 + 2.0;
            b = b * 0.92 + 3.0;
        }
        ColorScheme::DuskViolet => {
            r += 4.0 + f.high_energy * 6.0;
            g *= 0.88;
            b += 6.0;
        }
        ColorScheme::Vivid => {}
    }

    (
        r.clamp(0.0, 40.0) as u8,
        g.clamp(0.0, 38.0) as u8,
        b.clamp(0.0, 48.0) as u8,
    )
}

pub fn color32_for_cell(
    f: &AudioFeatures,
    t: f32,
    col: u32,
    row: u32,
    n_rows: u32,
    ch: char,
    scheme: ColorScheme,
) -> eframe::egui::Color32 {
    let (r, g, b) = rgb_for_cell(f, t, col, row, n_rows, ch, scheme);
    eframe::egui::Color32::from_rgb(r, g, b)
}

pub fn color32_background_cell(
    f: &AudioFeatures,
    t: f32,
    col: u32,
    row: u32,
    scheme: ColorScheme,
) -> eframe::egui::Color32 {
    let (r, g, b) = rgb_background_cell(f, t, col, row, scheme);
    eframe::egui::Color32::from_rgb(r, g, b)
}

pub fn ratatui_fg(
    f: &AudioFeatures,
    t: f32,
    col: u32,
    row: u32,
    n_rows: u32,
    ch: char,
    scheme: ColorScheme,
) -> ratatui::style::Color {
    let (r, g, b) = rgb_for_cell(f, t, col, row, n_rows, ch, scheme);
    ratatui::style::Color::Rgb(r, g, b)
}

pub fn ratatui_bg(
    f: &AudioFeatures,
    t: f32,
    col: u32,
    row: u32,
    scheme: ColorScheme,
) -> ratatui::style::Color {
    let (r, g, b) = rgb_background_cell(f, t, col, row, scheme);
    ratatui::style::Color::Rgb(r, g, b)
}
