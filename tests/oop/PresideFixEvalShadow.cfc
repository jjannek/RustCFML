/**
 * Mirrors Preside's cfflow condition CFCs, which declare a method named
 * `evaluate` (with a required `args` param). Defining this must NOT shadow the
 * `Evaluate()` BIF for bare calls elsewhere in the program.
 */
component {
    public boolean function evaluate( required any wfInstance, required struct args ) {
        return true;
    }

    // A bare Evaluate() call inside *this* component must still reach the BIF.
    public any function callBif( required string expr ) {
        return Evaluate( arguments.expr );
    }
}
