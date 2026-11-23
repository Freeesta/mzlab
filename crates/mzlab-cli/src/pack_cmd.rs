//! `file pack`, `file unpack`, `file check-pack`: files moved between machines as parts a transfer limit lets through, with a `pack/1` manifest that
//! proves nothing changed (docs/agenti/file-pack.md). The format and the checks are `mzlab_core::pack` and `pack_read`, which the page runs too; this
//! file only walks folders, reads and writes files and turns the result into exit codes and the log on the standard error.

use std::fs::{self, File};
use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use mzlab_core::error::Error;
use mzlab_core::pack::{self, Manifest, Options, PartSink};
use mzlab_core::pack_read::{self, FileSink, PartSource, PartState};

use crate::extract;
use crate::{exit, Outcome};

pub const PACK_HELP: &str = "mzlab-cli file pack: files and folders into parts of a fixed size plus a pack/1 manifest (SHA-256 of every file and part).

usage: mzlab-cli file pack FILE_OR_DIR... --out DIR [--part-size SIZE] [--name NAME] [--dry-run] [--force] [--json]

  FILE_OR_DIR  files, and folders taken whole (.D and .raw folders included); stored under their own name, entries sorted by path
  --out DIR    where NAME.part0001... and NAME.pack.json go (created if missing); existing outputs are never replaced without --force
  --part-size  default 25MB; K, M, G (KB, MB, GB) are decimal, KiB, MiB, GiB binary; at least 1KiB
  --name NAME  default pack
  --dry-run    list the files and the planned parts (sizes only, no hash), write nothing
the same input gives the same bytes; one part is a ZIP with the files at their paths (7z, unzip). Every choice is logged on the standard error.
exit: 0 done; 2 usage; 4 an input is missing; 6 an output exists (no --force) or cannot be written; 8 a file changed size while read (still being written)";

pub const UNPACK_HELP: &str = "mzlab-cli file unpack: rebuild the files of a pack from its parts, checking every part and every file.

usage: mzlab-cli file unpack NAME.pack.json --out DIR [--parts DIR] [--dry-run] [--force] [--json]

  --out DIR    where the files go (created if missing); an existing file is never replaced without --force (then nothing is written)
  --parts DIR  where the parts are; default the folder of the manifest
the files that lie in intact parts are written and listed even when others are lost; unpack-run/1 lists every part and file with its status.
exit: 0 done; 1 parts missing (the other files are written); 2 usage; 3 corrupt part or bad manifest; 4 manifest not found; 6 a file exists (no --force) or cannot be written";

pub const VERIFY_HELP: &str = "mzlab-cli file check-pack: check the parts of a pack against its manifest without writing anything.

usage: mzlab-cli file check-pack NAME.pack.json [--parts DIR] [--json]

verify-pack/1 gives each part's status and `resend`, the parts to send again (missing or corrupt), so only those are re-sent.
exit: 0 every part intact; 1 a part is missing; 2 usage; 3 a part is corrupt (outranks missing); 4 manifest not found";

fn size_of(text: &str) -> Result<u64, String> {
    let t = text.trim();
    let at = t.find(|c: char| !c.is_ascii_digit()).unwrap_or(t.len());
    let (n, unit) = t.split_at(at);
    let n: u64 = n
        .parse()
        .map_err(|_| format!("--part-size {text}: a number and a unit, like 25MB"))?;
    let mul: u64 = match unit.to_ascii_lowercase().as_str() {
        "" | "b" => 1,
        "k" | "kb" => 1_000,
        "m" | "mb" => 1_000_000,
        "g" | "gb" => 1_000_000_000,
        "kib" => 1 << 10,
        "mib" => 1 << 20,
        "gib" => 1 << 30,
        _ => {
            return Err(format!(
                "--part-size {text}: unit {unit} is not K, M, G, KiB, MiB or GiB"
            ))
        }
    };
    let v = n.checked_mul(mul).ok_or("--part-size is too large")?;
    if v < 1024 {
        return Err("--part-size must be at least 1KiB".into());
    }
    Ok(v)
}

/// One file to pack: its name in the pack, its path and its size now.
struct Item {
    rel: String,
    path: PathBuf,
    size: u64,
}

fn name_of(p: &Path) -> Result<String, String> {
    let c = p
        .canonicalize()
        .map_err(|e| format!("{}: {e}", p.display()))?;
    c.file_name()
        .and_then(|n| n.to_str())
        .map(str::to_string)
        .ok_or_else(|| {
            format!(
                "{}: the name is not valid UTF-8 or the path has none",
                p.display()
            )
        })
}

