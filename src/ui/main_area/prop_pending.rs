use std::collections::HashMap;
use std::sync::Mutex;
use once_cell::sync::Lazy;

use crate::nus3bank::structures::PropSection;

static PROP_PENDING: Lazy<Mutex<HashMap<String, PropSection>>> = Lazy::new(|| Mutex::new(HashMap::new()));

pub fn set(file_path: &str, prop: PropSection) -> Result<(), String> {
    PROP_PENDING
        .lock()
        .map_err(|e| format!("Failed to lock prop pending: {}", e))?
        .insert(file_path.to_string(), prop);
    Ok(())
}

pub fn get(file_path: &str) -> Option<PropSection> {
    PROP_PENDING
        .lock()
        .ok()?
        .get(file_path)
        .cloned()
}

pub fn clear(file_path: &str) -> Result<(), String> {
    PROP_PENDING
        .lock()
        .map_err(|e| format!("Failed to lock prop pending: {}", e))?
        .remove(file_path);
    Ok(())
}

pub fn take(file_path: &str) -> Option<PropSection> {
    PROP_PENDING
        .lock()
        .ok()?
        .remove(file_path)
}
