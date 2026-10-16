//! Cross-check against xtb 6.7.1 (`tests/data/xtb/`, `--gfn 2 [--alpb water] --vfukui` on the RDKit geometry of each `.xyz`):
//! f(+), f(−) and f(0) of the heavy atoms, written with 3 decimals. Prints SKIP when the data are missing.

use mzlab_chem::xtb::{alpb, System};
use std::collections::HashMap;
use std::path::Path;

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/data/xtb");
/// Same tolerance on f as the tblite parity.
const DF: f64 = 3e-3;

fn z_of(sym: &str) -> u8 {
    match sym {
        "H" => 1,
        "C" => 6,
        "N" => 7,
        "O" => 8,
        "F" => 9,
        "P" => 15,
        "S" => 16,
        "Cl" => 17,
        "Br" => 35,
        "I" => 53,
        _ => panic!("element {sym}"),
    }
}

fn xyz(path: &Path) -> (Vec<u8>, Vec<[f64; 3]>) {
    let text = std::fs::read_to_string(path).unwrap();
    let rows: Vec<Vec<&str>> = text
        .lines()
        .skip(2)
        .map(|l| l.split_whitespace().collect())
        .filter(|r: &Vec<&str>| r.len() == 4)
        .collect();
    let el = rows.iter().map(|r| z_of(r[0])).collect();
    let pos = rows
        .iter()
        .map(|r| {
            [
                r[1].parse().unwrap(),
                r[2].parse().unwrap(),
                r[3].parse().unwrap(),
            ]
        })
        .collect();
    (el, pos)
}

/// (atom, quantity) → value.
fn fukui_csv(path: &Path) -> HashMap<(usize, String), f64> {
    let text = std::fs::read_to_string(path).unwrap();
    text.lines()
        .skip(1)
        .map(|l| {
            let c: Vec<&str> = l.splitn(5, ',').collect();
            (
                (c[1].parse().unwrap(), c[2].to_string()),
                c[3].parse().unwrap(),
            )
        })
        .collect()
}

/// Max |Δf| over the heavy atoms of one molecule and phase.
fn compare(name: &str, water: bool) -> f64 {
    let dir = Path::new(DIR);
    let (el, pos) = xyz(&dir.join(format!("{name}.xyz")));
    let csv = fukui_csv(&dir.join(format!(
        "{name}{}_fukui.csv",
        if water { "" } else { "_gas" }
    )));
    let sys = System::new(&el, &pos, water.then_some(&alpb::WATER)).unwrap();
    let n = sys.solve(0, None).unwrap();
    let a = sys.solve(-1, Some(&n.guess)).unwrap();
    let c = sys.solve(1, Some(&n.guess)).unwrap();
    let mut worst: f64 = 0.0;
    for ((i, what), v) in &csv {
        let (fp, fm) = (n.q[*i] - a.q[*i], c.q[*i] - n.q[*i]);
        let ours = match what.as_str() {
            "f_plus" => fp,
            "f_minus" => fm,
            _ => 0.5 * (fp + fm),
        };
        worst = worst.max((ours - v).abs());
    }
    worst
}

#[test]
fn fukui_as_xtb_671() {
    let dir = Path::new(DIR);
    let Ok(list) = std::fs::read_dir(dir) else {
        println!("SKIP: {DIR} missing");
        return;
    };
    let mut names: Vec<String> = list
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter_map(|f| f.strip_suffix(".xyz").map(str::to_string))
        .collect();
    names.sort();
    let mut ok = true;
    for name in &names {
        let (w, g) = (compare(name, true), compare(name, false));
        ok &= w <= DF && g <= DF;
        println!("{name:20} water |Δf|={w:.1e} gas |Δf|={g:.1e}");
    }
    assert!(ok);
}
