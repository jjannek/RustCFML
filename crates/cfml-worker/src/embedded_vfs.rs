//! Read-only VFS over a static `&[(&str, &[u8])]` table.
//!
//! This is what `build.rs`-style embedding produces. The
//! `cfml-common::EmbeddedFs` exists but takes an owned `HashMap<String,
//! Vec<u8>>`; building one from a `&'static` slice on every request would
//! copy every byte. This wrapper avoids the copy by holding a reference to
//! the static table and computing paths on the fly.

use cfml_common::vfs::{Vfs, VfsDirEntry};
use std::collections::HashSet;
use std::io;
use std::time::SystemTime;

pub struct EmbeddedVfs {
    files: &'static [(&'static str, &'static [u8])],
    base_dir: String,
    /// Pre-computed lowercase directory set for is_dir / read_dir.
    dirs: HashSet<String>,
}

impl EmbeddedVfs {
    pub fn new(files: &'static [(&'static str, &'static [u8])], base_dir: String) -> Self {
        let mut dirs = HashSet::new();
        dirs.insert(String::new());
        for (path, _) in files {
            let normalized = path.replace('\\', "/").trim_start_matches('/').to_lowercase();
            let mut cur = String::new();
            for seg in normalized.split('/') {
                if !cur.is_empty() {
                    cur.push('/');
                }
                cur.push_str(seg);
                if cur != normalized {
                    dirs.insert(cur.clone());
                }
            }
        }
        Self { files, base_dir, dirs }
    }

    fn normalize(&self, path: &str) -> String {
        let path = path.replace('\\', "/");
        let stripped = if !self.base_dir.is_empty() {
            let base = self.base_dir.replace('\\', "/").to_lowercase();
            let lower = path.to_lowercase();
            if lower.starts_with(&base) {
                path[self.base_dir.len()..].trim_start_matches('/').to_lowercase()
            } else {
                path.trim_start_matches('/').to_lowercase()
            }
        } else {
            path.trim_start_matches('/').to_lowercase()
        };
        let mut parts: Vec<&str> = Vec::new();
        for seg in stripped.split('/') {
            match seg {
                "." | "" => {}
                ".." => {
                    parts.pop();
                }
                s => parts.push(s),
            }
        }
        parts.join("/")
    }

    fn lookup(&self, normalized: &str) -> Option<&'static [u8]> {
        for (p, bytes) in self.files {
            if p.replace('\\', "/").trim_start_matches('/').eq_ignore_ascii_case(normalized) {
                return Some(*bytes);
            }
        }
        None
    }
}

impl std::fmt::Debug for EmbeddedVfs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EmbeddedVfs")
            .field("file_count", &self.files.len())
            .field("base_dir", &self.base_dir)
            .finish()
    }
}

impl Vfs for EmbeddedVfs {
    fn read_to_string(&self, path: &str) -> io::Result<String> {
        let n = self.normalize(path);
        self.lookup(&n)
            .map(|b| String::from_utf8_lossy(b).to_string())
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, format!("not found: {path}")))
    }

    fn read(&self, path: &str) -> io::Result<Vec<u8>> {
        let n = self.normalize(path);
        self.lookup(&n)
            .map(|b| b.to_vec())
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, format!("not found: {path}")))
    }

    fn exists(&self, path: &str) -> bool {
        let n = self.normalize(path);
        self.lookup(&n).is_some() || self.dirs.contains(&n)
    }

    fn is_file(&self, path: &str) -> bool {
        self.lookup(&self.normalize(path)).is_some()
    }

    fn is_dir(&self, path: &str) -> bool {
        self.dirs.contains(&self.normalize(path))
    }

    fn read_dir(&self, path: &str) -> io::Result<Vec<VfsDirEntry>> {
        let n = self.normalize(path);
        if !self.dirs.contains(&n) {
            return Err(io::Error::new(io::ErrorKind::NotFound, format!("dir not found: {path}")));
        }
        let prefix = if n.is_empty() { String::new() } else { format!("{n}/") };
        let mut seen = HashSet::new();
        let mut entries = Vec::new();
        for (p, _) in self.files {
            let normalized = p.replace('\\', "/").trim_start_matches('/').to_lowercase();
            if normalized.starts_with(&prefix) {
                let rest = &normalized[prefix.len()..];
                if let Some(pos) = rest.find('/') {
                    let name = rest[..pos].to_string();
                    if seen.insert(name.clone()) {
                        entries.push(VfsDirEntry { name, is_file: false, is_dir: true });
                    }
                } else {
                    entries.push(VfsDirEntry {
                        name: rest.to_string(),
                        is_file: true,
                        is_dir: false,
                    });
                }
            }
        }
        Ok(entries)
    }

    fn modified(&self, path: &str) -> io::Result<SystemTime> {
        let n = self.normalize(path);
        if self.lookup(&n).is_some() {
            // Fixed mtime — embedded files don't change between requests.
            Ok(SystemTime::UNIX_EPOCH)
        } else {
            Err(io::Error::new(io::ErrorKind::NotFound, "not found"))
        }
    }

    fn canonicalize(&self, path: &str) -> io::Result<String> {
        let n = self.normalize(path);
        if self.lookup(&n).is_some() || self.dirs.contains(&n) {
            Ok(if self.base_dir.is_empty() {
                format!("/{n}")
            } else {
                format!("{}/{n}", self.base_dir)
            })
        } else {
            Err(io::Error::new(io::ErrorKind::NotFound, format!("not found: {path}")))
        }
    }
}
