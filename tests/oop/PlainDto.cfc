/**
 * No accessors and no init() — named construction args must NOT populate the
 * property; the declared default stands (Lucee/ACF parity).
 */
component {
	property name="api" type="string" default="/";

	public string function readApi() {
		return variables.api ?: "UNSET";
	}
}
