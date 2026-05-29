<cfscript>
suiteBegin("Functions: expandPath trailing-slash preservation");

// ============================================================
// Background
// ============================================================
// expandPath() resolves a relative path against the current base and
// returns an absolute path. A long-standing cross-engine convention --
// honored by Lucee 5/6/7, Adobe ColdFusion 2018-2025, and BoxLang -- is
// that expandPath MIRRORS the trailing slash of its input: if the input
// ends in a slash, so does the result; if it doesn't, the result doesn't
// gain one.
//
//     expandPath("foo/")  ->  "<base>/foo/"     (slash preserved)
//     expandPath("foo")   ->  "<base>/foo"      (no slash added)
//
// CFWheels/Wheels depends on this in public/Application.cfc:
//
//     this.appDir           = expandPath("../app/");      // keeps the slash
//     this.wheels.pluginDir = this.appDir & "../plugins";
//
// The kept slash makes `appDir & "../plugins"` resolve to a traversable
// "<root>/app/../plugins". On RustCFML v0.20.2 expandPath DROPS the
// trailing slash, but ONLY for a path that already EXISTS on disk: for an
// existing directory it canonicalizes the path (resolving "..", resolving
// symlinks) and the trailing slash is lost; for a non-existent path it
// falls back to a lexical join that keeps the slash. Wheels' "../app/"
// resolves to a real directory, so it hits the canonicalizing path and
// the slash is dropped. The concatenation then fuses into the malformed
// "<root>/app../plugins" (note "app.." -- no slash), and the subsequent
// DirectoryList() throws "No such file or directory", aborting the
// Application.cfc pseudo-constructor.
//
// To reproduce the EXISTING-path code path portably, this test creates a
// real probe directory under the current base, then asserts the trailing-
// slash PROPERTY and the malformed-fusion SUBSTRING (never an absolute
// path) so the assertions hold regardless of where the test tree lives.
// ============================================================

// Compute the probe directory's absolute path via expandPath itself (so
// the create step and the assert step share one base), then materialize
// it on disk so expandPath takes its existing-path branch.
probeRel = "rustcfml_probe_expand";
probeAbs = expandPath(probeRel);
if (!directoryExists(probeAbs)) {
    directoryCreate(probeAbs);
}

// ------------------------------------------------------------
// Slash on the input is preserved on the output (the directory now exists)
// ------------------------------------------------------------
withSlash = expandPath(probeRel & "/");
assert("expandPath preserves a trailing slash present in the input",
    right(withSlash, 1), "/");

// ------------------------------------------------------------
// No slash on the input -> no slash added (symmetry / sanity:
// passes on every engine, pins down that the gap is "preserve", not "add")
// ------------------------------------------------------------
noSlash = expandPath(probeRel);
assertFalse("expandPath does not append a slash when the input lacks one",
    right(noSlash, 1) EQ "/");

// ------------------------------------------------------------
// Concatenating "../x" onto a dir-with-slash must not fuse into "dir.."
// ------------------------------------------------------------
fused = expandPath(probeRel & "/") & "../sibling";
assertFalse("dir-with-slash + '../' must not collapse into 'dir..'",
    find(probeRel & "..", fused) GT 0);

// ------------------------------------------------------------
// The exact two-step shape Wheels' Application.cfc uses: the result must
// stay traversable (contains a "/../" segment, not a fused "dir..").
// ------------------------------------------------------------
appDir    = expandPath(probeRel & "/");
pluginDir = appDir & "../plugins";
assertTrue("Wheels shape: expandPath('dir/') & '../plugins' stays traversable ('/../' present)",
    find("/../", pluginDir) GT 0);

// ------------------------------------------------------------
// Clean up the probe directory.
// ------------------------------------------------------------
if (directoryExists(probeAbs)) {
    directoryDelete(probeAbs, true);
}

suiteEnd();
</cfscript>
