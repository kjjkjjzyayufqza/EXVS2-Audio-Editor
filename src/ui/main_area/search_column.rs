use serde::{Deserialize, Serialize};

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
        match self {
            SearchColumn::All => "All Columns",
            SearchColumn::Name => "Name",
            SearchColumn::Id => "ID",
            SearchColumn::Size => "Size",
            SearchColumn::Filename => "Filename",
            SearchColumn::Type => "Type",
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
