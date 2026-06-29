<cfscript>
suiteBegin( "YAML — yamlDeserialize/yamlSerialize BIFs + snakeyaml shim" );

// yamlDeserialize/yamlSerialize are BoxLang BIFs (not Lucee core), and the
// org.yaml.snakeyaml.Yaml shim is RustCFML-specific (no JVM jar on Lucee), so
// gate the suite on RustCFML.
if ( isRustCFML() ) {

	yml = "name: Preside
version: 7
enabled: true
ratio: 1.5
tags:
  - cms
  - cfml
db:
  host: localhost
  port: 3306";

	// --- yamlDeserialize ---
	data = yamlDeserialize( yml );
	assert( "string scalar", data.name, "Preside" );
	assert( "integer scalar", data.version, 7 );
	assertTrue( "boolean scalar is a real boolean", isBoolean( data.enabled ) && data.enabled );
	assert( "float scalar", data.ratio, 1.5 );
	assertTrue( "sequence -> array", isArray( data.tags ) );
	assert( "array element", data.tags[ 2 ], "cfml" );
	assertTrue( "nested mapping -> struct", isStruct( data.db ) );
	assert( "nested value", data.db.port, 3306 );

	// --- yamlSerialize round-trips ---
	out = yamlSerialize( { a=1, b=[ 2, 3 ], c=true } );
	assertTrue( "serialize produces a string", isSimpleValue( out ) && len( out ) );
	rt = yamlDeserialize( out );
	assert( "round-trip scalar", rt.a, 1 );
	assert( "round-trip nested array", rt.b[ 2 ], 3 );
	assertTrue( "round-trip boolean", rt.c );

	// --- snakeyaml shim (legacy Preside cfflow YamlParser path) ---
	y      = createObject( "java", "org.yaml.snakeyaml.Yaml" );
	loaded = y.load( yml );
	assert( "shim load: scalar", loaded.name, "Preside" );
	assert( "shim load: nested", loaded.db.port, 3306 );
	assert( "shim load: array element", loaded.tags[ 1 ], "cms" );

}

suiteEnd();
</cfscript>
