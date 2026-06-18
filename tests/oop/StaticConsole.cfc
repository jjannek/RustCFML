component {

    static {
        GREETING = "hello";
        COLORS = {
            reset : chr( 27 ) & "[0m",
            red   : chr( 27 ) & "[31m",
            green : chr( 27 ) & "[32m"
        };
        // Shared mutable counter — proves the static scope is initialised once
        // and shared across every instance of this type.
        count = 0;
    }

    function init() {
        return this;
    }

    function greet() {
        return static.GREETING;
    }

    function colorRed() {
        return static.COLORS.red;
    }

    // ConsoleUtil-style dynamic key access into a static struct.
    function colorByKey( required string key ) {
        return static.COLORS[ arguments.key ];
    }

    function bump() {
        static.count = static.count + 1;
        return static.count;
    }

    function getCount() {
        return static.count;
    }

    // Static function: callable on an instance and via `Component::fn()`,
    // reading the static scope without instance data.
    public static function wrap( required string style, required string text ) {
        return static.COLORS[ arguments.style ] & arguments.text & static.COLORS.reset;
    }

}
