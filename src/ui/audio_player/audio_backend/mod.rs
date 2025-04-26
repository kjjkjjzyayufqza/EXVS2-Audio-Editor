// Re-export the AudioBackend trait
mod trait_def;
pub use trait_def::AudioBackend;

// Platform-specific implementations
mod native;
mod web;

// Export the appropriate backend based on target platform
#[cfg(not(target_arch = "wasm32"))]
pub use native::NativeAudioBackend as PlatformAudioBackend;

#[cfg(target_arch = "wasm32")]
pub use web::WebAudioBackend as PlatformAudioBackend;

// Optionally expose the specific backends for advanced use cases
pub use native::NativeAudioBackend;
pub use web::WebAudioBackend;
