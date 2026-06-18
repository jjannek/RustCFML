component extends="oop.StaticConsole" {

    static {
        KID = "kid-only";
    }

    function init() {
        return this;
    }

    // Reads a static member declared on the PARENT — inherited static scope.
    function inheritedGreeting() {
        return static.GREETING;
    }

    function ownValue() {
        return static.KID;
    }

}
