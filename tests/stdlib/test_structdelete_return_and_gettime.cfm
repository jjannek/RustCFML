<cfscript>
suiteBegin("StructDelete boolean return + date.getTime()");

// StructDelete returns a BOOLEAN (Lucee/ACF), not the struct. Default form is
// always true; with indicateNotExisting=true it reports whether the key existed.
// (Wheels' flashDelete returns this value directly and asserts toBeTrue.)
s = {success: "Congrats!"};
r = StructDelete(s, "success");
assertTrue("StructDelete default returns boolean true", r);
assertTrue("StructDelete is boolean", isBoolean(r));
assertFalse("StructDelete actually removed the key", structKeyExists(s, "success"));

assertTrue("StructDelete indicateNotExisting=true, key existed", StructDelete({a: 1}, "a", true));
assertFalse("StructDelete indicateNotExisting=true, key missing", StructDelete({a: 1}, "zzz", true));

// The boolean return must NOT clobber the struct variable in statement form, nor
// via the member-function form (struct.delete()). The in-place mutation stands.
del = {a: 1, b: 2, c: 3};
StructDelete(del, "b");
assert("statement StructDelete leaves struct intact (count)", structCount(del), 2);
assertFalse("statement StructDelete removed the key", structKeyExists(del, "b"));

m = {x: 1, y: 2};
m.delete("x");
assert("member .delete() leaves struct intact (count)", structCount(m), 1);
assertFalse("member .delete() removed the key", structKeyExists(m, "x"));

// StructDelete on a SCOPE deletes from the live scope (scopes are snapshotted
// when passed as a builtin arg, so this routes through a scope-aware delete).
request.rcfmlDelTest = "x";
StructDelete(request, "rcfmlDelTest");
assertFalse("StructDelete(request, key) deletes from the request scope",
    structKeyExists(request, "rcfmlDelTest"));
variables.rcfmlVarDel = 1;
StructDelete(variables, "rcfmlVarDel");
assertFalse("StructDelete(variables, key) deletes from the variables scope",
    structKeyExists(variables, "rcfmlVarDel"));

// Deep-path member delete must not clobber the parent.
outer = {inner: {p: 1, q: 2}};
outer.inner.delete("p");
assert("deep member .delete() keeps inner a struct", structCount(outer.inner), 1);

// date.getTime() (java.util.Date.getTime) returns epoch milliseconds. Previously
// returned Null -> the null-delete assignment guard wiped the assigned-into
// local (Wheels propertiesSpec "epoch works").
epochtime = Now().getTime();
assertTrue("Now().getTime() is numeric epoch millis", isNumeric(epochtime));
assertTrue("Now().getTime() is a plausible 13-digit epoch", epochtime > 1000000000000);
// A fixed date round-trips through getTime consistently.
fixed = createDateTime(2020, 1, 1, 0, 0, 0).getTime();
assertTrue("fixed date getTime is numeric", isNumeric(fixed));

suiteEnd();
</cfscript>
