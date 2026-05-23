<cfscript>
suiteBegin("LocalMode (Lucee modern/classic compatibility)");

probe = createObject("component", "core.LocalModeProbe");

// --- Classic (default) ---
classicInside = probe.run();
classicAfter  = probe.after();
assertFalse("classic: local does NOT have implicitInside inside method", classicInside.localHasImplicit);
assertTrue ("classic: variables HAS implicitInside inside method",       classicInside.variablesHasImplicitInside);
assertTrue ("classic: variables RETAINS implicitInside after return",    classicAfter.variablesHasImplicitAfter);

// --- Modern (function attribute localMode="modern") ---
modernInside = probe.runModern();
modernAfter  = probe.afterModern();
assertTrue ("modern: local HAS modernImplicit inside method",            modernInside.localHasModernImplicit);
assertFalse("modern: variables does NOT have modernImplicit inside",     modernInside.variablesHasModernImplicitInside);
assertFalse("modern: variables does NOT retain modernImplicit after",    modernAfter.variablesHasModernImplicitAfter);

// --- Aliases ---
assertTrue ("alias localMode=""true"" behaves modern",   probe.runTrueAlias());
assertTrue ("alias localMode=""always"" behaves modern", probe.runAlwaysAlias());
assertTrue ("alias localMode=""classic"" behaves classic", probe.runClassicAlias());
assertTrue ("alias localMode=""false"" behaves classic",   probe.runFalseAlias());

// --- Unknown localMode value falls back to classic ---
assertTrue("unknown value (""garbage"") behaves classic", probe.runUnknownAlias());

// --- Compound LHS in modern: head identifier routes through local ---
compound = probe.runCompoundLhs();
assertTrue ("modern + foo.bar = …: foo lands in local",     compound.localHasCompound);
assertTrue ("modern + foo.bar = …: foo absent from variables", compound.variablesNoCompound);

// --- PR-2: closure inheritance ---
inh = probe.runClosureInheritsModern();
assertTrue ("closure inside modern fn: insideClosure went to closure's local", inh.closureSawLocal);
assertTrue ("closure inside modern fn: outer local untouched",                 inh.closureKeyNotInOuterLocal);
assertTrue ("closure inside modern fn: variables untouched",                   inh.closureKeyNotInVariables);

shadow = probe.runClosureShadowsCapturedVar();
assertTrue ("modern closure shadowing captured var: closure sees its own write",  shadow.closureSawAfter);
assertTrue ("modern closure shadowing captured var: outer's var is unchanged",    shadow.outerStillBefore);

override = probe.runClosureExplicitOverride();
assertTrue ("closure localMode=""classic"" override takes effect (unscoped write bypasses local)", override.classicOverrideTookEffect);
assertTrue ("closure localMode=""classic"" override surfaces on outer CFC variables",              override.leakedToVariables);
assertTrue ("closure inside classic fn stays classic (leaks to variables)",                    probe.runClosureInheritsClassic());

// --- HOF callback in modern context doesn't pollute caller locals ---
hof = probe.runHofCallbackModern();
assertTrue("modern HOF callback: outer local not polluted", hof.outerLocalUnpolluted);
assertTrue("modern HOF callback: variables not polluted",   hof.variablesUnpolluted);

// --- Explicit scope prefixes are unaffected ---
explicit = probe.runExplicitScopes();
assertTrue("modern: explicit variables.x still writes to variables", explicit.forcedVarInVariables);
assertTrue("modern: var-declared still goes to local",               explicit.localVarInLocal);
assertTrue("modern: var-declared does NOT leak to variables",        explicit.localVarNotInVariables);

suiteEnd();
</cfscript>
