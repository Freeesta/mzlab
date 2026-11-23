//! The synthetic set of the canonical-form tests: single molecules, mixtures, salts and 1 to 40 equal copies of one unit, with
//! enhanced-stereo groups inside one copy and across copies. Shared by `canon_components.rs` and `examples/golden_canon.rs`.

/// Copies tried for every unit: each count up to 12, then the larger ones where the old search grew fastest.
pub const COPIES: &[usize] = &[
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 14, 16, 20, 24, 30, 40,
];

/// Units repeated `COPIES` times (charged, isotopic, radical, aromatic, cis/trans, chiral, caged and salt units).
pub const UNITS: &[&str] = &[
    "O",
    "C",
    "[13CH4]",
    "[CH3]",
    "[NH4+]",
    "CC(=O)[O-]",
    "CCO",
    "OCC",
    "C/C=C/C",
    "C/C=C\\C",
    "N[C@@H](C)C(=O)O",
    "N[C@H](C)C(=O)O",
    "c1ccccc1",
    "c1ccncc1",
    "C1CCCCC1",
    "Cc1ccccc1",
    "c1ccc2ccccc2c1",
    "C12C3C4C1C5C2C3C45",
    "C1C2CC3CC1CC(C2)C3",
    "OC[C@H]1OC(O)[C@H](O)[C@@H](O)[C@@H]1O",
    "[Na+].[Cl-]",
    "[Ca+2].[Cl-].[Cl-]",
    "[Na+].[Na+].[O-]C(=O)C(=O)[O-]",
    "[Cl-].C[N+](C)(C)C",
    "O.CCO",
];

/// Every SMILES of the set, in a fixed order.
pub fn smiles() -> Vec<String> {
    let mut out = Vec::new();
    for unit in UNITS {
        for &k in COPIES {
            out.push(vec![*unit; k].join("."));
        }
    }
    // the same components grouped instead of interleaved, and unequal counts of two units
    for &k in &[2usize, 5, 12, 40] {
        out.push(format!(
            "{}.{}",
            vec!["O"; k].join("."),
            vec!["CCO"; k].join(".")
        ));
    }
    for &(a, b) in &[(1usize, 1usize), (2, 3), (5, 5), (10, 3), (3, 10), (20, 20)] {
        out.push(format!(
            "{}.{}",
            vec!["c1ccccc1"; a].join("."),
            vec!["Cc1ccccc1"; b].join(".")
        ));
    }
    out.push("c1ccccc1.c1ccncc1.CCO.O.[Na+].[Cl-]".into());
    out.push("[Cl-].[Na+].O.OCC.n1ccccc1.c1ccccc1".into());
    // enhanced stereo: one group per copy, and one group over every copy (the only layer that crosses components)
    for &k in &[1usize, 2, 3, 5, 8, 12, 20, 40] {
        let base = vec!["C[C@H](O)CC"; k].join(".");
        let centres: Vec<String> = (0..k).map(|i| (5 * i + 1).to_string()).collect();
        let own: Vec<String> = centres
            .iter()
            .enumerate()
            .map(|(i, c)| format!("&{}:{c}", i + 1))
            .collect();
        out.push(format!("{base} |{}|", own.join(",")));
        out.push(format!("{base} |&1:{}|", centres.join(",")));
        out.push(format!("{base} |o1:{}|", centres.join(",")));
    }
    out
}
