//! Descriptors against reference values computed with RDKit (Crippen logP/MR, TPSA, strict rotatable bonds, molecular weight) for a panel
//! of small molecules written by hand; the 625-row composti.csv comparison is not needed here.

use mzlab_mol::Mol;

/// (SMILES, logP, MR, TPSA, rotors, MW)
const PANEL: &[(&str, f64, f64, f64, u32, f64)] = &[
    ("CCO", -0.0014, 12.760, 20.23, 0, 46.069),
    ("c1ccccc1", 1.6866, 26.442, 0.00, 0, 78.114),
    ("CC(=O)O", 0.0909, 13.310, 37.30, 0, 60.052),
    (
        "Cn1cnc2c1c(=O)n(C)c(=O)n2C",
        -1.0293,
        51.196,
        61.82,
        0,
        194.194,
    ),
    (
        "CC(C)Cc1ccc(cc1)C(C)C(=O)O",
        3.0732,
        61.035,
        37.30,
        4,
        206.285,
    ),
    ("CC(C)(C)O", 0.7772, 21.972, 20.23, 0, 74.123),
    ("c1ccccc1O", 1.3922, 28.107, 20.23, 0, 94.113),
    ("c1ccccc1N", 1.2688, 30.854, 26.02, 0, 93.129),
    ("CCN(CC)CC", 1.3481, 33.462, 3.24, 3, 101.193),
    ("CNC", -0.1644, 14.984, 12.03, 0, 45.085),
    ("CC(=O)N", -0.5084, 14.868, 43.09, 0, 59.068),
    ("c1ccncc1", 1.0816, 24.237, 12.89, 0, 79.102),
    ("c1cc[nH]c1", 1.0147, 20.793, 15.79, 0, 67.091),
    ("c1ccoc1", 1.2796, 18.708, 13.14, 0, 68.075),
    ("c1ccsc1", 1.7481, 24.319, 0.00, 0, 84.143),
    ("CC(=O)OC", 0.1793, 17.690, 26.30, 0, 74.079),
    ("c1ccc(cc1)c1ccccc1", 3.3536, 51.878, 0.00, 1, 154.212),
    ("c1ccccc1Cl", 2.3400, 31.452, 0.00, 0, 112.559),
    ("c1ccccc1F", 1.8257, 26.400, 0.00, 0, 96.104),
    ("c1ccccc1Br", 2.4491, 34.142, 0.00, 0, 157.010),
    ("CC#N", 0.5299, 11.287, 23.79, 0, 41.053),
    ("CC=O", 0.2052, 11.738, 17.07, 0, 44.053),
    ("CC(C)=O", 0.5953, 16.355, 17.07, 0, 58.080),
    ("C=CC", 1.1923, 15.871, 0.00, 0, 42.081),
    ("c1ccc2ccccc2c1", 2.8398, 43.948, 0.00, 0, 128.174),
    ("OC(=O)c1ccccc1", 1.3848, 33.401, 37.30, 1, 122.123),
    ("c1ccccc1[N+](=O)[O-]", 1.5948, 33.096, 43.14, 1, 123.111),
    ("CS(C)=O", -0.0053, 19.990, 17.07, 0, 78.136),
    ("CS(=O)(=O)N", -1.0953, 19.031, 60.16, 0, 95.123),
    ("Nc1ccc(cc1)S(N)(=O)=O", -0.0838, 42.228, 86.18, 1, 172.209),
    ("CC(=O)Nc1ccc(O)cc1", 1.3506, 42.411, 49.33, 1, 151.165),
    ("OCC(O)CO", -1.6681, 20.178, 60.69, 2, 92.094),
    ("NCCc1ccccc1", 1.1878, 39.200, 26.02, 2, 121.183),
    (
        "CC(C)NCC(O)c1ccc(O)c(O)c1",
        1.1292,
        57.870,
        72.72,
        4,
        211.261,
    ),
    ("c1ccc2[nH]ccc2c1", 2.1679, 38.299, 15.79, 0, 117.151),
    ("C1CCCCC1", 2.3406, 27.702, 0.00, 0, 84.162),
    ("C=O", -0.1849, 7.121, 17.07, 0, 30.026),
    ("CNC(=O)OC", -0.0278, 21.465, 38.33, 0, 89.094),
    ("c1ccc(Cc2ccccc2)cc1", 3.2774, 55.690, 0.00, 2, 168.239),
    (
        "Oc1ccc(cc1)C(C)(C)c1ccc(O)cc1",
        3.4237,
        68.342,
        40.46,
        2,
        228.291,
    ),
    ("CC(N)C(=O)O", -0.5818, 21.285, 63.32, 1, 89.094),
    ("OC(=O)CCC(=O)O", -0.0642, 24.506, 74.60, 3, 118.088),
    ("CN(C)C=O", -0.2956, 19.751, 20.31, 1, 73.095),
    ("COc1ccccc1", 1.6952, 32.994, 9.23, 1, 108.140),
    ("NC(=O)N", -0.9762, 13.771, 69.11, 0, 60.056),
];

