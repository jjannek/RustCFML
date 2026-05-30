component {
    variables.val = 0;
    function init(numeric v = 0) { variables.val = arguments.v; return this; }
    function getVal() { return variables.val; }
    function setVal(required numeric v) { variables.val = arguments.v; }
}
