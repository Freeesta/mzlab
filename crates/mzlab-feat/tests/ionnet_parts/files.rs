//! `features.group@1` with fewer files than `min_corr_samples` (no correlation is tested, so no correlation edge and a warning) and past the memory
//! budget (the refusal `error.memoryBudget` of phase `group`, with the estimate counted in full and never held).

use super::{crowded, defaults, series, site};
use mzlab_core::budget::MemBudget;
use mzlab_core::json::{parse, Value};
use mzlab_feat::ionnet::{group, group_text, EdgeKind, Grouped, WARN_CORR_FILES};
use mzlab_feat::recipe::Recipe;
use mzlab_feat::store::{write_table, Origin};
use mzlab_feat::table::Aligned;

/// The first `k` files of `t`.
fn first_files(t: &Aligned, k: usize) -> Aligned {
    let mut o = Aligned::with_capacity(k, t.len());
    for r in 0..t.len() {
        o.mz.push(t.mz[r]);
        o.rt.push(t.rt[r]);
        for c in r * t.n_files..r * t.n_files + k {
            o.area.push(t.area[c]);
            o.height.push(t.height[c]);
            o.apex_rt.push(t.apex_rt[c]);
            o.width.push(t.width[c]);
            o.state.push(t.state[c]);
        }
    }
    o
}

fn run(t: &Aligned, budget: &MemBudget) -> mzlab_core::error::Result<Grouped> {
    group_text(
        &[],
        &write_table(t, &Recipe::default(), &Origin::default()),
        budget,
    )
}

fn param<'a>(e: &'a mzlab_core::error::Error, k: &str) -> &'a str {
    e.params
        .iter()
        .find(|p| p.0 == k)
        .map_or("", |p| p.1.as_str())
}

#[test]
fn below_min_corr_samples_no_correlation_edge_is_written() {
    let (t, _) = series(7, 40);
    for k in [1, 2] {
        let few = first_files(&t, k);
        let g = run(&few, &site()).unwrap();
        assert_eq!(g.warnings, vec![WARN_CORR_FILES], "{k} files");
        let v = parse(g.text.as_bytes()).unwrap();
        let kinds: Vec<&str> = v
            .get("edges")
            .unwrap()
            .arr()
            .iter()
            .filter_map(|e| e.get("EdgeType").and_then(Value::str))
            .collect();
        assert!(!kinds.contains(&"MS1 correlation"), "{k} files");
        assert!(kinds.contains(&"MS1 isotope"), "{k} files");
        // the pairs still lead `corr_group`: co-eluting rows share one
        let whole = group(&few, &defaults());
        assert!(whole.edges.iter().all(|e| e.kind != EdgeKind::Corr));
        let cg: Vec<u32> = v
            .get("corr_group")
            .unwrap()
            .arr()
            .iter()
            .map(|x| x.f64().unwrap() as u32)
            .collect();
        assert_eq!(cg, whole.corr_group);
        assert!(cg.iter().any(|g| cg.iter().filter(|h| *h == g).count() > 1));
    }
    // from min_corr_samples files on, nothing changes and nothing is said
    let three = run(&first_files(&t, 3), &site()).unwrap();
    assert!(three.warnings.is_empty());
    assert!(three.text.contains("\"MS1 correlation\""));
}

#[test]
fn past_the_budget_the_step_is_refused_with_its_whole_estimate() {
    let t = crowded(2026, 900);
    let (a, b) = (
        run(&t, &MemBudget::mb(0.05)).unwrap_err(),
        run(&t, &MemBudget::mb(0.2)).unwrap_err(),
    );
    for e in [&a, &b] {
        assert_eq!(e.key, "error.memoryBudget");
        assert_eq!(param(e, "fase"), "group");
    }
    // the pairs past the cap are counted, not dropped: the estimate does not depend on the budget
    assert_eq!(param(&a, "stima_mb"), param(&b, "stima_mb"));
    let est: f64 = param(&a, "stima_mb").parse().unwrap();
    assert!(est >= 1.0, "{est}");
    // a budget that holds the estimate writes the text
    assert!(run(&t, &MemBudget::mb(est + 1.0)).is_ok());
}
