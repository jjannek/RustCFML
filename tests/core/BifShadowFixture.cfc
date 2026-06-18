component {
    // A method whose name collides with a built-in function. A BARE call to the
    // builtin name inside the method must resolve to the BIF (Lucee binds BIFs at
    // compile time), NOT recurse into this method. The method is reachable via
    // `this.isJSON()` / `variables.isJSON()`. (TestBox Assertion.cfc wraps several
    // BIFs this way — isJSON/isXml/… — and without this they recurse infinitely.)
    function isJSON( required any actual ) {
        if ( !isJSON( arguments.actual ) ) {   // bare call -> the BIF
            return "invalid";
        }
        return "valid";
    }

    function callViaThis() {
        return this.isJSON( '{"a":1}' );        // explicit this. -> THIS method
    }
}
