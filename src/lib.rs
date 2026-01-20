#![warn(clippy::all, rust_2018_idioms)]

mod app;
mod ui;
mod version_check;
pub mod nus3bank;

pub use app::TemplateApp;
pub use version_check::{check_for_updates_async, get_version_check_result, VersionCheckResult};
