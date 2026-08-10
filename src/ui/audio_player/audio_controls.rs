use super::audio_state::{AudioState, LoopMode};
use crate::localized;
use crate::ui::waveform::{WaveformAction, WaveformOptions, WaveformWidget};
use egui::{Align, Color32, CornerRadius, Frame, Layout, RichText, Ui, Vec2, vec2};
use egui_phosphor::regular;
use std::sync::{Arc, Mutex};

/// Accent tokens for the player strip (cool blue + amber loop — matches dark tool chrome).
const ACCENT: Color32 = Color32::from_rgb(100, 150, 255);
const ACCENT_SOFT: Color32 = Color32::from_rgb(80, 120, 200);
const PLAY_ACTIVE: Color32 = Color32::from_rgb(255, 200, 100);
const PLAY_IDLE: Color32 = Color32::from_rgb(100, 220, 150);
const STOP_COLOR: Color32 = Color32::from_rgb(220, 100, 100);
const LOOP_AMBER: Color32 = Color32::from_rgb(232, 176, 72);

/// Audio player controls component
pub struct AudioControls {
    /// Reference to the audio state
    audio_state: Arc<Mutex<AudioState>>,
}

impl AudioControls {
    /// Create a new audio controls component
    pub fn new(audio_state: Arc<Mutex<AudioState>>) -> Self {
        Self { audio_state }
    }

