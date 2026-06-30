<cfscript>
// Mirrors Preside's i18n ResourceBundleService._propertiesFileToStruct(): read a
// Java `.properties` file via FileInputStream -> InputStreamReader ->
// PropertyResourceBundle, then iterate getKeys() and handleGetObject(key).
// RustCFML has no JVM, so this chain is a pure-data shim (read+parse). The whole
// suite is RustCFML-gated (no JVM on the cross-engine Lucee box, where these are
// real classes and would behave natively anyway).
if ( !isRustCFML() ) {
    suiteBegin( "Java Shims: PropertyResourceBundle (skipped - not RustCFML)" );
    assertTrue( "skipped on non-RustCFML engine", true );
    suiteEnd();
} else {
    suiteBegin( "Java Shims: .properties reader chain (FileInputStream/PropertyResourceBundle)" );

    propsFile = getDirectoryFromPath( getCurrentTemplatePath() ) & "test_bundle.properties";

    fis = createObject( "java", "java.io.FileInputStream" ).init( propsFile );
    fir = createObject( "java", "java.io.InputStreamReader" ).init( fis, "UTF-8" );
    prb = createObject( "java", "java.util.PropertyResourceBundle" ).init( fir );

    // Iterate exactly like Preside does, building a struct.
    keys = prb.getKeys();
    bundle = {};
    while ( keys.hasMoreElements() ) {
        k = keys.nextElement();
        bundle[ k ] = prb.handleGetObject( k );
    }
    fis.close();

    assert( "= separator value", bundle[ "greeting" ], "Hello" );
    assert( "placeholder value preserved", bundle[ "greeting.name" ], "Hello, {1}" );
    assert( ": separator value", bundle[ "with.colon" ], "colon value" );
    assert( "whitespace around = trimmed", bundle[ "spaced.key" ], "trimmed value" );
    assert( "line continuation joined", bundle[ "continued" ], "line one and line two" );
    assert( "utf-8 value decoded", bundle[ "unicode" ], "café" );
    assert( "empty value -> empty string", bundle[ "empty.value" ], "" );
    assert( "\t escape decoded", bundle[ "tab.value" ], "a" & chr(9) & "b" );

    // Comment lines (## and !) are not keys.
    assertFalse( "no key from comment line", structKeyExists( bundle, "sample resource bundle" ) );

    // handleGetObject for a direct lookup matches the iterated value.
    assert( "direct handleGetObject", prb.handleGetObject( "greeting" ), "Hello" );

    suiteEnd();
}
</cfscript>
