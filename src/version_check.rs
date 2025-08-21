use once_cell::sync::OnceCell;
use semver::Version;
use std::sync::{Arc, Mutex};
use std::time::Duration;

// Different imports for native and web
#[cfg(not(target_arch = "wasm32"))]
use reqwest::blocking::Client;

#[cfg(target_arch = "wasm32")]
use {
    wasm_bindgen_futures::spawn_local,
    wasm_bindgen::prelude::*,
    web_sys::console,
};

// Current version of the application (from Cargo.toml)
pub const CURRENT_VERSION: &str = "0.6.0";

// URL to check for latest version
// This URL should point to a raw file on GitHub that contains version info
const VERSION_CHECK_URL: &str = "https://raw.githubusercontent.com/kjjkjjzyayufqza/EXVS2-Audio-Editor/main/version.json";

// Timeout for HTTP requests in seconds
const REQUEST_TIMEOUT_SEC: u64 = 5;

// Structure to hold version check result
#[derive(Clone)]
pub struct VersionCheckResult {
    pub has_new_version: bool,
    pub current_version: String,
    pub latest_version: String,
    pub download_url: String,
}

// Global storage for the version check result
static VERSION_CHECK_RESULT: OnceCell<Arc<Mutex<Option<VersionCheckResult>>>> = OnceCell::new();

// Initialize the version check result storage
fn init_version_check_result() -> Arc<Mutex<Option<VersionCheckResult>>> {
    Arc::new(Mutex::new(None))
}

// Get the version check result
pub fn get_version_check_result() -> Arc<Mutex<Option<VersionCheckResult>>> {
    VERSION_CHECK_RESULT.get_or_init(init_version_check_result).clone()
}

// Parse the version JSON response
fn parse_version_response(response_text: &str) -> Result<(String, String), String> {
    // Parse JSON response
    let json_response: serde_json::Value = serde_json::from_str(response_text)
        .map_err(|e| format!("Failed to parse JSON response: {}", e))?;
    
    // Extract version info
    let latest_version = json_response["version"]
        .as_str()
        .ok_or_else(|| "Version field not found in response".to_string())?
        .to_string();
    
    let download_url = json_response["download_url"]
        .as_str()
        .ok_or_else(|| "Download URL not found in response".to_string())?
        .to_string();
    
    Ok((latest_version, download_url))
}

// Compare versions using semver
fn is_newer_version(current: &str, latest: &str) -> bool {
    match (Version::parse(current), Version::parse(latest)) {
        (Ok(current_ver), Ok(latest_ver)) => latest_ver > current_ver,
        _ => false, // If parsing fails, assume no update needed
    }
}

// Check for updates asynchronously
pub fn check_for_updates_async() {
    // Different implementation for native and web
    #[cfg(not(target_arch = "wasm32"))]
    {
        let version_result = get_version_check_result();
        
        // Clone Arc for thread
        let thread_version_result = version_result.clone();
        
        std::thread::spawn(move || {
            match check_for_updates_impl() {
                Ok(result) => {
                    if let Ok(mut data) = thread_version_result.lock() {
                        *data = Some(result);
                    }
                }
                Err(e) => {
                    eprintln!("Version check error: {}", e);
                }
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
                    console::log_1(&format!("Version check error: {}", e).into());
                }
            }
        });
    }
}

// Native version check implementation
#[cfg(not(target_arch = "wasm32"))]
fn check_for_updates_impl() -> Result<VersionCheckResult, String> {
    // Create HTTP client with timeout
    let client = Client::builder()
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SEC))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
    
    // Make request to version check URL
    let response = client
        .get(VERSION_CHECK_URL)
        .send()
        .map_err(|e| format!("Failed to send request: {}", e))?;
    
    // Check response status
    if !response.status().is_success() {
        return Err(format!("Server returned error status: {}", response.status()));
    }
    
    // Get response text
    let response_text = response
        .text()
        .map_err(|e| format!("Failed to read response body: {}", e))?;
    
    process_version_response(&response_text)
}

// Web version check implementation
#[cfg(target_arch = "wasm32")]
async fn check_for_updates_web() -> Result<VersionCheckResult, String> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{Request, RequestInit, RequestMode, Response};
    
    // Create request options
    let mut opts = RequestInit::new();
    opts.method("GET");
    opts.mode(RequestMode::Cors);
    
    // Create request
    let request = Request::new_with_str_and_init(VERSION_CHECK_URL, &opts)
        .map_err(|_| "Failed to create request".to_string())?;
    
    // Get window object
    let window = web_sys::window().ok_or_else(|| "No window found".to_string())?;
    
    // Fetch the URL
    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|_| "Failed to fetch".to_string())?;
    
    // Convert response to Response type
    let response: Response = resp_value.dyn_into()
        .map_err(|_| "Failed to convert response".to_string())?;
    
    // Check if response is ok
    if !response.ok() {
        return Err(format!("Server returned error status: {}", response.status()));
    }
    
    // Get text from response
    let text = JsFuture::from(response.text().map_err(|_| "Failed to get text".to_string())?)
        .await
        .map_err(|_| "Failed to read response body".to_string())?;
    
    // Convert JS value to Rust string
    let response_text = text.as_string()
        .ok_or_else(|| "Failed to convert response to string".to_string())?;
    
    process_version_response(&response_text)
}

// Common code for processing version response
fn process_version_response(response_text: &str) -> Result<VersionCheckResult, String> {
    // Parse version info
    let (latest_version, download_url) = parse_version_response(response_text)?;
    
    // Compare versions
    let has_new_version = is_newer_version(CURRENT_VERSION, &latest_version);
    
    // Return result
    Ok(VersionCheckResult {
        has_new_version,
        current_version: CURRENT_VERSION.to_string(),
        latest_version,
        download_url,
    })
}
