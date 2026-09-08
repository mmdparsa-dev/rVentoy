use std::fs::File;
use std::io::{BufReader, Cursor};

/// Decompresses an XZ byte slice into a Vec<u8> using `lzma-rs`.
pub fn decompress_xz(input: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut reader = Cursor::new(input);
    let mut output = Vec::new();
    lzma_rs::xz_decompress(&mut reader, &mut output)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
    Ok(output)
}

/// Decompresses an XZ file from disk.
pub fn decompress_xz_file(path: &str) -> std::io::Result<Vec<u8>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut output = Vec::new();
    lzma_rs::xz_decompress(&mut reader, &mut output)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
    Ok(output)
}
