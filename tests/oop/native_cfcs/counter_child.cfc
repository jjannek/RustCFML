component extends="rust:Counter" {

    // Calls a parent method explicitly via super.X
    public numeric function bumpTwice() {
        super.increment();
        super.increment();
        return super.get();
    }

    // Adds a property + a CFC-side override that wraps super
    public numeric function add(required numeric n) {
        super.add( n * 2 );
        return super.get();
    }

}