fn walk(
    dir: &Path,
    rel: &str,
    skip: Option<&Path>,
    out: &mut Vec<Item>,
    log: &mut String,
) -> Result<(), String> {
    let mut names: Vec<_> = fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .map(|e| e.map(|e| e.path()))
        .collect::<Result<_, _>>()
        .map_err(|e| format!("{}: {e}", dir.display()))?;
    names.sort();
    for p in names {
        let n = p
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| format!("{}: the name is not valid UTF-8", p.display()))?;
        let rel = format!("{rel}/{n}");
        let link = fs::symlink_metadata(&p).map_err(|e| format!("{}: {e}", p.display()))?;
        let meta = fs::metadata(&p).map_err(|e| format!("{}: {e}", p.display()))?;
        if skip.is_some_and(|s| p.canonicalize().is_ok_and(|c| c.starts_with(s))) {
            log.push_str(&format!("skipped {rel}: it is inside the --out folder\n"));
        } else if link.file_type().is_symlink() && meta.is_dir() {
            log.push_str(&format!(
                "skipped {rel}: a link to a folder is not followed\n"
            ));
        } else if meta.is_dir() {
            walk(&p, &rel, skip, out, log)?;
        } else if meta.is_file() {
            out.push(Item {
                rel,
                path: p,
                size: meta.len(),
            });
        } else {
            log.push_str(&format!("skipped {rel}: not a regular file\n"));
        }
    }
    Ok(())
}

fn gather(inputs: &[String], skip: Option<&Path>, log: &mut String) -> Result<Vec<Item>, String> {
    let mut items = Vec::new();
    for a in inputs {
        let p = Path::new(a);
        let meta = fs::metadata(p).map_err(|e| format!("{a}: {e}"))?;
        let name = name_of(p)?;
        if meta.is_dir() {
            walk(p, &name, skip, &mut items, log)?;
        } else if skip.is_some_and(|s| p.canonicalize().is_ok_and(|c| c.starts_with(s))) {
            log.push_str(&format!("skipped {name}: it is inside the --out folder\n"));
        } else {
            items.push(Item {
                rel: name,
                path: p.to_path_buf(),
                size: meta.len(),
            });
        }
    }
    items.sort_by(|a, b| a.rel.cmp(&b.rel));
    if let Some(w) = items
        .windows(2)
        .find(|w| w.first().map(|x| &x.rel) == w.get(1).map(|x| &x.rel))
    {
        let (a, b) = (&w[0], &w[1]);
        return Err(format!(
            "two inputs would be stored as {}: {} and {}",
            a.rel,
            a.path.display(),
            b.path.display()
        ));
    }
    Ok(items)
}

fn out_err(e: impl ToString) -> Error {
    Error::new("err.run.output").with("detail", e)
}

/// Parts written into a folder, one file each; the files this run created are known so a failed run can remove them.
struct DirSink {
    dir: PathBuf,
    name: String,
    cur: Option<(usize, BufWriter<File>)>,
    made: Vec<PathBuf>,
}

impl DirSink {
    fn finish(&mut self) -> Result<(), Error> {
        if let Some((_, mut w)) = self.cur.take() {
            w.flush().map_err(out_err)?;
        }
        Ok(())
    }
}

impl PartSink for DirSink {
    fn put(&mut self, part: usize, bytes: &[u8]) -> Result<(), Error> {
        if self.cur.as_ref().is_none_or(|(i, _)| *i != part) {
            self.finish()?;
            let path = self.dir.join(pack::part_name(&self.name, part));
            let f = File::create(&path).map_err(|e| out_err(format!("{}: {e}", path.display())))?;
            self.made.push(path);
            self.cur = Some((part, BufWriter::with_capacity(1 << 20, f)));
        }
        match self.cur.as_mut() {
            Some((_, w)) => w.write_all(bytes).map_err(out_err),
            None => Ok(()),
        }
    }
}

fn done_json_or_text(json: bool, doc: String, text: String) -> Vec<u8> {
    if json { doc } else { text }.into_bytes()
}

fn flags(args: &[String]) -> bool {
    args.iter().any(|a| a == "--json")
}

fn old_outputs(dir: &Path, name: &str) -> Vec<PathBuf> {
    let (prefix, manifest) = (format!("{name}.part"), format!("{name}.pack.json"));
    let Ok(rd) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut v: Vec<PathBuf> = rd
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
                n == manifest
                    || n.strip_prefix(&prefix)
                        .is_some_and(|d| !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit()))
            })
        })
        .collect();
    v.sort();
    v
}

