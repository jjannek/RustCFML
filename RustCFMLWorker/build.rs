//! Walks `cfml/` and emits a Rust source file with a single
//! `pub static CFML_FILES: &[(&str, &[u8])]` table the lib uses to
//! seed `cfml_worker::embedded_vfs::EmbeddedVfs`.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=cfml");

    let root: PathBuf = env::current_dir().expect("cwd").join("cfml");
    let mut entries: Vec<(String, PathBuf)> = Vec::new();
    if root.exists() {
        walk(&root, &root, &mut entries);
    }
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR");
    let dest = Path::new(&out_dir).join("embedded_files.rs");

    let mut src = String::new();
    src.push_str("pub static CFML_FILES: &[(&str, &[u8])] = &[\n");
    for (rel, abs) in &entries {
        // Embedded paths are virtual-root–relative; cfml-worker prefixes
        // them with `WorkerConfig.virtual_root` at lookup time.
        let abs_str = abs.to_string_lossy().replace('\\', "/");
        src.push_str(&format!(
            "    ({:?}, include_bytes!({:?})),\n",
            rel, abs_str
        ));
    }
    src.push_str("];\n");

    fs::write(dest, src).expect("write embedded_files.rs");
}

fn walk(base: &Path, dir: &Path, out: &mut Vec<(String, PathBuf)>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for entry in rd.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(base, &path, out);
        } else if path.is_file() {
            let rel = path
                .strip_prefix(base)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            out.push((rel, path));
        }
    }
}
