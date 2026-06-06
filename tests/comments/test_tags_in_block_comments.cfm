<cfscript>
suiteBegin("Comments: tags inside block comments");

// ============================================================
// Issue #69
// ============================================================
// A /* ... */ block comment in a SCRIPT-component body is opaque: literal
// CFML tags inside it (<cfset>, <cfoutput>) are documentation, not markup.
// Lucee, Adobe CF, and BoxLang ignore the comment interior and compile the
// component normally. RustCFML previously lexed and EXECUTED the tags inside
// the comment (the file flipped to template-echo mode), so the component
// failed to load. Real-world: Wheels' Test.cfc documents <cfset>/<cfoutput>
// usage inside a /* */ doc comment.
// ============================================================

obj = createObject("component", "comments.BlockCommentTags");
assert("script component with tags in a /* */ doc comment loads normally",
    obj.ping(), "pong");

// The documented tags must not have executed as a side effect.
md = getMetadata(obj);
assert("component identity intact (not mangled to template echo)",
    listLast(md.name, "."), "BlockCommentTags");

// Inline (page-level) block comment with tags is suppressed too — this path
// already worked; pinned as a regression guard.
/* <cfset blockCommentSideEffect = 99> <cfoutput>x</cfoutput> */
assertFalse("inline block-comment tags do not execute",
    isDefined("blockCommentSideEffect"));

suiteEnd();
</cfscript>
