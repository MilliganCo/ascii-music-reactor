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
    let w = w.max(12) as i32;
    let h = h.max(6) as i32;
    let palette: Vec<char> = charset.chars().collect();
    if palette.is_empty() {
        return String::new();
    }
    let last = palette.len() - 1;
    let amp = sensitivity;

    let breath = 1.0 + f.bass_smooth * amp * 0.55;
    let shake = f.mid_energy * amp * 0.08;
    let spark = f.high_energy * amp;

    let mut grid = vec![vec![' '; w as usize]; h as usize];
    let cx = (w - 1) as f32 * 0.5;

    for y in 0..h {
        let row = (y as f32 / (h - 1).max(1) as f32 - 0.5).abs();
        let arch = (1.0 - row * 1.4).max(0.0);
        let span = (cx * arch * breath) as i32;
        let x0 = (cx as i32) - span;
        let x1 = (cx as i32) + span;

        for x in 0..w {
            let inside = x >= x0 && x <= x1;
            if !inside {
                continue;
            }
            let dist = ((x as f32 - cx).abs() / (span.max(1) as f32)).clamp(0.0, 1.0);
            let col_strength = (1.0 - dist).powf(1.2);
            let jitter = (t * 3.0 + y as f32 * 0.4).sin() * shake;
            let mut v = col_strength * (0.4 + f.bass_smooth * amp * 0.6) + jitter;
            v += spark * music_noise(x as u32, y as u32, t, f.spectral_centroid) * 0.4;
            v += f.beat_pulse * 0.25 * col_strength;
            v = v.clamp(0.0, 1.0);
            let idx = (v * last as f32).round() as usize;
            let idx = idx.min(last);

            let wall = x == x0 || x == x1;
            let c = if wall {
                palette[(idx + 1).min(last)]
            } else {
                palette[idx]
            };
            grid[y as usize][x as usize] = c;
        }
    }

    // «арка» сверху
    for x in 0..w {
        let dx = (x as f32 - cx).abs() / cx.max(1.0);
        if dx < 0.92 {
            let cap = ((1.0 - dx / 0.92) * 4.0).round() as i32;
            for k in 0..cap.min(3) {
                let yy = k as usize;
                if yy < grid.len() {
                    let idx = ((1.0 - dx) * last as f32 * (0.5 + f.high_energy)) as usize;
                    let idx = idx.min(last);
                    grid[yy][x as usize] = palette[idx];
                }
            }
        }
    }

    grid
        .into_iter()
        .map(|r| r.into_iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}
