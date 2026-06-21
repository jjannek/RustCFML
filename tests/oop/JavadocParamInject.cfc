/**
 * A component-level hint line.
 * @author RustCFML
 */
component singleton=true {

	/**
	 * Constructor hint text.
	 * @configuredFeatures.inject coldbox:setting:features
	 * @logger.inject logbox:logger:{this}
	 */
	public any function init( required struct configuredFeatures, any logger ) {
		variables.cf = arguments.configuredFeatures;
		return this;
	}

	// Inline per-parameter attribute form (no javadoc): annotation rides on the param.
	public any function configure( required string dsn inject="coldbox:setting:datasource" ) {
		return arguments.dsn;
	}
}
