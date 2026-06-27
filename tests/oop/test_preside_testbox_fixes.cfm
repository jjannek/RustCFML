<cfscript>
suiteBegin("Preside TestBox suite fixes");

// ---------------------------------------------------------------------------
// FIX 1: an unknown datasource raises a CATCHABLE `database`-typed exception
//        (Lucee/ACF parity), not an uncatchable generic runtime error.
//        Preside's tests/Application.cfc probes for its datasource with
//        `dbinfo ... ` wrapped in `catch( database e )`; an uncatchable error
//        500'd the whole suite on its first request.
// ---------------------------------------------------------------------------
caughtAsDatabase = false;
caughtAtAll      = false;
try {
	queryExecute( "select 1", {}, { datasource = "no_such_datasource_xyz" } );
} catch ( database e ) {
	caughtAsDatabase = true;
	caughtAtAll      = true;
} catch ( any e ) {
	caughtAtAll = true;
}
assert("unknown datasource is catchable", caughtAtAll, true);
assert("unknown datasource throws type=database", caughtAsDatabase, true);

// ---------------------------------------------------------------------------
// FIX 2: getApplicationSettings() is an alias of getApplicationMetadata() and
//        must surface the live `mappings` table (TestBox's LuceeMappingHelper
//        does `getApplicationSettings().mappings.append( ... )`). Before the
//        fix it fell through to the pure builtin that returned only {name:""}.
// ---------------------------------------------------------------------------
appSettings = getApplicationSettings();
assert("getApplicationSettings returns a struct", isStruct( appSettings ), true);
assert("getApplicationSettings exposes mappings", structKeyExists( appSettings, "mappings" ), true);
assert("getApplicationSettings.mappings is a struct", isStruct( appSettings.mappings ), true);
// alias parity with getApplicationMetadata()
assert("getApplicationSettings == getApplicationMetadata (mappings key present in both)",
	structKeyExists( getApplicationMetadata(), "mappings" ), true);

// ---------------------------------------------------------------------------
// FIX 3: a CFC method whose name is a CASE-VARIANT of a BIF (`getMetaData`
//        vs the `getMetadata` BIF) must NOT leak into the global
//        user-functions table and shadow bare BIF calls program-wide.
//        Member dispatch (obj.getMetaData()) still routes to the method.
// ---------------------------------------------------------------------------
shadow = new PresideTestboxGetMetaDataShadow();

// Member dispatch still reaches the component's own method.
viaMember = shadow.getMetaData( "anything" );
assert("member dispatch reaches the CFC method", viaMember.iAmTheMethod ?: false, true);

// A BARE getMetadata( obj ) call must resolve to the BIF, returning real
// component metadata (a struct with a `name` key) — NOT the method's fake
// struct. This is the exact call TestBox's runners make to discover bundles.
meta = getMetadata( shadow );
assert("bare getMetadata() resolves to the BIF, not the shadowing method",
	structKeyExists( meta, "name" ) && !structKeyExists( meta, "iAmTheMethod" ), true);
assert("bare getMetadata() returns the real component name",
	meta.name contains "PresideTestboxGetMetaDataShadow", true);

suiteEnd();
</cfscript>
