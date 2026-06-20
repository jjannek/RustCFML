<!---
  Regression (v0.240.0):
  - Chained assignment whose inner target is `this.X`
    (`variables.a = this.a = v`) must store the value in BOTH scopes. The inner
    assignment used not to leave its value for the outer store, so variables.a
    stayed unset → "Variable $assert is undefined" in TestBox/Wheels BaseSpec.
  - ConcurrentHashMap.entrySet().toArray() must yield Map.Entry objects whose
    getKey()/getValue() work (Wheels Channel.publish iterates these); entrySet()
    used to return Null → "cannot call method [toArray] on a null value".
  Passes on RustCFML + Lucee 7.
--->
<cfscript>
suiteBegin("Chained this.X assignment + ConcurrentHashMap.entrySet");

obj = new ChainAssignFixture();
assert("chained variables.x = this.x = v sets both", obj.report(), "variables=hello|this=hello");

m = createObject("java", "java.util.concurrent.ConcurrentHashMap").init();
m.put("alpha", "one");
m.put("beta", "two");
entries = m.entrySet().toArray();
assert("entrySet().toArray() length", arrayLen(entries), 2);
roundtrip = {};
for (e in entries) {
	roundtrip[e.getKey()] = e.getValue();
}
assert("entry getKey/getValue alpha", roundtrip.alpha, "one");
assert("entry getKey/getValue beta", roundtrip.beta, "two");

suiteEnd();
</cfscript>