    /// Render the audio controls UI
    pub fn render(&mut self, ui: &mut Ui) {
        // Clone state snapshot to avoid holding the mutex during layout.
        // Also poll async waveform so bars appear as soon as the worker finishes.
        let state_copy = {
            let mut state = self.audio_state.lock().unwrap();
            let wave_updated = state.poll_waveform();
            if state.is_playing || state.waveform_loading || wave_updated {
                ui.ctx().request_repaint();
            }
            state.clone()
        };

        let has_audio = state_copy.current_audio.is_some();
        let full_width = ui.available_width();

        Frame::new()
            .inner_margin(egui::Margin::symmetric(12, 10))
            .fill(ui.visuals().window_fill)
            .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
            .corner_radius(CornerRadius::same(10))
            .show(ui, |ui| {
                ui.set_min_width(full_width);
                ui.set_max_width(full_width);

                // Row 1: track meta (left, fixed) + volume (right, fixed)
                ui.horizontal(|ui| {
                    ui.set_min_height(40.0);

                    // Track info — constrained width, text clipped
                    let meta_width = (full_width * 0.42).clamp(160.0, 360.0);
                    ui.allocate_ui_with_layout(
                        vec2(meta_width, 40.0),
                        Layout::left_to_right(Align::Center),
                        |ui| {
                            ui.set_max_width(meta_width);
                            self.render_track_meta(ui, &state_copy, meta_width);
                        },
                    );

                    // Spacer pushes volume to the right without stretching meta
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.set_max_width((full_width * 0.28).clamp(120.0, 220.0));
                        self.render_volume_controls(ui, &state_copy);
                    });
                });

                ui.add_space(6.0);

                // Row 2: sound wave (full width, clipped)
                // Show loop region when track loop is enabled (full file or A-B)
                let duration_for_wave = state_copy
                    .total_duration
                    .max(
                        state_copy
                            .waveform
                            .as_ref()
                            .map(|w| w.duration_secs)
                            .unwrap_or(0.0),
                    );
                let (show_loop, loop_start, loop_end) =
                    if let Some((a, b)) = state_copy.display_loop_range() {
                        (true, Some(a), Some(b.max(a)))
                    } else {
                        (false, None, None)
                    };
                let wave_opts = WaveformOptions {
                    height: 64.0,
                    playhead_secs: if has_audio {
                        Some(state_copy.current_position)
                    } else {
                        None
                    },
                    loop_start_secs: loop_start,
                    loop_end_secs: loop_end,
                    show_loop,
                    interactive_loop: false, // loop points edited in replace modal
                    interactive_seek: has_audio,
                    duration_override: if duration_for_wave > 0.0 {
                        Some(duration_for_wave)
                    } else {
                        None
                    },
                    loading: state_copy.waveform_loading && state_copy.waveform.is_none(),
                };

                let (_resp, action) = WaveformWidget::show(
                    ui,
                    "player_waveform",
                    state_copy.waveform.as_ref(),
                    &wave_opts,
                );

                match action {
                    // Drag scrub: move playhead in UI only (or rare throttled seek). Never seek every frame.
                    WaveformAction::Scrub(t) if has_audio => {
                        let mut state = self.audio_state.lock().unwrap();
                        state.scrub_to(t);
                    }
                    // Click / drag-release: single backend seek
                    WaveformAction::Seek(t) if has_audio => {
                        let mut state = self.audio_state.lock().unwrap();
                        state.commit_seek(t);
                    }
                    _ => {}
                }

                ui.add_space(4.0);

                // Row 3: time + progress + duration (fixed trailing so nothing overflows)
                ui.horizontal(|ui| {
                    ui.set_max_width(ui.available_width());
                    let time_color = ui.visuals().weak_text_color();
                    let full_w = ui.available_width();

                    // Fixed leading: current time
                    ui.label(
                        RichText::new(state_copy.format_position())
                            .monospace()
                            .size(11.0)
                            .color(time_color),
                    );

                    // Fixed trailing budget: duration label (+ optional compact loop chip)
                    let duration_w = 44.0_f32;
                    let chip_w = if show_loop { 96.0_f32 } else { 0.0 };
                    let gap = if show_loop { 8.0_f32 } else { 0.0 };
                    let trailing = duration_w + chip_w + gap + 12.0;
                    let slider_w = (full_w - 48.0 - trailing).max(48.0);
                    ui.spacing_mut().slider_width = slider_w;

                    let mut progress = state_copy.progress();
                    let slider = ui.add(
                        egui::Slider::new(&mut progress, 0.0..=1.0)
                            .show_value(false)
                            .trailing_fill(true),
                    );

                    if slider.changed() && has_audio {
                        let mut state = self.audio_state.lock().unwrap();
                        state.is_seeking = true;
                        state.current_position = progress * state.total_duration;
                    }
                    if slider.drag_stopped() && has_audio {
                        let mut state = self.audio_state.lock().unwrap();
                        state.is_seeking = false;
                        let new_position = progress * state.total_duration;
                        state.set_position(new_position);
                    }

                    ui.label(
                        RichText::new(state_copy.format_duration())
                            .monospace()
                            .size(11.0)
                            .color(time_color),
                    );

                    if show_loop {
                        ui.add_space(6.0);
                        // Hard-cap chip width + clip so "00:10–02:41" never overflows the panel
                        ui.allocate_ui_with_layout(
                            vec2(chip_w, 22.0),
                            Layout::left_to_right(Align::Center),
                            |ui| {
                                ui.set_max_width(chip_w);
                                ui.set_clip_rect(ui.max_rect());
                                self.render_loop_chip(ui, &state_copy, chip_w);
                            },
                        );
                    }
                });

                ui.add_space(6.0);

                // Row 4: transport — fixed height, equal hit boxes, forced vertical center
                self.render_transport_row(ui, &state_copy, has_audio);
            });
    }

    fn render_track_meta(&self, ui: &mut Ui, state: &AudioState, max_width: f32) {
        if let Some(audio) = &state.current_audio {
            let icon = match audio.file_type.as_str() {
                "OPUS Audio" => regular::MUSIC_NOTE,
                "IDSP Audio" => regular::HEADPHONES,
                _ => regular::FILE_AUDIO,
            };
            let type_color = match audio.file_type.as_str() {
                "OPUS Audio" => Color32::from_rgb(100, 200, 100),
                "IDSP Audio" => Color32::from_rgb(100, 150, 255),
                _ => Color32::from_rgb(200, 150, 100),
            };

            ui.label(RichText::new(icon.to_string()).size(22.0).color(type_color));
            ui.add_space(6.0);

            // Text column with hard clip so long names never overflow into volume
            let text_w = (max_width - 36.0).max(80.0);
            ui.allocate_ui_with_layout(
                vec2(text_w, 40.0),
                Layout::top_down(Align::LEFT),
                |ui| {
                    ui.set_max_width(text_w);
                    ui.set_clip_rect(ui.max_rect());

                    let name = elide_middle(&audio.name, 42);
                    ui.label(
                        RichText::new(name)
                            .color(ui.visuals().strong_text_color())
                            .size(14.0)
                            .strong(),
                    )
                    .on_hover_text(&audio.name);

                    let subtitle = format!("{}  ·  {}", audio.file_type, state.format_duration());
                    ui.label(
                        RichText::new(subtitle)
                            .color(type_color)
                            .size(11.0),
                    );
                },
            );
        } else {
            ui.label(
                RichText::new(regular::MUSIC_NOTES_PLUS.to_string())
                    .size(22.0)
                    .color(ui.visuals().weak_text_color()),
            );
            ui.add_space(6.0);
            ui.label(
                RichText::new(localized::no_track_selected())
                    .color(ui.visuals().weak_text_color())
                    .italics()
                    .size(13.0),
            );
        }
    }

    fn render_loop_chip(&self, ui: &mut Ui, state: &AudioState, max_width: f32) {
        let start = state.loop_start.unwrap_or(0.0);
        let end = state
            .loop_end
            .unwrap_or(state.total_duration)
            .max(start);
        // Compact: "A–B" times only (icon via phosphor may render as tofu in some fonts)
        let label = format!("{}–{}", format_mmss(start), format_mmss(end));
        let full_tip = localized::loop_parenthetical_range(
            &format!("{start:.2}s"),
            &format!("{end:.2}s"),
        );

        Frame::new()
            .inner_margin(egui::Margin::symmetric(6, 2))
            .corner_radius(CornerRadius::same(8))
            .fill(Color32::from_rgba_unmultiplied(
                LOOP_AMBER.r(),
                LOOP_AMBER.g(),
                LOOP_AMBER.b(),
                28,
            ))
            .stroke(egui::Stroke::new(
                1.0,
                Color32::from_rgba_unmultiplied(
                    LOOP_AMBER.r(),
                    LOOP_AMBER.g(),
                    LOOP_AMBER.b(),
                    120,
                ),
            ))
            .show(ui, |ui| {
                ui.set_max_width((max_width - 4.0).max(40.0));
                ui.label(
                    RichText::new(label)
                        .size(10.5)
                        .color(LOOP_AMBER)
                        .monospace(),
                );
            })
            .response
            .on_hover_text(full_tip);
    }

    fn render_transport_row(&mut self, ui: &mut Ui, state_copy: &AudioState, has_audio: bool) {
        const ROW_H: f32 = 44.0;
        const SIDE_HIT: f32 = 36.0;
        const PLAY_HIT: f32 = 44.0;
        const GAP: f32 = 12.0;

        let full_w = ui.available_width();
        let dim = ui.visuals().widgets.noninteractive.fg_stroke.color;

        // One non-wrapping strip: pad left so the cluster is centered, all icons share ROW_H.
        ui.allocate_ui_with_layout(
            vec2(full_w, ROW_H),
            Layout::left_to_right(Align::Center),
            |ui| {
                ui.set_min_height(ROW_H);
                ui.set_max_height(ROW_H);
                ui.spacing_mut().item_spacing = vec2(GAP, 0.0);

                let cluster_w = SIDE_HIT * 5.0 + PLAY_HIT + GAP * 5.0;
                let pad = ((full_w - cluster_w) * 0.5).max(0.0);
                ui.add_space(pad);

                let shuffle_color = if state_copy.shuffle { ACCENT } else { dim };
                if transport_icon_button(ui, regular::SHUFFLE, 18.0, shuffle_color, SIDE_HIT, ROW_H)
                    .on_hover_text(localized::shuffle_tooltip())
                    .clicked()
                {
                    self.audio_state.lock().unwrap().toggle_shuffle();
                }

                if transport_icon_button(ui, regular::SKIP_BACK, 20.0, dim, SIDE_HIT, ROW_H)
                    .on_hover_text(localized::previous_track_tooltip())
                    .clicked()
                {
                    self.audio_state.lock().unwrap().previous_track();
                }

                let play_icon = if state_copy.is_playing {
                    regular::PAUSE_CIRCLE
                } else {
                    regular::PLAY_CIRCLE
                };
                let play_color = if has_audio {
                    if state_copy.is_playing {
                        PLAY_ACTIVE
                    } else {
                        PLAY_IDLE
                    }
                } else {
                    dim.gamma_multiply(0.5)
                };
                if transport_icon_button(ui, play_icon, 32.0, play_color, PLAY_HIT, ROW_H)
                    .on_hover_text(if state_copy.is_playing {
                        localized::pause_tooltip()
                    } else {
                        localized::play_tooltip_player()
                    })
                    .clicked()
                    && has_audio
                {
                    self.audio_state.lock().unwrap().toggle_play();
                }

                if transport_icon_button(ui, regular::SKIP_FORWARD, 20.0, dim, SIDE_HIT, ROW_H)
                    .on_hover_text(localized::next_track_tooltip())
                    .clicked()
                {
                    self.audio_state.lock().unwrap().next_track();
                }

                let (loop_icon, loop_color, loop_tip) = match state_copy.loop_mode {
                    LoopMode::None => (regular::REPEAT, dim, localized::loop_off_tooltip()),
                    LoopMode::All => (regular::REPEAT, ACCENT, localized::loop_all_tooltip()),
                    LoopMode::Single => {
                        (regular::REPEAT_ONCE, ACCENT, localized::loop_one_tooltip())
                    }
                };
                if transport_icon_button(ui, loop_icon, 18.0, loop_color, SIDE_HIT, ROW_H)
                    .on_hover_text(loop_tip)
                    .clicked()
                {
                    self.audio_state.lock().unwrap().next_loop_mode();
                }

                if transport_icon_button(ui, regular::STOP_CIRCLE, 20.0, STOP_COLOR, SIDE_HIT, ROW_H)
                    .on_hover_text(localized::stop_playback_tooltip())
                    .clicked()
                    && has_audio
                {
                    self.audio_state.lock().unwrap().stop();
                }
            },
        );
    }

    fn render_volume_controls(&mut self, ui: &mut Ui, state_copy: &AudioState) {
        let mut volume = state_copy.volume * 100.0;

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;

            let (volume_icon, volume_color) = if state_copy.is_muted || state_copy.volume <= 0.0 {
                (regular::SPEAKER_X, Color32::from_gray(150))
            } else if state_copy.volume < 0.33 {
                (regular::SPEAKER_LOW, ACCENT_SOFT)
            } else {
                (regular::SPEAKER_HIGH, ACCENT_SOFT)
            };

            if icon_button(ui, volume_icon, 16.0, volume_color).clicked() {
                self.audio_state.lock().unwrap().toggle_mute();
            }

            // Fixed-width volume slider so layout stays stable
            ui.spacing_mut().slider_width = 96.0;
            let slider = ui.add(
                egui::Slider::new(&mut volume, 0.0..=100.0)
                    .show_value(false)
                    .trailing_fill(true),
            );
            if slider.changed() {
                self.audio_state.lock().unwrap().set_volume(volume / 100.0);
            }
        });
    }
}

