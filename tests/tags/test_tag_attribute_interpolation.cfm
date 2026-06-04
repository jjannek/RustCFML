<cfscript>
suiteBegin("Tag attribute string interpolation");
</cfscript>

<!---
    Lucee-compatible behavior: a quoted tag-attribute value evaluates #...#
    interpolation while preserving the literal text before, between, and after
    each interpolated segment. This must hold regardless of which tag the
    attribute belongs to.

    Moopa hit this with <cfthrow message="..."> and <cfargument default="...">
    that combine literal path/label text with interpolated variables. The local
    workaround rewrote every such string into explicit "&" concatenation.

    These tests pin the cross-tag behavior. Controls (cfset / cfparam) already
    pass and document the expected result; the cfthrow / cfargument cases are
    expected to fail on current upstream until tag-attribute interpolation is
    applied uniformly.
--->

<cfset app_name = "hub" />
<cfset field_name = "email" />
<cfset detail_part = "profiles" />
<cfset request.app_name = "hub" />

<!--- ============================================================
      CONTROLS: contexts where interpolation already works.
      ============================================================ --->
<cfset control_set = "/apps/#app_name#/#field_name#" />
<cfparam name="control_param" default="/apps/#app_name#/cfg" />

<cfscript>
assert("control: cfset interpolates literal + multiple segments",
    control_set, "/apps/hub/email");
assert("control: cfparam default interpolates literal + segment",
    control_param, "/apps/hub/cfg");
</cfscript>

<!--- ============================================================
      cfthrow message: literal text + interpolation.
      ============================================================ --->
<cfset msg_single = "" />
<cftry>
    <cfthrow message="#app_name#" />
    <cfcatch><cfset msg_single = cfcatch.message /></cfcatch>
</cftry>

<cfset msg_trailing = "" />
<cftry>
    <cfthrow message="APP_NAME '#app_name#' does not match an app directory at /apps/#app_name#." />
    <cfcatch><cfset msg_trailing = cfcatch.message /></cfcatch>
</cftry>

<cfset msg_member_fn = "" />
<cftry>
    <cfthrow message="route #request.app_name# took #ucase(detail_part)# ms" />
    <cfcatch><cfset msg_member_fn = cfcatch.message /></cfcatch>
</cftry>

<cfset msg_squote = "" />
<cftry>
    <cfthrow message="control '#app_name#' missing" />
    <cfcatch><cfset msg_squote = cfcatch.message /></cfcatch>
</cftry>

<cfscript>
assert("cfthrow message: bare single-segment interpolation",
    msg_single, "hub");
assert("cfthrow message: interpolation with trailing literal after segment",
    msg_trailing, "APP_NAME 'hub' does not match an app directory at /apps/hub.");
assert("cfthrow message: scoped member + function-call interpolation",
    msg_member_fn, "route hub took PROFILES ms");
// Single-quote-wrapped interpolation already works; documents the boundary.
assert("cfthrow message: single-quote-wrapped interpolation (current workaround)",
    msg_squote, "control 'hub' missing");
</cfscript>

<!--- ============================================================
      cfthrow type / detail: same rule applies to every attribute.
      ============================================================ --->
<cfset thrown_type = "" />
<cfset thrown_detail = "" />
<cftry>
    <cfthrow type="moopa.app.#app_name#" detail="failed for /apps/#app_name#/#field_name#" message="m" />
    <cfcatch>
        <cfset thrown_type = cfcatch.type />
        <cfset thrown_detail = cfcatch.detail />
    </cfcatch>
</cftry>

<cfscript>
assert("cfthrow type: interpolation in type attribute",
    thrown_type, "moopa.app.hub");
assert("cfthrow detail: interpolation in detail attribute",
    thrown_detail, "failed for /apps/hub/email");
</cfscript>

<!--- ============================================================
      cfargument default: interpolation in a function-signature attribute.
      ============================================================ --->
<cffunction name="buildPath" returntype="string" output="false">
    <cfargument name="path" type="string" default="/apps/#request.app_name#/routes" />
    <cfreturn arguments.path />
</cffunction>

<cfset arg_default = "" />
<cftry>
    <cfset arg_default = buildPath() />
    <cfcatch><cfset arg_default = "THROWN: " & cfcatch.message /></cfcatch>
</cftry>

<!--- ============================================================
      cffile: interpolation in a path attribute. Moopa reads package
      files via <cffile action="read" file="#local.filePath#">, i.e. a
      single-variable interpolated path. The file is created with the
      fileWrite() BIF (function-argument interpolation already works) so
      the assertion isolates the cffile attribute path specifically.
      ============================================================ --->
<cfset cffile_path = getTempDirectory() & "/rustcfml_attr_interp_" & createUUID() & ".txt" />
<cfset fileWrite(cffile_path, "payload-marker") />

<cfset cffile_read = "" />
<cftry>
    <cffile action="read" file="#cffile_path#" variable="cffile_contents" />
    <cfset cffile_read = cffile_contents />
    <cfcatch><cfset cffile_read = "THROWN: " & cfcatch.message /></cfcatch>
</cftry>
<cftry><cfset fileDelete(cffile_path) /><cfcatch></cfcatch></cftry>

<cfscript>
assert("cfargument default: interpolation in default attribute",
    arg_default, "/apps/hub/routes");
assert("cffile read: single-variable interpolation in file attribute",
    cffile_read, "payload-marker");

suiteEnd();
</cfscript>
