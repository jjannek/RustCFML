<cfscript>
suiteBegin("OOP: repeated instantiation / re-include (function-table merge dedup)");

// ============================================================
// Background
// ============================================================
// CFC instantiation and <cfinclude> merge the loaded file's functions into
// the running program's function table so their func_idx references stay
// valid. That merge used to run on EVERY instantiation/include with no
// dedup, growing the table linearly within a request — a memory leak that,
// on WASM hosts (Cloudflare Workers), ratchets the isolate heap up and never
// releases. The fix caches the merge offset per file at the top level.
//
// These assertions guard the BEHAVIOUR the fix must preserve: independent
// instances, working methods (including inherited super-calls), and a
// re-included function staying callable — across many repetitions.
// ============================================================

// ------------------------------------------------------------
// Plain CFC: many instances stay independent and functional
// ------------------------------------------------------------
for (i = 1; i <= 50; i++) {
    g = new oop.Greeter("Hi" & i);
    assert("greeter " & i & " greeting", g.getGreeting(), "Hi" & i);
    assert("greeter " & i & " greet()", g.greet("World"), "Hi" & i & ", World!");
}

// Two live instances built in the same request must not share state.
a = new oop.Greeter("Alpha");
b = new oop.Greeter("Beta");
assert("instance a isolated", a.getGreeting(), "Alpha");
assert("instance b isolated", b.getGreeting(), "Beta");

// ------------------------------------------------------------
// Inherited CFC: super.init() must resolve correctly every time.
// This is the case that regressed when offsets were cached naively:
// the parent (Animal) is resolved as a nested load, so its merge
// offset must NOT be reused at the top level.
// ------------------------------------------------------------
for (i = 1; i <= 50; i++) {
    d = new oop.Dog();
    assert("dog " & i & " species", d.getSpecies(), "Dog");
    assert("dog " & i & " speak()", d.speak(), "Woof");
}

// ------------------------------------------------------------
// Re-included function-defining file stays callable each time.
// ------------------------------------------------------------
for (i = 1; i <= 30; i++) {
    include "_repeated_helper.cfm";
    assert("re-include " & i & " helper callable", repeatedHelperFn(i), i * 2);
}

suiteEnd();
</cfscript>
