<cfscript>
suiteBegin("MockBox mock-creation mechanisms (GitHub ##177)");

// --- 1) structClear() must preserve component identity ---------------------
// MockBox.createEmptyMock does structClear(obj) then mixes methods on. After
// the clear the object must still be a component so a mixed-in this-referencing
// method can bind `this`.
factory = new oop.MockMixinTarget();

cleared = new oop.MockMixinTarget();
structClear( cleared );
assert("structClear keeps it an object", isObject( cleared ), true);
// Public methods are gone (Lucee parity) ...
assertFalse("public method removed by clear", structKeyExists( cleared, "init" ));
// ... but identity/private scope survive, so a mixed-in method still works.
factory.decorate( cleared );
cleared.dollar( "render" );
assert("mixed-in method bound `this` after clear", cleared._mockResults.render, "MOCKED:render");
assert("`this.factory` accessor reachable from mixin", cleared._lastFactoryPayload, "FACTORY");

// --- 2) decorate a NON-cleared component (createMock path) -----------------
plain = new oop.MockMixinTarget();
factory.decorate( plain );          // obj._mockResults = structNew() on a component
plain.dollar( "save" );
assert("decorate sets struct prop on component", plain._mockResults.save, "MOCKED:save");

// --- 3) function injection via include into a component scope --------------
// MockGenerator writes a .cfm and has the target `include` it; the declared
// function must become callable on the target (this was silently dropped).
host = new oop.MockMixinTarget();
// Relative include path — resolved against the CFC's directory (tests/oop)
// on both RustCFML and Lucee, mirroring MockBox's $include mixin.
host.runInclude( "mock_mixin_generated.cfm" );
assert("injected method is callable (public this)", host.injected(), "INJECTED-RESULT");

// --- 4) tag-based <cffunction> parsing (TestBox 5.4.0 / MockBox) -----------
tagcfc = new oop.TagArgCFC();
// (a) a multi-line <cfargument> must bind as the first positional parameter.
assert("multi-line cfargument binds positionally", tagcfc.cm( "Plain" ), "Plain");
// (b) a <cffunction> with a dotted component-path returntype must not be
//     dropped from the component (this was silently swallowed at parse time).
assert("dotted-returntype method is callable", tagcfc.getHelper().tag(), "HELPER");

suiteEnd();
</cfscript>
