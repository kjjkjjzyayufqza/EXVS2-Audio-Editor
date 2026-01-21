use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;

static PENDING_GRP_NAMES: Lazy<Mutex<HashMap<String, Vec<String>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

fn normalize_key(file_path: &str) -> String {
    // Normalize keys to avoid mismatches between different path string forms
    // (e.g. backslash vs slash, drive letter case) on Windows.
    #[cfg(windows)]
    {
        file_path.replace('\\', "/").to_ascii_lowercase()
    }
    #[cfg(not(windows))]
    {
        file_path.to_string()
    }
}

pub fn get(file_path: &str) -> Option<Vec<String>> {
    let map = PENDING_GRP_NAMES.lock().ok()?;
    map.get(&normalize_key(file_path)).cloned()
}

pub fn set(file_path: &str, names: Vec<String>) -> Result<(), String> {
    let mut map = PENDING_GRP_NAMES
        .lock()
        .map_err(|_| "Failed to acquire GRP pending lock".to_string())?;
    map.insert(normalize_key(file_path), names);
    Ok(())
}

pub fn clear(file_path: &str) -> Result<(), String> {
    let mut map = PENDING_GRP_NAMES
        .lock()
        .map_err(|_| "Failed to acquire GRP pending lock".to_string())?;
    map.remove(&normalize_key(file_path));
    Ok(())
}

pub fn has(file_path: &str) -> bool {
    let Ok(map) = PENDING_GRP_NAMES.lock() else {
        return false;
    };
    map.contains_key(&normalize_key(file_path))
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

    #[test]
    fn pending_key_normalization_is_stable() {
        let p1 = "E:\\Foo\\Bar.nus3bank";
        let p2 = "e:/foo/bar.nus3bank";
        let _ = clear(p1);

        set(p1, vec!["X".to_string()]).unwrap();
        assert!(has(p2));
        assert_eq!(get(p2), Some(vec!["X".to_string()]));
        clear(p2).unwrap();
        assert!(!has(p1));
    }
}

