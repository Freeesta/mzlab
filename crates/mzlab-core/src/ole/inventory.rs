//! What the container says about itself beyond the streams (`wiffwrite` feature, for `mzlab-cli file info --inventory`): sector sizes, the SHA-256 of the
//! file and the directory metadata of every entry (CLSID, state bits, creation and modification times), which the reader never looks at and a rewrite
//! of the file must carry. Read only.

use sha2::{Digest, Sha256};

use super::{Ole, END};

/// One directory entry (a storage or a stream) with the fields `Ole` does not keep.
pub struct DirMeta {
    pub path: String,
    pub stream: bool,
    pub size: u64,
    /// 16 bytes, lower-case hex.
    pub clsid: String,
    pub state: u32,
    /// FILETIME words as the file holds them (0 when unset), lower-case hex of the 8 bytes.
    pub created: String,
    pub modified: String,
}

/// The container: format version, sector sizes and bytes.
pub struct Info {
    pub version: u16,
    pub sector: usize,
    pub mini_sector: usize,
    pub mini_cutoff: u32,
    pub bytes: u64,
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

impl Ole {
    pub fn info(&self) -> Info {
        let h = self.src.read(0, 512);
        Info {
            version: u16::from_le_bytes([
                h.get(0x1A).copied().unwrap_or(0),
                h.get(0x1B).copied().unwrap_or(0),
            ]),
            sector: self.ss,
            mini_sector: self.mss,
            mini_cutoff: self.cutoff,
            bytes: self.src.len(),
        }
    }

    /// SHA-256 (lower-case hex) of the whole container file, read in blocks of 1 MiB.
    pub fn sha256_hex(&self) -> String {
        let (mut h, mut off, len) = (Sha256::new(), 0u64, self.src.len());
        while off < len {
            let b = self.src.read(off, 1 << 20);
            if b.is_empty() {
                break;
            }
            h.update(&b);
            off += b.len() as u64;
        }
        hex(&h.finalize())
    }

    /// Every entry of the directory tree below the root, in the order of `paths_list` (an in-order walk), storages included. Each entry once.
    pub fn dir_meta(&self) -> Vec<DirMeta> {
        let h = self.src.read(0, 512);
        let dir0 = u32::from_le_bytes(
            h.get(0x30..0x34)
                .and_then(|b| b.try_into().ok())
                .unwrap_or_default(),
        );
        let raw = self.chain_bytes(dir0, self.len().min(1 << 28));
        let mut seen = vec![false; self.dir.len()];
        let mut out = Vec::new();
        // (entry, path of its parent, emitted): the left subtree, the entry, its children, the right subtree
        let mut stack = vec![(
            self.dir.first().map_or(END, |e| e.child),
            Vec::<String>::new(),
            false,
        )];
        while let Some((idx, prefix, emit)) = stack.pop() {
            let (Some(e), Some(done)) = (self.dir.get(idx as usize), seen.get_mut(idx as usize))
            else {
                continue;
            };
            if emit {
                let mut p = prefix;
                if !e.name.is_empty() {
                    p.push(e.name.clone());
                }
                let r = raw
                    .get(idx as usize * 128..idx as usize * 128 + 128)
                    .unwrap_or(&[]);
                let part = |a: usize, n: usize| r.get(a..a + n).unwrap_or(&[]);
                out.push(DirMeta {
                    path: p.join("/"),
                    stream: e.typ == 2,
                    size: u64::from(e.size),
                    clsid: hex(part(0x50, 16)),
                    state: u32::from_le_bytes(part(0x60, 4).try_into().unwrap_or_default()),
                    created: hex(part(0x64, 8)),
                    modified: hex(part(0x6C, 8)),
                });
                if e.typ == 1 {
                    stack.push((e.child, p, false));
                }
                continue;
            }
            if std::mem::replace(done, true) {
                continue;
            }
            stack.push((e.right, prefix.clone(), false));
            stack.push((idx, prefix.clone(), true));
            stack.push((e.left, prefix, false));
        }
        out
    }
}
