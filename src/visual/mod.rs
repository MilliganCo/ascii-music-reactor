mod ocean_grid;
mod pulse_cathedral;
mod pulse_form_3d;
mod signal_rain;
mod tunnel_drive;

use crate::features::AudioFeatures;

/// Сцены: 1–3 MVP, 4 — Signal Rain, 5 — пульсирующий 3D-объект.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum SceneId {
    #[default]
    Ocean,
    Cathedral,
    Tunnel,
    SignalRain,
    PulseForm3d,
}

impl SceneId {
    pub fn from_index(i: u8) -> Self {
        match i {
            1 => SceneId::Ocean,
            2 => SceneId::Cathedral,
            3 => SceneId::Tunnel,
            4 => SceneId::SignalRain,
            5 => SceneId::PulseForm3d,
            _ => SceneId::Ocean,
        }
    }
}

/// Сжатие шкалы чувствительности: меньше «каши» на высоких значениях.
#[inline]
pub fn effective_sensitivity(sensitivity: f32) -> f32 {
    (sensitivity.powf(0.74) * 0.78 + 0.24).clamp(0.38, 2.05)
}

pub fn render_scene(
    scene: SceneId,
    features: &AudioFeatures,
    w: u16,
    h: u16,
    t: f32,
    sensitivity: f32,
    chars: &str,
) -> String {
    let sensitivity = effective_sensitivity(sensitivity);
    match scene {
        SceneId::Ocean => ocean_grid::render(features, w, h, t, sensitivity, chars),
        SceneId::Cathedral => pulse_cathedral::render(features, w, h, t, sensitivity, chars),
        SceneId::Tunnel => tunnel_drive::render(features, w, h, t, sensitivity, chars),
        SceneId::SignalRain => signal_rain::render(features, w, h, t, sensitivity, chars),
        SceneId::PulseForm3d => pulse_form_3d::render(features, w, h, t, sensitivity, chars),
    }
}

pub use ocean_grid::DEFAULT_CHARSET;
