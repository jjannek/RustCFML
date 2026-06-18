<cfscript>
suiteBegin("Static blocks and static scope");

// --- static scope reads inside instance methods -------------------------
a = new oop.StaticConsole();
assert("static scalar read", a.greet(), "hello");
assert("static struct dot read", a.colorRed(), chr(27) & "[31m");
assert("static struct dynamic key read", a.colorByKey("green"), chr(27) & "[32m");

// --- static scope is shared per type ------------------------------------
// Use RELATIVE assertions so the test is agnostic to whether statics persist
// for the application lifetime (Lucee/BoxLang) or per request.
before = a.getCount();
a.bump();
assert("bump increments shared counter", a.getCount(), before + 1);
b = new oop.StaticConsole();
assert("new instance sees shared count", b.getCount(), before + 1);
b.bump();
assert("mutation through one instance visible on another", a.getCount(), before + 2);

// --- static functions ---------------------------------------------------
// Callable on an instance, reading static scope.
assert("static fn on instance", a.wrap("green", "X"),
    chr(27) & "[32m" & "X" & chr(27) & "[0m");
// Callable via the `::` operator without an instance.
assert("static fn via ::", oop.StaticConsole::wrap("red", "Y"),
    chr(27) & "[31m" & "Y" & chr(27) & "[0m");

// --- `::` static member access (no instance) ----------------------------
assert("static var via ::", oop.StaticConsole::GREETING, "hello");

// --- getComponentStaticScope() ------------------------------------------
// The name-string form is the Lucee-documented signature (portable).
s2 = getComponentStaticScope("oop.StaticConsole");
assert("getComponentStaticScope by name", s2.GREETING, "hello");
// Passing a component instance is a RustCFML convenience (Lucee accepts a
// name string only), so guard it to keep the suite green on Lucee.
if (isRustCFML()) {
    s1 = getComponentStaticScope(a);
    assert("getComponentStaticScope on instance", s1.GREETING, "hello");
}

// --- <cfstatic> tag form ------------------------------------------------
t = new oop.StaticTagForm();
assert("cfstatic scoped write", t.scoped(), "from-cfstatic");
assert("cfstatic unscoped write", t.plainVal(), 7);

// --- static inheritance -------------------------------------------------
kid = new oop.StaticKid();
assert("child reads own static", kid.ownValue(), "kid-only");
assert("child reads inherited static", kid.inheritedGreeting(), "hello");
assert("inherited static via ::", oop.StaticKid::GREETING, "hello");

suiteEnd();
</cfscript>
