<cfscript>
suiteBegin("Javadoc & inline parameter annotations (WireBox DI)");

// Find a function struct by name within a metadata struct.
function findFn( required struct md, required string name ) {
	for ( fn in arguments.md.functions ) {
		if ( fn.name == arguments.name ) {
			return fn;
		}
	}
	return {};
}

// Find a parameter struct by name within a function metadata struct.
function findParam( required struct fn, required string name ) {
	for ( p in arguments.fn.parameters ) {
		if ( p.name == arguments.name ) {
			return p;
		}
	}
	return {};
}

obj = createObject( "component", "oop.JavadocParamInject" );

// --- getMetadata (instance form — the surface WireBox's getInheritedMetaData reads) ---
md     = getMetadata( obj );
initFn = findFn( md, "init" );

assertTrue( "init has parameters array", isArray( initFn.parameters ) );

cf = findParam( initFn, "configuredFeatures" );
assertTrue( "configuredFeatures param is a struct", isStruct( cf ) );
assert( "configuredFeatures.name", cf.name, "configuredFeatures" );
assert( "configuredFeatures.type", cf.type, "struct" );
assertTrue( "configuredFeatures.required is true", cf.required );
// The javadoc @configuredFeatures.inject annotation lands on the param.
assertTrue( "configuredFeatures carries inject annotation", structKeyExists( cf, "inject" ) );
assert( "configuredFeatures.inject value", cf.inject, "coldbox:setting:features" );

logger = findParam( initFn, "logger" );
assertTrue( "logger carries inject annotation", structKeyExists( logger, "inject" ) );
assert( "logger.inject value", logger.inject, "logbox:logger:{this}" );

// --- Inline per-parameter attribute form ---
confFn = findFn( md, "configure" );
dsn    = findParam( confFn, "dsn" );
assertTrue( "inline-attr param carries inject", structKeyExists( dsn, "inject" ) );
assert( "inline dsn.inject value", dsn.inject, "coldbox:setting:datasource" );

// --- getComponentMetadata returns the same parameter structs ---
gmd   = getComponentMetadata( "oop.JavadocParamInject" );
gInit = findFn( gmd, "init" );
gcf   = findParam( gInit, "configuredFeatures" );
assert( "getComponentMetadata param inject", gcf.inject, "coldbox:setting:features" );

suiteEnd();

suiteBegin("Javadoc annotation quoted-value stripping (Lucee parity)");

qmd = getMetadata( createObject( "component", "oop.JavadocQuotedAnnotations" ) );

// `@tablePrefix ""` -> empty string (NOT the literal `""`, NOT "true").
assertTrue( "tablePrefix annotation present", structKeyExists( qmd, "tablePrefix" ) );
assert( "tablePrefix quoted-empty -> empty string", qmd.tablePrefix, "" );
assert( "tablePrefix length is 0", len( qmd.tablePrefix ), 0 );

// Surrounding double / single quotes are stripped (one matching pair).
assert( "double-quoted value stripped", qmd.doubleQuoted, "hello world" );
assert( "single-quoted value stripped", qmd.singleQuoted, "sq value" );

// Unquoted values are unchanged.
assert( "bare unquoted value unchanged", qmd.bareValue, "plain" );

// A value-less annotation is boolean true.
assertTrue( "value-less annotation is true", qmd.boolFlag );

// Quote stripping also applies to per-parameter (dotted) annotations.
qInit = findFn( qmd, "init" );
qx    = findParam( qInit, "x" );
qy    = findParam( qInit, "y" );
assert( "param double-quoted inject stripped", qx.inject, "coldbox:setting:thing" );
assert( "param single-quoted inject stripped", qy.inject, "logbox:logger:{this}" );

suiteEnd();
</cfscript>
