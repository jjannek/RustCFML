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

	// --- trailing-comma tolerance (Preside schema files carry trailing commas;
	//     serde_json is strict, the Java validators Preside targets are lenient) ---
	// Trailing comma in the SCHEMA string.
	tcSchema = '{"type":"object","properties":{"name":{"type":"string"},}}';
	tcValid  = validateJSON( '{"name":"Bob"}', tcSchema );
	assertTrue( "trailing comma in schema tolerated -> valid", isArray( tcValid ) && arrayLen( tcValid ) == 0 );

	// Trailing comma in the INSTANCE string.
	tcInst = validateJSON( '{"name":"Bob",}', schema );
	assertTrue( "trailing comma in instance tolerated -> valid", isArray( tcInst ) && arrayLen( tcInst ) == 0 );

	// Comma inside a string literal must NOT be stripped.
	strComma = validateJSON( '{"name":"a,}"}', schema );
	assertTrue( "comma inside string literal preserved -> valid", isArray( strComma ) && arrayLen( strComma ) == 0 );

	// --- cross-file $ref where the referenced file has a trailing comma
	//     (mirrors Preside webflow.schema.json -> webflow.init.schema.json) ---
	schemaDir  = getDirectoryFromPath( getCurrentTemplatePath() ) & "jsonschema/";
	rootSchema = fileRead( schemaDir & "root.schema.json" );
	baseUri    = "file://" & schemaDir;
	refValid   = validateJSON( '{"name":"Bob","child":{"flag":true}}', rootSchema, false, baseUri );
	assertTrue( "$ref'd file with trailing comma resolves -> valid", isArray( refValid ) && arrayLen( refValid ) == 0 );

}

suiteEnd();
</cfscript>
