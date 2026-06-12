<cfscript>
// String / polymorphic kernel — what the JIT CANNOT JIT today. This is the
// surface v0.88.0 will measure (coverage signal) and v0.90.0 will start
// targeting (boxed `+` / concat). Expected baseline: ~1.0× (interpreter
// only, since the JIT bails on every operation that touches a string).
function buildLine(prefix, n) {
    var s = prefix;
    for (var i = 1; i <= n; i++) {
        s = s & "-" & i;
    }
    return s;
}
total = "";
for (k = 1; k <= 5000; k++) { total = buildLine("row" & k, 300); }
writeOutput(len(total));

</cfscript>
