//! `analysis.tsf` + `analysis.tsf_bin` (timsTOF fleX MALDI, TIMS off): one block per frame at `Frames.TimsId`, `u32` size of the profile, `u32` size of the line spectrum, then the line
//! spectrum (zstd: `NumPeaks` `f64` TOF positions and as many `f32` intensities) and the profile (zstd: `DigitizerNumSamples` `u32` samples). Same tables and calibration as the TDF.

use super::block::zstd;

use mzlab_core::error::{Error, Result};

use mzlab_core::idxread::ReadAt;

use mzlab_core::readers::Limits;

fn broken(m: &str) -> Error {
    Error::new("err.file.brukerDamaged").with("detail", format!("tsf_bin: {m}"))
}

/// A frame as TOF positions and intensities; `profilo` when there was no line spectrum and the samples of the profile stand in.
pub struct Lines {
    pub tof: Vec<f64>,
    pub int: Vec<f64>,
    pub profile: bool,
}

fn u32_le(b: &[u8]) -> u32 {
    u32::from_le_bytes([b[0], b[1], b[2], b[3]])
}

pub fn read_frame(
    d: &mut dyn ReadAt,
    dim: u64,
    pos: u64,
    peaks: u64,
    samples: u64,
    limits: &Limits,
) -> Result<Lines> {
    let mut h = [0u8; 8];
    if pos.checked_add(8).is_none_or(|e| e > dim) {
        return Err(broken("block past the end of the file"));
    }
    d.read_at(pos, &mut h)?;
    let (prof, row) = (u64::from(u32_le(&h)), u64::from(u32_le(&h[4..])));
    if prof.saturating_add(row).saturating_add(pos + 8) > dim || prof.max(row) > limits.byte {
        return Err(broken("block length"));
    }
    let read = |d: &mut dyn ReadAt, da: u64, n: u64| -> Result<Vec<u8>> {
        let mut b = vec![0u8; n as usize];
        d.read_at(da, &mut b)?;
        Ok(b)
    };
    if row > 0 && peaks > 0 {
        let n = limits.ok_entries(peaks)?;
        let z = read(d, pos + 8, row)?;
        return lines(&z, n, limits);
    }
    if prof == 0 {
        return Ok(Lines {
            tof: Vec::new(),
            int: Vec::new(),
            profile: false,
        });
    }
    let n = limits.ok_entries(samples)?;
    let z = read(d, pos + 8 + row, prof)?;
    profilo(&z, n, limits)
}

/// The line spectrum of a block: `n` TOF positions (f64) then `n` intensities (f32).
fn lines(z: &[u8], n: usize, limits: &Limits) -> Result<Lines> {
    let b = zstd(z, n.checked_mul(12).ok_or_else(|| broken("peaks"))?, limits)?;
    let tof = (0..n)
        .map(|i| f64::from_le_bytes(b[8 * i..8 * i + 8].try_into().unwrap_or([0; 8])))
        .collect();
    let o = 8 * n;
    let int = (0..n)
        .map(|i| {
            f64::from(f32::from_le_bytes(
                b[o + 4 * i..o + 4 * i + 4].try_into().unwrap_or([0; 4]),
            ))
        })
        .collect();
    Ok(Lines {
        tof,
        int,
        profile: false,
    })
}

/// The profile of a block: `n` samples (u32), kept where there is signal or a neighbour has it (the rest is zero).
fn profilo(z: &[u8], n: usize, limits: &Limits) -> Result<Lines> {
    let b = zstd(
        z,
        n.checked_mul(4).ok_or_else(|| broken("samples"))?,
        limits,
    )?;
    let v: Vec<u32> = b.chunks_exact(4).map(u32_le).collect();
    let (mut tof, mut int) = (Vec::new(), Vec::new());
    for i in 0..v.len() {
        if v[i] > 0 || v.get(i + 1).is_some_and(|&x| x > 0) || (i > 0 && v[i - 1] > 0) {
            tof.push(i as f64);
            int.push(f64::from(v[i]));
        }
    }
    Ok(Lines {
        tof,
        int,
        profile: true,
    })
}

#[cfg(test)]
mod prove {
    use super::*;

    #[test]
    fn block_past_the_end() {
        let mut b = vec![0u8; 16];
        b[0] = 0xff;
        b[1] = 0xff;
        assert!(read_frame(&mut b.clone(), 16, 0, 1, 1, &Limits::sito()).is_err());
    }
}
