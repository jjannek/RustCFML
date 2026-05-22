// A CFML class that inherits from the Rust-backed Tally native class.
// Demonstrates the `extends="rust:Name"` form: super.X calls the Rust
// parent, and any method the CFC doesn't override falls through to it.
component extends="rust:Tally" {

    // Add CFML behaviour on top of the parent's primitives.
    public numeric function bumpBy(required numeric n) {
        for (i = 1; i <= arguments.n; i++) {
            super.bump();
        }
        return super.value();
    }

}
