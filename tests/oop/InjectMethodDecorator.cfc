/**
 * Mirrors Preside's PresideObjectDecorator: injects its own `onMissingMethod`
 * onto a foreign object so that missing-method calls on the decorated object
 * proxy to a service. The method reference is read via `this.onMissingMethod`
 * inside `decorate()`, so it carries a captured scope binding it to THIS
 * decorator — but when it later fires as a method on the decorated object, the
 * decorated object's `this` must win (so `this._svc` resolves).
 */
component singleton=true {
	public any function init() { return this; }

	public any function decorate( required any objectInstance, required any svc ) {
		var decorated         = arguments.objectInstance;
		decorated._svc        = arguments.svc;
		decorated._objectName = "myObject";

		// Copy the injector, then use it to inject onMissingMethod (exactly the
		// Preside $methodInjector pattern).
		decorated.$methodInjector = this.$methodInjector;
		decorated.$methodInjector( "onMissingMethod", this.onMissingMethod );
		structDelete( decorated, "$methodInjector" );

		return decorated;
	}

	public any function onMissingMethod( required string missingMethodName, required struct missingMethodArguments ) {
		return this._svc[ missingMethodName ]( argumentCollection = missingMethodArguments );
	}

	public void function $methodInjector( required string methodName, required any method ) {
		this[ arguments.methodName ]      = arguments.method;
		variables[ arguments.methodName ] = arguments.method;
	}
}
