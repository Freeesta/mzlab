//! `metodo/1` of the Thermo `.raw` files against what is known of them: the mzML that msconvert made from the same file (instrument, serial number, software version),
//! the gradient that `chromeleon::parse` reads from the method text, the document that the conversion writes into the mzML, and the anonymous document against the complete one.
//! Needs the private data repository (`MZLAB_DATI`, or `mzlab-dati` beside this one); without it the test is skipped.
use mzlab_core::json::{self, Value};
use mzlab_core::{chromeleon, metodo, mzml, raw};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

fn dati() -> Option<PathBuf> {
    let mut cands: Vec<PathBuf> = std::env::var_os("MZLAB_DATI")
        .map(PathBuf::from)
        .into_iter()
        .collect();
    let here = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for up in here.ancestors().skip(1).take(3) {
        cands.push(up.join("mzlab-dati"));
        cands.push(
            up.parent()
                .map(|p| p.join("mzlab-dati"))
                .unwrap_or_default(),
        );
    }
    cands.into_iter().find(|p| p.join("HRMS").is_dir())
}

/// Every `.raw` with an mzML beside it (same stem) under `HRMS`.
fn pairs(d: &Path) -> Vec<(PathBuf, PathBuf)> {
    let mut out = vec![];
    let mut dirs = vec![d.join("HRMS")];
    while let Some(dir) = dirs.pop() {
        for e in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                dirs.push(p);
            } else if p.extension().is_some_and(|x| x == "raw") && p.with_extension("mzML").exists()
            {
                out.push((p.clone(), p.with_extension("mzML")));
            }
        }
    }
    out
}

fn doc(p: &Path, anonymous: bool) -> String {
    raw::metodo(
        BufReader::new(File::open(p).unwrap()),
        "sample.raw",
        anonymous,
    )
    .unwrap()
}

fn value_after(head: &str, accession: &str) -> Option<String> {
    let p = head.find(accession)?;
    Some(
        head[p..]
            .split("value=\"")
            .nth(1)?
            .split('"')
            .next()?
            .to_string(),
    )
}

fn text<'a>(v: &'a Value, path: &[&str]) -> Option<&'a str> {
    let mut v = v;
    for k in path {
        v = v.get(k)?;
    }
    v.str()
}

#[test]
fn the_raw_method_agrees_with_msconvert_and_with_the_script() {
    let Some(d) = dati() else {
        eprintln!("skipped: no data repository (MZLAB_DATI)");
        return;
    };
    let all = pairs(&d);
    assert!(
        !all.is_empty(),
        "no .raw with an mzML in the data repository"
    );
    for (raw_path, mzml_path) in all {
        let full = doc(&raw_path, false);
        let v = json::parse(full.as_bytes()).unwrap();
        let mut head = vec![0u8; 20_000];
        std::io::Read::read_exact(&mut File::open(&mzml_path).unwrap(), &mut head).unwrap();
        let head = String::from_utf8_lossy(&head);
        if let Some(serial) = value_after(&head, "MS:1000529") {
            assert_eq!(
                text(&v, &["strumento", "serie"]),
                Some(serial.as_str()),
                "serial number of {raw_path:?}"
            );
        }
        let sw = head
            .split("<software id=\"Xcalibur\" version=\"")
            .nth(1)
            .and_then(|t| t.split('"').next());
        let got = v
            .get("strumento")
            .and_then(|s| s.get("software"))
            .and_then(|s| s.at(0))
            .and_then(|s| s.get("versione"))
            .and_then(|s| s.str());
        assert_eq!(got, sw, "Xcalibur version of {raw_path:?}");
        // the gradient of the document is the one of the script
        let mut src = BufReader::new(File::open(&raw_path).unwrap());
        let script = raw::instrument_method_text(&mut src)
            .unwrap()
            .unwrap_or_default();
        if let Ok(g) = chromeleon::parse(&script) {
            let n = v
                .get("lc")
                .and_then(|l| l.get("gradiente"))
                .map_or(0, |g| g.arr().len());
            assert_eq!(n, g.points.len(), "gradient points of {raw_path:?}");
        }
    }
}

