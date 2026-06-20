<cfscript>
// Issue #183: cfscript-statement `loop` form (the script equivalent of
// <cfloop> with attributes). Verified to match Lucee/ACF/BoxLang.
suiteBegin("Script loop statement");

// from / to / index
out = "";
loop from=1 to=3 index="i" { out &= i; }
assertTrue("loop from-to-index", out eq "123");

// from / to / index with step
out = "";
loop from=1 to=5 index="i" step=2 { out &= i; }
assertTrue("loop step", out eq "135");

// negative step (descending)
out = "";
loop from=3 to=1 index="i" step=-1 { out &= i; }
assertTrue("loop negative step", out eq "321");

// times
out = "";
loop times=3 { out &= "x"; }
assertTrue("loop times", out eq "xxx");

// condition (expression written as a string)
n = 0;
out = "";
loop condition="n LT 3" { n++; out &= "c"; }
assertTrue("loop condition", out eq "ccc");

// array with item
out = "";
loop array=[10,20,30] item="v" { out &= v & " "; }
assertTrue("loop array item", trim(out) eq "10 20 30");

// array with item + index (1-based position)
out = "";
loop array=["a","b","c"] item="v" index="ix" { out &= ix & ":" & v & " "; }
assertTrue("loop array item+index", trim(out) eq "1:a 2:b 3:c");

// list with item
out = "";
loop list="a,b,c" item="x" { out &= x; }
assertTrue("loop list", out eq "abc");

// list with custom delimiters
out = "";
loop list="a|b|c" item="x" delimiters="|" { out &= x; }
assertTrue("loop list delimiters", out eq "abc");

// collection (struct) with item + key
st = {one=1, two=2, three=3};
out = "";
loop collection=st item="val" key="k" { out &= k & "=" & val & " "; }
assertTrue("loop collection key+item", out contains "one=1" and out contains "three=3");

// query without an index — q.col resolves to the current row
q = queryNew("a,b", "varchar,varchar", [["x","1"],["y","2"]]);
out = "";
loop query=q { out &= q.a & q.b & " "; }
assertTrue("loop query bare", trim(out) eq "x1 y2");

// query with an explicit index variable
out = "";
loop query=q index="row" { out &= row.a; }
assertTrue("loop query index", out eq "xy");

// break / continue bind to the synthesized loop
out = "";
loop from=1 to=10 index="i" { if (i eq 3) break; out &= i; }
assertTrue("loop break", out eq "12");

out = "";
loop from=1 to=5 index="i" { if (i mod 2 eq 0) continue; out &= i; }
assertTrue("loop continue", out eq "135");

suiteEnd();
</cfscript>
