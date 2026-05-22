<cfscript>
// This CFML app calls into the Rust module that lives in ./native/greeter/.
// The Rust functions and class become first-class CFML identifiers once the
// module is registered — there's no FFI ceremony on the CFML side.

writeOutput("Greeting from Rust: " & rustGreet("Alex") & chr(10));
writeOutput("2 + 3 (computed in Rust) = " & rustAdd(2, 3) & chr(10));

counter = createObject("rust", "Tally");
counter.bump();
counter.bump();
counter.bump();
writeOutput("Tally after 3 bumps: " & counter.value() & chr(10));

// A CFC inherits from the Rust Tally class via extends="rust:Tally".
boosted = createObject("component", "BoostedTally");
writeOutput("BoostedTally.bumpBy(5) = " & boosted.bumpBy(5) & chr(10));
// Implicit fall-through: the CFC has no .value() method, so it reaches Tally.
writeOutput("BoostedTally.value() (parent method) = " & boosted.value() & chr(10));
</cfscript>
