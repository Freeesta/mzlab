//! The synthetic mzML of the `quant.targets` tests (engine and CLI): MS1 centroid scans every 0.05 min, Gaussian ions on a 100 +- 20 baseline
//! from a fixed-seed generator, so every run writes the same bytes; `sparse` gives whole ion counts with empty scans (weak low-resolution data).

pub const SCANS: usize = 200;
/// Minutes per scan.
pub const STEP: f64 = 0.05;

fn b64(data: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::new();
    for c in data.chunks(3) {
        let n = (u32::from(c[0]) << 16)
            | (u32::from(*c.get(1).unwrap_or(&0)) << 8)
            | u32::from(*c.get(2).unwrap_or(&0));
        for i in 0..4 {
            s.push(if i <= c.len() {
                T[(n >> (18 - 6 * i) & 63) as usize] as char
            } else {
                '='
            });
        }
    }
    s
}

fn array(acc: &str, v: &[f64]) -> String {
    let raw: Vec<u8> = v.iter().flat_map(|x| x.to_le_bytes()).collect();
    format!(
        r#"<binaryDataArray encodedLength="0"><cvParam cvRef="MS" accession="MS:1000523" name="64-bit float"/><cvParam cvRef="MS" accession="MS:1000576" name="no compression"/><cvParam cvRef="MS" accession="{acc}" name=""/><binary>{}</binary></binaryDataArray>"#,
        b64(&raw)
    )
}

/// An ion: observed m/z, apex RT (min), height (0 = noise only), sigma (min).
#[derive(Clone, Copy)]
pub struct Ion(pub f64, pub f64, pub f64, pub f64);

/// A deterministic noise in [-1, 1).
fn noise(state: &mut u64) -> f64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    ((*state >> 11) as f64 / (1u64 << 53) as f64) * 2.0 - 1.0
}

/// An mzML file of MS1 centroid scans, one `(m/z, intensities)` pair per scan at RT 0, 0.05, ... min (a scan may hold no peak), all positive.
pub fn mzml_of(scans: &[(Vec<f64>, Vec<f64>)]) -> Vec<u8> {
    mzml_pol(scans, &vec![1; scans.len()])
}

/// As `mzml_of`, with the polarity of every scan (1 positive, -1 negative): a polarity-switching run.
pub fn mzml_pol(scans: &[(Vec<f64>, Vec<f64>)], pols: &[i8]) -> Vec<u8> {
    let mut x = String::from(
        r#"<?xml version="1.0" encoding="utf-8"?><mzML xmlns="http://psi.ms.info/ms/mzml"><run id="r"><spectrumList count=""#,
    );
    x += &format!("{}\">", scans.len());
    for (i, (mz, y)) in scans.iter().enumerate() {
        let rt = i as f64 * STEP;
        x += &format!(
            r#"<spectrum index="{i}" id="scan={}" defaultArrayLength="{}"><cvParam cvRef="MS" accession="MS:1000511" name="ms level" value="1"/><cvParam cvRef="MS" accession="MS:1000127" name="centroid spectrum"/>{}<scanList count="1"><scan><cvParam cvRef="MS" accession="MS:1000016" name="scan start time" value="{rt}" unitCvRef="UO" unitAccession="UO:0000031" unitName="minute"/></scan></scanList><binaryDataArrayList count="2">{}{}</binaryDataArrayList></spectrum>"#,
            i + 1,
            mz.len(),
            if pols[i] < 0 {
                r#"<cvParam cvRef="MS" accession="MS:1000129" name="negative scan"/>"#
            } else {
                r#"<cvParam cvRef="MS" accession="MS:1000130" name="positive scan"/>"#
            },
            array("MS:1000514", mz),
            array("MS:1000515", y)
        );
    }
    x += "</spectrumList></run></mzML>";
    x.into_bytes()
}

/// An mzML file whose MS1 scans (RT 0, 0.05, ... min) hold every ion on a baseline of 100 +- 20 counts.
pub fn mzml(ions: &[Ion], seed: u64) -> Vec<u8> {
    let mut ions = ions.to_vec();
    ions.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut st = seed;
    let scans: Vec<(Vec<f64>, Vec<f64>)> = (0..SCANS)
        .map(|i| {
            let rt = i as f64 * STEP;
            let mz: Vec<f64> = ions.iter().map(|o| o.0).collect();
            let y: Vec<f64> = ions
                .iter()
                .map(|o| {
                    let g = if o.2 > 0.0 {
                        o.2 * (-(rt - o.1).powi(2) / (2.0 * o.3 * o.3)).exp()
                    } else {
                        0.0
                    };
                    100.0 + 20.0 * noise(&mut st) + g
                })
                .collect();
            (mz, y)
        })
        .collect();
    mzml_of(&scans)
}

/// Sparse unit-resolution ion counts, as a weak single-ion trace on a low-resolution instrument gives: whole counts, no baseline, a peak only
/// where an ion arrived (most scans hold nothing at all). Each ion is (m/z, apex RT, apex counts, sigma); a lone count of 1 lands at `stray`
/// m/z on about one scan in eight.
pub fn sparse(ions: &[Ion], stray: f64, seed: u64) -> Vec<u8> {
    let mut st = seed;
    let scans: Vec<(Vec<f64>, Vec<f64>)> = (0..SCANS)
        .map(|i| {
            let rt = i as f64 * STEP;
            let mut peaks: Vec<(f64, f64)> = Vec::new();
            for o in ions {
                let g = o.2 * (-(rt - o.1).powi(2) / (2.0 * o.3 * o.3)).exp();
                let c = (g + 0.5 * noise(&mut st)).round();
                if c >= 1.0 {
                    peaks.push((o.0, c));
                }
            }
            if noise(&mut st) > 0.75 {
                peaks.push((stray, 1.0));
            }
            peaks.sort_by(|a, b| a.0.total_cmp(&b.0));
            peaks.into_iter().unzip()
        })
        .collect();
    mzml_of(&scans)
}

/// The sample: a clean peak, a small peak, a peak on the window edge, a shifted ion, the shifted standard, a noise-only m/z.
pub fn sample() -> Vec<Ion> {
    vec![
        Ion(200.0, 5.0, 20000.0, 0.1),
        Ion(220.0, 5.0, 20.0, 0.1),
        Ion(240.0, 6.0, 20000.0, 0.1),
        Ion(250.3, 3.0, 15000.0, 0.1),
        Ion(400.3, 2.0, 50000.0, 0.1),
        Ion(300.0, 0.0, 0.0, 0.1),
    ]
}

pub const TARGETS: &str = "id,mz,rt_min,rt_max,polarity,role\n\
clean,200.0,4.0,6.0,+,\n\
small,220.0,4.0,6.0,+,\n\
edge,240.0,4.0,6.0,+,\n\
shifted,250.0,2.5,3.5,+,\n\
std,400.0,1.5,2.5,+,standard\n\
empty,300.0,4.0,6.0,+,\n";
