<cfscript>
    request.pageTitle = "CFML language samples";
    request.activeNav = "cfml";

    // ── Variables and types
    sampleName       = "RustCFML";
    sampleVersion    = 1.0;
    sampleIsAwesome  = true;
    sampleItems      = [10, 20, 30, 40, 50];

    // ── Array operations
    //
    // (We'd love to write this as
    //     `numbers.map(triple).filter(keepOdd).reduce(totalize, 0)`
    //  but RustCFML currently has an engine bug where only the FIRST
    //  HOF with a named-function callback works inside an Application.cfc
    //  `onRequest`-included page. Tracked separately. Using manual
    //  loops here so the demo stays accurate.)
    numbers = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    tripled = [];
    odds    = [];
    total   = 0;
    for (n in numbers) {
        arrayAppend(tripled, n * 3);
        if (n mod 2 neq 0) arrayAppend(odds, n);
        total += n;
    }

    // ── Structs
    person = { name: "Ada", role: "engineer", skills: ["rust", "cfml", "wasm"] };

    // ── Recursion
    function fib(n) {
        return n < 2 ? n : fib(n - 1) + fib(n - 2);
    }
    fibResults = [];
    for (i = 0; i < 10; i++) {
        arrayAppend(fibResults, fib(i));
    }

    // ── FizzBuzz
    fizzbuzz = [];
    for (i = 1; i <= 15; i++) {
        if (i mod 15 eq 0)     arrayAppend(fizzbuzz, "#i#: FizzBuzz");
        else if (i mod 3 eq 0) arrayAppend(fizzbuzz, "#i#: Fizz");
        else if (i mod 5 eq 0) arrayAppend(fizzbuzz, "#i#: Buzz");
        else                   arrayAppend(fizzbuzz, "#i#");
    }
</cfscript>
<cfinclude template="includes/header.cfm">
<cfoutput>

<div class="panel">
    <div class="panel-header">Variables</div>
    <div class="panel-body">
        <pre class="code">name        = "RustCFML"
version     = 1.0
isAwesome   = true
items       = [10, 20, 30, 40, 50]</pre>
        <pre class="output">name:     #sampleName#
version:  #sampleVersion#
awesome:  #sampleIsAwesome#
items:    #arrayToList(sampleItems)#
count:    #arrayLen(sampleItems)#</pre>
    </div>
</div>

<div class="panel">
    <div class="panel-header">Higher-order functions on arrays</div>
    <div class="panel-body">
        <pre class="code">numbers = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
tripled = []; odds = []; total = 0;
for (n in numbers) {
    arrayAppend(tripled, n * 3);
    if (n mod 2 neq 0) arrayAppend(odds, n);
    total += n;
}</pre>
        <pre class="output">tripled: #arrayToList(tripled)#
odds:    #arrayToList(odds)#
total:   #total#</pre>
    </div>
</div>

<div class="panel">
    <div class="panel-header">Structs</div>
    <div class="panel-body">
        <pre class="code">person = { name: "Ada", role: "engineer", skills: ["rust", "cfml", "wasm"] };</pre>
        <pre class="output">person.name:   #person.name#
person.role:   #person.role#
person.skills: #arrayToList(person.skills)#</pre>
    </div>
</div>

<div class="panel">
    <div class="panel-header">Recursion — fib(0..9)</div>
    <div class="panel-body">
        <pre class="code">function fib(n) { return n < 2 ? n : fib(n - 1) + fib(n - 2); }</pre>
        <pre class="output">#arrayToList(fibResults, ", ")#</pre>
    </div>
</div>

<div class="panel">
    <div class="panel-header">FizzBuzz</div>
    <div class="panel-body">
        <pre class="output">#arrayToList(fizzbuzz, chr(10))#</pre>
    </div>
</div>

</cfoutput>
<cfinclude template="includes/footer.cfm">
