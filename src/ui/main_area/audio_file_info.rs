/// Structure to hold audio file information
#[derive(Clone)]
pub struct AudioFileInfo {
    pub name: String,
    pub id: String,
    pub size: usize,
    pub filename: String,
    pub file_type: String,
}
