/**
 * Probe for the metadata-annotation surface WireBox delegation reads:
 *   - a component-level custom annotation (`delegates="..."`)
 *   - property-level annotations that are bare (no value) AND arbitrary-named
 *     with values (delegateSuffix / delegateExcludes), interleaved.
 * No WireBox dependency — pure engine reflection via getMetadata /
 * getComponentMetadata.
 */
component delegates="Memory, >Cache" {

	// bare annotations: inject, delegate, delegatePrefix (no values)
	property name="memory" inject delegate delegatePrefix;

	// valued arbitrary annotations interleaved with a bare one
	property name="store" inject="Cache" delegate delegateSuffix="store" delegateExcludes="flush";

	function init(){
		return this;
	}

}
