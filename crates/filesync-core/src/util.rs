//! Small helpers: UTC time formatting without a date/time dependency, random ids,
//! and atomic file writes.

use std::fs;
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};

pub fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Converts Unix seconds to (year, month, day, hour, minute, second) in UTC.
/// Days-from-civil inverse, after Howard Hinnant's algorithm.
pub fn civil(secs: i64) -> (i64, u32, u32, u32, u32, u32) {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    (
        y,
        m,
        d,
        (rem / 3600) as u32,
        ((rem % 3600) / 60) as u32,
        (rem % 60) as u32,
    )
}

/// `YYYYMMDDTHHMMSSZ`, as used by AWS Signature Version 4.
pub fn amz_date(secs: i64) -> String {
    let (y, mo, d, h, mi, s) = civil(secs);
    format!("{y:04}{mo:02}{d:02}T{h:02}{mi:02}{s:02}Z")
}

/// `YYYY-MM-DD HH:MM:SS UTC`, for people.
pub fn human_utc(secs: i64) -> String {
    let (y, mo, d, h, mi, s) = civil(secs);
    format!("{y:04}-{mo:02}-{d:02} {h:02}:{mi:02}:{s:02} UTC")
}

/// `YYYYMMDD-HHMMSS`, for conflict file names.
pub fn compact_utc(secs: i64) -> String {
    let (y, mo, d, h, mi, s) = civil(secs);
    format!("{y:04}{mo:02}{d:02}-{h:02}{mi:02}{s:02}")
}

pub fn random_bytes<const N: usize>() -> [u8; N] {
    let mut b = [0u8; N];
    getrandom::getrandom(&mut b).expect("the OS random number generator failed");
    b
}

/// Writes `data` to `path` atomically: a temporary file in the same directory, flushed to disk,
/// then renamed over the target.
pub fn write_atomic(path: &Path, data: &[u8]) -> Result<()> {
    let dir = path.parent().context("path has no parent directory")?;
    let tmp = dir.join(format!(
        "{}{}.tmp",
        TEMP_PREFIX,
        hex::encode(random_bytes::<6>())
    ));
    let result = (|| {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(data)?;
        f.sync_all()?;
        fs::rename(&tmp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result.with_context(|| format!("writing {}", path.display()))
}

/// Prefix of the client's temporary files. Scans ignore it.
pub const TEMP_PREFIX: &str = ".~filesync-";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_known_dates() {
        assert_eq!(civil(0), (1970, 1, 1, 0, 0, 0));
        assert_eq!(amz_date(1_700_000_000), "20231114T221320Z");
        assert_eq!(civil(951_782_400), (2000, 2, 29, 0, 0, 0));
    }
}
