/// Блок PCM в float32, каналы чередуются (L, R, …).
#[derive(Clone, Debug)]
pub struct PcmF32 {
    pub sample_rate: u32,
    pub channels: usize,
    pub samples: Vec<f32>,
}
