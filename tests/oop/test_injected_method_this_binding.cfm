<cfscript>
// ---------------------------------------------------------------------------
// Injected-method `this` binding (Preside PresideObjectDecorator pattern).
//
// A method read via `this.method` inside a CFC method carries a captured scope
// that pins `this` to the component it was read from. When that bound method is
// injected onto ANOTHER component and later dispatched as a method on it (here
// via onMissingMethod), the RECEIVER's `this` must win — otherwise the captured
// `this` shadows the receiver and `this._svc` resolves to Null, producing
// "Variable is not a function or function '<unknown>' is not defined".
//
// This was the dominant Preside presideObjects blocker (insertData/updateData/
// insertDataFromSelect all proxied through the decorator's onMissingMethod).
// ---------------------------------------------------------------------------
suiteBegin("Injected method this-binding (decorator/onMissingMethod)");

deco      = new oop.InjectMethodDecorator();
svc       = new oop.InjectMethodService();
obj       = new oop.InjectMethodObject();
decorated = deco.decorate( objectInstance = obj, svc = svc );

// A missing method on the decorated object fires the injected onMissingMethod,
// which must run with `this` = decorated (so `this._svc` is the service).
result = decorated.insertData( objectName = "foo", data = "bar" );
assert("injected onMissingMethod proxies via receiver's this._svc", result, "INSERTED obj=foo data=bar");

// Positional + argumentCollection re-ordering still reaches the service.
ac = { objectName = "baz", data = "qux" };
result2 = decorated.insertData( argumentCollection = ac );
assert("injected onMissingMethod handles argumentCollection", result2, "INSERTED obj=baz data=qux");

// Sanity: a normal (non-injected) mixin call still binds `this` to the receiver.
suiteEnd();
</cfscript>
