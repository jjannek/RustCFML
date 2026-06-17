<cfscript>
suiteBegin("cfloop file / cfinclude assets + arguments");

// Resolve fixtures relative to THIS test file (works under CLI and HTTP).
thisDir = getDirectoryFromPath(getCurrentTemplatePath());

// --- <cfloop file="..."> iterates lines, preserving empty lines ---
// Fixture cfloop_file_fixture.txt: alpha / beta / (empty) / delta
fixturePath = thisDir & "cfloop_file_fixture.txt";
loopLines = [];
</cfscript>

<cfloop file="#fixturePath#" index="oneLine">
	<cfset arrayAppend(loopLines, oneLine)>
</cfloop>

<cfscript>
assert("cfloop file line count (empty line preserved)", arrayLen(loopLines), 4);
assert("cfloop file line 1", trim(loopLines[1]), "alpha");
assert("cfloop file line 2", trim(loopLines[2]), "beta");
assert("cfloop file line 3 is empty", trim(loopLines[3]), "");
assert("cfloop file line 4", trim(loopLines[4]), "delta");
</cfscript>

<!--- <cfinclude> of a static .js asset must splice verbatim, not parse as CFML --->
<cfsavecontent variable="jsOut"><cfinclude template="cfinclude_asset_fixture.js"></cfsavecontent>

<cfscript>
assertTrue("cfinclude .js spliced verbatim (typeof present)", findNoCase("typeof window", jsOut) GT 0);
assertTrue("cfinclude .js spliced verbatim (function present)", findNoCase("function f(e,t)", jsOut) GT 0);

// --- a cfincluded template inherits the calling function's arguments scope ---
function renderWith(required string tpl) {
	savecontent variable="local.out" {
		include "#arguments.tpl#";
	}
	return local.out;
}
rendered = renderWith(tpl = "cfinclude_args_target.cfm", injected = "HELLO");
assert("cfinclude inherits caller arguments scope", trim(rendered), "arg=[HELLO]");

suiteEnd();
</cfscript>
