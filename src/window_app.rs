//! Отдельное окно (eframe + egui), тот же захват и сцены.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Instant;

use crossbeam_channel::Receiver;
use eframe::egui::{self, Align2, FontId, Rect, Sense, Vec2, ViewportCommand};

use crate::analysis::{anim_time, FeatureAnalyzer};
use crate::audio::{spawn_loopback_capture, PcmF32};
use crate::music_colors::{color32_background_cell, color32_for_cell, ColorScheme};
use crate::visual::{render_scene, SceneId, DEFAULT_CHARSET};
use crate::AudioFeatures;

const PALETTES: [&str; 3] = [
    DEFAULT_CHARSET,
    " `'.,~-+xX#@",
    " >})|{/\\#@",
];

const CHAR_W: f32 = 7.6;
const LINE_H: f32 = 13.0;

pub struct ReactorWindow {
    rx: Receiver<PcmF32>,
    running: Arc<AtomicBool>,
    capture: Option<JoinHandle<()>>,
    analyzer: FeatureAnalyzer,
    feat: AudioFeatures,
    scene: SceneId,
    sensitivity: f32,
    charset: String,
    palette_idx: usize,
    color_scheme: ColorScheme,
    start: Instant,
    fullscreen: bool,
}

impl Drop for ReactorWindow {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(h) = self.capture.take() {
            let _ = h.join();
        }
    }
}

impl ReactorWindow {
    fn new(
        rx: Receiver<PcmF32>,
        running: Arc<AtomicBool>,
        capture: JoinHandle<()>,
    ) -> Self {
        Self {
            rx,
            running,
            capture: Some(capture),
            analyzer: FeatureAnalyzer::new(),
            feat: AudioFeatures::default(),
            scene: SceneId::default(),
            sensitivity: 1.0,
            charset: PALETTES[0].to_string(),
            palette_idx: 0,
            color_scheme: ColorScheme::default(),
            start: Instant::now(),
            fullscreen: false,
        }
    }

    fn handle_input(&mut self, ctx: &egui::Context) {
        let mut close = false;
        let mut toggle_fs = false;
        let mut scene = None::<u8>;
        let mut sens_up = false;
        let mut sens_down = false;
        let mut reset_an = false;
        let mut pal_prev = false;
        let mut pal_next = false;
        let mut color_cycle = false;

        ctx.input(|i| {
            close |= i.key_pressed(egui::Key::Escape) || i.key_pressed(egui::Key::Q);
            toggle_fs |= i.key_pressed(egui::Key::F);
            if i.key_pressed(egui::Key::Num1) {
                scene = Some(1);
            } else if i.key_pressed(egui::Key::Num2) {
                scene = Some(2);
            } else if i.key_pressed(egui::Key::Num3) {
                scene = Some(3);
            } else if i.key_pressed(egui::Key::Num4) {
                scene = Some(4);
            } else if i.key_pressed(egui::Key::Num5) {
                scene = Some(5);
            }
            sens_up |= i.key_pressed(egui::Key::Equals) || i.key_pressed(egui::Key::Plus);
            sens_down |= i.key_pressed(egui::Key::Minus);
            reset_an |= i.key_pressed(egui::Key::R);
            pal_prev |= i.key_pressed(egui::Key::OpenBracket);
            pal_next |= i.key_pressed(egui::Key::CloseBracket);
            color_cycle |= i.key_pressed(egui::Key::C);
        });

        if close {
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }
        if toggle_fs {
            self.fullscreen = !self.fullscreen;
            ctx.send_viewport_cmd(ViewportCommand::Fullscreen(self.fullscreen));
        }
        if let Some(n) = scene {
            self.scene = SceneId::from_index(n);
        }
        if sens_up {
            self.sensitivity = (self.sensitivity * 1.12).min(4.0);
        }
        if sens_down {
            self.sensitivity = (self.sensitivity / 1.12).max(0.15);
        }
        if reset_an {
            self.analyzer = FeatureAnalyzer::new();
        }
        if pal_prev {
            self.palette_idx = (self.palette_idx + PALETTES.len() - 1) % PALETTES.len();
            self.charset = PALETTES[self.palette_idx].to_string();
        }
        if pal_next {
            self.palette_idx = (self.palette_idx + 1) % PALETTES.len();
            self.charset = PALETTES[self.palette_idx].to_string();
        }
        if color_cycle {
            self.color_scheme = self.color_scheme.cycle();
        }
    }
}

impl eframe::App for ReactorWindow {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(chunk) = self.rx.try_recv() {
            self.feat = self.analyzer.push_pcm(&chunk);
        }

        self.handle_input(ctx);

        egui::TopBottomPanel::bottom("hints")
            .exact_height(24.0)
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new(
                        "1–5 сцены  C цвета  [ ] символы  +/- чувств.  F экран  R сброс  Esc/Q выход",
                    )
                    .small()
                    .color(egui::Color32::from_gray(160)),
                );
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                let t = anim_time(self.start);
                let avail = ui.available_size();
                let cols = ((avail.x / CHAR_W).floor().max(16.0)) as u16;
                let rows = ((avail.y / LINE_H).floor().max(8.0)) as u16;

                let art = render_scene(
                    self.scene,
                    &self.feat,
                    cols,
                    rows,
                    t,
                    self.sensitivity,
                    &self.charset,
                );

                let n_rows = art.lines().count().max(1) as u32;
                let font = FontId::monospace(12.0);
                let (rect, _resp) = ui.allocate_exact_size(
                    Vec2::new(cols as f32 * CHAR_W, rows as f32 * LINE_H),
                    Sense::hover(),
                );
                let painter = ui.painter_at(rect);

                for row in 0..rows {
                    for col in 0..cols {
                        let x = rect.min.x + col as f32 * CHAR_W;
                        let y = rect.min.y + row as f32 * LINE_H;
                        painter.rect_filled(
                            Rect::from_min_size(egui::pos2(x, y), Vec2::new(CHAR_W, LINE_H)),
                            0.0,
                            color32_background_cell(
                                &self.feat,
                                t,
                                col as u32,
                                row as u32,
                                self.color_scheme,
                            ),
                        );
                    }
                }

                for (row, line) in art.lines().enumerate() {
                    for (col, ch) in line.chars().enumerate() {
                        let color = color32_for_cell(
                            &self.feat,
                            t,
                            col as u32,
                            row as u32,
                            n_rows,
                            ch,
                            self.color_scheme,
                        );
                        let pos = rect.min + Vec2::new(col as f32 * CHAR_W, row as f32 * LINE_H);
                        painter.text(
                            pos,
                            Align2::LEFT_TOP,
                            ch.to_string(),
                            font.clone(),
                            color,
                        );
                    }
                }
            });

        ctx.request_repaint();
    }
}

#[cfg(windows)]
pub fn run_window() -> eframe::Result<()> {
    let running = Arc::new(AtomicBool::new(true));
    let (tx, rx) = crossbeam_channel::bounded::<PcmF32>(8);
    let capture = spawn_loopback_capture(tx, running.clone());

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([980.0, 680.0])
            .with_title("ASCII Music Reactor"),
        ..Default::default()
    };

    eframe::run_native(
        "ASCII Music Reactor",
        options,
        Box::new(move |_cc| {
            Ok(Box::new(ReactorWindow::new(rx, running, capture)) as Box<dyn eframe::App>)
        }),
    )
}
