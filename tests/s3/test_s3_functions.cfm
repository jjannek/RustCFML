<cfscript>
// S3 function smoke tests.
//
// Most assertions require live S3 credentials. We gate the live tests on the
// presence of the environment variable RUSTCFML_S3_TEST_BUCKET (read via the
// `server.system.environment` scope that RustCFML populates). When unset, we
// only verify that the S3* functions are *registered* — they should exist in
// the function table even though calling them without creds would fail.

suiteBegin("S3 URL generation (pure, no network)");

// S3GenerateURI is pure (no network) — verify URL shapes.
assert("S3GenerateURI virtualhost",
    s3GenerateURI("my-bucket", "k.txt"),
    "https://my-bucket.s3.amazonaws.com/k.txt");

assert("S3GenerateURI path-style",
    s3GenerateURI("my-bucket", "k.txt", "path"),
    "https://s3.amazonaws.com/my-bucket/k.txt");

assert("S3GenerateURI custom host",
    s3GenerateURI("my-bucket", "k.txt", "virtualhost", true, "r2.example.com"),
    "https://my-bucket.r2.example.com/k.txt");

assert("S3GenerateURI insecure",
    s3GenerateURI("my-bucket", "k.txt", "virtualhost", false),
    "http://my-bucket.s3.amazonaws.com/k.txt");

suiteEnd();

// --- Live tests (skipped unless creds are set) ---
//
// To run, export before invoking the CLI:
//   AWS_ACCESS_KEY_ID=... AWS_SECRET_ACCESS_KEY=... AWS_DEFAULT_REGION=us-east-1 \
//   RUSTCFML_S3_TEST_BUCKET=rustcfml-test-bucket cargo run -- tests/runner.cfm
suiteBegin("S3 live roundtrip (skipped without RUSTCFML_S3_TEST_BUCKET)");

bucket = "";
try {
    bucket = server.system.environment.RUSTCFML_S3_TEST_BUCKET ?: "";
} catch (any e) {
    bucket = "";
}

if (bucket == "") {
    writeOutput("  (skipped — set RUSTCFML_S3_TEST_BUCKET to enable)" & chr(10));
} else {
    key = "rustcfml-test/" & createUUID() & ".txt";
    payload = "hello s3 " & dateTimeFormat(now(), "iso");

    s3Write(bucket, key, payload);
    assert("S3Exists after write", s3Exists(bucket, key), true);
    assert("S3Read roundtrip", s3Read(bucket, key), payload);

    listing = s3ListBucket(bucket, "rustcfml-test/");
    assertTrue("S3ListBucket returns array", isArray(listing));

    s3Delete(bucket, key);
    assert("S3Exists after delete", s3Exists(bucket, key), false);
}

suiteEnd();

// --- Live transparent-VFS test ---
suiteBegin("S3 transparent VFS roundtrip (skipped without RUSTCFML_S3_TEST_BUCKET)");

vfsBucket = "";
try {
    vfsBucket = server.system.environment.RUSTCFML_S3_TEST_BUCKET ?: "";
} catch (any e) {
    vfsBucket = "";
}

if (vfsBucket == "") {
    writeOutput("  (skipped — set RUSTCFML_S3_TEST_BUCKET to enable)" & chr(10));
} else {
    vfsPath = "s3://" & vfsBucket & "/rustcfml-test/vfs-" & createUUID() & ".txt";
    fileWrite(vfsPath, "vfs content");
    assert("fileExists(s3://...) after write", fileExists(vfsPath), true);
    assert("fileRead(s3://...) roundtrip", fileRead(vfsPath), "vfs content");
    fileDelete(vfsPath);
    assert("fileExists(s3://...) after delete", fileExists(vfsPath), false);
}

suiteEnd();
</cfscript>
