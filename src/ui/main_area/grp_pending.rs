use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;

static PENDING_GRP_NAMES: Lazy<Mutex<HashMap<String, Vec<String>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub fn get(file_path: &str) -> Option<Vec<String>> {
    let map = PENDING_GRP_NAMES.lock().ok()?;
    map.get(file_path).cloned()
}

pub fn set(file_path: &str, names: Vec<String>) -> Result<(), String> {
    let mut map = PENDING_GRP_NAMES
        .lock()
        .map_err(|_| "Failed to acquire GRP pending lock".to_string())?;
    map.insert(file_path.to_string(), names);
    Ok(())
}

pub fn take(file_path: &str) -> Option<Vec<String>> {
    let mut map = PENDING_GRP_NAMES.lock().ok()?;
    map.remove(file_path)
}

pub fn clear(file_path: &str) -> Result<(), String> {
    let mut map = PENDING_GRP_NAMES
        .lock()
        .map_err(|_| "Failed to acquire GRP pending lock".to_string())?;
    map.remove(file_path);
    Ok(())
}

pub fn has(file_path: &str) -> bool {
    let Ok(map) = PENDING_GRP_NAMES.lock() else {
        return false;
    };
    map.contains_key(file_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_grp_roundtrip() {
        let path = "unit_test_file.nus3bank";
        let _ = clear(path);

        assert!(!has(path));
        assert_eq!(get(path), None);

        set(path, vec!["A".to_string(), "".to_string(), "B".to_string()]).unwrap();
        assert!(has(path));
        assert_eq!(get(path), Some(vec!["A".to_string(), "".to_string(), "B".to_string()]));

        clear(path).unwrap();
        assert!(!has(path));
        assert_eq!(get(path), None);
    }
}

