mod queue;
mod pcm;

#[cfg(windows)]
mod wasapi_loopback;

pub use pcm::PcmF32;

#[cfg(windows)]
pub use wasapi_loopback::spawn_loopback_capture;

#[cfg(not(windows))]
pub fn spawn_loopback_capture(
    _tx: crossbeam_channel::Sender<PcmF32>,
    _running: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> Result<std::thread::JoinHandle<()>, String> {
    Err("WASAPI loopback поддерживается только на Windows".into())
}
