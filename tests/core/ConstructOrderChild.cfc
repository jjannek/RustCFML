component extends="ConstructOrderParent" {
	// Inherited method called during the child's pseudo-constructor must see
	// inherited siblings in variables (bare and via this.method()).
	variables.inhBare = pHelper();
	variables.inhThis = this.pHelper();

	function inhBareR(){ return variables.inhBare; }
	function inhThisR(){ return variables.inhThis; }
}
