//! NUS3BANK file format support for EXVS2 Audio Editor
//! 
//! This module provides comprehensive support for reading, writing, and manipulating
//! NUS3BANK audio archive files with focus on WAV format audio only.

pub mod structures;
pub mod parser;
pub mod writer;
pub mod binary_utils;
pub mod error;
pub mod export;
pub mod replace;
pub mod integration;

#[cfg(debug_assertions)]
pub mod debug_test;

// Re-export main types
pub use structures::{Nus3bankFile, BankInfo, AudioTrack, AudioFormat, SectionOffsets};
pub use error::Nus3bankError;
pub use export::Nus3bankExporter;
pub use replace::Nus3bankReplacer;

/// Module version and compatibility information
pub const VERSION: &str = "1.0.0";
pub const SUPPORTED_FORMATS: &[&str] = &["WAV"];
