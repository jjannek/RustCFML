component {

    this.name = "RustCFMLTests";

    // Map "oop" to the tests/oop/ directory so createObject("component", "oop.Greeter") resolves
    this.mappings["/oop"] = getDirectoryFromPath(getCurrentTemplatePath()) & "oop/";

    // Map "tags" for any tag-based test includes
    this.mappings["/tags"] = getDirectoryFromPath(getCurrentTemplatePath()) & "tags/";

    // Distinct mapping name (NOT a real webroot subdirectory) used by
    // tags/test_mapping_include.cfm to isolate this.mappings-based cfinclude
    // resolution: a `/wheelsmapprobe/...` include can ONLY be found via this
    // mapping, never by webroot-relative path resolution. Points at tests/tags/.
    this.mappings["/wheelsmapprobe"] = getDirectoryFromPath(getCurrentTemplatePath()) & "tags/";

}
