//! Debug testing utilities for NUS3BANK parsing

use std::fs::File;
use std::io::BufReader;
use super::{
    error::Nus3bankError,
    parser::Nus3bankParser,
    binary_utils::BinaryReader,
};

/// Test function to parse a NUS3BANK file with extensive debugging
pub fn debug_parse_file<P: AsRef<std::path::Path>>(path: P) -> Result<(), Nus3bankError> {
    let path_str = path.as_ref().to_string_lossy().to_string();
    println!("=== DEBUG PARSING: {} ===", path_str);
    
    let file = File::open(&path).map_err(|e| {
        eprintln!("Failed to open file '{}': {}", path_str, e);
        Nus3bankError::Io(e)
    })?;
    
    let file_size = file.metadata().map(|m| m.len()).unwrap_or(0);
    println!("File size: {} bytes", file_size);
    
    let mut reader = BufReader::new(file);
    
    // Show first 256 bytes as hex dump
    println!("=== FILE STRUCTURE ANALYSIS ===");
    if let Err(e) = BinaryReader::debug_hex_dump(&mut reader, 256.min(file_size as usize), "File beginning") {
        eprintln!("Failed to create initial hex dump: {}", e);
    }
    
    // Try to parse normally with enhanced debugging
    match Nus3bankParser::parse_file(path) {
        Ok(nus3bank) => {
            println!("=== PARSING SUCCESSFUL ===");
            println!("Bank ID: {}", nus3bank.bank_info.bank_id);
            println!("Bank String: '{}'", nus3bank.bank_info.bank_string);
            println!("Total tracks: {}", nus3bank.tracks.len());
            
            for (i, track) in nus3bank.tracks.iter().enumerate().take(5) {
                println!("Track {}: {} - '{}' ({} bytes)", 
                    i, track.hex_id, track.name, track.size);
            }
            
            if nus3bank.tracks.len() > 5 {
                println!("... and {} more tracks", nus3bank.tracks.len() - 5);
            }
        }
        Err(e) => {
            println!("=== PARSING FAILED ===");
            println!("Error: {}", e);
            println!("This enhanced debugging should help identify the issue.");
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_parse() {
        // This test is for manual debugging - run with a real NUS3BANK file path
        // let result = debug_parse_file("path/to/test.nus3bank");
        // assert!(result.is_ok());
    }
}
