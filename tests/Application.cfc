component {

    this.name = "RustCFMLTests";

    // Session management on so session-scope tests can run (see
    // tests/core/test_session_scope_persist.cfm). Harmless for other tests.
    this.sessionManagement = true;
    this.sessionTimeout = createTimeSpan( 0, 0, 30, 0 );

    // Map "oop" to the tests/oop/ directory so createObject("component", "oop.Greeter") resolves
    this.mappings["/oop"] = getDirectoryFromPath(getCurrentTemplatePath()) & "oop/";

    // Map "tags" for any tag-based test includes
    this.mappings["/tags"] = getDirectoryFromPath(getCurrentTemplatePath()) & "tags/";

    // Distinct mapping name (NOT a real webroot subdirectory) used by
    // tags/test_mapping_include.cfm to isolate this.mappings-based cfinclude
    // resolution: a `/wheelsmapprobe/...` include can ONLY be found via this
    // mapping, never by webroot-relative path resolution. Points at tests/tags/.
    this.mappings["/wheelsmapprobe"] = getDirectoryFromPath(getCurrentTemplatePath()) & "tags/";

    // Per-application datasources (Lucee/BoxLang parity). Scoped to THIS
    // application and resolved by cfquery/queryExecute ahead of the global
    // cfconfig registry. Exercised by tests/config/test_app_datasources.cfm.
    //   rc_app_mem      — valid in-memory sqlite (struct form)
    //   rc_app_mem_str  — same, via the bare connection-string form
    //   rc_app_bad      — deliberately unreachable; used to PROVE the name is
    //                     resolved through this.datasources (a non-sqlite driver
    //                     must throw, not silently fall through to the sqlite
    //                     catch-all that an unresolved bare name would hit)
    this.datasources = {
        "rc_app_mem"     : { driver: "sqlite", database: ":memory:" },
        "rc_app_mem_str" : "sqlite://:memory:",
        "rc_app_bad"     : { driver: "postgresql", host: "127.0.0.1", port: "1", database: "definitely_absent", username: "x", password: "y" }
    };

}
