//! Regression coverage for GH #224: default-datasource resolution must be
//! consistent between a bare `queryExecute` (no datasource arg) and the
//! transaction path.
//!
//! Before the fix, `rewrite_query_datasource` only injected the per-application
//! default (`this.datasource`) when the call already carried an options struct
//! at arg[2]. A bare `queryExecute(sql)` therefore fell through to the
//! process-wide default (historically in-memory SQLite) OUTSIDE a transaction,
//! while the transaction path resolved `this.datasource` INSIDE one — so a bare
//! write and a same-named explicit-datasource read hit different databases.
//!
//! This runs the built `rustcfml` binary in CLI mode against a fixture app whose
//! `this.datasource` is a file-backed sqlite datasource, and asserts that bare
//! queries and the transaction path all resolve to it.

use std::path::PathBuf;
use std::process::Command;

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ds224_app/test.cfm")
}

#[test]
fn bare_query_resolves_to_application_default_datasource() {
    let output = Command::new(env!("CARGO_BIN_EXE_rustcfml"))
        .arg(fixture())
        .output()
        .expect("run rustcfml on ds224 fixture");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "rustcfml exited non-zero.\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        stdout.contains("BARE_WRITE_VISIBLE_TO_EXPLICIT:true"),
        "bare writes did not resolve to this.datasource (hit the :memory: default).\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        stdout.contains("BARE_READ_AFTER_TXN_SEES_BOTH:true"),
        "bare read after a transaction did not see the transaction's write.\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        stdout.contains("SEQUENTIAL_TXNS_OK:true"),
        "sequential top-level transactions failed (savepoint depth leak).\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
}
