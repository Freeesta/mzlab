//! `qc.mass_accuracy@1` through the plan runner on invented indexes (8 steady ions per file, no lab data): the report and the corrected table are made, a second run takes both from the
//! cache with the same bytes, and without a model no table is written.

use std::path::Path;

use mzlab_core::idxread::Index;
use mzlab_core::mzidx::build_mzml_to_vec;
use mzlab_core::upload::prepare;
use mzlab_feat::recipe::Recipe;
use mzlab_feat::store::{read_table, write_table, Origin};
use mzlab_feat::table::{Aligned, CELL_DETECTED};

const MZ: [f64; 8] = [
    149.0233, 279.1591, 391.2843, 445.1200, 579.1810, 683.4642, 797.5278, 881.2105,
];
const TRUE: [f64; 3] = [210.1234, 405.5678, 700.9012];

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

/// An index whose 20 scans hold the 8 ions at `ppm` from their m/z.
fn index_bytes(ppm: f64) -> Vec<u8> {
    let mz: Vec<f64> = MZ.iter().map(|m| m * (1.0 + ppm * 1e-6)).collect();
    let y = vec![1e6; mz.len()];
    let mut x = String::from(
        r#"<?xml version="1.0" encoding="utf-8"?><mzML xmlns="http://psi.ms.info/ms/mzml"><run id="r"><spectrumList count="20">"#,
    );
    for i in 0..20 {
        x += &format!(
            r#"<spectrum index="{i}" id="scan={}" defaultArrayLength="8"><cvParam cvRef="MS" accession="MS:1000511" name="ms level" value="1"/><cvParam cvRef="MS" accession="MS:1000127" name="centroid spectrum"/><cvParam cvRef="MS" accession="MS:1000130" name="positive scan"/><scanList count="1"><scan><cvParam cvRef="MS" accession="MS:1000016" name="scan start time" value="{}" unitCvRef="UO" unitAccession="UO:0000031" unitName="minute"/></scan></scanList><binaryDataArrayList count="2">{}{}</binaryDataArrayList></spectrum>"#,
            i + 1,
            1.0 + i as f64 * 0.1,
            array("MS:1000514", &mz),
            array("MS:1000515", &y)
        );
    }
    x += "</spectrumList></run></mzML>";
    let mzml = x.into_bytes();
    let mut src = mzml.clone();
    let p = prepare(&mut src, mzml.len() as u64, "x.mzML").unwrap();
    let bytes = build_mzml_to_vec(&p.table, &mzml, 64 << 20).unwrap().0;
    Index::open(bytes.clone(), bytes.len() as u64).unwrap();
    bytes
}

fn table(ppm: f64) -> Vec<u8> {
    let mut t = Aligned::with_capacity(2, TRUE.len());
    for (r, m) in TRUE.iter().enumerate() {
        t.mz.push(m * (1.0 + ppm * 1e-6));
        t.rt.push(2.0 + r as f64);
        for _ in 0..2 {
            t.area.push(1e6);
            t.height.push(1e5);
            t.apex_rt.push(2.0 + r as f32);
            t.width.push(0.2);
            t.state.push(CELL_DETECTED);
        }
    }
    let inputs = vec![
        ("a.mzML".to_string(), "aa".repeat(32)),
        ("b.mzML".to_string(), "bb".repeat(32)),
    ];
    write_table(
        &t,
        &Recipe::default(),
        &Origin {
            inputs,
            ..Origin::default()
        },
    )
}

fn dir(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab_cli_massacc_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("work")).unwrap();
    std::fs::write(d.join("a.mzidx"), index_bytes(4.0)).unwrap();
    std::fs::write(d.join("b.mzidx"), index_bytes(4.0)).unwrap();
    std::fs::write(d.join("table.mzfeat"), table(4.0)).unwrap();
    let csv: String = MZ
        .iter()
        .enumerate()
        .map(|(i, m)| format!("ion{i},{m}\n"))
        .collect();
    std::fs::write(d.join("refs.csv"), format!("name,ion\n{csv}")).unwrap();
    d
}

fn run(d: &Path, params: &str) -> Result<String, String> {
    let plan = format!(
        r#"{{"format":"mzflow/2","inputs":[{{"name":"a.mzidx","type":"mzidx"}},{{"name":"b.mzidx","type":"mzidx"}},{{"name":"table.mzfeat","type":"table"}},{{"name":"refs.csv","type":"report"}}],"steps":[
        {{"id":"q","step":"qc.mass_accuracy@1","in":{{"raw":["file:a.mzidx","file:b.mzidx"],"table":"file:table.mzfeat","references":"file:refs.csv"}},"params":{params}}}]}}"#
    );
    let (run, prov) =
        crate::flow::run_plan_cached(plan.as_bytes(), d, &d.join("work"), 1, None, Some(false))
            .map_err(|e| e.to_string())?;
    if !run.skipped.is_empty() || prov.contains("\"error\"") {
        return Err(prov);
    }
    Ok(prov)
}

#[test]
fn the_report_and_the_corrected_table_are_made_and_cached() {
    let d = dir("model");
    let prov = run(&d, r#"{"model":"offset"}"#).unwrap();
    assert!(prov.contains("warn.qc.mass.offset"), "{prov}");
    let (report, fixed) = (
        std::fs::read(d.join("work/q.json")).unwrap(),
        std::fs::read(d.join("work/q.mzfeat")).unwrap(),
    );
    assert!(String::from_utf8_lossy(&report).starts_with("{\"format\":\"massacc/1\""));
    let (t, _) = read_table(&fixed).unwrap();
    for (m, w) in t.mz.iter().zip(TRUE) {
        assert!(((m - w) / w * 1e6).abs() < 0.5, "{m} against {w}");
    }
    // the second run takes both outputs from the cache, byte for byte
    let again = run(&d, r#"{"model":"offset"}"#).unwrap();
    assert!(again.contains("\"cached\":true"), "{again}");
    assert_eq!(std::fs::read(d.join("work/q.json")).unwrap(), report);
    assert_eq!(std::fs::read(d.join("work/q.mzfeat")).unwrap(), fixed);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn without_a_model_no_table_is_written() {
    let d = dir("none");
    run(&d, "{}").unwrap();
    assert!(d.join("work/q.json").is_file());
    assert!(!d.join("work/q.mzfeat").exists());
    let _ = std::fs::remove_dir_all(&d);
}
