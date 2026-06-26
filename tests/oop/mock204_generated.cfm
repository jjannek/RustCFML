<cfscript>
    // Mirrors MockGenerator's stub: declare a temp fn, expose it under the
    // mocked name on `this`, then clean up. The generated method reads the
    // back-ref (`this.backref`) at call time — null on the v0.281.0 regression.
    this[ "m" ] = variables[ "tmpFn204" ];
    structDelete( variables, "tmpFn204" );
    function tmpFn204() output=false {
        return this.backref.normalize() & ":ok";
    }
</cfscript>
