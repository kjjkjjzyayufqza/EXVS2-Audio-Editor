use serde::{Deserialize, Serialize};

use crate::Locale;

/// Enum to represent the column to search in
#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
pub enum SearchColumn {
    All,
    Name,
    Id,
    Size,
    Filename,
    Type,
}

impl SearchColumn {
    pub fn display_name(&self) -> String {
        self.display_name_for_locale(Locale::En)
    }

    pub fn display_name_for_locale(&self, loc: Locale) -> String {
        let tag = loc.as_rust_i18n_locale();
        match self {
            SearchColumn::All => rust_i18n::t!("search_column_all", locale = tag).to_string(),
            SearchColumn::Name => rust_i18n::t!("search_column_name", locale = tag).to_string(),
            SearchColumn::Id => rust_i18n::t!("search_column_id", locale = tag).to_string(),
            SearchColumn::Size => rust_i18n::t!("search_column_size", locale = tag).to_string(),
            SearchColumn::Filename => rust_i18n::t!("search_column_filename", locale = tag).to_string(),
            SearchColumn::Type => rust_i18n::t!("search_column_type", locale = tag).to_string(),
        }
    }

    pub fn all_columns() -> Vec<SearchColumn> {
        vec![
            SearchColumn::All,
            SearchColumn::Name,
            SearchColumn::Id,
            SearchColumn::Size,
            SearchColumn::Filename,
            SearchColumn::Type,
        ]
    }
}
