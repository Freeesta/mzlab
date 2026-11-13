//! The LC/MS part of the Agilent reader (feature `estesi`): members read besides the data, calibration, method report, and the `variante` probe.

use super::*;

/// The profile spectra of a Q-TOF run, where `MSPeak.bin` is empty or absent (feature `estesi`).
pub(super) const PROFILE: &str = "AcqData/MSProfile.bin";
const MASS_CAL: &str = "AcqData/MSMassCal.bin";
const DEFAULT_CAL: &str = "AcqData/DefaultMassCal.xml";
/// The method report of an LC/MS run (feature `estesi`).
const ACQ_METHOD: &str = "AcqData/AcqMethod.xml";
/// The binary members of `AcqData` that are not decoded and are kept as size and SHA-1 when they are small (module traces, actuals, spectra of the DAD…).
const UNDECODED_MAX: u64 = 4 << 20;

/// The members under `AcqData/` (not in a subfolder) that an LC/MS run keeps besides the data: (index, path in the folder). `MSPeak.bin`, `MSProfile.bin` and the
/// files `apri` reads for the data are not among them.
fn lc_extras(fs: &FileSet) -> Vec<(usize, String)> {
    const KNOWN: [&str; 13] = [
        SCAN,
        XSD,
        TS,
        DEVICES,
        CONTENTS,
        TREE,
        PEAK,
        PROFILE,
        MASS_CAL,
        DEFAULT_CAL,
        ACQ_METHOD,
        "AcqData/sample_info.xml",
        "AcqData/sequence.xml",
    ];
    fs.membri
        .iter()
        .enumerate()
        .filter_map(|(i, m)| {
            let path = m.nome.replace('\\', "/");
            let parts: Vec<&str> = path.split('/').collect();
            let at = parts
                .iter()
                .rposition(|p| p.eq_ignore_ascii_case("AcqData"))?;
            let rest = &parts[at + 1..];
            let file = *rest.first().filter(|_| rest.len() == 1)?;
            let rel = format!("AcqData/{file}");
            let ext = file.rsplit_once('.')?.1.to_lowercase();
            let wanted = matches!(
                ext.as_str(),
                "xml" | "ini" | "cd" | "cg" | "sd" | "sp" | "ch" | "uv" | "pdf" | "bin"
            );
            (wanted && !KNOWN.iter().any(|k| k.eq_ignore_ascii_case(&rel))).then_some((i, rel))
        })
        .collect()
}

/// Records looked at by `variante`.
const VARIANT_SCANS: usize = 64;

/// True when the folder is a variant that the engine's reader (the one without `estesi`) opens or refuses but cannot read the points of: an xsd with nested types,
/// or a record whose spectrum is not 16 bytes per point of the GC/MS layout (other formats, profile, compressed). Looks at the head of `MSScan.bin` and at the xsd
/// only. A folder that is not an Agilent `.D` is not a variant.
pub fn variant(fs: &mut FileSet) -> bool {
    let Some(i) = find(fs, SCAN) else {
        return false;
    };
    let layout = match find(fs, XSD) {
        Some(x) if fs.membri[x].dim <= (1 << 20) => {
            let xsd = fs.testa(x, fs.membri[x].dim as usize);
            match crate::agilent::scan::Layout::from_xsd(&String::from_utf8_lossy(&xsd)) {
                Ok(l) => l,
                Err(_) => return true,
            }
        }
        _ => crate::agilent::scan::Layout::builtin(),
    };
    let head = fs.testa(i, crate::agilent::scan::HEADER_LEN);
    let Some(start) = head
        .get(88..92)
        .map(|b| i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .filter(|s| *s >= 92)
    else {
        return false;
    };
    let n = layout.record_len;
    let bytes = fs.testa(i, start as usize + VARIANT_SCANS * n);
    bytes.get(start as usize..).is_some_and(|rest| {
        rest.chunks_exact(n)
            .map(|r| crate::agilent::scan::record(&layout, r))
            .filter(|r| r.points > 0)
            .any(|r| {
                r.uncompressed.is_some_and(|u| u != r.bytes)
                    || r.format != 1
                    || r.bytes as usize != r.points as usize * 16
            })
    })
}

/// What an LC/MS run keeps besides the points, read within the byte budget of the folder.
pub(super) struct LcRead {
    cal: (Option<Vec<u8>>, Option<Vec<u8>>),
    method: Option<Vec<u8>>,
    files: Vec<(String, Vec<u8>)>,
    sizes: Vec<(String, u64)>,
    /// Side members that could not be read (`side`): left out and named in `avvisi`.
    pub(super) unread: Vec<String>,
}

/// The bytes of member `i`, counted against what is left of `Limiti::byte`.
pub(super) fn read_all(fs: &mut FileSet, i: usize, left: &mut u64) -> Result<Vec<u8>> {
    let m = &mut fs.membri[i];
    if m.dim > *left {
        return Err(Error::memory_budget());
    }
    *left -= m.dim;
    let mut b = vec![0u8; m.dim as usize];
    m.dati.read_at(0, &mut b)?;
    Ok(b)
}

/// Side member `n` (not the scan index, its layout or `data.ms`): one that cannot be read (I/O, byte budget) is left out and named in `unread`, never the run.
pub(super) fn side(
    fs: &mut FileSet,
    n: &str,
    left: &mut u64,
    unread: &mut Vec<String>,
) -> Option<Vec<u8>> {
    let i = find(fs, n)?;
    read_all(fs, i, left)
        .map_err(|_| unread.push(n.to_string()))
        .ok()
}

/// Reads the calibration, the method report and the small members of `AcqData` (a binary too large to hash is named by its size).
pub(super) fn read_lc(fs: &mut FileSet, left: &mut u64) -> LcRead {
    let mut unread = Vec::new();
    let cal = (
        side(fs, MASS_CAL, left, &mut unread),
        side(fs, DEFAULT_CAL, left, &mut unread),
    );
    let method = side(fs, ACQ_METHOD, left, &mut unread);
    let mut files = Vec::new();
    let mut sizes = Vec::new();
    for (i, rel) in lc_extras(fs) {
        let dim = fs.membri[i].dim;
        if dim > UNDECODED_MAX || dim > *left {
            sizes.push((rel, dim));
        } else {
            match read_all(fs, i, left) {
                Ok(b) => files.push((rel, b)),
                Err(_) => unread.push(rel),
            }
        }
    }
    LcRead {
        cal,
        method,
        files,
        sizes,
        unread,
    }
}

impl LcRead {
    /// The run with its calibration, method and LC files.
    pub(super) fn apply(
        &self,
        d: AgilentD,
        devices: Option<&[u8]>,
        data_file: &'static str,
    ) -> AgilentD {
        let d = d.with_calibration(self.cal.0.as_deref(), self.cal.1.as_deref());
        let refs: Vec<(&str, &[u8])> = self
            .files
            .iter()
            .map(|(n, b)| (n.as_str(), b.as_slice()))
            .collect();
        let sizes: Vec<(&str, u64)> = self.sizes.iter().map(|(n, s)| (n.as_str(), *s)).collect();
        let mut d = d.with_lc_files(self.method.as_deref(), devices, &refs, &sizes, data_file);
        d.notes.extend(
            self.unread
                .iter()
                .map(|n| format!("{n}: could not be read")),
        );
        d
    }
}
