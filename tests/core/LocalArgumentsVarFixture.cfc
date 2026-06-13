component {

	// A local-scoped variable that happens to be NAMED `arguments` must behave
	// like any other local struct. This is the shape a Moopa route dispatcher
	// uses: it builds a working struct `local.arguments = {}`, fills it with
	// the matched route params, and hands it to the endpoint. The name is
	// incidental — `local.arguments` is a variable in the `local` scope, not
	// the `arguments` scope.
	public string function build() {
		local.arguments = {};
		local.arguments["route"]    = "tracks/abc";
		local.arguments["track_id"] = "THE-ID";
		structAppend(local.arguments, { "extra": "Z" }, true);

		return "keys=[" & structKeyList(local.arguments) & "]"
			& " | track_id=[" & (local.arguments.track_id ?: "NULL") & "]"
			& " | extra=[" & (local.arguments.extra ?: "NULL") & "]";
	}

	// `local.arguments` must be FULLY independent of the `arguments` scope:
	// shadowing it with a local must not poison the function's own declared
	// parameters (Lucee + BoxLang both back local/arguments with separate scope
	// objects). Here the function HAS a declared param `id` — reading
	// `arguments.id` after `local.arguments = {}` must still return the param.
	public string function buildWithArgs( required string id ) {
		local.arguments = {};
		local.arguments["route"] = "tracks/abc";

		return "argId=[" & arguments.id & "]"
			& " | argCount=[" & structCount( arguments ) & "]"
			& " | localKeys=[" & structKeyList( local.arguments ) & "]";
	}
}
