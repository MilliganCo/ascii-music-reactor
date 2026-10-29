#[cfg(windows)]
fn main() {
    let window_mode = std::env::args()
        .any(|a| a == "--window" || a == "-w" || a == "/window");

    if window_mode {
        if let Err(e) = ascii_music_reactor::run_window() {
            eprintln!("Ошибка графического окна: {e}");
            std::process::exit(1);
        }
        return;
    }

    if let Err(e) = windows_run_tui() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

#[cfg(not(windows))]
fn main() {
    let window_mode = std::env::args()
        .any(|a| a == "--window" || a == "-w" || a == "/window");
    if window_mode {
        eprintln!("Режим --window поддерживается только на Windows (WASAPI).");
        return;
    }
    eprintln!("ASCII Music Reactor: WASAPI loopback доступен только в Windows.");
}

#[cfg(windows)]
fn windows_run_tui() -> std::io::Result<()> {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use crossbeam_channel::bounded;
    use crossterm::event::{self, Event, KeyCode, KeyEventKind};
    use ratatui::layout::{Constraint, Layout, Rect};
    use ratatui::style::{Color, Style};
    use ratatui::text::{Line, Span, Text};
    use ratatui::widgets::{Block, Borders, Paragraph};

    use ascii_music_reactor::analysis::{anim_time, FeatureAnalyzer};
    use ascii_music_reactor::audio::{spawn_loopback_capture, PcmF32};
    use ascii_music_reactor::music_colors::{ratatui_bg, ratatui_fg, ColorScheme};
    use ascii_music_reactor::visual::{render_scene, SceneId, DEFAULT_CHARSET};
    use ascii_music_reactor::AudioFeatures;

    const PALETTES: [&str; 3] = [
        DEFAULT_CHARSET,
        " `'.,~-+xX#@",
        " >})|{/\\#@",
    ];

    let running = Arc::new(AtomicBool::new(true));
    let (tx, rx) = bounded::<PcmF32>(8);
    let _capture_handle = spawn_loopback_capture(tx, running.clone());

    let mut terminal = ratatui::try_init()?;
    let mut analyzer = FeatureAnalyzer::new();
    let mut scene = SceneId::default();
    let mut sensitivity = 1.0_f32;
    let mut palette_idx = 0usize;
    let mut charset = PALETTES[0].to_string();
    let mut color_scheme = ColorScheme::default();
    let mut fullscreen = false;
    let start = Instant::now();
    let mut feat = AudioFeatures::default();

    let result = 'app: loop {
        while let Ok(chunk) = rx.try_recv() {
            feat = analyzer.push_pcm(&chunk);
        }

        while event::poll(Duration::from_millis(0))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => break 'app Ok(()),
                        KeyCode::Char('1') => scene = SceneId::from_index(1),
                        KeyCode::Char('2') => scene = SceneId::from_index(2),
                        KeyCode::Char('3') => scene = SceneId::from_index(3),
                        KeyCode::Char('4') => scene = SceneId::from_index(4),
                        KeyCode::Char('5') => scene = SceneId::from_index(5),
                        KeyCode::Char('c') | KeyCode::Char('C') => {
                            color_scheme = color_scheme.cycle();
                        }
                        KeyCode::Char('+') | KeyCode::Char('=') => {
                            sensitivity = (sensitivity * 1.12).min(4.0);
                        }
                        KeyCode::Char('-') | KeyCode::Char('_') => {
                            sensitivity = (sensitivity / 1.12).max(0.15);
                        }
                        KeyCode::Char('f') => fullscreen = !fullscreen,
                        KeyCode::Char('r') => {
                            analyzer = FeatureAnalyzer::new();
                        }
                        KeyCode::Char(']') => {
                            palette_idx = (palette_idx + 1) % PALETTES.len();
                            charset = PALETTES[palette_idx].to_string();
                        }
                        KeyCode::Char('[') => {
                            palette_idx = (palette_idx + PALETTES.len() - 1) % PALETTES.len();
                            charset = PALETTES[palette_idx].to_string();
                        }
                        _ => {}
                    }
                }
            }
        }

        terminal.draw(|frame| {
            let area = frame.area();
            let (main_rect, bar_rect) = if fullscreen {
                (area, Rect::default())
            } else {
                let p = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(area);
                (p[0], p[1])
            };

            let inner_w = main_rect.width.saturating_sub(2);
            let inner_h = main_rect.height.saturating_sub(2);
            let t = anim_time(start);
            let art = render_scene(scene, &feat, inner_w, inner_h, t, sensitivity, &charset);

            let n_rows = art.lines().count().max(1) as u32;
            let mut lines: Vec<Line> = Vec::new();
            for (row, line_str) in art.lines().enumerate() {
                let mut spans: Vec<Span> = Vec::new();
                for (col, ch) in line_str.chars().enumerate() {
                    let fg = ratatui_fg(
                        &feat,
                        t,
                        col as u32,
                        row as u32,
                        n_rows,
                        ch,
                        color_scheme,
                    );
                    let bg = ratatui_bg(&feat, t, col as u32, row as u32, color_scheme);
                    spans.push(Span::styled(
                        ch.to_string(),
                        Style::default().fg(fg).bg(bg),
                    ));
                }
                lines.push(Line::from(spans));
            }
            let text = Text::from(lines);

            let block = Block::default()
                .title(" ASCII Music Reactor ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .style(Style::default().bg(Color::Rgb(6, 8, 14)));

            let p = Paragraph::new(text).block(block);
            frame.render_widget(p, main_rect);

            if !fullscreen && bar_rect.height > 0 {
                let hint = Paragraph::new(
                    "1–5 сцены  C цвета  [ ] символы  +/- чувств.  f экран  r сброс  q выход  |  --window",
                )
                .style(Style::default().fg(Color::DarkGray));
                frame.render_widget(hint, bar_rect);
            }
        })?;

        std::thread::sleep(Duration::from_millis(16));
    };

    running.store(false, Ordering::Relaxed);
    let _ = _capture_handle.join();
    ratatui::restore();
    result
}
