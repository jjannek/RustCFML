<cfscript>
suiteBegin( "BCrypt — BCryptHash/BCryptVerify BIFs + jBCrypt shim" );

// BCryptHash/BCryptVerify are a Lucee crypto-EXTENSION (not core), and the
// org.mindrot.jbcrypt.BCrypt shim is RustCFML-specific (no JVM jar on Lucee), so
// gate the whole suite on RustCFML. Behaviour mirrors the Lucee crypto ext.
if ( isRustCFML() ) {

	// --- BIFs ---
	hash = bcryptHash( "correct horse", 10 );
	assert( "bcryptHash emits a $2a$ bcrypt hash", left( hash, 4 ), "$2a$" );
	assertTrue( "bcryptVerify matches the right password", bcryptVerify( "correct horse", hash ) );
	assertFalse( "bcryptVerify rejects the wrong password", bcryptVerify( "battery staple", hash ) );

	// Each call salts independently → different hashes, both verify.
	hash2 = bcryptHash( "correct horse" ); // default cost 10
	assertTrue( "independently-salted hash still verifies", bcryptVerify( "correct horse", hash2 ) );
	assert( "salting makes each hash unique", hash == hash2, false );

	// Malformed hash returns false (throwOnError defaults to false).
	assertFalse( "bcryptVerify on a malformed hash returns false", bcryptVerify( "x", "not-a-hash" ) );

	// Cost is honoured (encoded as the 2nd field).
	assert( "cost factor encoded in hash", listGetAt( bcryptHash( "x", 12 ), 2, "$" ), "12" );

	// --- jBCrypt shim (legacy Preside BCryptService path) ---
	jb   = createObject( "java", "org.mindrot.jbcrypt.BCrypt" );
	salt = jb.genSalt( javaCast( "int", 10 ) );
	h    = jb.hashpw( "s3cr3t", salt );
	assert( "shim hashpw emits a $2a$ hash", left( h, 4 ), "$2a$" );
	assertTrue( "shim checkpw matches", jb.checkpw( "s3cr3t", h ) );
	assertFalse( "shim checkpw rejects wrong password", jb.checkpw( "nope", h ) );

	// Interop: the BIF verifies a shim-produced hash and vice versa.
	assertTrue( "BCryptVerify verifies a shim-made hash", bcryptVerify( "s3cr3t", h ) );
	assertTrue( "shim checkpw verifies a BCryptHash-made hash", jb.checkpw( "correct horse", hash ) );

}

suiteEnd();
</cfscript>
