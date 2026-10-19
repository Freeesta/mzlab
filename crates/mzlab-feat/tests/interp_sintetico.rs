//! The interpretation of the rows (`interp/1`) over the synthetic identity series (invented molecules, `tools/feat_sintetici.py`): the block is written
//! into the `.mzfeat`, read back by row, the table still reads, and the MS2 scans of a product are linked with their purity. Skipped without python3 and
//! numpy (`MZLAB_GOLDEN_STRICT=1` fails instead).

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::idxread::Index;
use mzlab_core::json::parse;
use mzlab_feat::famalign::align_families;
use mzlab_feat::family::{analyse_families, FamParams};
use mzlab_feat::interp::{run::run_to, store};
use mzlab_feat::pipeline::{analyse, Steps};
use mzlab_feat::recipe::Recipe;
use mzlab_feat::store::{read_table, write_table, Origin, Slice};

use mzlab_feat::tp::{TpBlock, NO_PARENT, ST_PARENT, ST_TP};
use mzlab_feat::trace::Params;

const PARENT: &str = r#"{"neutral":"C22H31N3O4S"}"#;
// the product +3C of the parent of the synthetic series, as the registry of hypotheses would hold it
const REGISTRY: &str = r#"{"hypotheses":[{"id":1,"label":"x","formula":"C25H31N3O4S","mz":470.210804,"level":3,"sources":["regola"]}]}"#;

fn series() -> Option<PathBuf> {
    let out = std::env::temp_dir().join(format!("mzlab_feat_interp_{}", std::process::id()));
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let ok = Command::new("python3")
        .arg(root.join("tools/feat_sintetici.py"))
        .arg(&out)
        .args(["--solo", "identita"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !ok {
        assert!(
            std::env::var("MZLAB_GOLDEN_STRICT").is_err(),
            "python3 or numpy missing"
        );
        eprintln!("skipped: python3/numpy not available");
    }
    ok.then_some(out)
}

#[test]
fn the_block_is_built_from_the_dda_of_the_series_and_read_by_row() {
    let Some(out) = series() else { return };
    let open = |_: usize| {
        let f = std::fs::File::open(out.join("identita.mzidx")).unwrap();
        let size = f.metadata().unwrap().len();
        Index::open(f, size)
    };
    let mut ix = open(0).unwrap();
    let (mut f, links) = analyse(&mut ix, &Params::default(), &Steps::default()).unwrap();
    analyse_families(
        &mut ix,
        &mut f,
        &links,
        &Params::default(),
        &FamParams::default(),
    )
    .unwrap();
    let mut t = align_families(&[&f], &Recipe::default().align).unwrap();
    let parent = (0..t.len())
        .min_by(|&a, &b| {
            (t.mz[a] - 434.2108)
                .abs()
                .total_cmp(&(t.mz[b] - 434.2108).abs())
        })
        .unwrap();
    let product = (0..t.len())
        .find(|&r| (t.mz[r] - 470.2108).abs() < 0.005)
        .expect("the product is in the table");
    t.tp = TpBlock {
        status: vec![ST_TP; t.len()],
        notes: vec![0; t.len()],
        via: vec![0; t.len()],
        kinship: vec![0; t.len()],
        parent_row: parent as u32,
        ..TpBlock::default()
    };
    t.tp.status[parent] = ST_PARENT;
    assert_ne!(t.tp.parent_row, NO_PARENT);
    let bytes = write_table(&t, &Recipe::default(), &Origin::default());

    let mut sink = Vec::new();
    run_to(
        &mut Slice(&bytes),
        bytes.len() as u64,
        PARENT,
        REGISTRY,
        1,
        &mut |k| open(k),
        None::<&mut Index<std::fs::File>>,
        &mut sink,
    )
    .unwrap();
    let (back, _) = read_table(&sink).unwrap();
    assert_eq!(back.to_bytes(), t.to_bytes(), "the table reads as before");
    let (meta, ids) = store::read_meta(&mut Slice(&sink), sink.len() as u64)
        .unwrap()
        .expect("the block is there");
    assert_eq!(meta.get("version").and_then(|v| v.f64()), Some(1.0));
    assert!(
        !ids.contains(&t.id(parent)),
        "the parent is not interpreted"
    );
    let text = store::read_row(&mut Slice(&sink), sink.len() as u64, t.id(product))
        .unwrap()
        .expect("the product is interpreted");
    let row = parse(text.as_bytes()).unwrap();
    assert_eq!(row.get("schema").and_then(|v| v.str()), Some("interp/1"));
    assert_eq!(
        row.get("change")
            .and_then(|c| c.get("delta"))
            .and_then(|v| v.str()),
        Some("+3C")
    );
    let scans = row
        .get("ms2")
        .and_then(|m| m.get("scans"))
        .map(|a| a.arr())
        .unwrap_or_default();
    assert!(
        !scans.is_empty()
            && scans.iter().all(|s| s
                .get("purity")
                .and_then(|p| p.f64())
                .is_some_and(|p| p > 0.9))
    );
    // a second pass replaces the block instead of adding a second one
    let mut again = Vec::new();
    run_to(
        &mut Slice(&sink),
        sink.len() as u64,
        PARENT,
        REGISTRY,
        1,
        &mut |k| open(k),
        None::<&mut Index<std::fs::File>>,
        &mut again,
    )
    .unwrap();
    assert_eq!(again, sink);
    let _ = std::fs::remove_dir_all(&out);
}
