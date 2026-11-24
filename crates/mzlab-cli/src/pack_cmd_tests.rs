//! `file pack`, `file unpack`, `file check-pack` on synthetic folders: a round trip over many parts, the same bytes twice, a flipped byte, a missing part,
//! nothing overwritten without `--force`, an unsafe manifest, the exit codes and the log on the standard error.

use std::path::{Path, PathBuf};

use mzlab_core::json::parse;

use crate::execute;

/// The commands of this file with an input that does not exist (`contract_tests` runs them under the missing-input contract: exit 4, one `error/1`).
pub(crate) const MISSING: &[&[&str]] = &[
    &["file", "pack", "/nonexistent/x", "--out", "/nonexistent/o"],
    &[
        "file",
        "unpack",
        "/no/x.pack.json",
        "--out",
        "/nonexistent/o",
    ],
    &["file", "check-pack", "/nonexistent/x.pack.json"],
];

fn cli(a: &[&str]) -> crate::Outcome {
    execute(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>())
}

fn s(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

fn tmp(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab-pack-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn data(n: usize, seed: u32) -> Vec<u8> {
    (0..n)
        .map(|i| ((i as u32).wrapping_mul(2_654_435_761).wrapping_add(seed) >> 11) as u8)
        .collect()
}

/// A campaign-like tree: a `.D` folder with nested files, a loose file bigger than a part, an empty file.
fn campaign(d: &Path) -> Vec<(String, Vec<u8>)> {
    let files = vec![
        ("camp/s1.D/AcqData/MSScan.bin".to_string(), data(9000, 1)),
        ("camp/s1.D/AcqData/MSPeak.bin".to_string(), data(2500, 2)),
        ("camp/s1.D/sample.xml".to_string(), data(700, 3)),
        ("camp/s2.wiff".to_string(), data(5000, 4)),
        ("camp/empty.txt".to_string(), Vec::new()),
    ];
    for (n, b) in &files {
        let p = d.join(n);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, b).unwrap();
    }
    files
}

fn pack(src: &Path, out: &Path, extra: &[&str]) -> crate::Outcome {
    let mut a = vec!["file", "pack", "--part-size", "1KiB", "--json"];
    let camp = s(&src.join("camp"));
    a.push(&camp);
    a.push("--out");
    let o = s(out);
    a.push(&o);
    a.extend_from_slice(extra);
    cli(&a)
}

#[test]
fn pack_then_unpack_gives_back_every_file_byte_for_byte() {
    let (src, out, back) = (tmp("rt-src"), tmp("rt-out"), tmp("rt-back"));
    let files = campaign(&src);
    let o = pack(&src, &out, &[]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert!(
        o.stderr.contains("part size 1KiB = 1024 bytes")
            && o.stderr.contains("name: pack by default"),
        "{}",
        o.stderr
    );
    let doc = parse(&o.stdout).unwrap();
    assert_eq!(doc.get("schema").unwrap().str(), Some("pack-run/1"));
    let parts = doc.get("parts").unwrap().arr().len();
    assert!(parts >= 15, "{parts}");
    assert!(out.join("pack.part0001").exists() && out.join("pack.pack.json").exists());
    let u = cli(&[
        "file",
        "unpack",
        &s(&out.join("pack.pack.json")),
        "--out",
        &s(&back),
        "--json",
    ]);
    assert_eq!(u.code, 0, "{}", u.stderr);
    assert_eq!(
        parse(&u.stdout).unwrap().get("ok").unwrap().bool(),
        Some(true)
    );
    for (n, b) in &files {
        assert!(&std::fs::read(back.join(n)).unwrap() == b, "{n}");
    }
    let v = cli(&[
        "file",
        "check-pack",
        &s(&out.join("pack.pack.json")),
        "--json",
    ]);
    assert_eq!(
        (v.code, parse(&v.stdout).unwrap().get("ok").unwrap().bool()),
        (0, Some(true))
    );
}

#[test]
fn the_same_input_gives_the_same_bytes_whatever_the_folder_or_the_order_given() {
    let (src, a, b, c) = (tmp("det-src"), tmp("det-a"), tmp("det-b"), tmp("det-c"));
    campaign(&src);
    assert_eq!(pack(&src, &a, &[]).code, 0);
    let (s1, s2) = (s(&src.join("camp/s2.wiff")), s(&src.join("camp/s1.D")));
    let o = cli(&[
        "file",
        "pack",
        &s1,
        &s2,
        "--out",
        &s(&b),
        "--part-size",
        "1KiB",
    ]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let o2 = cli(&[
        "file",
        "pack",
        &s2,
        &s1,
        "--out",
        &s(&c),
        "--part-size",
        "1KiB",
    ]);
    assert_eq!(o2.code, 0);
    for f in std::fs::read_dir(&b).unwrap() {
        let f = f.unwrap();
        let other = c.join(f.file_name());
        assert!(
            std::fs::read(f.path()).unwrap() == std::fs::read(other).unwrap(),
            "{:?}",
            f.file_name()
        );
    }
    let (x, y) = (tmp("det-x"), tmp("det-y"));
    assert_eq!(pack(&src, &x, &[]).code, 0);
    assert_eq!(pack(&src, &y, &[]).code, 0);
    for f in std::fs::read_dir(&x).unwrap() {
        let f = f.unwrap();
        assert!(std::fs::read(f.path()).unwrap() == std::fs::read(y.join(f.file_name())).unwrap());
    }
}

#[test]
fn a_flipped_byte_exits_3_naming_the_part_and_unpack_keeps_the_other_files() {
    let (src, out, back) = (tmp("bad-src"), tmp("bad-out"), tmp("bad-back"));
    let files = campaign(&src);
    assert_eq!(pack(&src, &out, &[]).code, 0);
    let part = out.join("pack.part0003");
    let mut b = std::fs::read(&part).unwrap();
    b[5] ^= 0x40;
    std::fs::write(&part, b).unwrap();
    let man = s(&out.join("pack.pack.json"));
    let v = cli(&["file", "check-pack", &man, "--json"]);
    assert_eq!(v.code, 3, "{}", v.stderr);
    let d = parse(&v.stdout).unwrap();
    assert_eq!(
        d.get("resend").unwrap().arr()[0].str(),
        Some("pack.part0003")
    );
    assert!(
        v.stderr.contains("part 3/") && v.stderr.contains("corrupt"),
        "{}",
        v.stderr
    );
    let u = cli(&["file", "unpack", &man, "--out", &s(&back), "--json"]);
    assert_eq!(u.code, 3);
    let d = parse(&u.stdout).unwrap();
    let lost: Vec<&str> = d
        .get("lost")
        .unwrap()
        .arr()
        .iter()
        .map(|x| x.str().unwrap())
        .collect();
    assert!(!lost.is_empty());
    for (n, bytes) in &files {
        let rel = n.as_str();
        if lost.contains(&rel) {
            assert!(!back.join(rel).exists(), "{rel}");
        } else {
            assert!(&std::fs::read(back.join(rel)).unwrap() == bytes, "{rel}");
        }
    }
    assert!(std::fs::read_dir(&back).unwrap().count() > 0);
}

#[test]
fn a_missing_part_exits_1_and_only_that_part_has_to_be_sent_again() {
    let (src, out, back) = (tmp("miss-src"), tmp("miss-out"), tmp("miss-back"));
    let files = campaign(&src);
    assert_eq!(pack(&src, &out, &[]).code, 0);
    let saved = std::fs::read(out.join("pack.part0004")).unwrap();
    std::fs::remove_file(out.join("pack.part0004")).unwrap();
    let man = s(&out.join("pack.pack.json"));
    let v = cli(&["file", "check-pack", &man, "--json"]);
    assert_eq!(v.code, 1);
    let d = parse(&v.stdout).unwrap();
    assert_eq!(
        d.get("missing").unwrap().arr()[0].str(),
        Some("pack.part0004")
    );
    assert_eq!(d.get("resend").unwrap().arr().len(), 1);
    let u = cli(&["file", "unpack", &man, "--out", &s(&back), "--json"]);
    assert_eq!(u.code, 1, "{}", u.stderr);
    assert!(parse(&u.stdout).unwrap().get("lost").unwrap().arr().len() < files.len());
    // resent: the second round needs only the missing part, and nothing existing is replaced without --force
    std::fs::write(out.join("pack.part0004"), saved).unwrap();
    assert_eq!(cli(&["file", "check-pack", &man]).code, 0);
    let again = cli(&["file", "unpack", &man, "--out", &s(&back)]);
    assert_eq!(again.code, 6, "{}", again.stderr);
    let forced = cli(&["file", "unpack", &man, "--out", &s(&back), "--force"]);
    assert_eq!(forced.code, 0, "{}", forced.stderr);
    for (n, b) in &files {
        assert!(&std::fs::read(back.join(n)).unwrap() == b, "{n}");
    }
}

#[test]
fn nothing_is_overwritten_without_force_and_a_dry_run_writes_nothing() {
    let (src, out) = (tmp("ow-src"), tmp("ow-out"));
    campaign(&src);
    let dry = pack(&src, &out, &["--dry-run"]);
    assert_eq!(dry.code, 0, "{}", dry.stderr);
    assert_eq!(std::fs::read_dir(&out).unwrap().count(), 0);
    assert!(dry.stderr.contains("dry run"));
    assert!(parse(&dry.stdout).unwrap().get("files").unwrap().arr()[0]
        .get("sha256")
        .unwrap()
        .is_null());
    assert_eq!(pack(&src, &out, &[]).code, 0);
    let before = std::fs::read(out.join("pack.pack.json")).unwrap();
    let again = pack(&src, &out, &[]);
    assert_eq!(again.code, 6, "{}", again.stderr);
    assert!(std::fs::read(out.join("pack.pack.json")).unwrap() == before);
    assert_eq!(pack(&src, &out, &["--force"]).code, 0);
    // the dry run plans the parts the real run writes
    let planned = parse(&dry.stdout).unwrap();
    let real = parse(&pack(&src, &tmp("ow-out2"), &[]).stdout).unwrap();
    assert_eq!(
        planned.get("stream_bytes").unwrap().f64(),
        real.get("stream_bytes").unwrap().f64()
    );
    assert_eq!(
        planned.get("parts").unwrap().arr().len(),
        real.get("parts").unwrap().arr().len()
    );
}

#[test]
fn an_output_folder_inside_the_input_is_left_out_of_the_pack() {
    let src = tmp("in-src");
    campaign(&src);
    let out = src.join("camp/packed");
    let o = pack(&src, &out, &[]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let o = pack(&src, &out, &["--force", "--name", "second"]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert!(o.stderr.contains("inside the --out folder"), "{}", o.stderr);
    let d = parse(&o.stdout).unwrap();
    assert_eq!(d.get("files").unwrap().arr().len(), 5);
}

#[test]
fn a_manifest_with_a_path_that_leaves_the_folder_is_refused_and_nothing_is_written() {
    let (src, out, back) = (tmp("evil-src"), tmp("evil-out"), tmp("evil-back"));
    campaign(&src);
    assert_eq!(pack(&src, &out, &[]).code, 0);
    let man = out.join("pack.pack.json");
    let text = std::fs::read_to_string(&man)
        .unwrap()
        .replace("camp/empty.txt", "../escaped.txt");
    std::fs::write(&man, text).unwrap();
    let u = cli(&["file", "unpack", &s(&man), "--out", &s(&back.join("in"))]);
    assert_eq!(u.code, 3, "{}", u.stderr);
    assert!(!back.join("escaped.txt").exists() && !back.join("in").exists());
}

#[test]
fn usage_and_input_errors_have_their_own_codes() {
    let (src, out) = (tmp("use-src"), tmp("use-out"));
    campaign(&src);
    assert_eq!(cli(&["file", "pack", "--out", &s(&out)]).code, 2);
    assert_eq!(
        cli(&[
            "file",
            "pack",
            &s(&src),
            "--out",
            &s(&out),
            "--part-size",
            "5"
        ])
        .code,
        2
    );
    assert_eq!(
        cli(&[
            "file",
            "pack",
            &s(&src),
            "--out",
            &s(&out),
            "--part-size",
            "9X"
        ])
        .code,
        2
    );
    assert_eq!(
        cli(&["file", "pack", &s(&src.join("nope")), "--out", &s(&out)]).code,
        4
    );
    assert_eq!(
        cli(&["file", "unpack", "/no/such.pack.json", "--out", &s(&out)]).code,
        4
    );
    assert_eq!(cli(&["file", "check-pack", "/no/such.pack.json"]).code, 4);
    assert_eq!(cli(&["file", "check-pack"]).code, 2);
    let bad = out.join("x.pack.json");
    std::fs::write(&bad, "{}").unwrap();
    assert_eq!(cli(&["file", "check-pack", &s(&bad)]).code, 3);
    let j = cli(&["file", "check-pack", &s(&bad), "--json"]);
    assert_eq!(
        parse(&j.stdout).unwrap().get("schema").unwrap().str(),
        Some("error/1")
    );
}

#[test]
fn a_failed_force_repack_leaves_the_older_pack_as_it_was() {
    let (src, out) = (tmp("keep-src"), tmp("keep-out"));
    campaign(&src);
    let args = |extra: &[&str]| -> Vec<String> {
        [&["--out", &s(&out), "--part-size", "4096"], extra]
            .concat()
            .iter()
            .map(|x| x.to_string())
            .collect()
    };
    assert_eq!(
        crate::pack_cmd::pack_run(args(&[&s(&src)])).code,
        0,
        "first pack"
    );
    let snapshot = |d: &Path| -> Vec<(String, Vec<u8>)> {
        let mut v: Vec<_> = std::fs::read_dir(d)
            .unwrap()
            .map(|e| e.unwrap())
            .map(|e| {
                (
                    e.file_name().to_string_lossy().into_owned(),
                    std::fs::read(e.path()).unwrap(),
                )
            })
            .collect();
        v.sort();
        v
    };
    let before = snapshot(&out);
    // the input vanishes after it was listed: the pack stops half way through writing
    let job = crate::pack_cmd::job_of(args(&[&s(&src), "--force"]))
        .ok()
        .unwrap();
    let items = vec![crate::pack_cmd::Item {
        rel: "gone".into(),
        path: src.join("gone"),
        size: 10,
    }];
    let list = vec![("gone".to_string(), 10)];
    let o = mzlab_core::pack::Options {
        name: "pack".into(),
        part_size: 4096,
        zip64_at: mzlab_core::zipstream::ZIP64_AT,
    };
    assert_eq!(crate::pack_cmd::write_pack(job, &items, &list, &o).code, 4);
    assert_eq!(
        snapshot(&out),
        before,
        "old parts and manifest are untouched"
    );
}

/// A folder whose files have names no pack can carry: both the real run and `--dry-run` give exit 3 and write nothing.
#[test]
fn unusable_input_names_exit_three_in_a_real_run_and_a_dry_run() {
    let cases: [(&str, &[&str]); 3] = [
        ("case", &["A.txt", "a.txt"]),
        ("colon", &["c:x.txt"]),
        ("folder", &["a", "A/b"]),
    ];
    for (tag, names) in cases {
        let (src, out) = (
            tmp(&format!("bad-src-{tag}")),
            tmp(&format!("bad-out-{tag}")),
        );
        for n in names {
            let at = src.join(n);
            std::fs::create_dir_all(at.parent().unwrap()).unwrap();
            std::fs::write(&at, b"x").unwrap();
        }
        let (sd, od) = (s(&src), s(&out));
        for extra in [&[][..], &["--dry-run", "--json"][..]] {
            let mut a = vec!["file", "pack", &sd, "--out", &od];
            a.extend_from_slice(extra);
            let r = cli(&a);
            assert_eq!(r.code, 3, "{tag} {extra:?}: {}", r.stderr);
            assert!(!out.join("pack.pack.json").exists(), "{tag} {extra:?}");
        }
    }
}

#[test]
fn one_input_given_twice_is_a_clash_like_any_other_not_a_usage_error() {
    let (src, out) = (tmp("twice-src"), tmp("twice-out"));
    std::fs::write(src.join("f.bin"), b"x").unwrap();
    let (f, od) = (s(&src.join("f.bin")), s(&out));
    for extra in [&[][..], &["--dry-run"][..]] {
        let mut a = vec!["file", "pack", &f, &f, "--out", &od];
        a.extend_from_slice(extra);
        let r = cli(&a);
        assert_eq!(r.code, 3, "{extra:?}: {}", r.stderr);
        assert!(r.stderr.contains("err.pack.duplicate"), "{}", r.stderr);
    }
}

#[test]
fn a_manifest_that_cannot_be_written_leaves_the_older_parts_in_place() {
    let (src, out) = (tmp("mf-src"), tmp("mf-out"));
    campaign(&src);
    let (sd, od) = (s(&src), s(&out));
    let go = |extra: &[&str]| {
        let mut a = vec!["file", "pack", sd.as_str()];
        a.extend_from_slice(&["--out", od.as_str(), "--part-size", "4096"]);
        a.extend_from_slice(extra);
        cli(&a)
    };
    assert_eq!(go(&[]).code, 0);
    let part = out.join("pack.part0001");
    let before = std::fs::read(&part).unwrap();
    std::fs::write(src.join("added.bin"), data(5000, 9)).unwrap();
    // a folder where the manifest's temporary file goes: writing it fails before any older part is replaced
    std::fs::create_dir(out.join(".pack.pack.json.tmp")).unwrap();
    assert_eq!(go(&["--force"]).code, 6);
    assert_eq!(
        std::fs::read(&part).unwrap(),
        before,
        "the older part is untouched"
    );
}
