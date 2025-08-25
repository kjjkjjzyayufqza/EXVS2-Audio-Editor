use std::io::{Read, Seek, SeekFrom, Result as IoResult};
use super::error::Nus3bankError;

/// Binary reading utilities for NUS3BANK format
pub struct BinaryReader;

impl BinaryReader {
    /// Read single byte
    pub fn read_u8<R: Read>(reader: &mut R) -> IoResult<u8> {
        let mut buf = [0u8; 1];
        reader.read_exact(&mut buf)?;
        Ok(buf[0])
    }
    
    /// Read 16-bit little-endian unsigned integer
    pub fn read_u16_le<R: Read>(reader: &mut R) -> IoResult<u16> {
        let mut buf = [0u8; 2];
        reader.read_exact(&mut buf)?;
        Ok(u16::from_le_bytes(buf))
    }
    
    /// Read 32-bit little-endian unsigned integer
    pub fn read_u32_le<R: Read>(reader: &mut R) -> IoResult<u32> {
        let mut buf = [0u8; 4];
        match reader.read_exact(&mut buf) {
            Ok(_) => {
                let value = u32::from_le_bytes(buf);
                // Debug: print hex representation for unusual values
                if value > 1_000_000 {
                    println!("DEBUG: Read large u32 value: {} (0x{:08X}) from bytes [{:02X} {:02X} {:02X} {:02X}]", 
                        value, value, buf[0], buf[1], buf[2], buf[3]);
                }
                Ok(value)
            }
            Err(e) => {
                eprintln!("Error reading u32: {}", e);
                Err(e)
            }
        }
    }
    
    /// Read 32-bit little-endian float
    pub fn read_f32_le<R: Read>(reader: &mut R) -> IoResult<f32> {
        let mut buf = [0u8; 4];
        reader.read_exact(&mut buf)?;
        Ok(f32::from_le_bytes(buf))
    }
    
    /// Validate magic number
    pub fn assert_magic<R: Read>(reader: &mut R, expected: &[u8]) -> Result<(), Nus3bankError> {
        let mut buffer = vec![0u8; expected.len()];
        match reader.read_exact(&mut buffer) {
            Ok(_) => {
                if buffer != expected {
                    let expected_str = String::from_utf8_lossy(expected).to_string();
                    let found_str = String::from_utf8_lossy(&buffer).to_string();
                    eprintln!("Magic mismatch: expected '{}', found '{}'", expected_str, found_str);
                    return Err(Nus3bankError::InvalidMagic {
                        expected: expected_str,
                        found: found_str,
                    });
                }
                println!("Magic number '{}' validated successfully", String::from_utf8_lossy(expected));
                Ok(())
            }
            Err(e) => {
                eprintln!("Failed to read magic number: {}", e);
                Err(Nus3bankError::Io(e))
            }
        }
    }
    
    /// Calculate 4-byte alignment padding
    pub fn calculate_padding(size: usize) -> usize {
        (4 - (size % 4)) % 4
    }
    
    /// Read null-terminated string with padding
    pub fn read_padded_string<R: Read>(reader: &mut R, length: usize) -> IoResult<String> {
        if length == 0 {
            return Ok(String::new());
        }
        
        let mut buffer = vec![0u8; length];
        match reader.read_exact(&mut buffer) {
            Ok(_) => {
                // Find null terminator
                let end = buffer.iter().position(|&b| b == 0).unwrap_or(length);
                Ok(String::from_utf8_lossy(&buffer[..end]).to_string())
            }
            Err(e) => {
                eprintln!("Error reading string of length {}: {}", length, e);
                Err(e)
            }
        }
    }
    
    /// Read section magic bytes
    pub fn read_section_magic<R: Read>(reader: &mut R) -> Result<[u8; 4], Nus3bankError> {
        let mut magic = [0u8; 4];
        match reader.read_exact(&mut magic) {
            Ok(_) => {
                println!("Read section magic: {:?}", String::from_utf8_lossy(&magic));
                Ok(magic)
            }
            Err(e) => {
                println!("Failed to read section magic: {}", e);
                Err(Nus3bankError::Io(e))
            }
        }
    }
    
