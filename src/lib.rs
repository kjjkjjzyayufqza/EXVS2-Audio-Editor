#![warn(clippy::all, rust_2018_idioms)]

rust_i18n::i18n!("locales", fallback = "en");

mod app;
mod fonts;
mod locale;
mod localized;
mod ui;
mod version_check;
pub mod nus3bank;

pub use app::TemplateApp;
pub use locale::{locale_ctx_id, locale_from_ctx, sync_rust_i18n_locale, Locale};
pub use version_check::{check_for_updates_async, get_version_check_result, VersionCheckResult};
