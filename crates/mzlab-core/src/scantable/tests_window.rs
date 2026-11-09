use super::DeclaredWindow;
use std::time::Instant;

fn header(lo: f64, hi: f64, pad: usize) -> String {
    format!(
        "<spectrum index=\"0\"><cvParam cvRef=\"MS\" accession=\"MS:1000511\" name=\"ms level\" value=\"1\"/>{}<scanWindow><cvParam cvRef=\"MS\" accession=\"MS:1000501\" name=\"scan window lower limit\" value=\"{lo}\"/><cvParam cvRef=\"MS\" accession=\"MS:1000500\" name=\"scan window upper limit\" value=\"{hi}\"/></scanWindow>",
        " ".repeat(pad)
    )
}

fn run(n: usize, pad: usize) -> (Option<(f64, f64)>, f64) {
    let h = header(100.0, 900.0, pad);
    let late = header(1.0, 5000.0, pad);
    let mut w = DeclaredWindow::default();
    let t = Instant::now();
    for i in 0..n {
        w.note(
            if i < 100 { &h } else { &late },
            if i % 2 == 0 { 1 } else { 2 },
        );
    }
    (w.span, t.elapsed().as_secs_f64())
}

#[test]
fn only_the_first_50_surveys_set_the_window() {
    let (span, _) = run(1000, 0);
    assert_eq!(span, Some((100.0, 900.0)));
}

/// `cargo test -p mzlab-core --release -- --ignored --nocapture note_bench`: 100k scans, 2 KB headers.
#[test]
#[ignore]
fn note_bench() {
    let (_, s) = run(100_000, 2000);
    println!(
        "DeclaredWindow::note x 100000 headers of 2 KB: {:.1} ms",
        s * 1000.0
    );
}
