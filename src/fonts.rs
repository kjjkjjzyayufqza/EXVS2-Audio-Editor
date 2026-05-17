//! Load a system CJK font so Chinese UI text renders reliably (esp. on Windows/macOS/Linux).

use egui::{FontData, FontDefinitions, FontFamily};
use std::sync::Arc;

/// Insert a Chinese-capable font as the first fallback in proportional and monospace families.
pub fn install_cjk_font_if_available(fonts: &mut FontDefinitions) {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = fonts;
        return;
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let Some(bytes) = load_cjk_font_bytes() else {
            log::warn!("No CJK system font found; Chinese glyphs may not render.");
            return;
        };

        fonts.font_data.insert(
            "exvs2_cjk".to_owned(),
            Arc::new(FontData::from_owned(bytes)),
        );

        for family in [FontFamily::Proportional, FontFamily::Monospace] {
            fonts
                .families
                .entry(family)
                .or_default()
                .insert(0, "exvs2_cjk".to_owned());
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn load_cjk_font_bytes() -> Option<Vec<u8>> {
    #[cfg(windows)]
    {
        for path in [
            r"C:\Windows\Fonts\msyh.ttc",
            r"C:\Windows\Fonts\msyhbd.ttc",
            r"C:\Windows\Fonts\simhei.ttf",
            r"C:\Windows\Fonts\simsun.ttc",
            r"C:\Windows\Fonts\msjh.ttc",
        ] {
            if let Ok(data) = std::fs::read(path) {
                return Some(data);
            }
        }
    }

    use fontdb::{Database, Family, Query, Stretch, Style, Weight};

    let mut db = Database::new();
    db.load_system_fonts();

    for name in [
        "Noto Sans CJK SC",
        "Noto Sans CJK JP",
        "Source Han Sans SC",
        "WenQuanYi Micro Hei",
        "Droid Sans Fallback",
        "PingFang SC",
        "Hiragino Sans GB",
        "STHeiti",
        "Songti SC",
        "Microsoft YaHei",
    ] {
        let query = Query {
            families: &[Family::Name(name)],
            weight: Weight::NORMAL,
            stretch: Stretch::Normal,
            style: Style::Normal,
        };
        let Some(id) = db.query(&query) else {
            continue;
        };
        let (src, _) = db.face_source(id)?;
        match src {
            fontdb::Source::File(path) => {
                if let Ok(data) = std::fs::read(&path) {
                    return Some(data);
                }
            }
            fontdb::Source::Binary(data) => {
                return Some(data.as_ref().as_ref().to_vec());
            }
        }
    }

    None
}
