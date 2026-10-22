//! `tp.link@2` in a plan over the synthetic identity series (invented molecules, `tools/feat_sintetici.py --solo identita`: blank, dark, t0 and three
//! treated files with a parent and its products): the base chain makes the table, `tp.link` gives the `.mzfeat` with `tp/1` and `interp/1`. The same
//! files put through the calls of the page by hand (hypotheses searched, proofs, interpretation) give the same bytes; a second run gives them again.
//! Skipped without python3 and numpy (`MZLAB_GOLDEN_STRICT=1` fails instead).

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_core::budget::MemBudget;
use mzlab_core::campioni::{proponi, to_json};
use mzlab_core::idxread::Index;
use mzlab_feat::hypo::{generate, HypoParams, Inputs, Rules};
use mzlab_feat::interp::ms2link::LinkParams;
use mzlab_feat::interp::run::run_to_with;
use mzlab_feat::interp::store;
use mzlab_feat::store::{read_table_from, Slice};
use mzlab_feat::tp::TpRoles;
use mzlab_feat_wasm::run::{self, RowSources};

use crate::flow_base::Files;

pub(crate) const FORMULA: &str = "C22H31N3O4S";
const RULES: &[u8] = include_bytes!("../../../TP_Mine/data/regole_tp_2.json");
const NAMES: [&str; 6] = [
    "SIN_blank",
    "SIN_dark",
    "SIN_t0",
    "SIN_t05min",
    "SIN_t10min",
    "SIN_t20min",
];

