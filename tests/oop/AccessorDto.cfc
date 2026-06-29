/**
 * accessors=true with NO explicit init() — Lucee/ACF generate an implicit
 * constructor that maps NAMED args (and an argumentCollection spread) onto the
 * declared properties. `flag` deliberately collides with a same-named method to
 * prove the implicit constructor never clobbers the method.
 */
component accessors=true {
	property name="api"  type="string"  default="/";
	property name="uri"  type="string"  default="/";
	property name="verb" type="string"  default="GET";
	property name="flag" type="boolean" default=false;

	public string function flag() {
		return "FLAG-METHOD";
	}
}
