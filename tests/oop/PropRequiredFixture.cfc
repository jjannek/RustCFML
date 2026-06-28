/**
 * Fixture for the `required="false"` property-metadata regression: an explicit
 * `required="false"` must be PRESERVED in getMetadata().properties (Lucee keeps
 * `required:"false"`), not dropped. Previously RustCFML collapsed `required` to a
 * bool field that codegen only emitted when true, so `required="false"` vanished
 * — breaking Preside's PresideObjectReaderTest property comparison.
 */
component {
	property name="numprop" type="numeric" control="spinner" required="false" minValue="1" maxValue="10";
	property name="reqprop" type="string" required="true";
	property name="plainprop" type="string";
	// `default="…"` must surface verbatim in getMetadata().properties (Lucee
	// keeps it) — frameworks read it to auto-populate unprovided fields (e.g.
	// Preside insertData defaults). Both the plain literal and Preside's
	// prefixed forms (`cfml:`, `method:`) must be stored as the source string.
	property name="litdefault"    type="string"  default="hello default";
	property name="cfmldefault"   type="date"    default="cfml:Now()";
	property name="methoddefault" type="numeric" default="method:CalcIt";
}
