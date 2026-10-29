use std::sync::Arc;

use realfft::num_complex::Complex;
use realfft::{RealFftPlanner, RealToComplex};

use crate::audio::PcmF32;
use crate::features::AudioFeatures;

const FFT_SIZE: usize = 2048;
const SMOOTH: f32 = 0.91;
const FAST_DECAY: f32 = 0.42;
const MID_HIGH_SMOOTH: f32 = 0.92;
const CENTROID_SMOOTH: f32 = 0.88;

pub struct FeatureAnalyzer {
    fft: Arc<dyn RealToComplex<f32>>,
    spectrum: Vec<Complex<f32>>,
    ring: Vec<f32>,
    sample_rate: u32,
    prev_mag_sum: f32,
    beat_hold: f32,
    last_smooth: AudioFeatures,
}

impl FeatureAnalyzer {
    pub fn new() -> Self {
        let mut planner = RealFftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(FFT_SIZE);
        let spectrum = fft.make_output_vec();
        Self {
            fft,
            spectrum,
            ring: Vec::with_capacity(FFT_SIZE * 2),
            sample_rate: 48_000,
            prev_mag_sum: 0.0,
            beat_hold: 0.0,
            last_smooth: AudioFeatures::default(),
        }
    }

    /// Накопить PCM и при достаточном количестве сэмплов обновить метрики.
    pub fn push_pcm(&mut self, chunk: &PcmF32) -> AudioFeatures {
        self.sample_rate = chunk.sample_rate.max(1);
        self.beat_hold *= 0.94;

        let mono: Vec<f32> = if chunk.channels >= 2 {
            chunk
                .samples
                .chunks_exact(chunk.channels)
                .map(|c| (c[0] + c.get(1).copied().unwrap_or(c[0])) * 0.5)
                .collect()
        } else if chunk.channels == 1 {
            chunk.samples.clone()
        } else {
            return self.last_smooth.clone();
        };

        self.ring.extend(mono);

        let mut result = self.last_smooth.clone();
        while self.ring.len() >= FFT_SIZE {
            let frame: Vec<f32> = self.ring.drain(..FFT_SIZE).collect();
            let mut input = frame;
            if self.fft.process(&mut input, &mut self.spectrum).is_err() {
                continue;
            }

            let sr = self.sample_rate as f32;
            let bin_hz = sr / FFT_SIZE as f32;

            let mut bass = 0.0f32;
            let mut mid = 0.0f32;
            let mut high = 0.0f32;
            let mut mag_sum = 0.0f32;
            let mut weighted_hz = 0.0f32;

            for (i, c) in self.spectrum.iter().enumerate() {
                let hz = i as f32 * bin_hz;
                let m = c.norm();
                mag_sum += m;
                weighted_hz += hz * m;
                if hz < 250.0 {
                    bass += m;
                } else if hz < 2000.0 {
                    mid += m;
                } else {
                    high += m;
                }
            }

            let centroid = if mag_sum > 1e-6 {
                weighted_hz / mag_sum / sr
            } else {
                0.0
            };

            let rms = (input.iter().map(|&x| x * x).sum::<f32>() / FFT_SIZE as f32).sqrt();
            let volume = (rms * 4.0).clamp(0.0, 1.0);

            let bass_n = (bass * 0.12).tanh();
            let mid_n = (mid * 0.09).tanh();
            let high_n = (high * 0.075).tanh();

            let d = (mag_sum - self.prev_mag_sum).max(0.0);
            let transient = (d * 1.85).clamp(0.0, 1.0).powf(0.82);
            self.prev_mag_sum = lerp(mag_sum, self.prev_mag_sum, 0.78);

            self.beat_hold = self.beat_hold.max(transient);
            let beat_pulse = self.beat_hold.clamp(0.0, 1.0);

            let stereo_width = stereo_width_metric(&chunk.samples, chunk.channels);

            let mid_sm = lerp(mid_n, self.last_smooth.mid_energy, MID_HIGH_SMOOTH);
            let high_sm = lerp(high_n, self.last_smooth.high_energy, MID_HIGH_SMOOTH);
            let cent_sm = lerp(centroid, self.last_smooth.spectral_centroid, CENTROID_SMOOTH);

            let mut f = AudioFeatures {
                volume,
                bass_energy: bass_n,
                mid_energy: mid_sm,
                high_energy: high_sm,
                transient_strength: lerp(transient, self.last_smooth.transient_strength, 0.72),
                beat_pulse,
                spectral_centroid: cent_sm,
                stereo_width: lerp(stereo_width, self.last_smooth.stereo_width, 0.85),
                volume_fast: 0.0,
                bass_fast: 0.0,
                volume_smooth: 0.0,
                bass_smooth: 0.0,
            };

            f.volume_fast = f.volume.max(self.last_smooth.volume_fast * FAST_DECAY);
            f.bass_fast = f.bass_energy.max(self.last_smooth.bass_fast * FAST_DECAY);

            f.volume_smooth = lerp(f.volume, self.last_smooth.volume_smooth, SMOOTH);
            f.bass_smooth = lerp(f.bass_energy, self.last_smooth.bass_smooth, SMOOTH);

            self.last_smooth = f.clone();
            result = f;
        }

        result
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a * (1.0 - t) + b * t
}

fn stereo_width_metric(samples: &[f32], channels: usize) -> f32 {
    if channels < 2 || samples.len() < 2 {
        return 0.0;
    }
    let mut acc = 0.0f32;
    let mut n = 0u32;
    for w in samples.chunks_exact(channels) {
        let l = w[0];
        let r = w[1];
        let m = (l + r).abs() * 0.5 + 1e-6;
        acc += (l - r).abs() / m;
        n += 1;
    }
    if n == 0 {
        return 0.0;
    }
    ((acc / n as f32) * 0.5).tanh()
}

impl Default for FeatureAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// Время для фазовых анимаций (секунды), монотонно от старта приложения.
pub fn anim_time(start: std::time::Instant) -> f32 {
    start.elapsed().as_secs_f32()
}

/// Небольшой детерминированный «шум», завязанный на частоты и координаты.
pub fn music_noise(x: u32, y: u32, t: f32, centroid: f32) -> f32 {
    let s = (x as f32 * 12.9898 + y as f32 * 78.233 + t * 43.0 + centroid * 100.0).sin();
    (s.fract() * 2.0 - 1.0).abs()
}
