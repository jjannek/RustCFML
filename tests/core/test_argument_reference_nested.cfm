<cfscript>
// Regression coverage adapted from Blute's PR #9 ("Preserve nested argument
// references"). The behaviour these assertions exercise is now provided by the
// reference-type work (arrays PR-1 dd85c19, structs PR-2 701b889); this suite
// guards against regressions. Passes on both RustCFML and Lucee 7.
suiteBegin("Nested argument references");

function mutateNestedTarget(required any target) {
    arguments.target.foo = "bar";
}

holder = { child = {} };
mutateNestedTarget(holder.child);
assert("nested struct argument writes back to caller path", holder.child.foo ?: "missing", "bar");

application.service = {};
mutateNestedTarget(application.service);
assert("application scope argument writes back to nested scope path", application.service.foo ?: "missing", "bar");

original = { child = { value = "before" } };
alias = original.child;
original.child.value = "after";
assert("assigned struct reference observes later nested mutation", alias.value, "after");

function enrichSchema(required any schema) {
    arguments.schema.route.fields.profiles.generated = "yes";
}

input = {
    route = {
        fields = {
            profiles = {}
        }
    }
};
out = {};
out.route = input.route;
enrichSchema(input);
assert("deep argument writeback updates original", input.route.fields.profiles.generated ?: "missing", "yes");
assert("deep argument writeback updates alias", out.route.fields.profiles.generated ?: "missing", "yes");

suiteEnd();
</cfscript>
