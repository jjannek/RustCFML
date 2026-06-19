/**
 * Fixture for the MockBox mock-creation mechanisms (GitHub #177).
 * Reproduces, without TestBox, the two engine behaviours MockBox relies on:
 *   1. structClear() on a component must keep it a usable object (identity +
 *      private scope), so a mixed-in `this`-referencing method still binds.
 *   2. A function declared inside an `include`d template (run in this object's
 *      method context) must land in the component scope and become callable.
 */
component accessors=true {

    property name="payload";

    function init(){
        variables.payload = "FACTORY";
        return this;
    }

    // A `this`-referencing method that gets copied onto another object (mixin).
    function dollar( required string method ){
        // Reaches the factory through `this`, then back through an accessor —
        // exercises method dispatch on a component reached via a copied scope key.
        this._lastFactoryPayload = this.factory.getPayload();
        this._mockResults[ arguments.method ] = "MOCKED:" & arguments.method;
        return this;
    }

    // Decorate an arbitrary target object the way MockBox.decorateMock does.
    function decorate( required target ){
        var obj = arguments.target;
        obj._mockResults = structNew();   // set a property on a component held in a local
        obj.dollar       = variables.dollar;
        obj.factory      = this;
    }

    // The MockBox `$include` mixin trick: included template runs in the target's
    // method scope, so `this[...] = variables[...]` and function decls bind here.
    function runInclude( required string templatePath ){
        include "#arguments.templatePath#";
    }
}
