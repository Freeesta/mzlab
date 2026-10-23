//! `upload::prepare_late` with `mzidx::build_mzml_late` writes the same `.mzidx` as `prepare` with `build_mzml_spill`: the fingerprints taken
//! by the second thread reach the metadata when it is written.

use std::fs::File;
use std::path::Path;

use mzlab_core::mzidx::{build_mzml_late, build_mzml_spill};
use mzlab_core::upload::{prepare, prepare_late};

#[test]
fn late_source_gives_the_same_index() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mzlab/web/esempi");
    let mut seen = 0;
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let path = e.path();
        if path.extension().is_none_or(|x| x != "mzML") {
            continue;
        }
        let size = std::fs::metadata(&path).unwrap().len();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let p = prepare(&mut File::open(&path).unwrap(), size, &name).unwrap();
        let mut want = std::io::Cursor::new(Vec::new());
        build_mzml_spill(
            &p.table,
            std::io::BufReader::new(File::open(&path).unwrap()),
            &mut want,
            1 << 24,
            None,
        )
        .unwrap();
        let (table, mut late) = prepare_late(
            &mut File::open(&path).unwrap(),
            File::open(&path).unwrap(),
            size,
            &name,
        )
        .unwrap();
        let mut got = std::io::Cursor::new(Vec::new());
        build_mzml_late(
            &table,
            std::io::BufReader::new(File::open(&path).unwrap()),
            &mut got,
            1 << 24,
            None,
            Some(&mut late),
        )
        .unwrap();
        assert!(got.into_inner() == want.into_inner(), "{name}");
        seen += 1;
        if seen == 3 {
            break;
        }
    }
    assert!(seen > 0, "no example files");
}
