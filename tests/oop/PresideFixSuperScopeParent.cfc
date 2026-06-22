component {

	// Mirrors ColdBox cbi18n models/i18n.cfc init(): no explicit `instance = {}`,
	// the first compound write auto-vivifies `instance`, then an UNQUALIFIED call
	// to a method that is overridden in the child dispatches to the child, which
	// reaches back via `super` to read `instance` here.
	function init() {
		instance.aLocale = "SEEDED";          // unscoped compound auto-viv
		variables.seed = buildLocale();        // unqualified → most-derived override
		return this;
	}

	private function buildLocale() {
		return instance.aLocale;               // reads variables.instance.aLocale
	}

	function getSeed() {
		return variables.seed;
	}

}
