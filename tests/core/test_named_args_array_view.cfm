<cfscript>
suiteBegin("Core: named-argument calls keep the arguments scope array-addressable");

// On Lucee/ACF the arguments scope is a hybrid array/struct for NAMED calls
// too, not just positional ones:
//
//   function s() { return ArrayLen(arguments) & "|" & arguments[1]; }
//   s(name = "x")
//     Lucee 5.4.8.2    -> "1|x"
//     RustCFML 0.105.0 -> "0|" (ArrayLen == 0, arguments[1] is null)
//
// Positional calls index fine on RustCFML; only the ARRAY view of named calls
// is missing. The STRUCT view (StructCount / structKeyExists / StructKeyList)
// is already correct on both engines for named calls — asserted below as
// controls so a regression there can't masquerade as this gap.
//
// For a function that DECLARES params, numeric indexing follows DECLARATION
// order, independent of the order the named args were written at the call
// site (verified on Lucee 5.4.8.2: d(second="B", first="A") -> ArrayLen == 2,
// arguments[1] == "A", arguments[2] == "B"). On RustCFML 0.105.0 the declared
// slots ARE numerically readable, but ArrayLen still reports 0.
//
// Real-world impact: Wheels' config setter $set() (vendor/wheels/Global.cfc)
// is a PARAMLESS function that branches on the array view of its own
// arguments scope:
//
//   public void function $set() {
//       if (ArrayLen(arguments) > 1) { ...per-function settings... }
//       else { application[appKey][StructKeyList(arguments)] = arguments[1]; }
//   }
//
// Every Wheels config call is set(name = value) — a single NAMED argument. On
// RustCFML arguments[1] returned null for that call shape, so every setting
// was silently written as an empty value: set(dataSourceName = "wheels-dev")
// left application.wheels.dataSourceName == "" and the entire ORM introspected
// the wrong (default in-memory) database. No error anywhere — just a config
// store full of blanks.

// --- paramless probe: reports the shape of its own arguments scope ---
function aavParamlessProbe() {
    var view = {
        arrayLen    = arrayLen(arguments),
        structCount = structCount(arguments),
        keys        = structKeyList(arguments),
        first       = "(unreadable)"
    };
    try {
        view.first = isNull(arguments[1]) ? "(null)" : arguments[1];
    } catch (any e) {
        view.first = "(error: " & e.message & ")";
    }
    return view;
}

// 1. The Wheels $set() shape: ONE named argument to a paramless function.
aavNamed = aavParamlessProbe(dataSourceName = "wheels-dev");
assert("paramless fn, one named arg -> ArrayLen(arguments) == 1",
    aavNamed.arrayLen, 1);
assert("paramless fn, one named arg -> arguments[1] is the value",
    aavNamed.first, "wheels-dev");
// struct view of the same call (CONTROL — already correct on both engines)
assert("CONTROL: struct view of the named call -> StructCount == 1",
    aavNamed.structCount, 1);
assertTrue("CONTROL: struct view of the named call sees the key by name",
    listFindNoCase(aavNamed.keys, "dataSourceName") gt 0);

// 2. TWO named arguments: the exact ArrayLen(arguments) > 1 branch test that
//    $set() uses to distinguish per-function settings from a global setting.
aavNamedTwo = aavParamlessProbe(a = "x", b = "y");
assert("paramless fn, two named args -> ArrayLen(arguments) == 2",
    aavNamedTwo.arrayLen, 2);
assert("CONTROL: struct view of the two-named call -> StructCount == 2",
    aavNamedTwo.structCount, 2);

// 3. Positional control (already correct on RustCFML — guards the wiring so a
//    broken probe can't masquerade as the gap under test).
aavPositional = aavParamlessProbe("wheels-dev");
assert("CONTROL: positional call -> ArrayLen(arguments) == 1",
    aavPositional.arrayLen, 1);
assert("CONTROL: positional call -> arguments[1] is the value",
    aavPositional.first, "wheels-dev");

// --- declared-params probe: numeric index = declaration order ---
function aavDeclaredProbe(any first, any second) {
    var view = {
        arrayLen = arrayLen(arguments),
        slot1    = "(unreadable)",
        slot2    = "(unreadable)"
    };
    try {
        view.slot1 = isNull(arguments[1]) ? "(null)" : arguments[1];
    } catch (any e) {
        view.slot1 = "(error: " & e.message & ")";
    }
    try {
        view.slot2 = isNull(arguments[2]) ? "(null)" : arguments[2];
    } catch (any e) {
        view.slot2 = "(error: " & e.message & ")";
    }
    return view;
}

// 4. Named args written OUT OF declaration order: the array view still counts
//    the slots and indexes them in declaration order.
aavDeclared = aavDeclaredProbe(second = "B", first = "A");
assert("declared-params fn, named out-of-order -> ArrayLen(arguments) == 2",
    aavDeclared.arrayLen, 2);
assert("arguments[1] is the FIRST DECLARED param's value (declaration order)",
    aavDeclared.slot1, "A");
assert("arguments[2] is the SECOND DECLARED param's value (declaration order)",
    aavDeclared.slot2, "B");

suiteEnd();
</cfscript>
