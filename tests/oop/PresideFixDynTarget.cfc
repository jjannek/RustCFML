/**
 * Target for the computed-method dispatch test. Its methods read component
 * (`variables`) state set in init — so if a dynamic `obj[ name ]()` call runs
 * them against the caller's scope instead of this instance's, the read fails.
 */
component {
    public any function init() {
        _setState( "DYN-STATE" );
        return this;
    }
    public string function readState() {
        return _getState();
    }
    private string function _getState() {
        return _state;
    }
    private void function _setState( required string v ) {
        _state = arguments.v;
    }
}
