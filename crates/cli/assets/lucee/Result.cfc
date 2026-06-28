/**
 * Engine-bundled compatibility shim for Lucee's org.lucee.cfml.Result — the
 * object returned by Query.execute(). getResult() yields the query (or array/
 * struct returntype), getPrefix() yields the cfquery result metadata struct.
 */
component output=false {

	public any function init() {
		variables.result = "";
		variables.prefix = {};
		return this;
	}

	public any function getResult() {
		return variables.result ?: "";
	}

	public any function setResult( required any result ) {
		variables.result = arguments.result;
		return this;
	}

	public struct function getPrefix() {
		return variables.prefix ?: {};
	}

	public any function setPrefix( required struct prefix ) {
		variables.prefix = arguments.prefix;
		return this;
	}
}
