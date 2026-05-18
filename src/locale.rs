//! Application [`Locale`] and egui temp-slot wiring for [`crate::sync_rust_i18n_locale`].

use egui::Context;

/// Stored each frame in egui temp data (`TemplateApp::update`).
#[inline]
pub fn locale_ctx_id() -> egui::Id {
    egui::Id::new("exvs2_app_locale")
}

/// Sync rust-i18n global locale (`rust_i18n::set_locale`). Call early each frame — see [`crate::TemplateApp::update`].
#[inline]
pub fn sync_rust_i18n_locale(locale: Locale) {
    rust_i18n::set_locale(locale.as_rust_i18n_locale());
}

#[derive(Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Debug)]
pub enum Locale {
    #[serde(rename = "en")]
    En,
    #[serde(rename = "zh")]
    Zh,
}

impl Default for Locale {
    fn default() -> Self {
        Self::detect_system()
    }
}

impl Locale {
    pub fn detect_system() -> Self {
        if let Some(loc) = sys_locale::get_locale() {
            let l = loc.to_lowercase();
            if l.starts_with("zh") {
                return Self::Zh;
            }
        }
        Self::En
    }

    /// Tag for `rust_i18n::set_locale` or `t!(..., locale = tag)` overrides (`en` / `zh`).
    #[must_use]
    pub fn as_rust_i18n_locale(self) -> &'static str {
        match self {
            Locale::En => "en",
            Locale::Zh => "zh",
        }
    }
}

#[inline]
pub fn locale_from_ctx(ctx: &Context) -> Locale {
    ctx.data(|d| d.get_temp::<Locale>(locale_ctx_id()))
        .unwrap_or_else(Locale::detect_system)
}
