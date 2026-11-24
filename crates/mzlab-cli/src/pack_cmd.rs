//! `file pack`, `file unpack`, `file check-pack`: files moved between machines as parts a transfer limit lets through, with a `pack/1` manifest that
//! proves nothing changed (docs/agenti/file-pack.md). The format and the checks are `mzlab_core::pack` and `pack_read`, which the page runs too; this
//! file only walks folders, reads and writes files and turns the result into exit codes and the log on the standard error.

use std::fs::{self, File};
use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use mzlab_core::error::Error;
use mzlab_core::pack::{self, Manifest, Options, PartSink};

use crate::extract;
use crate::Outcome;

pub const PACK_HELP: &str = "mzlab-cli file pack: files and folders into parts of a fixed size plus a pack/1 manifest (SHA-256 of every file and part).

usage: mzlab-cli file pack FILE_OR_DIR... --out DIR [--part-size SIZE] [--name NAME] [--dry-run] [--force] [--json]

  FILE_OR_DIR  files, and folders taken whole (.D and .raw folders included); stored under their own name, entries sorted by path
  --out DIR    where NAME.part0001... and NAME.pack.json go (created if missing); existing outputs are never replaced without --force
  --part-size  default 25MB; K, M, G (KB, MB, GB) are decimal, KiB, MiB, GiB binary; at least 1KiB
  --name NAME  default pack
  --dry-run    list the files and the planned parts (sizes only, no hash), write nothing
the same input gives the same bytes; one part is a ZIP with the files at their paths (7z, unzip). Every choice is logged on the standard error.
exit: 0 done; 2 usage; 3 an unusable path (not plain, or two that clash: equal, equal in case, a file named like a folder); 4 an input is missing; 6 an output exists (no --force) or cannot be written; 8 a file changed size while read (still being written)";

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
pub(crate) struct Item {
    pub(crate) rel: String,
    pub(crate) path: PathBuf,
    pub(crate) size: u64,
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
    // two inputs stored under one name are refused with the other clashes (`pack::check_files`), not here
    items.sort_by(|a, b| a.rel.cmp(&b.rel));
    Ok(items)
}

pub(crate) fn out_err(e: impl ToString) -> Error {
    Error::new("err.run.output").with("detail", e)
}

/// Parts written into a folder, one file each, under temporary names: a failed run removes them and leaves any older pack as it was, a good one renames them.
struct DirSink {
    dir: PathBuf,
    name: String,
    cur: Option<(usize, BufWriter<File>)>,
    /// (temporary, final) path of every part started.
    made: Vec<(PathBuf, PathBuf)>,
}

impl DirSink {
    fn finish(&mut self) -> Result<(), Error> {
        if let Some((_, mut w)) = self.cur.take() {
            w.flush().map_err(out_err)?;
        }
        Ok(())
    }
}

impl DirSink {
    /// Drops the temporary parts, leaving every older output as it was.
    fn discard(&mut self) {
        drop(self.cur.take());
        for (tmp, _) in &self.made {
            let _ = fs::remove_file(tmp);
        }
    }

    /// Moves the parts into place, then the manifest last, so a manifest never points at parts that are not there.
    fn commit(&self, manifest: &Path, text: &str) -> Result<(), Error> {
        // the manifest is written first under its temporary name, so a full disk is found before any older part is replaced
        let tmp = self.dir.join(format!(".{}.pack.json.tmp", self.name));
        let fail = |p: &Path, e: std::io::Error| {
            let _ = fs::remove_file(&tmp);
            out_err(format!("{}: {e}", p.display()))
        };
        fs::write(&tmp, text).map_err(|e| fail(manifest, e))?;
        for (part, to) in &self.made {
            fs::rename(part, to).map_err(|e| fail(to, e))?;
        }
        fs::rename(&tmp, manifest).map_err(|e| fail(manifest, e))
    }
}

impl PartSink for DirSink {
    fn put(&mut self, part: usize, bytes: &[u8]) -> Result<(), Error> {
        if self.cur.as_ref().is_none_or(|(i, _)| *i != part) {
            self.finish()?;
            let to = self.dir.join(pack::part_name(&self.name, part));
            let path = self
                .dir
                .join(format!(".{}.mzpack-tmp", pack::part_name(&self.name, part)));
            let f = File::create(&path).map_err(|e| out_err(format!("{}: {e}", path.display())))?;
            self.made.push((path, to));
            self.cur = Some((part, BufWriter::with_capacity(1 << 20, f)));
        }
        match self.cur.as_mut() {
            Some((_, w)) => w.write_all(bytes).map_err(out_err),
            None => Ok(()),
        }
    }
}

pub(crate) fn done_json_or_text(json: bool, doc: String, text: String) -> Vec<u8> {
    if json { doc } else { text }.into_bytes()
}

