//! ASCII Music Reactor — захват системного звука, анализ, ASCII-сцены.

pub mod analysis;
pub mod audio;
pub mod features;
pub mod music_colors;
pub mod visual;

#[cfg(windows)]
pub mod window_app;

pub use features::AudioFeatures;
pub use music_colors::ColorScheme;

#[cfg(windows)]
pub use window_app::run_window;
