/**
 * Mirrors Preside's DelayedInjector: onMissingMethod resolves the real target
 * and forwards via a computed-name method call
 * `instance[ missingMethodName ]( argumentCollection=... )`. That dynamic call
 * must run the target method against the TARGET's component scope, not this
 * proxy's.
 */
component {
    public any function init( required any target ) {
        variables._target = arguments.target;
        return this;
    }
    public any function onMissingMethod( required string missingMethodName, struct missingMethodArguments={} ) {
        var instance = variables._target;
        return instance[ arguments.missingMethodName ]( argumentCollection=arguments.missingMethodArguments );
    }
}
