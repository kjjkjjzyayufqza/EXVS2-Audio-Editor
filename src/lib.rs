#![warn(clippy::all, rust_2018_idioms)]

mod app;
mod fonts;
mod i18n;
mod ui;
mod version_check;
pub mod nus3bank;

pub use app::TemplateApp;
pub use i18n::{locale_ctx_id, I18n, Locale};
pub use version_check::{check_for_updates_async, get_version_check_result, VersionCheckResult};
