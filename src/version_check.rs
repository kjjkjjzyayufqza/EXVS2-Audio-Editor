use once_cell::sync::OnceCell;
use semver::Version;
use serde::Deserialize;
use std::sync::{Arc, Mutex};
use std::time::Duration;

// Different imports for native and web
#[cfg(not(target_arch = "wasm32"))]
use reqwest::blocking::Client;

#[cfg(target_arch = "wasm32")]
use {wasm_bindgen::prelude::*, wasm_bindgen_futures::spawn_local, web_sys::console};

/// Current version of the application (keep in sync with Cargo.toml)
pub const CURRENT_VERSION: &str = "0.8.0";

/// Bundled fallback when network is unavailable (update history still works offline).
const EMBEDDED_VERSION_JSON: &str = include_str!("../version.json");

const VERSION_CHECK_URL: &str =
    "https://raw.githubusercontent.com/kjjkjjzyayufqza/EXVS2-Audio-Editor/main/version.json";

const REQUEST_TIMEOUT_SEC: u64 = 5;

#[derive(Clone, Debug, Deserialize)]
pub struct HistoryEntry {
    pub version: String,
    #[serde(default)]
    pub date: String,
    #[serde(default)]
    pub changes: Vec<String>,
}

/// Full version payload from version.json
#[derive(Clone, Debug)]
pub struct VersionCheckResult {
    pub has_new_version: bool,
    pub current_version: String,
    pub latest_version: String,
    pub download_url: String,
    /// What's new in the latest remote version
    pub changelog: Vec<String>,
    /// Full release history (newest first preferred)
    pub history: Vec<HistoryEntry>,
}

#[derive(Deserialize)]
struct VersionJson {
    version: String,
    #[serde(default)]
    download_url: String,
    #[serde(default)]
    changelog: Vec<String>,
    #[serde(default)]
    history: Vec<HistoryEntry>,
}

static VERSION_CHECK_RESULT: OnceCell<Arc<Mutex<Option<VersionCheckResult>>>> = OnceCell::new();

fn init_version_check_result() -> Arc<Mutex<Option<VersionCheckResult>>> {
    Arc::new(Mutex::new(None))
}

pub fn get_version_check_result() -> Arc<Mutex<Option<VersionCheckResult>>> {
    VERSION_CHECK_RESULT
        .get_or_init(init_version_check_result)
        .clone()
}

/// Local/offline history for Help → Update History (always available).
pub fn embedded_history() -> VersionCheckResult {
    parse_version_document(EMBEDDED_VERSION_JSON).unwrap_or_else(|_| VersionCheckResult {
        has_new_version: false,
        current_version: CURRENT_VERSION.to_string(),
        latest_version: CURRENT_VERSION.to_string(),
        download_url: String::new(),
        changelog: Vec::new(),
        history: vec![HistoryEntry {
            version: CURRENT_VERSION.to_string(),
            date: String::new(),
            changes: vec!["Current build".to_string()],
        }],
    })
}

fn is_newer_version(current: &str, latest: &str) -> bool {
    match (Version::parse(current), Version::parse(latest)) {
        (Ok(current_ver), Ok(latest_ver)) => latest_ver > current_ver,
        _ => false,
    }
}

fn parse_version_document(response_text: &str) -> Result<VersionCheckResult, String> {
    let doc: VersionJson = serde_json::from_str(response_text)
        .map_err(|e| format!("Failed to parse version JSON: {e}"))?;

    let latest_version = doc.version;
    let download_url = if doc.download_url.is_empty() {
        "https://github.com/kjjkjjzyayufqza/EXVS2-Audio-Editor/releases/latest".to_string()
    } else {
        doc.download_url
    };

    let mut history = doc.history;
    // If history empty, synthesize one entry from top-level changelog
    if history.is_empty() && !doc.changelog.is_empty() {
        history.push(HistoryEntry {
            version: latest_version.clone(),
            date: String::new(),
            changes: doc.changelog.clone(),
        });
    }

    let changelog = if doc.changelog.is_empty() {
        history
            .first()
            .map(|h| h.changes.clone())
            .unwrap_or_default()
    } else {
        doc.changelog
    };

    let has_new_version = is_newer_version(CURRENT_VERSION, &latest_version);

    Ok(VersionCheckResult {
        has_new_version,
        current_version: CURRENT_VERSION.to_string(),
        latest_version,
        download_url,
        changelog,
        history,
    })
}

pub fn check_for_updates_async() {
    // Seed with embedded data so history UI works immediately
    {
        let seeded = embedded_history();
        if let Ok(mut data) = get_version_check_result().lock() {
            if data.is_none() {
                *data = Some(seeded);
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let version_result = get_version_check_result();
        let thread_version_result = version_result.clone();

        std::thread::spawn(move || match check_for_updates_impl() {
            Ok(result) => {
                if let Ok(mut data) = thread_version_result.lock() {
                    *data = Some(result);
                }
            }
            Err(e) => {
                eprintln!("Version check error: {e}");
            }
        });
    }

    #[cfg(target_arch = "wasm32")]
    {
        let version_result = get_version_check_result();
        let thread_version_result = version_result.clone();

        spawn_local(async move {
            match check_for_updates_web().await {
                Ok(result) => {
                    if let Ok(mut data) = thread_version_result.lock() {
                        *data = Some(result);
                    }
                }
                Err(e) => {
                    console::log_1(&format!("Version check error: {e}").into());
                }
            }
        });
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn check_for_updates_impl() -> Result<VersionCheckResult, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SEC))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {e}"))?;

    let response = client
        .get(VERSION_CHECK_URL)
        .send()
        .map_err(|e| format!("Failed to send request: {e}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "Server returned error status: {}",
            response.status()
        ));
    }

    let response_text = response
        .text()
        .map_err(|e| format!("Failed to read response body: {e}"))?;

    parse_version_document(&response_text)
}

#[cfg(target_arch = "wasm32")]
async fn check_for_updates_web() -> Result<VersionCheckResult, String> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let mut opts = RequestInit::new();
    opts.method("GET");
    opts.mode(RequestMode::Cors);

    let request = Request::new_with_str_and_init(VERSION_CHECK_URL, &opts)
        .map_err(|_| "Failed to create request".to_string())?;

    let window = web_sys::window().ok_or_else(|| "No window found".to_string())?;

    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|_| "Failed to fetch".to_string())?;

    let response: Response = resp_value
        .dyn_into()
        .map_err(|_| "Failed to convert response".to_string())?;

    if !response.ok() {
        return Err(format!(
            "Server returned error status: {}",
            response.status()
        ));
    }

    let text = JsFuture::from(
        response
            .text()
            .map_err(|_| "Failed to get text".to_string())?,
    )
    .await
    .map_err(|_| "Failed to read response body".to_string())?;

    let response_text = text
        .as_string()
        .ok_or_else(|| "Failed to convert response to string".to_string())?;

    parse_version_document(&response_text)
}
