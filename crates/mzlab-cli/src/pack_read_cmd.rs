//! `file unpack` and `file check-pack` on a folder of parts (docs/agenti/file-pack.md): the parts are read from a folder, the files written under another,
//! and the result of `mzlab_core::pack_read` becomes the exit code, the JSON document and the log on the standard error.

use std::fs::{self, File};
use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use mzlab_core::error::Error;
use mzlab_core::pack::{self, Manifest};
use mzlab_core::pack_read::{self, FileSink, PartSource, PartState};

use crate::extract;
use crate::pack_cmd::{done_json_or_text, flags, out_err, with_log, UNPACK_HELP, VERIFY_HELP};
use crate::{exit, Outcome};

/// The parts in a folder, by the names the manifest lists.
struct DirParts {
    dir: PathBuf,
    names: Vec<String>,
}

impl PartSource for DirParts {
    fn size(&mut self, i: usize) -> Result<Option<u64>, Error> {
        let p = self.dir.join(self.names.get(i).map_or("", String::as_str));
        match fs::metadata(&p) {
            Ok(m) if m.is_file() => Ok(Some(m.len())),
            Ok(_) => Ok(None),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => {
                Err(Error::new("err.run.input").with("detail", format!("{}: {e}", p.display())))
            }
        }
    }
    fn open(&mut self, i: usize, from: u64) -> Result<Box<dyn Read>, Error> {
        use std::io::{Seek, SeekFrom};
        let p = self.dir.join(self.names.get(i).map_or("", String::as_str));
        let mut f = File::open(&p).map_err(|e| {
            Error::new("err.run.input").with("detail", format!("{}: {e}", p.display()))
        })?;
        f.seek(SeekFrom::Start(from))
            .map_err(|e| Error::new("err.run.input").with("detail", e))?;
        Ok(Box::new(f))
    }
}

/// Files written under a folder through a temporary name, renamed only when the SHA-256 matched.
struct DirFiles {
    root: PathBuf,
    cur: Option<(PathBuf, PathBuf, BufWriter<File>)>,
}

impl FileSink for DirFiles {
    fn exists(&mut self, path: &str) -> Result<bool, Error> {
        Ok(fs::symlink_metadata(self.root.join(path)).is_ok())
    }
    fn begin(&mut self, path: &str) -> Result<(), Error> {
        let to = self.root.join(path);
        if let Some(d) = to.parent() {
            fs::create_dir_all(d).map_err(|e| out_err(format!("{}: {e}", d.display())))?;
        }
        let tmp = to.with_file_name(format!(
            ".{}.mzpack-tmp",
            to.file_name().and_then(|n| n.to_str()).unwrap_or("f")
        ));
        let f = File::create(&tmp).map_err(|e| out_err(format!("{}: {e}", tmp.display())))?;
        self.cur = Some((tmp, to, BufWriter::with_capacity(1 << 20, f)));
        Ok(())
    }
    fn write(&mut self, bytes: &[u8]) -> Result<(), Error> {
        match self.cur.as_mut() {
            Some((_, _, w)) => w.write_all(bytes).map_err(out_err),
            None => Ok(()),
        }
    }
    fn end(&mut self, keep: bool) -> Result<(), Error> {
        let Some((tmp, to, mut w)) = self.cur.take() else {
            return Ok(());
        };
        let flushed = w.flush();
        drop(w);
        if keep && flushed.is_ok() {
            // `--force` replaces on purpose; rename over a file is the replacement
            fs::rename(&tmp, &to).map_err(|e| out_err(format!("{}: {e}", to.display())))
        } else {
            let _ = fs::remove_file(&tmp);
            flushed.map_err(out_err)
        }
    }
}

fn load(manifest: &str) -> Result<(Manifest, PathBuf), Outcome> {
    let bytes = fs::read(manifest)
        .map_err(|_| Outcome::fail_error(&Error::new("err.cli.file").with("path", manifest)))?;
    let m = pack::parse_manifest(&bytes).map_err(|e| Outcome::fail_error(&e))?;
    Ok((
        m,
        Path::new(manifest)
            .parent()
            .map_or_else(|| PathBuf::from("."), Path::to_path_buf),
    ))
}

fn code_of(parts: &[PartState], files: &[pack_read::FileState]) -> u8 {
    match pack_read::damage(parts, files) {
        (true, _) => exit::INTEGRITY,
        (false, true) => exit::DATA,
        _ => exit::OK,
    }
}

