component {
    // Calls expandPath with a leading-slash (webroot-relative) path from
    // inside a CFC living in a SUBDIRECTORY. The result must be identical
    // to the same call made at page level — leading-slash paths are
    // webroot/entry-relative and caller-independent, never resolved
    // against the calling CFC's own directory (GH #215).
    function probe( required string p ) {
        return expandPath( arguments.p );
    }
}
