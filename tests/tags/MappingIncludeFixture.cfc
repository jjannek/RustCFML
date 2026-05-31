component {

	// Pseudo-constructor (runs at instantiation) includes a this.mappings path.
	// On Lucee/ACF/BoxLang the "/wheelsmapprobe" mapping resolves, this include
	// runs, and request.mappedIncludeMarker is set. On RustCFML the mapping is
	// NOT applied to cfinclude paths -> the literal "/wheelsmapprobe/..." read
	// fails -> the pseudo-constructor errors -> the component degrades to a
	// non-object SILENTLY at instantiation. This is the exact mechanism by which
	// wheels.Global fails: its pseudo-constructor does
	// `include "/app/global/functions.cfm"`.
	include "/wheelsmapprobe/mapped_include_target.cfm";

	this.markerFromInclude = StructKeyExists(request, "mappedIncludeMarker") ? request.mappedIncludeMarker : "(unset)";

	function getMarker() {
		return this.markerFromInclude;
	}

}