pub(crate) fn series(tag: &str) -> Option<PathBuf> {
    let out = std::env::temp_dir().join(format!("mzlab_cli_link_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&out);
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

pub(crate) fn hypotheses() -> String {
    let inp = Inputs {
        parent: FORMULA,
        suspects: None,
        families: &[],
        imported: &[],
        fragments: &[],
        groups: None,
    };
    let rules = Rules::parse_profile(RULES, None).unwrap();
    generate(&rules, &inp, &HypoParams::default())
        .unwrap()
        .to_json()
}

/// The folder of the run: the six mzML, the samples, the parent (with its time) and the hypotheses of `tp.generate`.
pub(crate) fn fixture(out: &Path, parent: &str, hyp: &str) -> PathBuf {
    let dir = out.join("run");
    std::fs::create_dir_all(dir.join("work")).unwrap();
    for n in NAMES {
        std::fs::copy(
            out.join("identita").join(format!("{n}.mzML")),
            dir.join(format!("{n}.mzML")),
        )
        .unwrap();
    }
    let files: Vec<_> = NAMES
        .iter()
        .map(|n| (format!("{n}.mzML"), String::new(), String::new()))
        .collect();
    std::fs::write(dir.join("samples.json"), to_json(&proponi(&files))).unwrap();
    std::fs::write(dir.join("parent.json"), parent).unwrap();
    std::fs::write(dir.join("hyp.json"), hyp).unwrap();
    dir
}

pub(crate) fn plan(params: &str) -> String {
    let raw: Vec<String> = NAMES
        .iter()
        .map(|n| format!(r#"{{"name":"{n}.mzML","type":"raw"}}"#))
        .collect();
    let src: Vec<String> = NAMES
        .iter()
        .map(|n| format!(r#""file:{n}.mzML""#))
        .collect();
    format!(
        r#"{{"format":"mzflow/2","inputs":[{},{{"name":"samples.json","type":"samples"}},{{"name":"parent.json","type":"structure"}},{{"name":"hyp.json","type":"transform"}}],"steps":[
        {{"id":"idx","step":"index.mzidx@1","in":{{"source":[{}]}}}},
        {{"id":"det","step":"features.detect@1","in":{{"index":["idx.index"]}}}},
        {{"id":"ali","step":"features.align@1","in":{{"peaks":["det.peaks"]}}}},
        {{"id":"gap","step":"features.gapfill@1","in":{{"table":"ali.table","index":["idx.index"]}}}},
        {{"id":"lnk","step":"tp.link@2","in":{{"table":"gap.table","samples":"file:samples.json","parent":"file:parent.json","index":["idx.index"],"hypotheses":"file:hyp.json"}},"params":{params}}}]}}"#,
        raw.join(","),
        src.join(",")
    )
}

pub(crate) fn run(dir: &Path, params: &str) -> Result<Vec<u8>, String> {
    let (done, prov) =
        crate::flow::run_plan(plan(params).as_bytes(), dir, &dir.join("work"), 2, None)
            .map_err(|e| e.key.to_string())?;
    if !done.skipped.is_empty() || prov.contains("\"error\"") {
        return Err(prov);
    }
    Ok(std::fs::read(dir.join("work/lnk.mzfeat")).unwrap())
}

/// The calls of the page on the files the base chain left in the work folder.
fn by_hand(dir: &Path) -> Vec<u8> {
    let w = dir.join("work");
    let idx: Vec<PathBuf> = NAMES
        .iter()
        .map(|k| w.join(format!("idx-{k}.mzidx")))
        .collect();
    let peaks: Vec<PathBuf> = NAMES
        .iter()
        .map(|k| w.join(format!("det-{k}.mzfeatf")))
        .collect();
    let open = |p: &PathBuf| {
        let f = std::fs::File::open(p).unwrap();
        let n = f.metadata().unwrap().len();
        Index::open(f, n)
    };
    let mut recipe = run::recipe_for_estimated("", &mut open(&idx[0]).unwrap()).unwrap();
    recipe.detect.polarity = Some(1);
    let table = std::fs::read(w.join("gap.mzfeat")).unwrap();
    let samples = std::fs::read_to_string(dir.join("samples.json")).unwrap();
    let roles = TpRoles::from_json(&samples).unwrap();
    let mut ix: Vec<_> = idx.iter().map(|p| open(p).unwrap()).collect();
    let hypo = run::hypotheses_stored(
        &mut ix,
        &mut Files(peaks.clone()),
        RULES,
        "",
        FORMULA,
        Some(1.0),
        &recipe,
    )
    .unwrap();
    let mz = 434.210804;
    let mut with_tp = Vec::new();
    let mut again = Files(peaks);
    let mut reopen = |k: usize| open(&idx[k]);
    run::evidence_sources_to(
        &mut Slice(&table),
        table.len() as u64,
        &hypo,
        &roles.minutes,
        Some(&roles),
        Some((mz, 1.0)),
        &recipe,
        &MemBudget::mb(mzlab_core::budget_auto::native_auto_mb()),
        Some(RowSources {
            stored: &mut again,
            open: &mut reopen,
        }),
        &mut with_tp,
    )
    .unwrap();
    let mut out = Vec::new();
    let parent = format!(r#"{{"neutral":"{FORMULA}","adduct":"[M+H]+","rt":1}}"#);
    run_to_with(
        &mut Slice(&with_tp),
        with_tp.len() as u64,
        (&parent, &hypo.registry.to_json(), 1),
        &LinkParams::default(),
        &mut reopen,
        None::<&mut Index<std::fs::File>>,
        &mut out,
    )
    .unwrap();
    out
}

#[test]
fn the_plan_gives_the_table_with_the_view_and_the_interpretation_of_the_page() {
    let Some(out) = series("plan") else { return };
    let dir = fixture(
        &out,
        &format!(r#"{{"formula":"{FORMULA}","rt":1.0}}"#),
        &hypotheses(),
    );
    let got = run(&dir, "{}").unwrap();
    assert_eq!(got, by_hand(&dir));
    let (t, _) = read_table_from(&mut Slice(&got), got.len() as u64).unwrap();
    assert_eq!(
        t.tp.status.len(),
        t.len(),
        "the view of the products is there"
    );
    assert!(
        store::read_meta(&mut Slice(&got), got.len() as u64)
            .unwrap()
            .is_some(),
        "so is the interpretation"
    );
    // a second run reads its steps from the folder and gives the same bytes
    assert_eq!(got, run(&dir, "{}").unwrap());
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn the_thresholds_reach_the_interpretation_and_a_wrong_input_is_refused() {
    let Some(out) = series("thresholds") else {
        return;
    };
    let dir = fixture(
        &out,
        &format!(r#"{{"formula":"{FORMULA}","rt":1.0}}"#),
        &hypotheses(),
    );
    let got = run(&dir, r#"{"purity_min":0.99,"select_ppm":3}"#).unwrap();
    let (meta, _) = store::read_meta(&mut Slice(&got), got.len() as u64)
        .unwrap()
        .unwrap();
    let params = meta.get("params").unwrap();
    assert_eq!(params.get("purity_min").and_then(|v| v.f64()), Some(0.99));
    assert_eq!(params.get("select_ppm").and_then(|v| v.f64()), Some(3.0));
    // hypotheses that are not the ones of this parent
    std::fs::write(dir.join("hyp.json"), "{}").unwrap();
    assert!(run(&dir, "{}").is_err());
    let _ = std::fs::remove_dir_all(&out);
}
