component {
    function onMissingMethod(missingMethodName, missingMethodArguments) {
        return "missing:" & arguments.missingMethodName;
    }
}
