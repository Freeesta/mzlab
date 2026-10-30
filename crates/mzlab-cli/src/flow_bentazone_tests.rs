//! Bentazone (C10H12N2O3S, a public standard) through the whole Tabui chain of mzFlow in both polarities and both resolutions, on small invented series
//! written here (a parent that fades, two products that grow, a constant background; the names `BEN_T2p5` check the time with a `p`). The polarity and the
//! unit mass of the file reach the steps: the parent ion is [M-H]- 239.050 or [M+H]+ 241.064 (nominal 239 / 241 at unit resolution).

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use mzlab_core::campioni::{proponi, to_json};
use mzlab_feat::tp::{ST_PARENT, ST_TP};

pub(crate) const FORMULA: &str = "C10H12N2O3S";
pub(crate) const SMILES: &str = "CC(C)N1C(=O)C2=CC=CC=C2NS1(=O)=O";
const PROTON: f64 = 1.007_276;
pub(crate) const NAMES: [&str; 5] = ["BEN_blank", "BEN_T00", "BEN_T2p5", "BEN_T5", "BEN_T10p25"];
/// Heights of (parent, +O, -C3H6) in each file.
const HEIGHTS: [[f64; 3]; 5] = [
    [0.0, 0.0, 0.0],
    [1e6, 0.0, 0.0],
    [7e5, 2e5, 1e5],
    [4e5, 4e5, 2e5],
    [1e5, 3e5, 3e5],
];
/// Neutral masses and RTs of the parent, the hydroxylated product and the one that lost propene.
const ION: [(f64, f64); 3] = [(240.0568, 5.0), (256.0517, 4.3), (198.0098, 3.6)];

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

pub(crate) fn array(acc: &str, v: &[f64]) -> String {
    let raw: Vec<u8> = v.iter().flat_map(|x| x.to_le_bytes()).collect();
    format!(
        r#"<binaryDataArray encodedLength="0"><cvParam cvRef="MS" accession="MS:1000523" name="64-bit float"/><cvParam cvRef="MS" accession="MS:1000576" name="no compression"/><cvParam cvRef="MS" accession="{acc}" name=""/><binary>{}</binary></binaryDataArray>"#,
        b64(&raw)
    )
}

/// A deterministic pseudo-random number in [0, 1).
fn rnd(seed: &mut u64) -> f64 {
    *seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    (*seed >> 11) as f64 / (1u64 << 53) as f64
}

/// One centroid mzML: `file` is the column of `HEIGHTS`, `pol` ±1, `hr` the analyzer written in the scan filter (the instrument decides the recipe).
pub(crate) fn mzml(file: usize, pol: i8, hr: bool) -> String {
    let mut x = String::from(
        r#"<?xml version="1.0" encoding="utf-8"?><mzML xmlns="http://psi.ms.info/ms/mzml"><run id="r"><spectrumList count="400">"#,
    );
    let mut seed = 7 + file as u64;
    let (acc, sign) = if pol < 0 {
        ("MS:1000129", "-")
    } else {
        ("MS:1000130", "+")
    };
    let filter = if hr {
        format!("FTMS {sign} p ESI Full ms [50.00-500.00]")
    } else {
        format!("ITMS {sign} c ESI Full ms [50.00-500.00]")
    };
    for i in 0..400 {
        let rt = i as f64 * 0.02;
        let (mut mz, mut y) = (Vec::new(), Vec::new());
        let mut add = |m: f64, h: f64, seed: &mut u64| {
            // unit resolution: the accuracy is a tenth of a Da, as a triple quadrupole gives
            let e = if hr {
                0.0
            } else {
                0.05 + 0.04 * (rnd(seed) - 0.5)
            };
            mz.push(m + e);
            y.push(h);
        };
        for (k, (neutral, t)) in ION.iter().enumerate() {
            let h = HEIGHTS[file][k] * (-((rt - t) / 0.08).powi(2) / 2.0).exp();
            if h < 1e3 {
                continue;
            }
            let ion = neutral + f64::from(pol) * PROTON;
            add(ion, h, &mut seed);
            add(ion + 1.00335, h * 0.14, &mut seed);
            add(ion + 1.9958, h * 0.06, &mut seed);
        }
        add(
            300.1234,
            4e5 * (-((rt - 2.0) / 0.1).powi(2) / 2.0).exp() + 1e3,
            &mut seed,
        );
        for _ in 0..10 {
            let m = 100.0 + 300.0 * rnd(&mut seed);
            add(m, 500.0 + 1500.0 * rnd(&mut seed), &mut seed);
        }
        let mut o: Vec<usize> = (0..mz.len()).collect();
        o.sort_by(|a, b| mz[*a].total_cmp(&mz[*b]));
        let (mz, y): (Vec<f64>, Vec<f64>) = o.iter().map(|&j| (mz[j], y[j])).unzip();
        let _ = write!(
            x,
            r#"<spectrum index="{i}" id="scan={}" defaultArrayLength="{}"><cvParam cvRef="MS" accession="MS:1000511" name="ms level" value="1"/><cvParam cvRef="MS" accession="MS:1000127" name="centroid spectrum"/><cvParam cvRef="MS" accession="{acc}" name="scan"/><cvParam cvRef="MS" accession="MS:1000512" name="filter string" value="{filter}"/><scanList count="1"><scan><cvParam cvRef="MS" accession="MS:1000016" name="scan start time" value="{rt}" unitCvRef="UO" unitAccession="UO:0000031" unitName="minute"/></scan></scanList><binaryDataArrayList count="2">{}{}</binaryDataArrayList></spectrum>"#,
            i + 1,
            mz.len(),
            array("MS:1000514", &mz),
            array("MS:1000515", &y)
        );
    }
    x + "</spectrumList></run></mzML>"
}

