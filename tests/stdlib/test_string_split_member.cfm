<cfscript>
suiteBegin("String .split() member (Java/regex semantics)");

// CFML's string member `.split(regex [, limit])` follows Java's String.split:
// the argument is a REGEX delimiter, NOT a CFML listToArray character set.
// (RustCFML previously aliased .split() to listToArray, so .split("ON") split
// on the chars {O,N} — corrupting Wheels' updateAll include→EXISTS JOIN parsing
// via `JOIN.Split("ON")[2]`.)

// Multi-char literal delimiter: splits on the substring, not its characters.
on = "LEFT OUTER JOIN c_o_r_e_comments ON c.postid = p.id".split("ON");
assert("literal substring split count", arrayLen(on), 2);
assert("part after ON keyword", trim(on[2]), "c.postid = p.id");

// Regex metacharacter: "." matches any char, so every char is a delimiter and
// (with Java limit 0) trailing empties are removed -> empty array.
assert("regex dot matches any char", arrayLen("a.b.c".split(".")), 0);

// Regex whitespace class collapses runs.
ws = "a  b   c".split("\s+");
assert("regex \s+ split count", arrayLen(ws), 3);
assert("regex \s+ first", ws[1], "a");
assert("regex \s+ last", ws[3], "c");

// Interior empties kept, trailing empties removed (Java limit 0).
comma = "a,,b,".split(",");
assert("empties: count", arrayLen(comma), 3);
assert("empties: [1]", comma[1], "a");
assert("empties: [2] kept", comma[2], "");
assert("empties: [3]", comma[3], "b");

// Limit argument (Java semantics): at most `limit` parts.
lim = "a,b,c,d".split(",", 2);
assert("limit count", arrayLen(lim), 2);
assert("limit remainder kept whole", lim[2], "b,c,d");

suiteEnd();
</cfscript>
