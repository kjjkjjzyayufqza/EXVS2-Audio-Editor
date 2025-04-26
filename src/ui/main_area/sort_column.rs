use serde::{Deserialize, Serialize};

/// Enum representing sortable columns
#[derive(Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum SortColumn {
    Name,
    Id,
    Size,
    Filename,
    Type,
    None,
}

impl SortColumn {
    /// Get display name for the column
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Name => "Name",
            Self::Id => "ID",
            Self::Size => "Size",
            Self::Filename => "Filename",
            Self::Type => "Type",
            Self::None => "",
        }
    }
}
