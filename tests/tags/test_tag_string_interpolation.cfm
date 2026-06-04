<cfscript>
suiteBegin("Tag string interpolation");
</cfscript>

<cfset attributes = { model_record: "current_record", table_name: "moo_profile" }>
<cfset field_name = "email">
<cfset model = "#attributes.model_record#.#field_name#">
<cfset control_id = "#attributes.table_name#_#field_name#">

<cfscript>
assert("tag-form cfset interpolates nested struct value in quoted string", model, "current_record.email");
assert("tag-form cfset interpolates multiple values in quoted string", control_id, "moo_profile_email");

route = { url: "/sysadmin/profiles" };
endpoint = "save";
start_time = getTickCount() - 7;
message = "Security check for route #route.url# and endpoint #endpoint# took #getTickCount() - start_time#ms";

assert("script string interpolation remains available as control", find("Security check for route /sysadmin/profiles and endpoint save took ", message) EQ 1, true);

suiteEnd();
</cfscript>
