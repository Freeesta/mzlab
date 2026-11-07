//! A scan data packet decoded from bytes held in memory, with the same decoders the file reader uses (`ScanDataPacket::read`, `read_peaks_only`): the benches
//! and tests of other crates build packets and need the decoding without a whole `.raw` container around them.
use std::io::Cursor;

use crate::error::Result;
use crate::reader::BinaryReader;
use crate::scan_data::{Peak, ScanDataPacket};

/// The whole packet: profile chunks, centroid list, resolutions and noise nodes.
pub fn decodifica(bytes: &[u8]) -> Result<ScanDataPacket> {
    ScanDataPacket::read(&mut BinaryReader::new(Cursor::new(bytes)))
}

/// The centroid list alone, the profile skipped (what `extract_arrays` reads for a scan that has a list).
pub fn decodifica_picchi(bytes: &[u8]) -> Result<Vec<Peak>> {
    ScanDataPacket::read_peaks_only(&mut BinaryReader::new(Cursor::new(bytes)))
}

/// The profile alone appended to `mz` and `int` as m/z and intensity (what `extract_arrays` reads for a profile scan); `Ok(false)`: the packet has no profile.
pub fn decodifica_profilo(
    bytes: &[u8],
    coefficients: &[f64],
    mz: &mut Vec<f64>,
    int: &mut Vec<f32>,
) -> Result<bool> {
    ScanDataPacket::read_profile_into(
        &mut BinaryReader::new(Cursor::new(bytes)),
        coefficients,
        mz,
        int,
    )
}
