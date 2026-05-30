component {
    property name="x" type="numeric";
    property name="y" type="numeric";

    function init( x, y ) {
        this.x = arguments.x;
        this.y = arguments.y;
        return this;
    }

    // Non-trivial method: reads two instance properties via this.X — exercises
    // GetProperty / LoadLocalProperty, the T1.3 + T3.2 hot path.
    function distSq() {
        return ( this.x * this.x ) + ( this.y * this.y );
    }
}
