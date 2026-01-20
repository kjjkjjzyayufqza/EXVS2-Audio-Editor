use super::error::Nus3bankError;
use std::io::{Read, Result as IoResult, Seek, SeekFrom};

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
        reader.read_exact(&mut buf)?;
        Ok(u32::from_le_bytes(buf))
    }

    /// Read 32-bit little-endian signed integer
    pub fn read_i32_le<R: Read>(reader: &mut R) -> IoResult<i32> {
        let raw = Self::read_u32_le(reader)?;
        Ok(raw as i32)
    }

    /// Read 32-bit little-endian float
    pub fn read_f32_le<R: Read>(reader: &mut R) -> IoResult<f32> {
        let mut buf = [0u8; 4];
        reader.read_exact(&mut buf)?;
        Ok(f32::from_le_bytes(buf))
    }

    /// Read fixed-length bytes into a Vec
    pub fn read_bytes<R: Read>(reader: &mut R, len: usize) -> IoResult<Vec<u8>> {
        let mut buf = vec![0u8; len];
        reader.read_exact(&mut buf)?;
        Ok(buf)
    }

    /// Skip bytes forward
    pub fn skip<R: Read + Seek>(reader: &mut R, len: i64) -> IoResult<()> {
        reader.seek(SeekFrom::Current(len))?;
        Ok(())
    }

    /// Align current position to a 4-byte boundary
    pub fn align4<R: Read + Seek>(reader: &mut R) -> IoResult<()> {
        let pos = reader.stream_position()? as usize;
        let pad = Self::calculate_padding(pos);
        if pad > 0 {
            reader.seek(SeekFrom::Current(pad as i64))?;
        }
        Ok(())
    }

    /// Read an ASCII/UTF-8 string of exactly `len` bytes (no null trimming)
    pub fn read_string_exact<R: Read>(reader: &mut R, len: usize) -> IoResult<String> {
        let bytes = Self::read_bytes(reader, len)?;
        Ok(String::from_utf8_lossy(&bytes).to_string())
    }

    /// Read a length-prefixed string where the prefix includes the null terminator.
    ///
    /// This matches the C# pattern:
    /// - read `len_with_null` as u8
    /// - read `len_with_null - 1` bytes as content
    /// - skip the null terminator
    pub fn read_len_u8_string<R: Read>(reader: &mut R) -> IoResult<String> {
        let len_with_null = Self::read_u8(reader)? as usize;
        if len_with_null == 0 {
            return Ok(String::new());
        }
        let content_len = len_with_null.saturating_sub(1);
        let content = Self::read_string_exact(reader, content_len)?;
        // Consume null terminator if present
        let mut nul = [0u8; 1];
        reader.read_exact(&mut nul)?;
        Ok(content)
    }

    /// Validate magic number
    pub fn assert_magic<R: Read>(reader: &mut R, expected: &[u8]) -> Result<(), Nus3bankError> {
        let mut buffer = vec![0u8; expected.len()];
        reader.read_exact(&mut buffer)?;
        if buffer != expected {
            return Err(Nus3bankError::InvalidMagic {
                expected: String::from_utf8_lossy(expected).to_string(),
                found: String::from_utf8_lossy(&buffer).to_string(),
            });
        }
        Ok(())
    }

    /// Calculate 4-byte alignment padding
    pub fn calculate_padding(size: usize) -> usize {
        (4 - (size % 4)) % 4
    }

    /// Read section magic bytes
    pub fn read_section_magic<R: Read>(reader: &mut R) -> Result<[u8; 4], Nus3bankError> {
        let mut magic = [0u8; 4];
        reader.read_exact(&mut magic)?;
        Ok(magic)
    }

    /// Get current position (helper for readers with Seek)
    pub fn get_current_position<R: Read + Seek>(reader: &mut R) -> Result<u32, Nus3bankError> {
        Ok(reader.stream_position()? as u32)
    }

    /// Write 32-bit little-endian unsigned integer
    pub fn write_u32_le(value: u32) -> [u8; 4] {
        value.to_le_bytes()
    }

    /// Write 16-bit little-endian unsigned integer
    pub fn write_u16_le(value: u16) -> [u8; 2] {
        value.to_le_bytes()
    }

    /// Write 32-bit little-endian signed integer
    pub fn write_i32_le(value: i32) -> [u8; 4] {
        value.to_le_bytes()
    }

    /// Write 32-bit little-endian float
    pub fn write_f32_le(value: f32) -> [u8; 4] {
        value.to_le_bytes()
    }

    /// Debug helper: read and display hex dump of data at current position
    pub fn debug_hex_dump<R: Read + Seek>(
        reader: &mut R,
        size: usize,
        label: &str,
    ) -> Result<(), Nus3bankError> {
        let current_pos = reader.stream_position()? as u32;
        let mut buffer = vec![0u8; size];

        match reader.read_exact(&mut buffer) {
            Ok(_) => {
                println!(
                    "=== HEX DUMP: {} (at offset 0x{:08X}, {} bytes) ===",
                    label, current_pos, size
                );
                for (i, chunk) in buffer.chunks(16).enumerate() {
                    print!("{:08X}  ", current_pos + (i * 16) as u32);

                    // Print hex bytes
                    for (j, byte) in chunk.iter().enumerate() {
                        if j == 8 {
                            print!(" ");
                        }
                        print!("{:02X} ", byte);
                    }

                    // Pad if last line is shorter
                    if chunk.len() < 16 {
                        for j in chunk.len()..16 {
                            if j == 8 {
                                print!(" ");
                            }
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
    pub fn peek_hex_dump<R: Read + Seek>(
        reader: &mut R,
        size: usize,
        label: &str,
    ) -> Result<(), Nus3bankError> {
        let current_pos = reader.stream_position()?;
        let result = Self::debug_hex_dump(reader, size, label);
        reader.seek(SeekFrom::Start(current_pos))?;
        result
    }
}
