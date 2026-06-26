<cfscript>
suiteBegin("Functions: expandPath leading-slash is caller-independent (GH ##215)");

// ============================================================
// Background
// ============================================================
// A leading-slash path passed to expandPath() is "webroot-relative" in
// CFML (Lucee 5/6/7, Adobe ColdFusion, BoxLang): the leading "/" anchors
// to the web root (serve mode) or the entry template's directory (CLI),
// and the result is CALLER-INDEPENDENT — it does NOT vary with the
// directory of the template/CFC that happens to make the call, and it is
// always returned as an ABSOLUTE path even when the target file does not
// exist.
//
// RustCFML (pre-fix, GH #215) resolved the leading "/" against the
// CALLING CFC's own directory. From a CFC in a subdirectory this DOUBLED
// the path segment ("/sub/.../sub/...") and the result came back relative
// ("./...") rather than absolute. See bug_expandpath_leading_slash_issue215.
//
// Assertions below test PROPERTIES (absolute, single-segment, caller-
// independent) rather than an exact absolute path, so they hold in both
// CLI mode (entry-template-relative) and serve mode (webroot-relative),
// and pass identically on Lucee.
// ============================================================

// A unique marker path that does NOT exist on disk, so resolution stays
// purely lexical (no canonicalize) and exercises the fallback branch.
marker  = "rustcfml_ep215_marker_dir";
relPath = "/" & marker & "/inner.txt";

pageLevel = expandPath( relPath );

probe     = new ep_probe_sub.EpProbe();
fromCfc   = probe.probe( relPath );

// ------------------------------------------------------------
// Caller-independence: the same leading-slash path resolves identically
// whether called at page level or from a CFC in a subdirectory.
// ------------------------------------------------------------
assert("expandPath('/x') is caller-independent (page level == from nested CFC)",
    fromCfc, pageLevel);

// ------------------------------------------------------------
// No doubled path segment: the unique marker appears exactly once.
// ------------------------------------------------------------
assert("expandPath('/x') from a nested CFC does not double the path segment",
    listLen(replace(fromCfc, marker, "|", "all"), "|") - 1, 1);

// ------------------------------------------------------------
// Absolute, not relative: the result is never returned as a "./..." path.
// ------------------------------------------------------------
assertFalse("expandPath('/x') returns an absolute path, not a './' relative one",
    left(pageLevel, 2) EQ "./");
assertFalse("expandPath('/x') from a nested CFC returns an absolute path, not './'",
    left(fromCfc, 2) EQ "./");

// ------------------------------------------------------------
// The marker is present (sanity: we actually resolved the input path).
// ------------------------------------------------------------
assertTrue("resolved path contains the marker segment",
    find(marker, pageLevel) GT 0);

suiteEnd();
</cfscript>
