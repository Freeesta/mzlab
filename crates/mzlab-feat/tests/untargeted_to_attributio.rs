//! The `untargeted` table as the input of the cascade of Attributio (`untargeted::to_attributio`): the aligned list of the synthetic data goes through
//! isotopes, identities and the cascade without errors and every row gets a level (0: no source holds anything for it); the marks of isotopes and
//! identities are columns of the table of the campaign, and the sources that need other programs are empty.

use std::collections::BTreeSet;
use std::path::Path;

use mzlab_feat::attributio::annotate::{gnps_comparer, load_annotations, BasicCuration};
use mzlab_feat::table::{Aligned, CELL_ABSENT, CELL_DETECTED};
use mzlab_feat::untargeted::ident::{ident, IdentParams};
use mzlab_feat::untargeted::isotopes::{group, IsoParams};
use mzlab_feat::untargeted::to_attributio::build;

fn aligned() -> Aligned {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/data/untargeted/mzmine39/aligned.csv");
    let text = std::fs::read_to_string(p).unwrap();
    let rows: Vec<Vec<String>> = text
        .lines()
        .skip(1)
        .map(|l| l.split(',').map(str::to_string).collect())
        .collect();
    let files: Vec<String> = rows
        .iter()
        .map(|r| r[5].clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let n = rows
        .iter()
        .map(|r| r[0].parse::<usize>().unwrap())
        .max()
        .unwrap();
    let mut t = Aligned::with_capacity(files.len(), n);
    t.n_files = files.len();
    for i in 1..=n {
        let mine: Vec<&Vec<String>> = rows
            .iter()
            .filter(|r| r[0].parse::<usize>().unwrap() == i)
            .collect();
        t.mz.push(mine[0][1].parse().unwrap());
        t.rt.push(mine[0][2].parse().unwrap());
        for f in &files {
            let c = mine.iter().find(|r| &r[5] == f);
            t.area.push(c.map_or(0.0, |r| r[9].parse().unwrap()));
            t.height.push(c.map_or(0.0, |r| r[8].parse().unwrap()));
            t.apex_rt
                .push(c.map_or(f32::NAN, |r| r[7].parse().unwrap()));
            t.width.push(0.1);
            t.state.push(if c.is_some() {
                CELL_DETECTED
            } else {
                CELL_ABSENT
            });
        }
    }
    t
}

#[test]
fn the_table_goes_through_the_cascade_and_every_row_has_a_level() {
    let t = aligned();
    let iso = group(&t, &IsoParams::default());
    let r = ident(&t, &iso, 1, &IdentParams::default());
    let (campaign, sources) = build(&t, &iso, &r, &vec![None; t.len()], 1).expect("sources");
    let a = load_annotations(&campaign, &sources, None, &BasicCuration, &gnps_comparer)
        .expect("annotations");
    assert_eq!(a.features.len(), t.len(), "one feature per row");
    assert!(a.features.iter().all(|f| (0..=4).contains(&f.level)));
    assert_eq!(sources.count("features POS"), Some(t.len()));
    assert_eq!(sources.count("feature spectra POS"), Some(0));
}

#[test]
fn the_marks_are_columns_and_the_other_sources_are_empty() {
    let t = aligned();
    let iso = group(&t, &IsoParams::default());
    let r = ident(&t, &iso, 1, &IdentParams::default());
    let (_, s) = build(&t, &iso, &r, &vec![None; t.len()], 1).expect("sources");
    let p = s.pol("POS").expect("POS");
    let (g, z) = (
        p.table.col("iso_group").unwrap(),
        p.table.col("charge").unwrap(),
    );
    let grouped = (0..p.table.len())
        .filter(|&i| p.table.int(i, g).unwrap_or(0) > 0)
        .count();
    assert_eq!(grouped, 6, "the ions of the two isotope patterns");
    assert!((0..p.table.len()).any(|i| p.table.int(i, z) == Some(2)));
    assert!(p.mzmine.is_empty() && p.gnps_library.is_empty() && p.gnps_network.is_empty());
    assert!(s.sirius.canopus.is_empty() && s.standards.is_none());
}
