/**
 * Component-level javadoc annotations with quoted values. Lucee strips a single
 * matching pair of surrounding quotes; `@k ""` is an empty string (not "true"),
 * while a bare `@k` with no value is boolean true. (Preside's system objects use
 * `@tablePrefix ""` — keeping the quotes produced malformed table names.)
 *
 * @tablePrefix ""
 * @doubleQuoted "hello world"
 * @singleQuoted 'sq value'
 * @bareValue plain
 * @boolFlag
 */
component {

	/**
	 * @x.inject "coldbox:setting:thing"
	 * @y.inject 'logbox:logger:{this}'
	 */
	public any function init( required string x, any y ) {
		return this;
	}

}
