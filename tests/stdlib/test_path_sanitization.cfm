<cfscript>
suiteBegin("Path sanitization: getFileFromPath + java.io.File.getCanonicalPath");

// getFileFromPath treats BOTH / and \ as separators (Lucee/ACF, all OSes), so a
// Windows-style traversal string is reduced to its bare filename.
assert("forward-slash path -> filename", getFileFromPath("../../etc/passwd"), "passwd");
assert("backslash path -> filename", getFileFromPath("..\..\windows\system32\config\sam"), "sam");
assert("mixed separators -> filename", getFileFromPath("a/b\c/d"), "d");
assert("bare filename unchanged", getFileFromPath("screenshot.png"), "screenshot.png");
assertTrue("sanitized backslash path has no ..",
    find("..", getFileFromPath("..\..\windows\system32\config\sam")) == 0);

// java.io.File.getCanonicalPath() resolves . and .. and strips a trailing
// separator, even for paths that do not exist on disk. Wheels' asset
// path-traversal guard relies on this to detect an escape from a base dir.
// (Assertions are structural, not exact strings, so they hold on both engines
// — real Java additionally resolves symlinks, e.g. /var -> /private/var.)
assetsDir = "/opt/rcfmltest/assets/";
canonAssets = createObject("java", "java.io.File").init(assetsDir).getCanonicalPath();
assertTrue("trailing slash stripped from canonical dir", right(canonAssets, 1) != "/");
assertTrue("canonical dir keeps the assets segment", findNoCase("assets", canonAssets) > 0);

traversal = createObject("java", "java.io.File").init(assetsDir & "../../secret.cfc").getCanonicalPath();
assertTrue("traversal canonical has no dot-dot", find("..", traversal) == 0);
assertTrue("traversal escapes the assets base dir",
    compareNoCase(left(traversal, len(canonAssets)), canonAssets) != 0);

legit = createObject("java", "java.io.File").init(assetsDir & "logo.png").getCanonicalPath();
assertTrue("legitimate file stays within the assets base dir",
    compareNoCase(left(legit, len(canonAssets)), canonAssets) == 0);

suiteEnd();
</cfscript>
