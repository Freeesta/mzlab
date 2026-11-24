//! The mzML of a file or folder through the registry (`readers::open` and `Conversione`): what the tests that used the converters of each format call.
#![allow(dead_code)]

use std::path::Path;

use mzlab_core::readers::{self, FileSet, Limits, Member};
use mzlab_core::readers_mzml::Conversion;
use mzlab_core::spectrum_type::{Basis, SpectrumType, TypeChoice};

/// The conversion that writes each scan with the flag of its reader (what the writer did before the spectrum-type decision): the tests that compare the
/// mzML with the rows of the reader use it, since their synthetic points are sparse and `auto` would write them as centroids.
pub fn keep_flags() -> Conversion<'static> {
    Conversion {
        decided: Some(TypeChoice::keep(SpectrumType::Auto, Basis::Reader)),
        ..Conversion::default()
    }
}

pub fn member(name: &str, data: &[u8]) -> Member {
    Member {
        name: name.into(),
        dim: data.len() as u64,
        data: Box::new(data.to_vec()),
        sha1: None,
    }
}

/// The mzML of the first sample of `fs`.
pub fn mzml(fs: FileSet) -> Vec<u8> {
    let mut l = readers::open(fs, &Limits::sito()).unwrap();
    keep_flags()
        .write(&mut *l, 0, Vec::new(), &mut |_, _| {})
        .unwrap()
}

/// The mzML of one file on disk, named `name` for the reader.
pub fn mzml_file(path: &Path, name: &str) -> Vec<u8> {
    let dim = std::fs::metadata(path).unwrap().len();
    mzml(FileSet {
        members: vec![Member {
            name: name.into(),
            dim,
            data: Box::new(std::fs::File::open(path).unwrap()),
            sha1: None,
        }],
    })
}

/// The mzML of a vendor folder (`x.D`): every file below it is a member named `x.D/<relative path>`.
pub fn mzml_folder(folder: &Path) -> Vec<u8> {
    let base = folder.file_name().unwrap().to_string_lossy().into_owned();
    let mut members = Vec::new();
    let mut stack = vec![folder.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
                continue;
            }
            let rel = p
                .strip_prefix(folder)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            members.push(member(
                &format!("{base}/{rel}"),
                &std::fs::read(&p).unwrap(),
            ));
        }
    }
    mzml(FileSet { members })
}

/// The mzML of sample `c` of a `.wiff` pair (`name.wiff`, `name.wiff.scan`) in `dir`; with `centroid` the reader reduces the profile to centroids.
pub fn mzml_wiff(dir: &Path, name: &str, c: usize, centroid: bool) -> Vec<u8> {
    let members = [name.to_string() + ".wiff", name.to_string() + ".wiff.scan"]
        .iter()
        .map(|n| {
            let p = dir.join(n);
            Member {
                name: n.clone(),
                dim: std::fs::metadata(&p).unwrap().len(),
                data: Box::new(std::fs::File::open(&p).unwrap()),
                sha1: None,
            }
        })
        .collect();
    let l = readers::open(FileSet { members }, &Limits::desktop()).unwrap();
    let mut l = if centroid {
        mzlab_core::readers_centroids::centroidato(l)
    } else {
        l
    };
    keep_flags()
        .write(&mut *l, c, Vec::new(), &mut |_, _| {})
        .unwrap()
}
