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

// Re-export main types
pub use structures::{Nus3bankFile};
pub use export::Nus3bankExporter;


