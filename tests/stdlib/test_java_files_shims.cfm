<cfscript>
suiteBegin("java.nio.file.Files + java.lang.ProcessBuilder shims");

// These shims back Wheels' plugin loader (vendor/wheels/Plugins.cfc), which
// uses java.lang.ProcessBuilder (List ctor) to `ln -s` a symlink, then
// java.nio.file.Files.isSymbolicLink/delete (via java.io.File.toPath()) to
// detect and remove symlinked plugin directories without following the link.
// Skip on Windows / engines without `ln`.

isPosix = !(structKeyExists(server, "os") && findNoCase("windows", server.os.name));

if (isPosix) {
	tmp = getTempDirectory() & "/jfileshim_" & getTickCount();
	directoryCreate(tmp);
	target = tmp & "/realdir";
	directoryCreate(target);
	link = tmp & "/thelink";

	// ProcessBuilder.init(List).start().waitFor()/exitValue()
	pb = CreateObject("java", "java.lang.ProcessBuilder").init(["ln", "-s", target, link]);
	proc = pb.start();
	proc.waitFor();
	assert("ProcessBuilder ln -s exits 0", proc.exitValue(), 0);

	jFiles = CreateObject("java", "java.nio.file.Files");
	linkPath = CreateObject("java", "java.io.File").init(link).toPath();
	targetPath = CreateObject("java", "java.io.File").init(target).toPath();

	assertTrue("Files.isSymbolicLink true for the symlink", jFiles.isSymbolicLink(linkPath));
	assertFalse("Files.isSymbolicLink false for a real dir", jFiles.isSymbolicLink(targetPath));

	// File.exists()/isDirectory() resolve through the link.
	linkFile = CreateObject("java", "java.io.File").init(link);
	assertTrue("symlink resolves to existing dir", linkFile.exists() && linkFile.isDirectory());

	// Files.delete removes the symlink itself, never the target.
	jFiles.delete(linkPath);
	assertFalse("Files.delete removed the symlink", directoryExists(link));
	assertTrue("Files.delete preserved the target dir", directoryExists(target));

	directoryDelete(tmp, true);
} else {
	assertTrue("symlink shims skipped on non-POSIX", true);
}

suiteEnd();
</cfscript>
