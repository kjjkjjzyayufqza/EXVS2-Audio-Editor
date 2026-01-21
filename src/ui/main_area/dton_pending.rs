use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;

use crate::nus3bank::structures::ToneDes;

static PENDING_DTON_TONES: Lazy<Mutex<HashMap<String, Vec<ToneDes>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

fn normalize_key(file_path: &str) -> String {
    #[cfg(windows)]
    {
        file_path.replace('\\', "/").to_ascii_lowercase()
    }
    #[cfg(not(windows))]
    {
        file_path.to_string()
    }
}

pub fn get(file_path: &str) -> Option<Vec<ToneDes>> {
    let map = PENDING_DTON_TONES.lock().ok()?;
    map.get(&normalize_key(file_path)).cloned()
}

pub fn set(file_path: &str, tones: Vec<ToneDes>) -> Result<(), String> {
    let mut map = PENDING_DTON_TONES
        .lock()
        .map_err(|_| "Failed to acquire DTON pending lock".to_string())?;
    map.insert(normalize_key(file_path), tones);
    Ok(())
}

pub fn clear(file_path: &str) -> Result<(), String> {
    let mut map = PENDING_DTON_TONES
        .lock()
        .map_err(|_| "Failed to acquire DTON pending lock".to_string())?;
    map.remove(&normalize_key(file_path));
    Ok(())
}

pub fn has(file_path: &str) -> bool {
    let Ok(map) = PENDING_DTON_TONES.lock() else {
        return false;
    };
    map.contains_key(&normalize_key(file_path))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tone(name: &str) -> ToneDes {
        ToneDes {
            hash: 1,
            unk1: 2,
            name: name.to_string(),
            data: vec![0.0, 1.0],
        }
    }

    #[test]
    fn pending_dton_roundtrip() {
        let path = "unit_test_file.nus3bank";
        let _ = clear(path);

        assert!(!has(path));
        assert!(get(path).is_none());

        set(path, vec![make_tone("Default"), make_tone("Voice")]).unwrap();
        assert!(has(path));
        assert_eq!(get(path).unwrap().len(), 2);

        clear(path).unwrap();
        assert!(!has(path));
        assert!(get(path).is_none());
    }

    #[test]
    fn pending_key_normalization_is_stable() {
        let p1 = "E:\\Foo\\Bar.nus3bank";
        let p2 = "e:/foo/bar.nus3bank";
        let _ = clear(p1);

        set(p1, vec![make_tone("X")]).unwrap();
        assert!(has(p2));
        assert_eq!(get(p2).unwrap()[0].name, "X");
        clear(p2).unwrap();
        assert!(!has(p1));
    }
}