pub fn pack_run(args: Vec<String>) -> Outcome {
    let json = flags(&args);
    let p = match extract::parse(
        args,
        &["--out", "--part-size", "--name"],
        &["--dry-run", "--force"],
    ) {
        Ok(Some(p)) => p,
        Ok(None) => return Outcome::ok(format!("{PACK_HELP}\n").into_bytes()),
        Err(e) => return Outcome::fail(&e),
    };
    let dry = p.flag("--dry-run");
    let Some(out) = p.get("--out").map(PathBuf::from) else {
        return Outcome::fail("file pack needs --out DIR");
    };
    if p.pos.is_empty() {
        return Outcome::fail("file pack needs at least one file or folder");
    }
    let part_text = p.get("--part-size").unwrap_or("25MB");
    let part_size = match size_of(part_text) {
        Ok(n) => n,
        Err(e) => return Outcome::fail(&e),
    };
    let name = p.get("--name").unwrap_or("pack").to_string();
    if !pack::safe_path(&name) || name.contains('/') {
        return Outcome::fail("--name must be a plain file name");
    }
    let mut log = String::new();
    if p.get("--part-size").is_none() {
        log.push_str("part size: 25MB by default\n");
    }
    log.push_str(&format!("part size {part_text} = {part_size} bytes\n"));
    if p.get("--name").is_none() {
        log.push_str("name: pack by default\n");
    }
    let skip = out.canonicalize().ok();
    let items = match gather(&p.pos, skip.as_deref(), &mut log) {
        Ok(i) => i,
        Err(e) => return with_log(Outcome::fail(&e), log),
    };
    let list: Vec<(String, u64)> = items.iter().map(|i| (i.rel.clone(), i.size)).collect();
    let o = Options {
        name: name.clone(),
        part_size,
        zip64_at: mzlab_core::zipstream::ZIP64_AT,
    };
    log.push_str(&format!(
        "{} files in path order, {} bytes\n",
        items.len(),
        list.iter().map(|x| x.1).sum::<u64>()
    ));
    if dry {
        let m = pack::plan(&list, &o);
        log.push_str(&format!(
            "dry run: {} parts planned, nothing written\n",
            m.parts.len()
        ));
        return with_log(
            Outcome::ok(done_json_or_text(
                json,
                pack::run_json(&m, true),
                text_of(&m, true),
            )),
            log,
        );
    }
    let old = old_outputs(&out, &name);
    if !old.is_empty() && !p.flag("--force") {
        let e = Error::new("err.pack.exists")
            .with("path", old[0].display())
            .with("count", old.len());
        return with_log(Outcome::fail_error(&e), log);
    }
    if let Err(e) = fs::create_dir_all(&out) {
        return with_log(
            Outcome::fail_error(&out_err(format!("{}: {e}", out.display()))),
            log,
        );
    }
    if !old.is_empty() {
        log.push_str(&format!(
            "--force: {} existing output(s) of {name} are replaced\n",
            old.len()
        ));
    }
    let mut sink = DirSink {
        dir: out.clone(),
        name: name.clone(),
        cur: None,
        made: Vec::new(),
    };
    let mut open = |i: usize| -> Result<Box<dyn Read>, Error> {
        let path = &items[i].path;
        File::open(path)
            .map(|f| Box::new(f) as Box<dyn Read>)
            .map_err(|_| Error::new("err.cli.file").with("path", path.display()))
    };
    let result =
        pack::pack(&list, &mut open, &mut sink, &o).and_then(|m| sink.finish().map(|()| m));
    let m = match result {
        Ok(m) => m,
        Err(e) => {
            for f in &sink.made {
                let _ = fs::remove_file(f);
            }
            log.push_str(&format!(
                "removed the {} part(s) this run had written\n",
                sink.made.len()
            ));
            return with_log(Outcome::fail_error(&e), log);
        }
    };
    let manifest = out.join(format!("{name}.pack.json"));
    let tmp = out.join(format!(".{name}.pack.json.tmp"));
    if let Err(e) =
        fs::write(&tmp, pack::manifest_json(&m)).and_then(|()| fs::rename(&tmp, &manifest))
    {
        return with_log(
            Outcome::fail_error(&out_err(format!("{}: {e}", manifest.display()))),
            log,
        );
    }
    for stale in old
        .iter()
        .filter(|f| !sink.made.contains(f) && **f != manifest)
    {
        if fs::remove_file(stale).is_ok() {
            log.push_str(&format!(
                "removed {}: a part of an older pack of the same name\n",
                stale.display()
            ));
        }
    }
    log.push_str(&format!(
        "wrote {} parts and {}\n",
        m.parts.len(),
        manifest.display()
    ));
    with_log(
        Outcome::ok(done_json_or_text(
            json,
            pack::run_json(&m, false),
            text_of(&m, false),
        )),
        log,
    )
}

fn text_of(m: &Manifest, dry: bool) -> String {
    let verb = if dry { "would pack" } else { "packed" };
    format!(
        "{verb} {} files into {} parts ({} bytes), manifest {}.pack.json\n",
        m.files.len(),
        m.parts.len(),
        m.stream_bytes,
        m.name
    )
}

fn with_log(mut o: Outcome, log: String) -> Outcome {
    o.stderr = format!("{log}{}", o.stderr);
    o
}

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
