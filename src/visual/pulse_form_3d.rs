//! Псевдо-3D каркас (куб), пульсация и неравномерное масштабирование по осям от баса / мидов / высоких.

use std::f32::consts::PI;

use crate::features::AudioFeatures;

const VERTS: [[f32; 3]; 8] = [
    [-1.0, -1.0, -1.0],
    [1.0, -1.0, -1.0],
    [1.0, 1.0, -1.0],
    [-1.0, 1.0, -1.0],
    [-1.0, -1.0, 1.0],
    [1.0, -1.0, 1.0],
    [1.0, 1.0, 1.0],
    [-1.0, 1.0, 1.0],
];

const EDGES: [(usize, usize); 12] = [
    (0, 1),
    (1, 2),
    (2, 3),
    (3, 0),
    (4, 5),
    (5, 6),
    (6, 7),
    (7, 4),
    (0, 4),
    (1, 5),
    (2, 6),
    (3, 7),
];

fn rot_x(p: [f32; 3], a: f32) -> [f32; 3] {
    let (s, c) = a.sin_cos();
    [p[0], p[1] * c - p[2] * s, p[1] * s + p[2] * c]
}

fn rot_y(p: [f32; 3], a: f32) -> [f32; 3] {
    let (s, c) = a.sin_cos();
    [p[2] * s + p[0] * c, p[1], p[2] * c - p[0] * s]
}

fn rot_z(p: [f32; 3], a: f32) -> [f32; 3] {
    let (s, c) = a.sin_cos();
    [p[0] * c - p[1] * s, p[0] * s + p[1] * c, p[2]]
}

fn transform_vertex(
    p: [f32; 3],
    f: &AudioFeatures,
    t: f32,
    sens: f32,
) -> [f32; 3] {
    let s = sens.clamp(0.35, 2.3);
    let pulse = 1.0 + f.bass_smooth * 0.22 * s + f.beat_pulse * 0.12;
    let sx = (1.0 + f.bass_smooth * 0.5 * s + f.transient_strength * 0.15) * pulse;
    let sy = (1.0 + f.mid_energy * 0.48 * s) * pulse;
    let sz = (1.0 + f.high_energy * 0.42 * s) * pulse;

    let mut q = [p[0] * sx, p[1] * sy, p[2] * sz];

    let rx = t * 0.55 + f.bass_smooth * 1.35 * s;
    let ry = t * 0.38 + f.mid_energy * 1.75 * s;
    let rz = f.high_energy * (t * 2.2).sin() * 1.25 * s + f.stereo_width * 0.6 * s;

    q = rot_z(rot_y(rot_x(q, rx), ry), rz);
    q
}

fn project(
    p: [f32; 3],
    w: i32,
    h: i32,
    cam: f32,
    scale: f32,
) -> (f32, f32, f32) {
    let d = p[2] + cam;
    let inv = 1.0 / d.max(0.35);
    let cx = w as f32 * 0.5;
    let cy = h as f32 * 0.5;
    let x = cx + p[0] * inv * scale;
    let y = cy - p[1] * inv * scale;
    (x, y, p[2])
}

fn edge_char(dx: f32, dy: f32) -> char {
    if dx.abs() < 0.35 && dy.abs() > 0.5 {
        return '|';
    }
    if dy.abs() < 0.35 && dx.abs() > 0.5 {
        return '-';
    }
    let a = dy.atan2(dx);
    if a > -PI * 0.35 && a < PI * 0.35 {
        '-'
    } else if a.abs() > PI * 0.65 {
        '-'
    } else if a > 0.0 {
        '/'
    } else {
        '\\'
    }
}

fn plot(
    buf: &mut [char],
    zbuf: &mut [f32],
    w: i32,
    h: i32,
    x: i32,
    y: i32,
    z: f32,
    c: char,
) {
    if x < 0 || y < 0 || x >= w || y >= h {
        return;
    }
    let i = (y * w + x) as usize;
    if z >= zbuf[i] {
        zbuf[i] = z;
        buf[i] = c;
    }
}

fn line_3d(
    buf: &mut [char],
    zbuf: &mut [f32],
    w: i32,
    h: i32,
    p0: (f32, f32, f32),
    p1: (f32, f32, f32),
) {
    let (x0, y0, z0) = p0;
    let (x1, y1, z1) = p1;
    let dx = x1 - x0;
    let dy = y1 - y0;
    let len = (dx * dx + dy * dy).sqrt();
    let steps = (len.ceil() as i32).max(2);

    for i in 0..steps {
        let ta = i as f32 / (steps - 1) as f32;
        let tb = (i + 1).min(steps - 1) as f32 / (steps - 1) as f32;
        let x = x0 + dx * ta;
        let y = y0 + dy * ta;
        let z = z0 + (z1 - z0) * ta;
        let xb = x0 + dx * tb;
        let yb = y0 + dy * tb;
        let c = edge_char(xb - x, yb - y);
        plot(buf, zbuf, w, h, x.round() as i32, y.round() as i32, z, c);
    }
}

pub fn render(
    f: &AudioFeatures,
    w: u16,
    h: u16,
    t: f32,
    sensitivity: f32,
    charset: &str,
) -> String {
    let w = w.max(24) as i32;
    let h = h.max(12) as i32;
    let n = (w * h) as usize;
    let mut buf = vec![' '; n];
    let mut zbuf = vec![-1e20f32; n];

    let accent: Vec<char> = charset.chars().collect();
    let hub = accent.last().copied().unwrap_or('@');

    let scale = (w.min(h) as f32) * 0.26 * sensitivity.clamp(0.4, 2.2);
    let cam = 2.8_f32;

    let mut scr = [[0f32; 3]; 8];
    for (i, v) in VERTS.iter().enumerate() {
        let q = transform_vertex(*v, f, t, sensitivity);
        let (sx, sy, sz) = project(q, w, h, cam, scale);
        scr[i] = [sx, sy, sz];
    }

    for &(a, b) in &EDGES {
        line_3d(
            &mut buf,
            &mut zbuf,
            w,
            h,
            (scr[a][0], scr[a][1], scr[a][2]),
            (scr[b][0], scr[b][1], scr[b][2]),
        );
    }

    for p in &scr {
        let xi = p[0].round() as i32;
        let yi = p[1].round() as i32;
        let zi = p[2];
        plot(&mut buf, &mut zbuf, w, h, xi, yi, zi + 0.05, hub);
    }

    let mut lines = Vec::with_capacity(h as usize);
    for y in 0..h {
        let row: String = (0..w).map(|x| buf[(y * w + x) as usize]).collect();
        lines.push(row);
    }
    lines.join("\n")
}
