use crate::analysis::music_noise;
use crate::features::AudioFeatures;

pub fn render(
    f: &AudioFeatures,
    w: u16,
    h: u16,
    t: f32,
    sensitivity: f32,
    charset: &str,
) -> String {
    let w = w.max(16) as f32;
    let h = h.max(8) as f32;
    let palette: Vec<char> = charset.chars().collect();
    if palette.is_empty() {
        return String::new();
    }
    let last = palette.len() - 1;
    let amp = sensitivity;

    let speed = 0.35 + f.bass_fast * amp * 1.8 + f.bass_smooth * amp * 0.5;
    let warp = f.mid_energy * amp * 0.4;
    let flash = f.high_energy * amp;

    let cx = w * 0.5;
    let cy = h * 0.5;

    let mut lines = Vec::with_capacity(h as usize);
    for y in 0..h as i32 {
        let mut row = String::with_capacity(w as usize);
        let fy = y as f32 / h;
        for x in 0..w as i32 {
            let dx = (x as f32 - cx) / w;
            let dy = (y as f32 - cy) / h;

            let depth = (0.35 + (fy - 0.5).abs() * 1.6).clamp(0.15, 1.0);
            let z = 1.0 / depth;
            let twist = (t * speed + z * 2.5).sin() * warp;
            let u = dx * z + twist * 0.15;
            let rings = ((u * 18.0 + t * speed * 6.0).sin() * 0.5 + 0.5) * (0.4 + z * 0.6);
            let side = (dy.abs() * z * 3.2 + rings * 0.35).clamp(0.0, 1.0);

            let mut v = side;
            v += flash * music_noise(x as u32, y as u32, t * 2.0, f.spectral_centroid) * 0.5;
            v += f.transient_strength * 0.4 * (1.0 - depth);
            v += f.beat_pulse * 0.2 * rings;
            v = (v * (0.5 + amp * 0.5)).clamp(0.0, 1.0);

            let idx = (v * last as f32).round() as usize;
            let idx = idx.min(last);
            row.push(palette[idx]);
        }
        lines.push(row);
    }
    lines.join("\n")
}
