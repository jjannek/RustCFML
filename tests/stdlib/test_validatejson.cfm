<cfscript>
suiteBegin( "validateJSON BIF + ca.vanmulligen json-schema shim" );

// validateJSON is a Lucee JSON-extension BIF (absent on the test box) and the
// ca.vanmulligen.json.schema.Validator shim is RustCFML-specific, so gate on
// RustCFML.
if ( isRustCFML() ) {

	schema = '{"type":"object","required":["name"],"properties":{"name":{"type":"string"},"age":{"type":"integer"}}}';

	// --- BIF: empty error array == valid ---
	valid = validateJSON( '{"name":"Bob","age":30}', schema );
	assertTrue( "valid doc -> empty error array", isArray( valid ) && arrayLen( valid ) == 0 );

	bad = validateJSON( '{"age":"not-an-int"}', schema );
	assertTrue( "invalid doc -> non-empty error array", isArray( bad ) && arrayLen( bad ) >= 1 );
	assertTrue( "error struct has a message", len( bad[ 1 ].message ?: "" ) > 0 );

	// throwOnError=true throws on a violation.
	assertThrows( "throwOnError throws on invalid", function(){
		validateJSON( '{}', schema, true );
	} );

	// --- ca.vanmulligen shim: init(schema, baseUri) then isValid(json) ---
	v   = createObject( "java", "ca.vanmulligen.json.schema.Validator" ).init( schema, "" );
	okRes  = deserializeJSON( v.isValid( '{"name":"Bob"}' ) );
	badRes = deserializeJSON( v.isValid( '{"age":"x"}' ) );
	assertTrue( "shim isValid: valid doc -> valid=true", okRes.valid );
	assertFalse( "shim isValid: invalid doc -> valid=false", badRes.valid );
	assertTrue( "shim invalid result carries an error message", len( badRes.error.message ?: "" ) > 0 );
	assert( "shim invalid result violationCount", badRes.error.violationCount >= 1, true );

}

suiteEnd();
</cfscript>
