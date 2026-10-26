//! The optical detector (DAD) in the desktop engine: the ops `lr_pda`, `lr_pda_map`, `lr_pda_spectrum` and `lr_pda_wavelength` answer, for a demo mzML with the
//! matrix and for a synthetic `.wiff` + `.wiff.scan` pair with a DAD, the very strings of the routes of `mzlab-core` that the wasm engine runs on the same file.
//! The pair is written by `tools/wiff_riferimento.py` (no binary in the repository); the wiff cases are skipped when python3 is not there.
use mzlab_core::idxread::ReadAt;
use mzlab_core::lettori::{FileSet, Lettore, Limiti, Membro};
use mzlab_core::mzmlhead::Mzml;
use mzlab_core::{itemroutes, lettori_mzml};
use mzlab_desktop::rpc::Answer;
use mzlab_desktop::State;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const PEAK_NM: f64 = 260.0;
const PEAK_RT: f64 = 1.0;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn text(a: Answer) -> String {
    match a {
        Answer::Text(t) => t,
        Answer::Bytes(_) => panic!("bytes"),
    }
}

/// `uv.wiff` (DAD with a peak at 260 nm, 1.0 min) and `plain.wiff` (no detector) in `dir`; false when python3 cannot write them.
fn write_pairs(dir: &Path) -> bool {
    let script = r#"
import sys, math
sys.path.insert(0, sys.argv[1] + "/tools")
from pathlib import Path
import wiff_riferimento as wr
out = Path(sys.argv[2])
blocks = [("scan", [(100 + 7 * j, 50 + k + (500 if j == 5 else 0)) for j in range(12)], 1000.0 * k, 50.0 + k) for k in range(40)]
ts = [0.05 * k for k in range(40)]
rows = [[10 + 400 * math.exp(-((j - 60) / 12) ** 2) * math.exp(-((t - 1.0) / 0.2) ** 2) for j in range(201)] for t in ts]
meth = {"Experiment0/ExperimentHeader": wr.header(15, 30.0)}
w = wr.Writer(); w.sample(1, blocks, meth, dad=(ts, 200.0, 1.0, rows)); w.save(out, "uv")
w = wr.Writer(); w.sample(1, blocks, meth); w.save(out, "plain")
"#;
    std::process::Command::new("python3")
        .args(["-I", "-c", script])
        .arg(root())
        .arg(dir)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn state(dir: &Path) -> State {
    let st = State::new();
    st.set_data_folder(dir.join("app"));
    st.set_index_folder(dir.join("idx")).unwrap();
    st
}

/// What the routes read for a native sample: its `guscio` and its reader, as `Src::Nat` of the engines.
struct Shell {
    g: Vec<u8>,
    l: Box<dyn Lettore>,
}

impl ReadAt for Shell {
    fn read_at(&mut self, off: u64, buf: &mut [u8]) -> mzlab_core::Result<()> {
        self.g.read_at(off, buf)
    }
    fn lettore(&mut self) -> Option<(&mut dyn Lettore, usize)> {
        Some((&mut *self.l, 0))
    }
}

/// The mzML of the sample 0 of a `.wiff` pair, opened as the wasm engine does.
fn reference_wiff(wiff: &Path) -> Mzml<Shell> {
    let member = |name: &str, p: PathBuf| {
        let bytes = std::fs::read(p).unwrap();
        Membro {
            nome: name.to_string(),
            dim: bytes.len() as u64,
            dati: Box::new(bytes),
            sha1: None,
        }
    };
    let n = wiff.file_name().unwrap().to_str().unwrap();
    let mut scan = wiff.as_os_str().to_owned();
    scan.push(".scan");
    let set = FileSet {
        membri: vec![
            member(n, wiff.to_path_buf()),
            member(&format!("{n}.scan"), scan.into()),
        ],
    };
    let mut l = mzlab_lettori::apri(set, &Limiti::desktop()).unwrap();
    let g = lettori_mzml::guscio(&mut *l, 0).unwrap();
    let size = g.len() as u64;
    Mzml::new(Shell { g, l }, size)
}

fn register_wiff(st: &State, wiff: &Path) -> String {
    let n = wiff.file_name().unwrap().to_str().unwrap().to_string();
    let scan = PathBuf::from(format!("{}.scan", wiff.display()));
    let mut members = Vec::new();
    for (name, p) in [(n.clone(), wiff.to_path_buf()), (format!("{n}.scan"), scan)] {
        let (fid, _) = st.pick(p).unwrap();
        members.push(json!({"name": name, "fid": fid}));
    }
    let nat = json!({"key": n, "c": 0, "id": 0, "centroid": false, "members": members});
    st.rpc("lr_put", &json!({"name": n, "nat": nat}), &[])
        .unwrap();
    st.rpc("lr_index", &json!({"name": n}), &[]).unwrap();
    n
}

type Pair = (&'static str, Result<String, String>, Result<String, String>);

/// The four routes of the detector through `State::rpc`, each as `(desktop, core)`; the spectrum twice (one time, an interval).
fn answers<R: ReadAt>(st: &State, name: &str, m: &mut Mzml<R>) -> Vec<Pair> {
    let nat = |op: &str, p: Value| st.rpc(op, &p, &[]).map(text).map_err(|e| e.key.to_string());
    let chroms = m.chromatograms(0).unwrap();
    let e = |r: mzlab_core::Result<String>| r.map_err(|e| e.key.to_string());
    vec![
        (
            "map",
            nat("lr_pda_map", json!({"name": name})),
            e(itemroutes::pda_map(m)),
        ),
        (
            "spectrum at a time",
            nat(
                "lr_pda_spectrum",
                json!({"name": name, "rt0": PEAK_RT, "rt1": PEAK_RT}),
            ),
            e(itemroutes::pda_spectrum(m, PEAK_RT, PEAK_RT)),
        ),
        (
            "spectrum over an interval",
            nat(
                "lr_pda_spectrum",
                json!({"name": name, "rt0": 0.9, "rt1": 1.1}),
            ),
            e(itemroutes::pda_spectrum(m, 0.9, 1.1)),
        ),
        (
            "wavelength trace",
            nat(
                "lr_pda_wavelength",
                json!({"name": name, "nm0": PEAK_NM - 2.0, "nm1": PEAK_NM + 2.0}),
            ),
            e(itemroutes::pda_wavelength(m, PEAK_NM - 2.0, PEAK_NM + 2.0)),
        ),
        (
            "trace",
            nat("lr_pda", json!({"name": name})),
            e(itemroutes::pda_trace(m, &chroms)),
        ),
    ]
}

#[test]
fn a_demo_mzml_with_the_matrix_answers_as_the_core() {
    let ex = root().join("mzlab/web/esempi/FullScan_t10.mzML");
    let dir = tempfile::tempdir().unwrap();
    let st = state(dir.path());
    let (id, _) = st.pick(ex.clone()).unwrap();
    let name = "FullScan_t10.mzML";
    st.rpc("lr_put", &json!({"name": name, "fid": id}), &[])
        .unwrap();
    st.rpc("lr_index", &json!({"name": name}), &[]).unwrap();
    let size = std::fs::metadata(&ex).unwrap().len();
    let mut m = Mzml::new(std::fs::File::open(&ex).unwrap(), size);
    for (what, got, want) in answers(&st, name, &mut m) {
        assert!(want.is_ok(), "{what}: the demo has the matrix: {want:?}");
        assert_eq!(got, want, "{what}");
    }
}

#[test]
fn a_wiff_with_a_dad_answers_as_the_core_with_the_peak_at_260_nm() {
    let dir = tempfile::tempdir().unwrap();
    if !write_pairs(dir.path()) {
        eprintln!("SKIP: python3 cannot write the synthetic .wiff pair");
        return;
    }
    let st = state(dir.path());
    let name = register_wiff(&st, &dir.path().join("uv.wiff"));
    let mut m = reference_wiff(&dir.path().join("uv.wiff"));
    for (what, got, want) in answers(&st, &name, &mut m) {
        assert!(
            want.is_ok(),
            "{what}: the wiff has the detector: core {want:?}, desktop {got:?}"
        );
        assert_eq!(got, want, "{what}");
    }
    let spec: Value = serde_json::from_str(&text(
        st.rpc(
            "lr_pda_spectrum",
            &json!({"name": name, "rt0": PEAK_RT, "rt1": PEAK_RT}),
            &[],
        )
        .unwrap(),
    ))
    .unwrap();
    let col = |k: &str| {
        spec[k]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect::<Vec<_>>()
    };
    let (nm, mau) = (col("nm"), col("mau"));
    let top = (0..mau.len())
        .max_by(|&a, &b| mau[a].total_cmp(&mau[b]))
        .unwrap();
    assert_eq!(
        nm[top], PEAK_NM,
        "the UV spectrum at the peak has its maximum at 260 nm"
    );
}

#[test]
fn a_wiff_without_a_dad_has_no_pda_in_the_desktop_and_in_the_core() {
    let dir = tempfile::tempdir().unwrap();
    if !write_pairs(dir.path()) {
        eprintln!("SKIP: python3 cannot write the synthetic .wiff pair");
        return;
    }
    let st = state(dir.path());
    let name = register_wiff(&st, &dir.path().join("plain.wiff"));
    let mut m = reference_wiff(&dir.path().join("plain.wiff"));
    for (what, got, want) in answers(&st, &name, &mut m) {
        if what == "trace" {
            assert_eq!(got, want, "{what}: an empty trace in both");
        } else {
            assert_eq!(want, Err("err.pda.none".to_string()), "{what}");
            assert_eq!(got, want, "{what}");
        }
    }
}
