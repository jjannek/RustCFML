component extends="PresideFixArgCollParent" {
	// Paramless init forwarding positional args via argumentCollection — the
	// ColdBox Controller shape. arguments here is keyed {1:..,2:..}.
	public any function init() {
		super.init( argumentCollection = arguments );
		return this;
	}
}
