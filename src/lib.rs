#![warn(clippy::all, rust_2018_idioms)]

rust_i18n::i18n!("locales", fallback = "en");

mod app;
mod fonts;
mod locale;
mod localized;
pub mod nus3bank;
mod ui;
mod version_check;

pub use app::TemplateApp;
pub use locale::{Locale, locale_ctx_id, locale_from_ctx, sync_rust_i18n_locale};
pub use version_check::{VersionCheckResult, check_for_updates_async, get_version_check_result};
