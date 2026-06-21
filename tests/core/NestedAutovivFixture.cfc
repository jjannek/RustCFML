component {

	// beforeEach-style closure stashes into an undeclared `stash` container;
	// afterEach-style closure reads it back. They share the component variables
	// scope, so the auto-vivified `stash` must be visible across both.
	function run() {
		var be = () => { stash.request.cgi = "world"; };
		var ae = () => { return stash.request.cgi; };
		be();
		return ae();
	}

	// Direct (non-closure) nested auto-viv from a method must also write the
	// component (variables) scope under classic localmode, so a second method
	// sees it.
	function deep() {
		holder.a.b = 7;
		return readHolder();
	}

	function readHolder() {
		return holder.a.b;
	}

}
