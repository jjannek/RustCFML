<cfscript>
suiteBegin("Custom tags: attributeCollection");

control_attrs = {
    model: "current_record.full_name",
    placeholder: "Full name",
    class: "input w-full"
};
</cfscript>

<cfsavecontent variable="moduleOutput">
    <cfmodule template="customtags/showattrs.cfm" attributeCollection="#control_attrs#" model="override.model">
</cfsavecontent>

<cfscript>
assert("cfmodule merges attributeCollection and explicit attrs override", trim(moduleOutput), "override.model|Full name|input w-full");
assert("cfmodule attributeCollection source struct is not mutated", control_attrs.model, "current_record.full_name");

control_attrs = {
    model: "current_record.email",
    label: "Email",
    class: "input w-full"
};
</cfscript>

<cfsavecontent variable="prefixOutput">
    <cf_showattrs attributeCollection="#control_attrs#" model="override.email"></cf_showattrs>
</cfsavecontent>

<cfscript>
assert("cf_ custom tag merges attributeCollection and explicit attrs override", trim(prefixOutput), "override.email|Email|input w-full");
assert("cf_ custom tag attributeCollection source struct is not mutated", control_attrs.model, "current_record.email");

suiteEnd();
</cfscript>
