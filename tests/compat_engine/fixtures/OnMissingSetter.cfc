component accessors="true" {
    property name="color" type="string";
    function onMissingMethod(missingMethodName, missingMethodArguments) {
        return "OMM:" & missingMethodName;
    }
}
