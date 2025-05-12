// Re-export the AudioBackend trait
mod trait_def;
pub use trait_def::AudioBackend;

// Platform-specific implementations
mod native;

// Export the native audio backend
pub use native::NativeAudioBackend as PlatformAudioBackend;

// Optionally expose the specific backends for advanced use cases
