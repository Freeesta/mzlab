//! What `RawFileReader::open` does with a file that is not exactly as the format says: a version number nobody documented, a copy that stops early, a stream
//! that cannot be read. Each case opens the file as far as it is sound and says what was left out (`Warning`); a file with nothing sound in it is refused with
//! an error that names the structure, the format version and the byte where it stopped. Used by `reader.rs` and by mzLab's `raw.rs`/`rawlettore.rs`.

use crate::error::Error;
use crate::scan_format::ScanDataFormat;
use crate::scan_index::ScanIndexEntry;

/// The format versions whose layout is documented and checked against real files.
pub const DECLARED_VERSIONS: [u32; 8] = [8, 47, 57, 60, 62, 63, 64, 66];

/// Highest version number that is tried with the nearest declared layout; anything above is a damaged header rather than a newer format.
const MAX_TRIED_VERSION: u32 = 100;

/// What `open` left out or assumed. Order of the file is kept; every entry is one line in `metodo/1.avvisi`.
#[derive(Debug, Clone, PartialEq)]
pub enum Warning {
    /// The file ends inside the scan data: the scans that lie whole before byte `file_len` are in the index, the others are not.
    Truncated {
        file_len: u64,
        /// First byte of the first scan that is not whole.
        cut_at: u64,
        scans_declared: u32,
        scans_readable: u32,
    },
    /// The version in the header is not one of [`DECLARED_VERSIONS`]; the layout of `used` was read and the scan index passed the checks of `plausible_index`.
    UnverifiedVersion { declared: u32, used: u32 },
    /// A structure that is not needed to list and read the scans could not be read; the file opens without it.
    Skipped {
        stage: &'static str,
        offset: u64,
        detail: String,
    },
}

impl Warning {
    /// One line of English for the method document and the diagnosis (`metodo/1.avvisi`, "Copy diagnostics").
    pub fn text(&self) -> String {
        match self {
            Self::Truncated {
                file_len,
                cut_at,
                scans_declared,
                scans_readable,
            } => format!(
                "File cut: {scans_readable} of {scans_declared} scans are whole before byte {file_len} (scan {} starts at byte {cut_at})",
                scans_readable + 1
            ),
            Self::UnverifiedVersion { declared, used } => format!(
                "Format version {declared} is not verified: read with the layout of version {used}"
            ),
            Self::Skipped {
                stage,
                offset,
                detail,
            } => format!("{stage} not read (byte {offset}): {detail}"),
        }
    }
}

/// The documented version whose layout a header version is read with: itself when declared, the nearest declared one for a number between 1 and 100 (a tie
/// goes to the lower), `None` for anything else.
pub fn schema_version(declared: u32) -> Option<u32> {
    if DECLARED_VERSIONS.contains(&declared) {
        return Some(declared);
    }
    if declared == 0 || declared > MAX_TRIED_VERSION {
        return None;
    }
    DECLARED_VERSIONS
        .iter()
        .copied()
        .min_by_key(|v| (v.abs_diff(declared), *v))
}

/// Smallest packet of a scan in the packet layout (its header).
const PACKET_HEADER_BYTES: u64 = 40;

/// First byte after the record of scan `k`, as far as the index says.
/// Flat TSQ v63: the offset is already the end of the record. Flat TSQ v66: the record has `data_size` bytes. Packet layout (`data_size` is not a byte count there):
/// records follow each other, so the next scan starts where this one ends; the last one ends where the nearest stream after it starts (`streams`: the other
/// addresses of the run header), or after its header when no stream follows.
fn record_end(
    index: &[ScanIndexEntry],
    k: usize,
    format: ScanDataFormat,
    data_addr: u64,
    streams: &[u64],
) -> u64 {
    let Some(e) = index.get(k) else {
        return 0;
    };
    let at = data_addr.saturating_add(e.offset);
    match format {
        ScanDataFormat::FlatV63 => at,
        ScanDataFormat::FlatV66 => at.saturating_add(u64::from(e.data_size)),
        ScanDataFormat::PacketHeader => {
            let floor = at.saturating_add(PACKET_HEADER_BYTES);
            match index.get(k + 1) {
                Some(n) => data_addr.saturating_add(n.offset).max(floor),
                None => streams
                    .iter()
                    .copied()
                    .filter(|&a| a >= floor)
                    .min()
                    .unwrap_or(floor),
            }
        }
    }
}

/// How many entries at the start of the index lie whole before `file_len`, and the first byte of the first one that does not (`None` when all do).
pub fn whole_prefix(
    index: &[ScanIndexEntry],
    format: ScanDataFormat,
    data_addr: u64,
    streams: &[u64],
    file_len: u64,
) -> (usize, Option<u64>) {
    for (k, e) in index.iter().enumerate() {
        if record_end(index, k, format, data_addr, streams) > file_len {
            return (k, Some(data_addr.saturating_add(e.offset)));
        }
    }
    (index.len(), None)
}

/// Length of the longest prefix of the index whose times are finite, not negative and never decrease. An index that the file cuts short is followed by bytes of
/// other structures that read as entries; they do not keep the order.
pub fn ordered_prefix(index: &[ScanIndexEntry]) -> usize {
    let mut last = 0.0f64;
    for (k, e) in index.iter().enumerate() {
        if !e.start_time.is_finite() || e.start_time < last {
            return k;
        }
        last = e.start_time;
    }
    index.len()
}

