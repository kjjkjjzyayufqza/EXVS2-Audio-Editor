use serde::{Deserialize, Serialize};

use crate::i18n::Locale;

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
    pub fn display_name(&self) -> &'static str {
        self.display_name_for_locale(Locale::En)
    }

    pub fn display_name_for_locale(&self, loc: Locale) -> &'static str {
        match (self, loc) {
            (SearchColumn::All, Locale::En) => "All Columns",
            (SearchColumn::All, Locale::Zh) => "全部列",
            (SearchColumn::Name, Locale::En) => "Name",
            (SearchColumn::Name, Locale::Zh) => "名称",
            (SearchColumn::Id, _) => "ID",
            (SearchColumn::Size, Locale::En) => "Size",
            (SearchColumn::Size, Locale::Zh) => "大小",
            (SearchColumn::Filename, Locale::En) => "Filename",
            (SearchColumn::Filename, Locale::Zh) => "文件名",
            (SearchColumn::Type, Locale::En) => "Type",
            (SearchColumn::Type, Locale::Zh) => "类型",
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
