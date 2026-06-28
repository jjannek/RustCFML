/**
 * Stand-in for Preside's PresideObjectService: the real target the decorator
 * proxies missing methods to.
 */
component {
	public any function init() { return this; }

	public any function insertData( required string objectName, string data = "" ) {
		return "INSERTED obj=" & arguments.objectName & " data=" & arguments.data;
	}
}
