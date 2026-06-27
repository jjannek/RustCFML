/**
 * Regression fixture for the Preside TestBox bundle-discovery bug.
 *
 * Declares a component method spelled `getMetaData` (capital D) — a
 * case-variant of the `getMetadata` BIF. CFML identifiers are
 * case-insensitive, so this method must NOT leak into the global
 * user-functions table and steal bare `getMetadata( obj )` calls
 * elsewhere in the program. (Preside's DocumentMetadataService declares
 * exactly this signature; before the fix it silently shadowed every bare
 * getMetadata() call after it was instantiated, returning this fake
 * struct and breaking TestBox bundle discovery.)
 */
component {
	public struct function getMetaData( required any fileContent ) {
		return { "fake" = true, "iAmTheMethod" = true };
	}
}
