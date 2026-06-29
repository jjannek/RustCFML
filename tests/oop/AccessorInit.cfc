/**
 * accessors=true WITH an explicit init() — the explicit constructor is solely
 * responsible; the implicit property population must NOT run.
 */
component accessors=true {
	property name="api" type="string" default="/";

	public any function init() {
		variables.api = "from-init";
		return this;
	}
}