fn counts(parts: &[PartState]) -> String {
    let n = |s: PartState| parts.iter().filter(|p| p.key() == s.key()).count();
    format!(
        "{} parts: {} ok, {} missing, {} corrupt",
        parts.len(),
        n(PartState::Ok),
        n(PartState::Missing),
        n(PartState::Corrupt(""))
    )
}

pub fn verify_run(args: Vec<String>) -> Outcome {
    let json = flags(&args);
    let p = match extract::parse(args, &["--parts"], &[]) {
        Ok(Some(p)) => p,
        Ok(None) => return Outcome::ok(format!("{VERIFY_HELP}\n").into_bytes()),
        Err(e) => return Outcome::fail(&e),
    };
    let [manifest] = p.pos.as_slice() else {
        return Outcome::fail("file check-pack needs one manifest (NAME.pack.json)");
    };
    let (m, here) = match load(manifest) {
        Ok(x) => x,
        Err(o) => return o,
    };
    let dir = p.get("--parts").map_or(here, PathBuf::from);
    let mut log = format!("parts looked for in {}\n", dir.display());
    let mut src = DirParts {
        dir,
        names: m.parts.iter().map(|x| x.name.clone()).collect(),
    };
    let total = m.parts.len();
    let mut seen =
        |i: usize, s: PartState| log.push_str(&format!("part {}/{total} {}\n", i + 1, s.key()));
    let parts = match pack_read::verify_parts(&m, &mut src, &mut seen) {
        Ok(v) => v,
        Err(e) => return with_log(Outcome::fail_error(&e), log),
    };
    log.push_str(&format!("{}\n", counts(&parts)));
    let mut o = Outcome::ok(done_json_or_text(
        json,
        pack_read::verify_json(&m, &parts),
        format!("{}\n", counts(&parts)),
    ));
    o.code = code_of(&parts, &[]);
    with_log(o, log)
}

pub fn unpack_run(args: Vec<String>) -> Outcome {
    let json = flags(&args);
    let p = match extract::parse(args, &["--out", "--parts"], &["--dry-run", "--force"]) {
        Ok(Some(p)) => p,
        Ok(None) => return Outcome::ok(format!("{UNPACK_HELP}\n").into_bytes()),
        Err(e) => return Outcome::fail(&e),
    };
    let [manifest] = p.pos.as_slice() else {
        return Outcome::fail("file unpack needs one manifest (NAME.pack.json)");
    };
    let Some(out) = p.get("--out").map(PathBuf::from) else {
        return Outcome::fail("file unpack needs --out DIR");
    };
    let (m, here) = match load(manifest) {
        Ok(x) => x,
        Err(o) => return o,
    };
    let dry = p.flag("--dry-run");
    let dir = p.get("--parts").map_or(here, PathBuf::from);
    let mut log = format!("parts looked for in {}\n", dir.display());
    if p.get("--parts").is_none() {
        log.push_str("--parts: the folder of the manifest by default\n");
    }
    if !dry && let Err(e) = fs::create_dir_all(&out) {
        return with_log(
            Outcome::fail_error(&out_err(format!("{}: {e}", out.display()))),
            log,
        );
    }
    let mut src = DirParts {
        dir,
        names: m.parts.iter().map(|x| x.name.clone()).collect(),
    };
    let mut sink = DirFiles {
        root: out,
        cur: None,
    };
    let total = m.parts.len();
    let mut seen =
        |i: usize, s: PartState| log.push_str(&format!("part {}/{total} {}\n", i + 1, s.key()));
    let r = pack_read::unpack(&m, &mut src, &mut sink, p.flag("--force"), !dry, &mut seen);
    let (parts, files) = match r {
        Ok(x) => x,
        Err(e) => return with_log(Outcome::fail_error(&e), log),
    };
    let lost = files
        .iter()
        .filter(|f| {
            matches!(
                f,
                pack_read::FileState::Lost | pack_read::FileState::Mismatch
            )
        })
        .count();
    log.push_str(&format!(
        "{}; {} of {} files {}, {lost} lost\n",
        counts(&parts),
        files.len() - lost,
        files.len(),
        if dry { "ready" } else { "written" }
    ));
    let text = format!(
        "{} of {} files {}\n",
        files.len() - lost,
        files.len(),
        if dry { "ready" } else { "written" }
    );
    let mut o = Outcome::ok(done_json_or_text(
        json,
        pack_read::unpack_json(&m, &parts, &files, dry),
        text,
    ));
    o.code = code_of(&parts, &files);
    with_log(o, log)
}