fn icon_button(
    ui: &mut Ui,
    icon: &str,
    size: f32,
    color: Color32,
) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(icon.to_string()).size(size).color(color))
            .frame(false)
            .min_size(Vec2::splat(size + 8.0)),
    )
}

/// Transport control with a fixed hit box so mixed glyph sizes stay on one baseline.
fn transport_icon_button(
    ui: &mut Ui,
    icon: &str,
    icon_size: f32,
    color: Color32,
    hit_w: f32,
    hit_h: f32,
) -> egui::Response {
    ui.add_sized(
        vec2(hit_w, hit_h),
        egui::Button::new(
            RichText::new(icon.to_string())
                .size(icon_size)
                .color(color),
        )
        .frame(false),
    )
}

fn format_mmss(secs: f32) -> String {
    let s = secs.max(0.0);
    let m = (s / 60.0).floor() as u32;
    let r = (s % 60.0).floor() as u32;
    format!("{m:02}:{r:02}")
}

/// Elide long strings in the middle to keep both prefix and extension visible.
fn elide_middle(s: &str, max_chars: usize) -> String {
    let count = s.chars().count();
    if count <= max_chars {
        return s.to_string();
    }
    if max_chars < 8 {
        return s.chars().take(max_chars).collect();
    }
    let keep = max_chars - 1;
    let head = keep / 2;
    let tail = keep - head;
    let head_s: String = s.chars().take(head).collect();
    let tail_s: String = s.chars().skip(count - tail).collect();
    format!("{head_s}…{tail_s}")
}