    /// Get current position (helper for readers with Seek)
    pub fn get_current_position<R: Read + Seek>(reader: &mut R) -> Result<u32, Nus3bankError> {
        Ok(reader.stream_position()? as u32)
    }
    
    /// Write 32-bit little-endian unsigned integer
    pub fn write_u32_le(value: u32) -> [u8; 4] {
        value.to_le_bytes()
    }
    
    /// Write string with null termination and 4-byte alignment padding
    pub fn write_padded_string(s: &str) -> Vec<u8> {
        let mut data = s.as_bytes().to_vec();
        data.push(0); // null terminator
        
        // Add padding to align to 4 bytes
        let padding = Self::calculate_padding(data.len());
        data.extend(vec![0u8; padding]);
        
        data
    }
    
    /// Check if file starts with zlib compression header
    pub fn is_compressed<R: Read + Seek>(reader: &mut R) -> Result<bool, Nus3bankError> {
        let mut header = [0u8; 2];
        match reader.read_exact(&mut header) {
            Ok(_) => {
                reader.seek(SeekFrom::Start(0))?;
                let is_compressed = header == [0x78, 0x9C] || header == [0x78, 0x01] || header == [0x78, 0xDA];
                println!("File header: {:02X} {:02X}, compressed: {}", header[0], header[1], is_compressed);
                Ok(is_compressed)
            }
            Err(e) => {
                eprintln!("Failed to read file header: {}", e);
                Err(Nus3bankError::Io(e))
            }
        }
    }
    
    /// Debug helper: read and display hex dump of data at current position
    pub fn debug_hex_dump<R: Read + Seek>(reader: &mut R, size: usize, label: &str) -> Result<(), Nus3bankError> {
        let current_pos = reader.stream_position()? as u32;
        let mut buffer = vec![0u8; size];
        
        match reader.read_exact(&mut buffer) {
            Ok(_) => {
                println!("=== HEX DUMP: {} (at offset 0x{:08X}, {} bytes) ===", label, current_pos, size);
                for (i, chunk) in buffer.chunks(16).enumerate() {
                    print!("{:08X}  ", current_pos + (i * 16) as u32);
                    
                    // Print hex bytes
                    for (j, byte) in chunk.iter().enumerate() {
                        if j == 8 { print!(" "); }
                        print!("{:02X} ", byte);
                    }
                    
                    // Pad if last line is shorter
                    if chunk.len() < 16 {
                        for j in chunk.len()..16 {
                            if j == 8 { print!(" "); }
                            print!("   ");
                        }
                    }
                    
                    print!(" |");
                    
                    // Print ASCII representation
                    for byte in chunk {
                        if byte.is_ascii_graphic() || *byte == b' ' {
                            print!("{}", *byte as char);
                        } else {
                            print!(".");
                        }
                    }
                    
                    println!("|");
                }
                println!("=== END HEX DUMP ===\n");
                
                // Reset position to after the read data
                reader.seek(SeekFrom::Start((current_pos + size as u32) as u64))?;
                Ok(())
            }
            Err(e) => {
                eprintln!("Failed to read data for hex dump: {}", e);
                // Try to reset position
                let _ = reader.seek(SeekFrom::Start(current_pos as u64));
                Err(Nus3bankError::Io(e))
            }
        }
    }
    
    /// Debug helper: read hex dump without advancing position
    pub fn peek_hex_dump<R: Read + Seek>(reader: &mut R, size: usize, label: &str) -> Result<(), Nus3bankError> {
        let current_pos = reader.stream_position()?;
        let result = Self::debug_hex_dump(reader, size, label);
        reader.seek(SeekFrom::Start(current_pos))?;
        result
    }
}