#[test]
fn panel_matches_reference() {
    let mut bad = Vec::new();
    for &(smi, lp, mr, tpsa, rot, mw) in PANEL {
        let m = Mol::from_smiles(smi).unwrap();
        let d = m.descriptors();
        let mut why = String::new();
        if (d.logp - lp).abs() > 0.005 {
            why += &format!(" logP {:.4} vs {lp}", d.logp);
        }
        if (d.mr - mr).abs() > 0.02 {
            why += &format!(" MR {:.3} vs {mr}", d.mr);
        }
        if (d.tpsa - tpsa).abs() > 0.01 {
            why += &format!(" TPSA {:.2} vs {tpsa}", d.tpsa);
        }
        if d.rotors != rot {
            why += &format!(" rot {} vs {rot}", d.rotors);
        }
        if (d.mw.unwrap() - mw).abs() > 0.01 {
            why += &format!(" MW {:.3} vs {mw}", d.mw.unwrap());
        }
        if !why.is_empty() {
            bad.push(format!("{smi}:{why}"));
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

/// Named molecules: Wildman-Crippen logP of the model (the model, not the measured logP, which differs by up to 1 for caffeine and
/// ibuprofen), donors, acceptors and counts that follow from the definitions in `descr.rs`.
#[test]
fn known_molecules() {
    // (SMILES, logP, HBD, HBA, rotors, TPSA)
    let known = [
        ("CCO", -0.0014, 1, 1, 0, 20.23),
        ("c1ccccc1", 1.6866, 0, 0, 0, 0.0),
        ("CC(=O)O", 0.0909, 1, 2, 0, 37.30),
        ("Cn1cnc2c1c(=O)n(C)c(=O)n2C", -1.0293, 0, 3, 0, 61.82),
        ("CC(C)Cc1ccc(cc1)C(C)C(=O)O", 3.0732, 1, 2, 4, 37.30),
    ];
    for (smi, lp, hbd, hba, rot, tpsa) in known {
        let d = Mol::from_smiles(smi).unwrap().descriptors();
        assert!((d.logp - lp).abs() < 0.005, "{smi} logP {}", d.logp);
        assert_eq!((d.hbd, d.hba, d.rotors), (hbd, hba, rot), "{smi}");
        assert!((d.tpsa - tpsa).abs() < 0.01, "{smi} TPSA {}", d.tpsa);
    }
}

/// An explicit [H] atom counts like an implicit hydrogen on the same atom.
#[test]
fn explicit_hydrogens_equal_implicit() {
    let a = Mol::from_smiles("CCO").unwrap().descriptors();
    let b = Mol::from_smiles("[H]C([H])([H])C([H])([H])O[H]")
        .unwrap()
        .descriptors();
    assert!(
        (a.logp - b.logp).abs() < 1e-9 && (a.mr - b.mr).abs() < 1e-9,
        "{a:?} {b:?}"
    );
    assert_eq!((a.hbd, a.tpsa), (b.hbd, b.tpsa));
    assert!((a.mw.unwrap() - b.mw.unwrap()).abs() < 1e-9);
}

/// Unknown element weight gives no molecular weight instead of a wrong one.
#[test]
fn unknown_weight_is_none() {
    assert!(Mol::from_smiles("[Xe]")
        .unwrap()
        .molecular_weight()
        .is_none());
}