/// The run folder: the five mzML, the samples, the parent and the work folder.
pub(crate) fn fixture(tag: &str, pol: i8, hr: bool) -> PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab_cli_ben_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("work")).unwrap();
    for (k, n) in NAMES.iter().enumerate() {
        std::fs::write(d.join(format!("{n}.mzML")), mzml(k, pol, hr)).unwrap();
    }
    let files: Vec<_> = NAMES
        .iter()
        .map(|n| (format!("{n}.mzML"), String::new(), String::new()))
        .collect();
    std::fs::write(d.join("samples.json"), to_json(&proponi(&files))).unwrap();
    std::fs::write(
        d.join("parent.json"),
        format!(r#"{{"name":"bentazone","formula":"{FORMULA}","smiles":"{SMILES}"}}"#),
    )
    .unwrap();
    d
}

/// The chain of the page: base chain, hypotheses, link, filters, positions, network (and the Dig at unit resolution). `params` sets a step's parameters.
pub(crate) fn plan(params: &dyn Fn(&str) -> String, lr: bool) -> String {
    let raw: Vec<String> = NAMES
        .iter()
        .map(|n| format!(r#"{{"name":"{n}.mzML","type":"raw"}}"#))
        .collect();
    let src: Vec<String> = NAMES
        .iter()
        .map(|n| format!(r#""file:{n}.mzML""#))
        .collect();
    let p = |id: &str| {
        let v = params(id);
        if v.is_empty() {
            String::new()
        } else {
            format!(r#","params":{v}"#)
        }
    };
    let dig = if lr {
        r#",{"id":"dig","step":"tp.dig_lr@1","in":{"table":"gap.table","samples":"file:samples.json","parent":"file:parent.json","index":["idx.index"]}}"#
    } else {
        ""
    };
    format!(
        r#"{{"format":"mzflow/2","inputs":[{},{{"name":"samples.json","type":"samples"}},{{"name":"parent.json","type":"structure"}}],"steps":[
        {{"id":"idx","step":"index.mzidx@1","in":{{"source":[{}]}}}},
        {{"id":"det","step":"features.detect@1","in":{{"index":["idx.index"]}}}},
        {{"id":"ali","step":"features.align@1","in":{{"peaks":["det.peaks"]}}}},
        {{"id":"gap","step":"features.gapfill@1","in":{{"table":"ali.table","index":["idx.index"]}}}},
        {{"id":"gen","step":"tp.generate@1","in":{{"parent":"file:parent.json","index":["idx.index"]}}{}}},
        {{"id":"lnk","step":"tp.link@2","in":{{"table":"gap.table","samples":"file:samples.json","parent":"file:parent.json","index":["idx.index"],"hypotheses":"gen.hypotheses"}}{}}},
        {{"id":"flt","step":"tp.filter@2","in":{{"tp":"lnk.tp","hypotheses":"gen.hypotheses"}}}},
        {{"id":"pos","step":"tp.position@1","in":{{"tp":"lnk.tp","parent":"file:parent.json"}}}},
        {{"id":"net","step":"tp.network@1","in":{{"tp":"lnk.tp","table":"lnk.tp","samples":"file:samples.json"}}}}{dig}]}}"#,
        raw.join(","),
        src.join(","),
        p("gen"),
        p("lnk")
    )
}

pub(crate) fn run(d: &Path, text: &str) -> Result<String, String> {
    let (done, prov) = crate::flow::run_plan(text.as_bytes(), d, &d.join("work"), 2, None)
        .map_err(|e| format!("{} {:?}", e.key, e.params))?;
    if !done.skipped.is_empty() || prov.contains("\"error\"") {
        let at = prov.find("\"error\"").unwrap_or(0);
        return Err(format!(
            "skipped {:?}: {}",
            done.skipped,
            &prov[at..prov.len().min(at + 300)]
        ));
    }
    Ok(prov)
}

fn table(d: &Path) -> mzlab_feat::table::Aligned {
    let bytes = std::fs::read(d.join("work/lnk.mzfeat")).unwrap();
    mzlab_feat::store::read_table_from(&mut mzlab_feat::store::Slice(&bytes), bytes.len() as u64)
        .unwrap()
        .0
}

/// One case: the plan with the polarity NOT written in it (the files decide), the parent row at the m/z of the ion, the hydroxylated product accepted.
fn case(tag: &str, pol: i8, hr: bool) {
    let d = fixture(tag, pol, hr);
    let prov = run(&d, &plan(&|_| String::new(), !hr)).unwrap();
    // the files decide: the plan writes neither the polarity nor the unit resolution of the hypotheses
    let gen = &prov[prov.find("\"gen\":{").unwrap()..];
    let gen = &gen[..gen.find("\"params_sha256\"").unwrap()];
    assert!(gen.contains(&format!("\"nominal\":{}", !hr)), "{gen}");
    assert!(gen.contains(&format!("\"polarity\":{pol}")), "{gen}");
    let hyp = std::fs::read_to_string(d.join("work/gen.json")).unwrap();
    let ion = |m: f64| m + f64::from(pol) * PROTON;
    let adduct = if pol < 0 { "[M-H]-" } else { "[M+H]+" };
    assert!(
        hyp.contains(adduct),
        "hypotheses without {adduct}: {}",
        &hyp[..hyp.len().min(400)]
    );
    let t = table(&d);
    let tol = if hr { 0.01 } else { 0.6 };
    assert_eq!(t.tp.status.iter().filter(|&&s| s == ST_PARENT).count(), 1);
    let parent = t.tp.parent_row as usize;
    assert!(
        (t.mz[parent] - ion(ION[0].0)).abs() < tol,
        "{} vs {}",
        t.mz[parent],
        ion(ION[0].0)
    );
    let hydroxy =
        (0..t.mz.len()).any(|r| t.tp.status[r] == ST_TP && (t.mz[r] - ion(ION[1].0)).abs() < tol);
    let rows: Vec<String> = (0..t.mz.len())
        .map(|r| format!("{:.3}:{}", t.mz[r], t.tp.status[r]))
        .collect();
    assert!(
        hydroxy,
        "no accepted +O product at {}; rows {rows:?} near {:?}",
        ion(ION[1].0),
        hyp.split("{\"id\"")
            .filter(|h| h.contains("\"mz\":257.0")
                || h.contains("\"mz\":256.9")
                || h.contains("\"mz\":257.1"))
            .map(|h| &h[..h.len().min(260)])
            .collect::<Vec<_>>()
    );
    if !hr {
        let dig = std::fs::read_to_string(d.join("work/dig.json")).unwrap();
        let adduct = if pol < 0 {
            "\"adduct\":\"[M-H]-\""
        } else {
            "\"adduct\":\"[M+H]+\""
        };
        assert!(
            dig.contains(adduct),
            "the Dig reads the parent in the other polarity: {}",
            &dig[..dig.len().min(300)]
        );
        // the +O product is a candidate of the Dig at the nominal mass of its ion, detected and scored
        let at = dig.find(&format!("\"mz\":{}", (ion(ION[1].0) * 10.0).floor() / 10.0));
        let row = &dig[at.expect("no candidate at the m/z of the +O ion")..];
        assert!(
            row[..row.len().min(900)].contains("\"status\":\"Detected\""),
            "{}",
            &row[..300]
        );
    }
    let net = std::fs::read_to_string(d.join("work/net.json")).unwrap();
    assert!(net.starts_with("{\"schema\":\"rete/1\""), "{net}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn bentazone_negative_high_resolution() {
    case("neg_hr", -1, true);
}

#[test]
fn bentazone_positive_high_resolution() {
    case("pos_hr", 1, true);
}

#[test]
fn bentazone_negative_unit_resolution() {
    case("neg_lr", -1, false);
}

#[test]
fn bentazone_positive_unit_resolution() {
    case("pos_lr", 1, false);
}
