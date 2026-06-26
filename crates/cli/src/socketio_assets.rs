//! Engine-bundled socket.io-lucee compat CFCs, overlaid onto the serve-mode
//! VFS so `new SocketIoServer()` resolves out of the box.
//!
//! [`SocketIoOverlay`] wraps the real filesystem and serves the three compat
//! CFCs (`SocketIoServer` / `SocketIoNamespace` / `SocketIoSocket`) for any
//! path whose basename matches — *only when the base VFS does not already have
//! that file*. So a user's own same-named file always wins; the engine copy is
//! a fallback for the reserved names. All other paths pass straight through.

use std::io;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use cfml_common::vfs::{Vfs, VfsDirEntry};

const SERVER_CFC: &str = include_str!("../assets/socketio/SocketIoServer.cfc");
const NAMESPACE_CFC: &str = include_str!("../assets/socketio/SocketIoNamespace.cfc");
const SOCKET_CFC: &str = include_str!("../assets/socketio/SocketIoSocket.cfc");

/// The embedded source for a reserved compat-CFC path, keyed by basename
/// (case-insensitive). `None` for any other path.
fn engine_cfc(path: &str) -> Option<&'static str> {
    let base = path.replace('\\', "/");
    let base = base.rsplit('/').next().unwrap_or(&base);
    match base.to_ascii_lowercase().as_str() {
        "socketioserver.cfc" => Some(SERVER_CFC),
        "socketionamespace.cfc" => Some(NAMESPACE_CFC),
        "socketiosocket.cfc" => Some(SOCKET_CFC),
        _ => None,
    }
}

pub struct SocketIoOverlay {
    base: Arc<dyn Vfs>,
}

impl std::fmt::Debug for SocketIoOverlay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SocketIoOverlay").finish()
    }
}

impl SocketIoOverlay {
    pub fn new(base: Arc<dyn Vfs>) -> Self {
        Self { base }
    }

    /// Whether this path should be served from the overlay (reserved name AND
    /// the base FS doesn't have a real file there — real files always win).
    fn overlaid(&self, path: &str) -> Option<&'static str> {
        if self.base.exists(path) {
            return None;
        }
        engine_cfc(path)
    }
}

impl Vfs for SocketIoOverlay {
    fn read_to_string(&self, path: &str) -> io::Result<String> {
        match self.overlaid(path) {
            Some(src) => Ok(src.to_string()),
            None => self.base.read_to_string(path),
        }
    }

    fn read(&self, path: &str) -> io::Result<Vec<u8>> {
        match self.overlaid(path) {
            Some(src) => Ok(src.as_bytes().to_vec()),
            None => self.base.read(path),
        }
    }

    fn exists(&self, path: &str) -> bool {
        self.base.exists(path) || engine_cfc(path).is_some()
    }

    fn is_file(&self, path: &str) -> bool {
        self.base.is_file(path) || (!self.base.exists(path) && engine_cfc(path).is_some())
    }

    fn is_dir(&self, path: &str) -> bool {
        self.base.is_dir(path)
    }

    fn read_dir(&self, path: &str) -> io::Result<Vec<VfsDirEntry>> {
        self.base.read_dir(path)
    }

    fn modified(&self, path: &str) -> io::Result<SystemTime> {
        match self.overlaid(path) {
            // Stable mtime so the bytecode cache treats the engine CFCs as fixed.
            Some(_) => Ok(UNIX_EPOCH),
            None => self.base.modified(path),
        }
    }

    fn canonicalize(&self, path: &str) -> io::Result<String> {
        match self.base.canonicalize(path) {
            Ok(p) => Ok(p),
            // A reserved-name path that isn't on disk canonicalizes to itself so
            // the VM can use it as a stable source_file key.
            Err(e) => {
                if engine_cfc(path).is_some() {
                    Ok(path.to_string())
                } else {
                    Err(e)
                }
            }
        }
    }
}
