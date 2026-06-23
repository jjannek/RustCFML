<cfscript>
suiteBegin("JWT functions (JwtSign / JwtVerify / JwtDecode)");

// These are RustCFML's native JWT BIFs (Lucee crypto-extension names). Guard with
// isRustCFML() since a cross-engine Lucee run may not have the Cryptography
// Extension installed. RFC 7519 / HMAC compliance is proven by verifying the
// canonical jwt.io HS256 test vector below.
if (isRustCFML()) {

	// 1) Canonical jwt.io HS256 vector — verifies against a token minted elsewhere.
	knownToken = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
	claims = JwtVerify(knownToken, "your-256-bit-secret");
	assert("known vector sub", claims.sub, "1234567890");
	assert("known vector name", claims.name, "John Doe");
	assert("known vector iat", claims.iat, 1516239022);

	// 2) JwtDecode returns claims without verifying (wrong key still decodes).
	assert("decode without verify", JwtDecode(knownToken).sub, "1234567890");

	// 3) Round-trip HS256.
	tok = JwtSign({sub: "42", role: "admin"}, "secret123");
	assert("token has 3 segments", ListLen(tok, "."), 3);
	rt = JwtVerify(tok, "secret123");
	assert("roundtrip sub", rt.sub, "42");
	assert("roundtrip role", rt.role, "admin");

	// 4) HS384 / HS512.
	assert("HS384 roundtrip", JwtVerify(JwtSign({a: 1}, "k", "HS384"), "k").a, 1);
	assert("HS512 roundtrip", JwtVerify(JwtSign({a: 2}, "k", "HS512"), "k").a, 2);

	// 5) Wrong key is rejected.
	assertThrows("wrong key throws", function() { JwtVerify(tok, "wrongkey"); });

	// 6) Tampered token is rejected.
	assertThrows("tampered token throws", function() { JwtVerify(knownToken & "x", "your-256-bit-secret"); });

	// 7) expiresIn adds exp; an already-expired token is rejected.
	expired = JwtSign({u: "x"}, "k", "HS256", -10);
	assertThrows("expired token throws", function() { JwtVerify(expired, "k"); });

	// 8) A fresh expiresIn token verifies and carries exp.
	fresh = JwtSign({u: "y"}, "k", "HS256", 3600);
	freshClaims = JwtVerify(fresh, "k");
	assert("expiresIn sets u", freshClaims.u, "y");
	assertTrue("expiresIn sets exp", StructKeyExists(freshClaims, "exp"));

	// 9) Unsupported asymmetric algorithm fails loudly (not silently).
	assertThrows("RS256 unsupported throws", function() { JwtSign({a: 1}, "k", "RS256"); });

	// 10) Algorithm-substitution attempt rejected when an expected alg is pinned.
	assertThrows("alg mismatch throws", function() { JwtVerify(tok, "secret123", "HS512"); });
}

suiteEnd();
</cfscript>
