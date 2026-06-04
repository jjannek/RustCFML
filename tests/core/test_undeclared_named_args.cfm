<cfscript>
suiteBegin("Core: undeclared named arguments keep their names");

o = createObject("component", "UndeclaredArgFixture");
assert("extra named args are reachable by name in the arguments scope",
	o.probe(), "a=A,b=B,c=C");

suiteEnd();
</cfscript>