#[test]
fn the_conversion_carries_the_document_and_the_anonymous_one_has_no_identity() {
    let Some(d) = dati() else {
        eprintln!("skipped: no data repository (MZLAB_DATI)");
        return;
    };
    for (raw_path, _) in pairs(&d) {
        let full = doc(&raw_path, false);
        let mut out = Vec::new();
        raw::to_mzml(
            BufReader::new(File::open(&raw_path).unwrap()),
            &mut out,
            "sample.raw",
        )
        .unwrap();
        let head = String::from_utf8_lossy(&out[..out.len().min(2 << 20)]).into_owned();
        assert_eq!(
            metodo::from_userparam(&head).as_deref(),
            Some(full.as_str()),
            "document in the mzML of {raw_path:?}"
        );
        // no value of the scan trailer is lost: what the registered fields do not carry is written as `opentfraw.trailer.*` in every spectrum
        let first = out.windows(10).position(|w| w == b"<spectrum ").unwrap();
        let end = first
            + out[first..]
                .windows(11)
                .position(|w| w == b"</spectrum>")
                .unwrap();
        let body = String::from_utf8_lossy(&out[first..end]);
        let n = body.matches("name=\"opentfraw.trailer.").count();
        assert!(
            n >= 10,
            "only {n} trailer fields in the first spectrum of {raw_path:?}"
        );
        // identity: every value the complete document marks as personal is absent from the anonymous one
        let v = json::parse(full.as_bytes()).unwrap();
        let anon = doc(&raw_path, true);
        let mut secrets: Vec<String> = ["utente", "computer"]
            .iter()
            .filter_map(|k| text(&v, &["operatore", k]).map(String::from))
            .collect();
        secrets.extend(text(&v, &["strumento", "serie"]).map(String::from));
        for c in v.get("campi").map_or(&[][..], |c| c.arr()) {
            if c.get("identita").is_some() {
                secrets.extend(text(c, &["valore"]).map(String::from));
            }
        }
        assert!(!secrets.is_empty());
        // a user name can be a word of the model («Fusion»): what the model says stays
        let model = text(&v, &["strumento", "modello"])
            .unwrap_or("")
            .to_lowercase();
        for s in secrets
            .iter()
            .filter(|s| s.len() >= 4 && !model.contains(&s.to_lowercase()))
        {
            assert!(!anon.to_lowercase().contains(&s.to_lowercase()), "anonymous document of {raw_path:?} still has the identity value of {} characters (starts with {:?})", s.len(), s.chars().take(3).collect::<String>());
        }
        assert!(anon.len() > 1000 && !anon.contains("\"operatore\""));
    }
}

#[test]
fn the_pump_pressure_is_the_one_msconvert_wrote_in_the_unit_of_the_file() {
    let Some(d) = dati() else {
        eprintln!("skipped: no data repository (MZLAB_DATI)");
        return;
    };
    let mut checked = 0;
    for (raw_path, mzml_path) in pairs(&d) {
        let mut out = Vec::new();
        raw::to_mzml(
            BufReader::new(File::open(&raw_path).unwrap()),
            &mut out,
            "sample.raw",
        )
        .unwrap();
        let ours = mzml::read(&out[..]).unwrap();
        let theirs = mzml::read(BufReader::new(File::open(&mzml_path).unwrap())).unwrap();
        let pressure = |r: &mzml::Run| {
            r.chromatograms
                .iter()
                .find(|c| c.kind == mzml::ChromKind::Pressure)
                .cloned()
        };
        let (Some(a), Some(b)) = (pressure(&ours), pressure(&theirs)) else {
            continue;
        };
        assert_eq!(a.unit, "pascal");
        assert!(a.time.len() >= b.time.len(), "samples of {raw_path:?}");
        // msconvert takes the number of the file for bar and writes it x 1e5; the unit of the file is psi or bar (in the channel name)
        let psi = std::fs::read(&raw_path)
            .map(|x| x.windows(8).any(|w| w == b" \0p\0s\0i\0"))
            .unwrap_or(false);
        let k = if psi { 6_894.757_293_168 / 1e5 } else { 1.0 };
        for (t, y) in b.time.iter().zip(&b.intensity) {
            let j = a.time.partition_point(|x| x < &(t - 1e-6));
            assert!((a.time[j] - t).abs() < 1e-5, "time {t} of {raw_path:?}");
            let want = y * k;
            assert!(
                (a.intensity[j] - want).abs() <= want.abs() * 2e-6,
                "pressure at {t} min of {raw_path:?}: {} against {want}",
                a.intensity[j]
            );
        }
        checked += 1;
    }
    assert!(checked > 0, "no .raw with a pressure channel");
}
