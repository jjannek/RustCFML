component extends="PresideFixSuperScopeParent" {

	variables._cache = {};

	function init() {
		super.init( argumentCollection = arguments );
		return this;
	}

	// Overrides the parent; the parent's init() calls buildLocale() unqualified,
	// which lands here, then defers to the parent via super.
	private function buildLocale() {
		return super.buildLocale( argumentCollection = arguments );
	}

}