pub(crate) fn flags(args: &[String]) -> bool {
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

/// What the command line asked for, and the log of the choices made while reading it.
pub(crate) struct Job {
    json: bool,
    dry: bool,
    force: bool,
    out: PathBuf,
    name: String,
    part_size: u64,
    inputs: Vec<String>,
    log: String,
}

/// The arguments of `file pack`; `Err` is the answer to give at once (help or a usage error).
pub(crate) fn job_of(args: Vec<String>) -> Result<Job, Outcome> {
    let json = flags(&args);
    let p = match extract::parse(
        args,
        &["--out", "--part-size", "--name"],
        &["--dry-run", "--force"],
    ) {
        Ok(Some(p)) => p,
        Ok(None) => return Err(Outcome::ok(format!("{PACK_HELP}\n").into_bytes())),
        Err(e) => return Err(Outcome::fail(&e)),
    };
    let Some(out) = p.get("--out").map(PathBuf::from) else {
        return Err(Outcome::fail("file pack needs --out DIR"));
    };
    if p.pos.is_empty() {
        return Err(Outcome::fail("file pack needs at least one file or folder"));
    }
    let part_text = p.get("--part-size").unwrap_or("25MB");
    let part_size = size_of(part_text).map_err(|e| Outcome::fail(&e))?;
    let name = p.get("--name").unwrap_or("pack").to_string();
    if !pack::safe_path(&name) || name.contains('/') {
        return Err(Outcome::fail("--name must be a plain file name"));
    }
    let mut log = String::new();
    if p.get("--part-size").is_none() {
        log.push_str("part size: 25MB by default\n");
    }
    log.push_str(&format!("part size {part_text} = {part_size} bytes\n"));
    if p.get("--name").is_none() {
        log.push_str("name: pack by default\n");
    }
    Ok(Job {
        json,
        dry: p.flag("--dry-run"),
        force: p.flag("--force"),
        out,
        name,
        part_size,
        inputs: p.pos.clone(),
        log,
    })
}

pub fn pack_run(args: Vec<String>) -> Outcome {
    let mut job = match job_of(args) {
        Ok(j) => j,
        Err(o) => return o,
    };
    let skip = job.out.canonicalize().ok();
    let items = match gather(&job.inputs, skip.as_deref(), &mut job.log) {
        Ok(i) => i,
        Err(e) => return with_log(Outcome::fail(&e), job.log),
    };
    let list: Vec<(String, u64)> = items.iter().map(|i| (i.rel.clone(), i.size)).collect();
    let o = Options {
        name: job.name.clone(),
        part_size: job.part_size,
        zip64_at: mzlab_core::zipstream::ZIP64_AT,
    };
    // the dry run and the real run are refused for the same names, with the same error
    if let Err(e) = pack::check_files(&list, &o) {
        return with_log(Outcome::fail_error(&e), job.log);
    }
    job.log.push_str(&format!(
        "{} files in path order, {} bytes\n",
        items.len(),
        list.iter().map(|x| x.1).sum::<u64>()
    ));
    if job.dry {
        let m = pack::plan(&list, &o);
        job.log.push_str(&format!(
            "dry run: {} parts planned, nothing written\n",
            m.parts.len()
        ));
        let doc = done_json_or_text(job.json, pack::run_json(&m, true), text_of(&m, true));
        return with_log(Outcome::ok(doc), job.log);
    }
    write_pack(job, &items, &list, &o)
}

/// Writes the parts and the manifest; a failed run removes what it had written and leaves an older pack of the same name untouched.
pub(crate) fn write_pack(
    mut job: Job,
    items: &[Item],
    list: &[(String, u64)],
    o: &Options,
) -> Outcome {
    let (out, name) = (job.out.clone(), job.name.clone());
    let old = old_outputs(&out, &name);
    if !old.is_empty() && !job.force {
        let e = Error::new("err.pack.exists")
            .with("path", old[0].display())
            .with("count", old.len());
        return with_log(Outcome::fail_error(&e), job.log);
    }
    if let Err(e) = fs::create_dir_all(&out) {
        let e = out_err(format!("{}: {e}", out.display()));
        return with_log(Outcome::fail_error(&e), job.log);
    }
    if !old.is_empty() {
        job.log.push_str(&format!(
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
    let result = pack::pack(list, &mut open, &mut sink, o).and_then(|m| sink.finish().map(|()| m));
    let m = match result {
        Ok(m) => m,
        Err(e) => {
            sink.discard();
            job.log.push_str(&format!(
                "removed the {} temporary part(s) this run had written; existing outputs are untouched\n",
                sink.made.len()
            ));
            return with_log(Outcome::fail_error(&e), job.log);
        }
    };
    let manifest = out.join(format!("{name}.pack.json"));
    if let Err(e) = sink.commit(&manifest, &pack::manifest_json(&m)) {
        sink.discard();
        return with_log(Outcome::fail_error(&e), job.log);
    }
    for stale in old
        .iter()
        .filter(|f| !sink.made.iter().any(|m| &m.1 == *f) && **f != manifest)
    {
        if fs::remove_file(stale).is_ok() {
            job.log.push_str(&format!(
                "removed {}: a part of an older pack of the same name\n",
                stale.display()
            ));
        }
    }
    job.log.push_str(&format!(
        "wrote {} parts and {}\n",
        m.parts.len(),
        manifest.display()
    ));
    let doc = done_json_or_text(job.json, pack::run_json(&m, false), text_of(&m, false));
    with_log(Outcome::ok(doc), job.log)
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

pub(crate) fn with_log(mut o: Outcome, log: String) -> Outcome {
    o.stderr = format!("{log}{}", o.stderr);
    o
}
