use crate::analysis::music_noise;
use crate::features::AudioFeatures;

fn col_phase(x: u32) -> f32 {
    let h = x.wrapping_mul(0x9e37_79b9);
    (h as f32) * 2.328_306_4e-10
}

/// ASCII-дождь / поток сигнала: плотность от ритма, бас — импульсы снизу, высокие — редкие искры.
pub fn render(
    f: &AudioFeatures,
    w: u16,
    h: u16,
    t: f32,
    sensitivity: f32,
    charset: &str,
) -> String {
    let w = w.max(16) as i32;
    let h = h.max(8) as i32;
    let palette: Vec<char> = charset.chars().collect();
    if palette.is_empty() {
        return String::new();
    }
    let last = palette.len() - 1;
    let amp = sensitivity;

    let rhythm =
        f.beat_pulse * 0.55 + f.transient_strength * 0.45 + f.volume_smooth * 0.2;
    let density = (0.12 + rhythm * amp * 0.9).clamp(0.04, 1.0);

    let fall = t * (7.0 + f.bass_smooth * 14.0 * amp + f.beat_pulse * 22.0);
    let shear = f.mid_energy * amp * 0.35;

    let mut lines = Vec::with_capacity(h as usize);
    for y in 0..h {
        let fy = y as f32 / h as f32;
        let mut row = String::with_capacity(w as usize);
        for x in 0..w {
            let cx = x as f32 / w as f32;
            let ph = col_phase(x as u32) * std::f32::consts::TAU;

            let drift = col_phase((x as u32).wrapping_add(y as u32)) * 2.0 - 1.0;
            let vy = y as f32 + fall * (0.65 + col_phase(x as u32) * 0.7);
            let vx = x as f32 + shear * (fy - 0.5) * w as f32 + drift * 0.8;

            let stream = ((vy * 0.5 + vx * 0.09 + ph + t * 0.7).sin() * 0.5 + 0.5) * density;
            let weave = ((vx * 0.12 - vy * 0.31 + f.spectral_centroid * 8.0).cos() * 0.5 + 0.5) * 0.25;
            let band = (stream * 0.75 + weave * density).clamp(0.0, 1.0);

            let bottom_weight = (1.0 - fy).powf(1.35);
            let center = 1.0 - (cx - 0.5).abs() * 2.2;
            let drip = f.bass_fast * amp * bottom_weight * center.max(0.0);
            let bass_glob = f.bass_smooth * amp * bottom_weight * 0.35;

            let spark_n = music_noise(x as u32, y as u32, t * 4.2, f.spectral_centroid);
            let spark = if f.high_energy * amp > 0.06 {
                spark_n * f.high_energy * amp * 0.65
            } else {
                0.0
            };

            let mut v = band + drip * 0.95 + bass_glob + spark * 0.7;
            v = (v * (0.55 + amp * 0.45)).clamp(0.0, 1.0);

            let thresh = 0.38 - density * 0.14;
            let c = if v > thresh {
                let span = (1.0 - thresh).max(0.08);
                let mut idx = (((v - thresh) / span) * last as f32).round() as usize;
                idx = idx.min(last);
                if spark > 0.42 && last > 0 {
                    idx = (idx + 1).min(last);
                }
                palette[idx]
            } else if f.mid_energy * amp > 0.2 && spark_n > 0.92 {
                palette[(last / 2).max(1).min(last)]
            } else {
                ' '
            };
            row.push(c);
        }
        lines.push(row);
    }
    lines.join("\n")
}
