/**
 * Minimal MockBox-shaped factory. Reproduces the decoration pattern from
 * issue #204: a control method (`doMock`) is copied onto the target along with
 * a back-reference (`obj.backref = this`). doMock drives generation through the
 * back-ref, which injects a method that reads `this.backref` at call time.
 */
component {
    function init(){ variables.gen = new Mock204Gen(); variables.calls = 0; return this; }
    function getGen(){ return variables.gen; }
    function normalize(){ return "norm"; }
    // A variables-MUTATING method invoked through the back-ref (mirrors MockBox
    // calling this.mockBox.<method>() — the scope-prefixed multi-segment
    // writeback that v0.281.0 broke).
    function logCall(){ variables.calls = variables.calls + 1; return this; }

    // Control method copied onto the target (like MockBox.$). Note the casing:
    // the call reads `this.Backref` (capital B) while decorate() stored
    // `backref` (lowercase) — mirroring MockBox's `this.MockBox` vs the stored
    // `mockBox`. CFML keys are case-insensitive, so this MUST still resolve.
    function doMock( required method ){
        this.Backref.logCall();             // scope-prefixed write, mixed casing
        var g = this.Backref.getGen();      // scope-prefixed read
        g.gen( targetObject = this );        // -> arguments.targetObject.doInc(...)
        return this;
    }

    // Decorate an arbitrary target the way MockBox.decorateMock does.
    function decorate( required target ){
        var obj = arguments.target;
        obj.doMock  = variables.doMock;
        obj.doInc   = variables.doInc;
        obj.backref = this;                  // the back-ref that must survive
        return obj;
    }

    // MockBox.$include — runs a template in the target's method scope.
    function doInc( required template ){
        include "#arguments.template#";
    }

    function createEmptyMock( required className ){
        var obj = createObject( "component", arguments.className ).init();
        structClear( obj );
        return decorate( obj );
    }
}
