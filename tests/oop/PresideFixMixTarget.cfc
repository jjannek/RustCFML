component {
	public any function run() {
		variables.cacheBox = "TARGET-CACHEBOX";
		// Inject another component's method as a mixin, then invoke it AS a
		// method on this host — it must run with this host's variables.
		this.injected = new PresideFixMixHost().readCacheBox;
		return this.injected();
	}
}
