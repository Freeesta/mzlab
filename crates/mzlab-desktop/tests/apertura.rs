//! Files opened from outside (`src/apertura.rs`): which arguments count, the queue, and the alternate notice «make mzLab the default app?».

use mzlab_desktop::apertura::{
    paths_from_args, registra_apertura, rifai, rispondi, Pending, Risposta,
};
use mzlab_desktop::librerie::{read_config, write_config, Config};

fn s(x: &std::path::Path) -> String {
    x.to_string_lossy().into_owned()
}

#[test]
fn only_single_mzml_and_raw_files_are_taken() {
    let t = tempfile::tempdir().unwrap();
    let d = t.path();
    for n in ["a.mzML", "b.mzml", "c.RAW", "d.txt", "e.dam"] {
        std::fs::write(d.join(n), b"x").unwrap();
    }
    // a Waters/Agilent-like folder named like a file cannot be associated
    std::fs::create_dir(d.join("w.raw")).unwrap();
    let args: Vec<String> = [
        "--flag",
        "a.mzML",
        "b.mzml",
        "c.RAW",
        "d.txt",
        "e.dam",
        "w.raw",
        "missing.mzML",
    ]
    .iter()
    .map(|x| x.to_string())
    .collect();
    let got = paths_from_args(&args, Some(d));
    assert_eq!(
        got,
        vec![d.join("a.mzML"), d.join("b.mzml"), d.join("c.RAW")]
    );
    // an absolute path wins over the working directory
    let abs = vec![s(&d.join("a.mzML"))];
    assert_eq!(
        paths_from_args(&abs, Some(std::path::Path::new("/nowhere"))),
        vec![d.join("a.mzML")]
    );
    assert!(paths_from_args(&[], None).is_empty());
}

#[test]
fn second_launch_argv_skips_the_program_and_resolves_against_its_cwd() {
    let t = tempfile::tempdir().unwrap();
    let d = t.path();
    std::fs::write(d.join("x.mzML"), b"x").unwrap();
    // what the single-instance plugin hands over: the program name first, then the relative file
    let argv: Vec<String> = ["mzlab", "x.mzML"].iter().map(|x| x.to_string()).collect();
    assert_eq!(
        paths_from_args(argv.get(1..).unwrap_or_default(), Some(d)),
        vec![d.join("x.mzML")]
    );
    assert!(paths_from_args([].as_slice(), Some(d)).is_empty());
}

#[test]
fn pending_queue_keeps_order_and_is_emptied_once() {
    let q = Pending::default();
    q.push(["/x/a.mzML".into(), "/x/b.mzML".into()]);
    q.push(["/x/a.mzML".into()]);
    assert_eq!(q.take().len(), 2);
    assert!(q.take().is_empty());
}

#[test]
fn notice_comes_every_other_opening_until_a_final_answer() {
    let mut c = Config::default();
    let seq: Vec<bool> = (0..6).map(|_| registra_apertura(&mut c)).collect();
    assert_eq!(seq, [true, false, true, false, true, false]);
    // «No (not now)»: nothing changes
    rispondi(&mut c, None);
    assert!(registra_apertura(&mut c));
    // «Don't ask again» stops for good, and so does «Yes»
    rispondi(&mut c, Some(Risposta::Mai));
    assert!((0..5).all(|_| !registra_apertura(&mut c)));
    rifai(&mut c);
    assert_eq!((c.aperture, c.risposta), (0, None));
    assert!(registra_apertura(&mut c));
    rispondi(&mut c, Some(Risposta::Si));
    assert!((0..5).all(|_| !registra_apertura(&mut c)));
}

#[test]
fn counter_and_answer_survive_in_the_config_file() {
    let t = tempfile::tempdir().unwrap();
    let mut c = read_config(t.path());
    registra_apertura(&mut c);
    rispondi(&mut c, Some(Risposta::Si));
    write_config(t.path(), &c).unwrap();
    assert_eq!(read_config(t.path()), c);
    assert_eq!(read_config(t.path()).risposta, Some(Risposta::Si));
    // a config.json written before this existed still reads
    std::fs::write(t.path().join("config.json"), r#"{"esterne":true}"#).unwrap();
    let old = read_config(t.path());
    assert!(old.esterne && old.aperture == 0 && old.risposta.is_none());
}