/// The checks that stand in for a documented layout when the version is not documented: times finite, not negative and non-decreasing over the whole index,
/// and every scan record starting inside the file.
pub fn plausible_index(index: &[ScanIndexEntry], data_addr: u64, file_len: u64) -> bool {
    let (Some(first), Some(last)) = (index.first(), index.last()) else {
        return false;
    };
    let times_ok = index
        .windows(2)
        .all(|w| matches!(w, [a, b] if a.start_time.is_finite() && b.start_time.is_finite() && a.start_time >= 0.0 && b.start_time >= a.start_time));
    times_ok
        && first.start_time.is_finite()
        && last.start_time.is_finite()
        && data_addr.saturating_add(first.offset) <= file_len
        && data_addr.saturating_add(last.offset) <= file_len
}

/// An error of a structure the file cannot do without, with the structure and the version in front of the reason. Errors that already say what is wrong
/// with the file as a whole (signature, version, a cut) pass through; a stream that ends inside the structure is a cut (`Error::Truncated`).
pub fn stage_error(stage: &'static str, version: u32, file_len: u64, e: Error) -> Error {
    match e {
        // The stream ended inside the structure: a cut file, whatever the structure.
        Error::UnexpectedEof { offset, needed } => Error::Truncated {
            stage,
            file_len,
            needed: offset.saturating_add(needed as u64),
            readable: 0,
            declared: 0,
        },
        // A fixed-size structure that the remaining bytes cannot hold is cut too (the reader checks the size before it reads).
        Error::AllocationTooLarge {
            offset, requested, ..
        } if matches!(stage, "FileHeader" | "RunHeader")
            && offset.saturating_add(requested) > file_len =>
        {
            Error::Truncated {
                stage,
                file_len,
                needed: offset.saturating_add(requested),
                readable: 0,
                declared: 0,
            }
        }
        Error::BadMagic(_)
        | Error::BadSignature(_)
        | Error::UnsupportedVersion(_)
        | Error::Truncated { .. }
        | Error::Stage { .. } => e,
        source => Error::Stage {
            stage,
            version,
            file_len,
            source: Box::new(source),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(t: f64, offset: u64, size: u32) -> ScanIndexEntry {
        ScanIndexEntry {
            index: 0,
            scan_event: 1,
            scan_segment: 1,
            data_size: size,
            start_time: t,
            total_current: 0.0,
            base_intensity: 0.0,
            base_mz: 0.0,
            low_mz: 0.0,
            high_mz: 0.0,
            offset,
        }
    }

    #[test]
    fn declared_versions_map_to_themselves_and_others_to_the_nearest() {
        for v in DECLARED_VERSIONS {
            assert_eq!(schema_version(v), Some(v));
        }
        assert_eq!(schema_version(65), Some(64));
        assert_eq!(schema_version(67), Some(66));
        assert_eq!(schema_version(99), Some(66));
        assert_eq!(schema_version(53), Some(57));
        assert_eq!(schema_version(52), Some(47));
        assert_eq!(schema_version(1), Some(8));
        assert_eq!(schema_version(0), None);
        assert_eq!(schema_version(101), None);
        assert_eq!(schema_version(u32::MAX), None);
    }

    #[test]
    fn whole_prefix_stops_at_the_first_scan_that_does_not_fit() {
        let ix: Vec<_> = (0..5).map(|k| entry(k as f64, k * 100, 100)).collect();
        let f = ScanDataFormat::PacketHeader;
        // the last record ends where the nearest later stream starts
        assert_eq!(whole_prefix(&ix, f, 1000, &[1500, 9999], 1500), (5, None));
        assert_eq!(whole_prefix(&ix, f, 1000, &[1500], 1499), (4, Some(1400)));
        assert_eq!(whole_prefix(&ix, f, 1000, &[1500], 1299), (2, Some(1200)));
        assert_eq!(whole_prefix(&ix, f, 1000, &[1500], 999), (0, Some(1000)));
        // no stream after the data: the last record is whole when its header is
        assert_eq!(whole_prefix(&ix, f, 1000, &[], 1440), (5, None));
        assert_eq!(whole_prefix(&ix, f, 1000, &[], 1439), (4, Some(1400)));
        // flat v63: the offset is the end of the record
        let flat: Vec<_> = (0..3).map(|k| entry(k as f64, (k + 1) * 50, 2)).collect();
        assert_eq!(
            whole_prefix(&flat, ScanDataFormat::FlatV63, 100, &[], 200),
            (2, Some(250))
        );
    }

    #[test]
    fn the_ordered_prefix_ends_where_the_times_stop_rising() {
        let ix = vec![entry(0.1, 0, 1), entry(0.2, 1, 1), entry(0.0, 2, 1)];
        assert_eq!(ordered_prefix(&ix), 2);
        assert_eq!(ordered_prefix(&ix[..2]), 2);
        assert_eq!(ordered_prefix(&[entry(f64::NAN, 0, 1)]), 0);
        assert_eq!(ordered_prefix(&[]), 0);
    }

    #[test]
    fn an_index_that_runs_backwards_or_leaves_the_file_is_not_plausible() {
        let good = vec![entry(0.1, 0, 10), entry(0.2, 10, 10)];
        assert!(plausible_index(&good, 0, 100));
        assert!(!plausible_index(&[], 0, 100));
        assert!(!plausible_index(&good, 0, 5));
        let back = vec![entry(0.2, 0, 10), entry(0.1, 10, 10)];
        assert!(!plausible_index(&back, 0, 100));
        let nan = vec![entry(f64::NAN, 0, 10)];
        assert!(!plausible_index(&nan, 0, 100));
    }
}
