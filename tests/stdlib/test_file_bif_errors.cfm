<cfscript>
suiteBegin( "file BIFs — getFileInfo / fileRead* missing-file semantics" );

tmp     = getTempDirectory();
sep     = right( tmp, 1 ) == "/" || right( tmp, 1 ) == "\" ? "" : "/";
file    = tmp & sep & "rcfml_fstest_" & createUUID() & ".txt";
missing = tmp & sep & "rcfml_nope_"   & createUUID() & ".txt";

fileWrite( file, "hello world test!!" ); // 18 bytes

// getFileInfo returns a numeric size and a date-typed lastmodified.
info = getFileInfo( file );
assert( "getFileInfo size", info.size, 18 );
assertTrue( "getFileInfo lastmodified is a date", IsDate( info.lastmodified ) );

// getFileInfo on a missing file throws a 'does not exist' message (callers
// branch on it).
getInfoMsgOk = false;
try {
	getFileInfo( missing );
} catch( any e ) {
	getInfoMsgOk = ( e.message contains "does not exist" );
}
assertTrue( "getFileInfo missing -> 'does not exist' message", getInfoMsgOk );

// fileRead / fileReadBinary on a missing file throw a FileNotFoundException-typed
// error (Preside's storage provider branches on e.type).
readTypeOk = false;
try {
	fileReadBinary( missing );
} catch( any e ) {
	readTypeOk = ( e.type contains "FileNotFoundException" );
}
assertTrue( "fileReadBinary missing -> FileNotFoundException type", readTypeOk );

// fileRead on a missing file throws (Lucee surfaces this as an `expression`
// error, not FileNotFoundException — assert the throw, not the type).
assertThrows( "fileRead missing throws", function(){
	fileRead( missing );
} );

fileDelete( file );

suiteEnd();
</cfscript>
