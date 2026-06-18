<cfscript>
suiteBegin("File BIFs: relative paths resolve against the calling template (GitHub 171)");

// A relative path passed to FileRead/FileExists from inside a CFC must resolve
// against the directory of THAT CFC — the same base ExpandPath already uses —
// not the entry template / process cwd. Before the fix, FileRead("./x") from a
// component in a subdirectory threw "No such file or directory" while
// ExpandPath("./x") from the same component pointed at the sibling correctly.
// The test runner's cwd is the repo root, so the fixture lives in a subdir
// (tests/stdlib/relpath/) where cwd != the component's directory.

reader = new relpath.Reader();

assert(
	"FileRead('./x') resolves against the CFC's own directory",
	reader.readDotRelative(),
	'{"who":"sibling"}'
);

assert(
	"FileRead('x') (bare relative) resolves against the CFC's own directory",
	reader.readBareRelative(),
	'{"who":"sibling"}'
);

assertTrue(
	"FileExists('./x') uses the same base as FileRead",
	reader.existsRelative()
);

// The relative read must agree with the ExpandPath-wrapped workaround.
assert(
	"FileRead('./x') agrees with FileRead(ExpandPath('./x'))",
	reader.readDotRelative(),
	reader.readViaExpandPath()
);

suiteEnd();
</cfscript>
