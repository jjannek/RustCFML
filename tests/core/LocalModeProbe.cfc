component {
    // Classic (default): unscoped CFC-method assignments persist on variables.
    function run() {
        implicitInside = "implicit assignment in cfc method";
        return {
            localHasImplicit: structKeyExists(local, "implicitInside"),
            variablesHasImplicitInside: structKeyExists(variables, "implicitInside")
        };
    }

    function after() {
        return {
            variablesHasImplicitAfter: structKeyExists(variables, "implicitInside")
        };
    }

    // Modern: unscoped assignments stay in local.
    function runModern() localMode="modern" {
        modernImplicit = "implicit assignment in localMode modern method";
        return {
            localHasModernImplicit: structKeyExists(local, "modernImplicit"),
            variablesHasModernImplicitInside: structKeyExists(variables, "modernImplicit")
        };
    }

    function afterModern() {
        return {
            variablesHasModernImplicitAfter: structKeyExists(variables, "modernImplicit")
        };
    }

    // Alias coverage.
    function runTrueAlias() localMode="true" {
        aliasTrue = 1;
        return structKeyExists(local, "aliasTrue") AND NOT structKeyExists(variables, "aliasTrue");
    }

    function runAlwaysAlias() localMode="always" {
        aliasAlways = 1;
        return structKeyExists(local, "aliasAlways") AND NOT structKeyExists(variables, "aliasAlways");
    }

    function runClassicAlias() localMode="classic" {
        aliasClassic = 1;
        return structKeyExists(variables, "aliasClassic");
    }

    function runFalseAlias() localMode="false" {
        aliasFalse = 1;
        return structKeyExists(variables, "aliasFalse");
    }

    // Unknown values fall back to classic (helper returns false on garbage).
    function runUnknownAlias() localMode="garbage" {
        unk = 1;
        return structKeyExists(variables, "unk");
    }

    // Compound LHS in modern: foo.bar = 1 should land foo in local, not variables.
    // (PR-1 confirmation that the Load-mutate-Store codegen pattern routes through
    // the modern branch on the root identifier.)
    function runCompoundLhs() localMode="modern" {
        compound = {};
        compound.bar = "value";
        return {
            localHasCompound: structKeyExists(local, "compound"),
            variablesNoCompound: NOT structKeyExists(variables, "compound")
        };
    }

    // PR-2: Closure inside a modern function inherits modern semantics.
    function runClosureInheritsModern() localMode="modern" {
        // Closure has no own attribute → inherits enclosing function's modern mode.
        // An unscoped write inside the closure should land in the closure's own
        // local (and be invisible after the closure returns).
        cb = function() {
            insideClosure = "modern-inherited";
            return structKeyExists(local, "insideClosure");
        };
        result = cb();
        return {
            closureSawLocal: result,
            closureKeyNotInOuterLocal: NOT structKeyExists(local, "insideClosure"),
            closureKeyNotInVariables: NOT structKeyExists(variables, "insideClosure")
        };
    }

    // PR-3 strict-Lucee: a closure shadowing a captured outer var with an
    // unscoped write does NOT mutate the outer var.
    function runClosureShadowsCapturedVar() localMode="modern" {
        var captured = "before";
        cb = function() {
            captured = "after";  // unscoped → closure's local in modern, NOT outer's
            return local.captured;
        };
        closureSaw = cb();
        return {
            closureSawAfter: closureSaw EQ "after",
            outerStillBefore: captured EQ "before"
        };
    }

    // PR-2: A closure with an explicit attribute overrides the enclosing
    // mode. Returns two distinct facts so the suite can tell apart "classic
    // mode actually took effect inside the closure" (the unscoped write
    // bypassed `local`) from "the write surfaced on the outer CFC's
    // variables" (the side-effect callers see).
    function runClosureExplicitOverride() localMode="modern" {
        var sawAsLocal = true;
        cb = function() localMode="classic" {
            overrideKey = "classic-override";
            // In classic mode, unscoped writes target variables — not local.
            return structKeyExists(local, "overrideKey");
        };
        sawAsLocal = cb();
        return {
            // Classic override took effect inside the closure: the write
            // did NOT land in local.
            classicOverrideTookEffect: sawAsLocal EQ false,
            // The write surfaced on the enclosing CFC's variables, as
            // closures share variables with their defining method.
            leakedToVariables: structKeyExists(variables, "overrideKey")
        };
    }

    // PR-2: A closure inside a classic function stays classic by default.
    function runClosureInheritsClassic() {
        cb = function() {
            classicInherited = "classic-default";
        };
        cb();
        return structKeyExists(variables, "classicInherited");
    }

    // PR-3 follow-up: HOF callbacks invoked from a modern method should
    // themselves run in modern context — an unscoped write inside the
    // callback should land in the callback's own local, not surface on the
    // outer method's local or on variables.
    function runHofCallbackModern() localMode="modern" {
        arrayMap([1, 2, 3], function(n) {
            // Unscoped write; in modern context this targets the callback's
            // own `local` scope only — not the outer method's local or
            // variables.
            hofTouched = "n=" & n;
        });
        return {
            outerLocalUnpolluted: NOT structKeyExists(local, "hofTouched"),
            variablesUnpolluted:  NOT structKeyExists(variables, "hofTouched")
        };
    }

    // Explicit scope prefixes are unaffected by localMode.
    function runExplicitScopes() localMode="modern" {
        variables.forcedVar = "v";
        var localVar = "l";
        return {
            forcedVarInVariables: structKeyExists(variables, "forcedVar"),
            localVarInLocal: structKeyExists(local, "localVar"),
            localVarNotInVariables: NOT structKeyExists(variables, "localVar")
        };
    }
}
