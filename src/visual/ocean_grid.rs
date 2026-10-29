use crate::analysis::music_noise;
use crate::features::AudioFeatures;

pub const DEFAULT_CHARSET: &str = " .:-=+*#%@";

pub fn render(
    f: &AudioFeatures,
    w: u16,
    h: u16,
    t: f32,
    sensitivity: f32,
    charset: &str,
) -> String {
    let w = w.max(8) as i32;
    let h = h.max(4) as i32;
    let palette: Vec<char> = charset.chars().collect();
    if palette.is_empty() {
        return String::new();
    }
    let last = palette.len() - 1;

    let amp = sensitivity;
    let bass = f.bass_smooth * amp;
    let mid = f.mid_energy * amp;
    let high = f.high_energy * amp;
    let cx = w as f32 * 0.5;
    let cy = h as f32 * 0.5;

    let mut lines = Vec::with_capacity(h as usize);
    for y in 0..h {
        let mut row = String::with_capacity(w as usize);
        let fy = y as f32 / h as f32;
        for x in 0..w {
            let dx = (x as f32 - cx) / w as f32;
            let dy = (y as f32 - cy) / h as f32;

            let wave1 = (dx * 14.0 + t * 1.7 + bass * 6.0).sin();
            let wave2 = (dy * 11.0 - t * 1.2 + mid * 5.0).cos();
            let ripple = (wave1 * wave2) * (0.35 + high * 0.65);
            let n = music_noise(x as u32, y as u32, t, f.spectral_centroid) * 0.12 * high;

            let mut v = ripple + (fy - 0.5).abs() * mid * 0.4 + f.volume_smooth * 0.25 + n;
            v += f.beat_pulse * 0.35 * (1.0 - (dx * dx + dy * dy).sqrt() * 1.2).max(0.0);
            v *= 0.5 + amp * 0.5;

            let idx = ((v + 1.0) * 0.5 * last as f32).round() as usize;
            let idx = idx.min(last);
            row.push(palette[idx]);
        }
        lines.push(row);
    }
    lines.join("\n")
}
