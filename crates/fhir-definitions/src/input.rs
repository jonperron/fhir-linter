//! Bounded reads of untrusted input files.

use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

/// Largest single input file read (64 MiB), so one huge file cannot exhaust memory.
pub const MAX_INPUT_BYTES: u64 = 64 * 1024 * 1024;

/// Read `path` whole, refusing a file over `cap` bytes with an
/// [`io::ErrorKind::FileTooLarge`] error before it is loaded. The read itself is
/// bounded, so a file that grows after the size check cannot slip past the limit.
pub fn read_capped(path: &Path, cap: u64) -> io::Result<Vec<u8>> {
    let too_large = || {
        io::Error::new(
            io::ErrorKind::FileTooLarge,
            format!(
                "file is larger than the {} MiB input limit",
                cap / (1024 * 1024)
            ),
        )
    };
    let file = File::open(path)?;
    let len = file.metadata()?.len();
    if len > cap {
        return Err(too_large());
    }
    // Sized up front so the read does not double the allocation of a large file.
    let mut bytes = Vec::with_capacity(len as usize);
    file.take(cap + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > cap {
        return Err(too_large());
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_over_cap_is_refused_and_file_at_cap_is_read() {
        let path = std::env::temp_dir().join(format!("{}_read_capped.json", std::process::id()));
        std::fs::write(&path, b"12345").unwrap();
        let at_cap = read_capped(&path, 5);
        let over_cap = read_capped(&path, 4);
        let _ = std::fs::remove_file(&path);
        assert_eq!(at_cap.unwrap(), b"12345");
        assert_eq!(over_cap.unwrap_err().kind(), io::ErrorKind::FileTooLarge);
    }
}
