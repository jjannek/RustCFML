component {
	// Read a value from this object's variables scope. When this method is
	// injected onto another component as a mixin and invoked via that host
	// (`host.injected()`), it must read the HOST's variables — not this
	// definer's. (ColdBox: cfg.getPropertyMixin = mixerUtil.getPropertyMixin.)
	public any function readCacheBox() {
		var thisScope = variables;
		return structKeyExists( thisScope, "cacheBox" ) ? thisScope.cacheBox : "MISSING";
	}
}
