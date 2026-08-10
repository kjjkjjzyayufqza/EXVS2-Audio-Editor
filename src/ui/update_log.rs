//! Update notice + release history UI.

use crate::localized;
use crate::version_check::{self, HistoryEntry, VersionCheckResult};
use egui::{Color32, Context, RichText, ScrollArea, Ui, Window};
use once_cell::sync::Lazy;
use std::sync::Mutex;

#[derive(Clone, Default)]
struct UpdateLogState {
    /// One-shot "what's new" when a newer version is available
    show_update_notice: bool,
    /// Manual / menu-opened history browser
    show_history: bool,
    /// Already auto-shown update notice this session
    notice_shown: bool,
    /// Snapshot of remote/local payload for the open windows
    payload: Option<VersionCheckResult>,
}

static UPDATE_LOG_STATE: Lazy<Mutex<UpdateLogState>> =
    Lazy::new(|| Mutex::new(UpdateLogState::default()));

/// UI actions returned from panels (applied after the global mutex is released).
#[derive(Clone, Copy, Debug, Default)]
struct NoticeActions {
    close_notice: bool,
    open_history: bool,
}

#[derive(Clone, Copy, Debug, Default)]
struct HistoryActions {
    close_history: bool,
}

/// Open the full update history window (Help menu).
pub fn open_history() {
    if let Ok(mut st) = UPDATE_LOG_STATE.lock() {
        st.payload = Some(
            version_check::get_version_check_result()
                .lock()
                .ok()
                .and_then(|g| g.clone())
                .unwrap_or_else(version_check::embedded_history),
        );
        st.show_history = true;
    }
}

/// Poll version check and open the update notice once when a new version exists.
pub fn poll_and_maybe_open_notice() {
    let version_result = version_check::get_version_check_result();
    let Ok(guard) = version_result.try_lock() else {
        return;
    };
    let Some(result) = guard.as_ref() else {
        return;
    };
    if !result.has_new_version {
        return;
    }

    if let Ok(mut st) = UPDATE_LOG_STATE.lock() {
        if st.notice_shown {
            return;
        }
        st.payload = Some(result.clone());
        st.show_update_notice = true;
        st.notice_shown = true;
    }
}

/// Draw update notice + history windows (call each frame from top panel).
pub fn show_windows(ctx: &Context) {
    poll_and_maybe_open_notice();

    // Snapshot flags under a short lock — never hold the mutex while handling clicks
    // (re-locking inside button handlers would deadlock / freeze the UI).
    let (show_notice, show_history, payload) = {
        let Ok(st) = UPDATE_LOG_STATE.lock() else {
            return;
        };
        (
            st.show_update_notice,
            st.show_history,
            st.payload
                .clone()
                .unwrap_or_else(version_check::embedded_history),
        )
    };

    let mut notice_actions = NoticeActions::default();
    let mut history_actions = HistoryActions::default();

    if show_notice {
        let mut open = true;
        Window::new(localized::update_available_title())
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(420.0)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                notice_actions = render_update_notice(ui, &payload);
            });
        if !open {
            notice_actions.close_notice = true;
        }
    }

    if show_history {
        let mut open = true;
        Window::new(localized::update_history_title())
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(480.0)
            .default_height(420.0)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                history_actions = render_history(ui, &payload);
            });
        if !open {
            history_actions.close_history = true;
        }
    }

    // Apply UI results with a single lock
    if notice_actions.close_notice
        || notice_actions.open_history
        || history_actions.close_history
    {
        if let Ok(mut st) = UPDATE_LOG_STATE.lock() {
            if notice_actions.close_notice {
                st.show_update_notice = false;
            }
            if notice_actions.open_history {
                st.show_history = true;
                st.payload = Some(payload);
            }
            if history_actions.close_history {
                st.show_history = false;
            }
        }
    }
}

fn render_update_notice(ui: &mut Ui, payload: &VersionCheckResult) -> NoticeActions {
    let mut actions = NoticeActions::default();

    ui.label(
        RichText::new(localized::update_available_body(
            &payload.current_version,
            &payload.latest_version,
        ))
        .size(14.0),
    );

    ui.add_space(10.0);
    ui.label(
        RichText::new(localized::whats_new_heading())
            .strong()
            .size(13.0),
    );
    ui.add_space(4.0);

    ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
        if payload.changelog.is_empty() {
            ui.label(
                RichText::new(localized::no_changelog_available())
                    .italics()
                    .color(ui.visuals().weak_text_color()),
            );
        } else {
            for line in &payload.changelog {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("•").color(Color32::from_rgb(100, 150, 255)));
                    ui.label(line);
                });
            }
        }
    });

    ui.add_space(12.0);
    ui.horizontal(|ui| {
        if !payload.download_url.is_empty() {
            ui.hyperlink_to(localized::download_latest(), &payload.download_url);
        }
        if ui.button(localized::view_full_history()).clicked() {
            actions.open_history = true;
        }
        if ui.button(localized::ok()).clicked() {
            actions.close_notice = true;
        }
    });

    actions
}

fn render_history(ui: &mut Ui, payload: &VersionCheckResult) -> HistoryActions {
    let mut actions = HistoryActions::default();

    ui.label(
        RichText::new(localized::update_history_subtitle(
            &payload.current_version,
            &payload.latest_version,
        ))
        .color(ui.visuals().weak_text_color())
        .size(12.0),
    );
    ui.add_space(8.0);

    ScrollArea::vertical().show(ui, |ui| {
        if payload.history.is_empty() {
            ui.label(
                RichText::new(localized::no_changelog_available())
                    .italics()
                    .color(ui.visuals().weak_text_color()),
            );
            return;
        }

        for (i, entry) in payload.history.iter().enumerate() {
            render_history_entry(ui, entry, i == 0 && payload.has_new_version);
            if i + 1 < payload.history.len() {
                ui.add_space(6.0);
                ui.separator();
                ui.add_space(6.0);
            }
        }
    });

    ui.add_space(10.0);
    ui.horizontal(|ui| {
        if !payload.download_url.is_empty() {
            ui.hyperlink_to(localized::download_latest(), &payload.download_url);
        }
        if ui.button(localized::ok()).clicked() {
            actions.close_history = true;
        }
    });

    actions
}

fn render_history_entry(ui: &mut Ui, entry: &HistoryEntry, highlight: bool) {
    ui.horizontal(|ui| {
        let ver = RichText::new(format!("v{}", entry.version))
            .strong()
            .size(14.0);
        if highlight {
            ui.label(ver.color(Color32::from_rgb(100, 200, 140)));
            ui.label(
                RichText::new(localized::latest_badge())
                    .size(11.0)
                    .color(Color32::from_rgb(100, 200, 140)),
            );
        } else {
            ui.label(ver);
        }
        if !entry.date.is_empty() {
            ui.label(
                RichText::new(&entry.date)
                    .size(11.0)
                    .color(ui.visuals().weak_text_color()),
            );
        }
    });
    ui.add_space(4.0);
    for line in &entry.changes {
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            ui.label(RichText::new("•").color(Color32::from_rgb(100, 150, 255)));
            ui.label(line);
        });
    }
}
