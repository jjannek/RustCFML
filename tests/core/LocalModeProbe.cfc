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

    // PR-2: A closure with an explicit attribute overrides the enclosing mode.
    function runClosureExplicitOverride() localMode="modern" {
        cb = function() localMode="classic" {
            overrideKey = "classic-override";
            return structKeyExists(local, "overrideKey");
        };
        cb();
        // classic override: write went to variables of the *outer* CFC scope
        // (closures share the enclosing method's variables in CFML).
        return structKeyExists(variables, "overrideKey");
    }

    // PR-2: A closure inside a classic function stays classic by default.
    function runClosureInheritsClassic() {
        cb = function() {
            classicInherited = "classic-default";
        };
        cb();
        return structKeyExists(variables, "classicInherited");
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
