component {
	// Deliberately empty receiver — methods are grafted onto it at runtime,
	// then invoked via `cfinvoke component=this`. Mirrors a Wheels model that
	// receives a spec's callback method.
}
