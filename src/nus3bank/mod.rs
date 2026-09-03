//! NUS3BANK file format support for EXVS2 Audio Editor
//!
//! PACK payloads for EXVS2 character SE are BNSF/IS14. The UI previews WAV;
//! save converts WAV replacements to BNSF/IS14.

pub mod binary_utils;
pub mod bnsf;
pub mod debug_json;
pub mod error;
pub mod export;
pub mod ob_tone_decode;
pub mod parser;
pub mod replace;
pub mod structures;
pub mod writer;

// Re-export main types
pub use structures::Nus3bankFile;

pub use export::Nus3bankExporter;

#[cfg(test)]
mod tests;
