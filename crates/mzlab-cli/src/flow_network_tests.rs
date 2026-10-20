//! `tp.network@1` in a plan: the `.mzfeat` with the blocks `tp/1` and `interp/1` (written by the real store on invented rows of the timolol formula)
//! gives the same `rete/1` as the engine called directly, which `mzlab-feat/tests/rete.rs` checks arc by arc.

use std::path::Path;

use mzlab_core::campioni::{proponi, to_json};
use mzlab_feat::interp::store::append;
use mzlab_feat::recipe::Recipe;
use mzlab_feat::store::{write_table, Origin, Slice};
use mzlab_feat::table::{Aligned, CELL_ABSENT, CELL_DETECTED};
use mzlab_feat::tp::{TpBlock, KIN_PROBABLE, ST_PARENT, ST_TP};

const NAMES: [&str; 4] = [
    "X_blank",
    "X_TiO2_t000min",
    "X_TiO2_t005min",
    "X_TiO2_t010min",
];
const BASE: f64 = 317.1642;
const O: f64 = 15.994915;

fn table() -> Vec<u8> {
    let rows = [
        (BASE, 5.0, [0.0, 1e7, 9e6, 7e6]),
        (BASE + O, 4.5, [0.0, 0.0, 2e5, 5e5]),
        (BASE + 2.0 * O, 4.0, [0.0, 0.0, 0.0, 1e5]),
    ];
    let mut t = Aligned::with_capacity(NAMES.len(), rows.len());
    for (mz, rt, a) in rows {
        t.mz.push(mz);
        t.rt.push(rt);
        for v in a {
            t.area.push(v);
            t.height.push((v / 10.0) as f32);
            t.apex_rt.push(rt as f32);
            t.width.push(0.1);
            t.state
                .push(if v > 0.0 { CELL_DETECTED } else { CELL_ABSENT });
        }
    }
    t.tp = TpBlock {
        status: vec![ST_PARENT, ST_TP, ST_TP],
        notes: vec![0; 3],
        via: vec![0; 3],
        kinship: vec![0, KIN_PROBABLE, KIN_PROBABLE],
        parent_row: 0,
        ..TpBlock::default()
    };
    write_table(&t, &Recipe::default(), &Origin::default())
}

fn interp(bytes: &[u8]) -> Vec<u8> {
    let row = |id: u32, f: &str, ion: &str| {
        let s = format!("{{\"id\":{id},\"change\":{{\"delta\":\"x\",\"neutral_formula\":\"{f}\",\"ion_formula\":\"{ion}\",\"ppm\":0.3,\"source\":\"rule\",\"alternatives\":[]}},\"vs_parent\":null}}");
        (id, s)
    };
    let rows = vec![
        row(2, "C13H24N4O4S", "C13H25N4O4S"),
        row(3, "C13H24N4O5S", "C13H25N4O5S"),
    ];
    let meta = "{\"version\":1,\"parent\":{\"ion_formula\":\"C13H25N4O3S\",\"mz\":317.1642}}";
    let mut out = Vec::new();
    append(&mut Slice(bytes), bytes.len() as u64, &mut out, meta, &rows).unwrap();
    out
}

fn dir() -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab_cli_network_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("work")).unwrap();
    d
}

fn run(dir: &Path, samples: bool) -> String {
    let (extra_in, extra_step) = if samples {
        (
            r#",{"name":"samples.json","type":"samples"}"#,
            r#","samples":"file:samples.json""#,
        )
    } else {
        ("", "")
    };
    let plan = format!(
        r#"{{"format":"mzflow/2","inputs":[{{"name":"tp.mzfeat","type":"tp"}},{{"name":"table.mzfeat","type":"table"}}{extra_in}],"steps":[
        {{"id":"n","step":"tp.network@1","in":{{"tp":"file:tp.mzfeat","table":"file:table.mzfeat"{extra_step}}}}}]}}"#
    );
    let (run, prov) =
        crate::flow::run_plan(plan.as_bytes(), dir, &dir.join("work"), 1, None).unwrap();
    assert!(
        run.skipped.is_empty() && !prov.contains("\"error\""),
        "{prov}"
    );
    std::fs::read_to_string(dir.join("work/n.json")).unwrap()
}

#[test]
fn the_plan_gives_the_network_of_the_engine() {
    let d = dir();
    let (t, p) = (table(), interp(&table()));
    std::fs::write(d.join("table.mzfeat"), &t).unwrap();
    std::fs::write(d.join("tp.mzfeat"), &p).unwrap();
    let files: Vec<_> = NAMES
        .iter()
        .map(|n| (n.to_string(), String::new(), String::new()))
        .collect();
    let samples = to_json(&proponi(&files));
    std::fs::write(d.join("samples.json"), &samples).unwrap();

    let got = run(&d, true);
    assert!(got.starts_with("{\"schema\":\"rete/1\""), "{got}");
    assert!(got.contains("\"n_arcs\":3"), "{got}");
    // the time course changes only the direction, and the same bytes come out on a second run
    assert_eq!(got, run(&d, true));
    assert!(run(&d, false).contains("\"kinetic\":\"n/a\""));
    let _ = std::fs::remove_dir_all(&d);
}
