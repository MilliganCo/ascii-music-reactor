/// Нормализованные метрики для визуального движка (0..1, кроме centroid в bin-единицах).
#[derive(Clone, Debug, Default)]
pub struct AudioFeatures {
    pub volume: f32,
    pub bass_energy: f32,
    pub mid_energy: f32,
    pub high_energy: f32,
    pub transient_strength: f32,
    pub beat_pulse: f32,
    pub spectral_centroid: f32,
    pub stereo_width: f32,
    /// Резкие значения для вспышек
    pub volume_fast: f32,
    pub bass_fast: f32,
    /// Сглаженные для геометрии
    pub volume_smooth: f32,
    pub bass_smooth: f32,
}
