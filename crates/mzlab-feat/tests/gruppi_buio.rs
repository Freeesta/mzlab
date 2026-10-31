//! `gruppi/1`: the dark controls of a condition become a dark course (`dark`) only when they have known times; without them the key is absent.

mod gruppi_comune;

use gruppi_comune::*;
use mzlab_feat::gruppi::{build, GroupOpts};

/// One condition "A" at two times (duplicates), plus dark controls of the same series at the same times (`dark_t` = their times).
fn run(dark_t: &[Option<f64>]) -> mzlab_feat::gruppi::GroupsOut {
    let mut v = vec![file("t0.mzML", "t0", "", Some(0.0), 1)];
    for t in [10.0, 20.0] {
        for r in 0..2 {
            let n = v.len() as u32 + 1;
            v.push(file(&format!("A_{t}_{r}.mzML"), "sample", "A", Some(t), n));
        }
    }
    for (i, t) in dark_t.iter().enumerate() {
        let n = v.len() as u32 + 1;
        v.push(file(&format!("A_dark_{i}.mzML"), "dark", "A", *t, n));
    }
    let cs = files(v);
    let mut cells = vec![x()];
    for a in [1e5, 1e5, 4e5, 4e5] {
        cells.push(d(a));
    }
    for i in 0..dark_t.len() {
        cells.push(d(2e4 * (i as f64 + 1.0)));
    }
    let rows = vec![(meta(0, "TP 1", 300.0), cells)];
    build(&table(rows, cs.len()), &cs, &GroupOpts::default()).unwrap()
}

#[test]
fn dark_course_follows_the_dark_times() {
    let g = run(&[Some(10.0), Some(20.0)]);
    let c = &g.rows[0].cond[0];
    assert_eq!(c.dark.len(), 2);
    assert_eq!((c.dark[0].t, c.dark[0].area), (10.0, 2e4));
    assert_eq!((c.dark[1].t, c.dark[1].area), (20.0, 4e4));
    assert!(mzlab_feat::gruppi::to_json(&g).contains("\"dark\":[{\"t\":10"));
}

#[test]
fn no_dark_course_without_times() {
    for dark_t in [vec![], vec![None]] {
        let g = run(&dark_t);
        assert!(g.rows[0].cond[0].dark.is_empty());
        assert!(!mzlab_feat::gruppi::to_json(&g).contains("\"dark\":[{"));
    }
}
