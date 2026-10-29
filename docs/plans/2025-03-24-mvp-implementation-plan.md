# ASCII Music Reactor MVP — план реализации

> **Цель:** Windows desktop: захват системного звука (WASAPI loopback), анализ (FFT, полосы), ASCII-визуализация (3 сцены), hotkeys из DEVELOPMENT.md.

**Архитектура:** три слоя — `audio` (захват), `analysis` (метрики + сглаживание), `visual` (сетка + сцены). Бинарь на Rust; рендер через ratatui (MVP), затем при необходимости отдельное окно.

**Стек:** Rust, `windows` (WASAPI), `realfft`, `ratatui`, `crossterm`.

---

### Task 1: Каркас Cargo и модули

**Файлы:** `Cargo.toml`, `src/main.rs`, `src/lib.rs`, `src/audio/mod.rs`, `src/analysis/mod.rs`, `src/visual/mod.rs`

**Шаги:** `cargo init --name ascii_music_reactor`; разнести модули; `cargo check`.

---

### Task 2: WASAPI loopback → PCM f32

**Файлы:** `src/audio/wasapi_loopback.rs`

**Шаги:** CoInitialize, enumerator, default render endpoint, `IAudioClient` + `AUDCLNT_STREAMFLAGS_LOOPBACK`, `GetMixFormat`, конвертация в моно f32, кольцевой/канал в поток анализа.

**Проверка:** при воспроизведении звука RMS > 0, без паники.

---

### Task 3: `AudioFeatures` и FFT-полосы

**Файлы:** `src/features.rs`, `src/analysis/fft_bands.rs`

**Шаги:** структура полей из DEVELOPMENT.md; realfft окно 1024/2048; bass/mid/high; fast/smooth сглаживание.

---

### Task 4: Базовый TUI и Ocean Grid

**Файлы:** `src/visual/ocean_grid.rs`, `src/main.rs`

**Шаги:** ratatui цикл 30–60 FPS; первая сцена по спецификации; клавиши 1/2/3 заглушки для сцен 2–3.

---

### Task 5: Сцены Pulse Cathedral и Tunnel Drive

**Файлы:** `src/visual/pulse_cathedral.rs`, `src/visual/tunnel_drive.rs`

---

### Task 6: UX MVP

**Файлы:** `src/main.rs`, конфиг чувствительности

**Шаги:** `+/-`, `f` fullscreen, `r` reset, `q` quit; выбор набора символов.

---

**Команды верификации:** `cargo build --release`, `cargo clippy`, ручной запуск под музыкой на Windows.
