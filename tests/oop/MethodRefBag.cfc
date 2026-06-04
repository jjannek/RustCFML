component {

    function init() {
        variables.log = [];
        return this;
    }

    function record( item ) {
        // Both `this` and `variables` must be bound when this method is
        // invoked through a higher-order callback (.each / .map / .filter).
        arrayAppend( variables.log, item );
        return this;
    }

    function getLog() {
        return variables.log;
    }

    function runEach( required array items ) {
        // Pass the method by bare name. Lucee/ACF bind `this`/`variables`
        // at the load site so the callback still sees the receiver.
        arguments.items.each( record );
        return variables.log;
    }

    function runMap( required array items ) {
        return arguments.items.map( upper );
    }

    function upper( item ) {
        // Method reads variables to prove the binding survives — store a
        // prefix on init and use it here.
        return uCase( variables.prefix ?: "" ) & uCase( item );
    }

    function setPrefix( required string p ) {
        variables.prefix = arguments.p;
        return this;
    }

    function runFilter( required array items ) {
        return arguments.items.filter( shouldKeep );
    }

    function shouldKeep( item ) {
        return listFindNoCase( variables.allow ?: "", item ) gt 0;
    }

    function setAllow( required string list ) {
        variables.allow = arguments.list;
        return this;
    }

    function nestedCall( required array items ) {
        // The callee `record` itself calls another bare-name method
        // (`stamp`). Both hops must keep the binding.
        arguments.items.each( stamp );
        return variables.log;
    }

    function stamp( item ) {
        record( "stamped:" & item );
        return this;
    }
}
