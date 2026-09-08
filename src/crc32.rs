/// Computes the IEEE CRC-32 checksum, identical to Ventoy's C implementation.
#[inline]
pub fn crc32(buffer: &[u8]) -> u32 {
    let mut hasher = crc32fast::Hasher::new();
    hasher.update(buffer);
    hasher.finalize()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crc32() {
        assert_eq!(crc32(b"123456789"), 0xCBF43926);
    }
}
