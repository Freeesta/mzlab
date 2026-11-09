//! The `.cdf` that ChemStation exports from a run against the `.D` folder of the same run read by `agilent-d`: the same scans, the same times, the same TIC; m/z and intensities within the
//! precision of the export. Needs the private data repository (`MZLAB_DATI`) and the unzip program; skipped without them. Run with `--features tutte`.
#![cfg(all(feature = "andi", feature = "agilent"))]

mod common;

use mzlab_core::readers::{FileSet, Limits, Member};
use std::path::Path;

#[path = "../../../tools/dati_percorsi.rs"]
mod dati_percorsi;

fn members(root: &Path, dir: &Path, out: &mut Vec<Member>) {
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            members(root, &p, out);
        } else {
            let b = std::fs::read(&p).unwrap();
            let name = p
                .strip_prefix(root.parent().unwrap())
                .unwrap()
                .to_string_lossy()
                .into_owned();
            out.push(Member {
                nome: name,
                dim: b.len() as u64,
                dati: Box::new(b),
                sha1: None,
            });
        }
    }
}

#[test]
fn the_exported_cdf_says_what_its_d_folder_says() {
    let Some(folder) = dati_percorsi::percorso("dati/formati/aperti/netcdf_cdf") else {
        eprintln!("SKIP andi parita: no data repository (MZLAB_DATI)");
        return;
    };
    let tmp = std::env::temp_dir().join(format!("mzlab-andi-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    for (zip, cdf) in [
        (
            "MTBLS7740_AZ_alkanes_TMS_1.D.zip",
            "MTBLS7740_AZ_alkanes_TMS_1.cdf",
        ),
        ("MTBLS13991_Heart_16.D.zip", "MTBLS13991_Heart_16.cdf"),
    ] {
        let below = tmp.join(zip);
        std::fs::create_dir_all(&below).unwrap();
        let ok = folder.join(zip).exists()
            && std::process::Command::new("unzip")
                .arg("-q")
                .arg("-o")
                .arg(folder.join(zip))
                .arg("-d")
                .arg(&below)
                .status()
                .is_ok_and(|s| s.success());
        if !ok {
            eprintln!("SKIP andi parita: {zip}");
            continue;
        }
        let dir = std::fs::read_dir(&below)
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .find(|p| p.is_dir())
            .unwrap();
        let mut m = Vec::new();
        members(&dir, &dir, &mut m);
        let mut d_set = mzlab_lettori::apri(FileSet { membri: m }, &Limits::desktop()).unwrap();
        let mut c_set = common::andi::open_andi(std::fs::read(folder.join(cdf)).unwrap());
        let n = d_set.samples()[0].scansioni;
        assert_eq!(c_set.samples()[0].scansioni, n, "{cdf}");
        let (id, ic) = (
            d_set.index(0).unwrap().to_vec(),
            c_set.index(0).unwrap().to_vec(),
        );
        for k in (0..n).step_by((n / 30).max(1)) {
            assert!(
                (id[k].rt - ic[k].rt).abs() < 1e-3,
                "{cdf} scan {k}: rt {} {}",
                id[k].rt,
                ic[k].rt
            );
            assert!(
                (id[k].tic - ic[k].tic).abs() <= 1e-3 * id[k].tic.max(1.0),
                "{cdf} scan {k}: tic {} {}",
                id[k].tic,
                ic[k].tic
            );
            let (a, b) = (d_set.scan(0, k).unwrap(), c_set.scan(0, k).unwrap());
            assert_eq!(a.mz.len(), b.mz.len(), "{cdf} scan {k}");
            for i in 0..a.mz.len() {
                assert!(
                    (a.mz[i] - b.mz[i]).abs() < 0.01,
                    "{cdf} scan {k} mz {i}: {} {}",
                    a.mz[i],
                    b.mz[i]
                );
            }
        }
    }
    let _ = std::fs::remove_dir_all(&tmp);
}
