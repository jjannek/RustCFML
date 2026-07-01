component {
    this.name = "ds224_default_datasource_test";

    // A file-backed sqlite datasource persists across connections (unlike
    // :memory:, which is isolated per connection). A unique per-run file avoids
    // cross-run collisions. It is registered as a per-application datasource AND
    // set as the singular default, so a bare queryExecute (no datasource arg)
    // must resolve to it — the crux of GH #224.
    dbPath = getTempDirectory() & "rcfml_ds224_" & createUUID() & ".db";
    this.datasources = {
        "appds" : { driver: "sqlite", database: dbPath }
    };
    this.datasource = "appds";
}
