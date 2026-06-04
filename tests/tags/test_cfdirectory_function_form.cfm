<cfscript>
suiteBegin("Tags: cfdirectory function-call form");

function listViaCfdirectory() {
	var tmp = getTempDirectory() & "rustcfml_cfdir_" & createUUID() & "/";
	directoryCreate(tmp);
	fileWrite(tmp & "a.txt", "1");
	fileWrite(tmp & "b.txt", "2");
	var result = "";
	try {
		cfdirectory(action = "list", directory = tmp, filter = "*.txt", name = "dirQ");
		result = isQuery(dirQ) ? "rows=" & dirQ.recordCount : "not-a-query";
	} catch (any e) {
		result = "ERROR: " & e.message;
	}
	directoryDelete(tmp, true);
	return result;
}

assert("cfdirectory(action='list', directory=, filter=, name=) lists the directory as a query",
	listViaCfdirectory(), "rows=2");

suiteEnd();
</cfscript>
