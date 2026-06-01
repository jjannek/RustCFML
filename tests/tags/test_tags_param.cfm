<cfscript>suiteBegin("Tags: Param");</cfscript>

<!--- cfparam with default for undefined variable --->
<cfparam name="undefinedVar1" default="defaultValue">
<cfscript>assert("cfparam sets default", undefinedVar1, "defaultValue");</cfscript>

<!--- cfparam with numeric default --->
<cfparam name="undefinedNum" default="99">
<cfscript>assert("cfparam numeric default", undefinedNum, 99);</cfscript>

<!--- cfparam does not override existing variable --->
<cfset existingVar = "original">
<cfparam name="existingVar" default="overridden">
<cfscript>assert("cfparam no override", existingVar, "original");</cfscript>

<!--- cfparam with type="string" on valid string --->
<cfset validStr = "hello">
<cfparam name="validStr" type="string">
<cfscript>assert("cfparam type string valid", validStr, "hello");</cfscript>

<!--- cfparam with type="numeric" on valid number --->
<cfset validNum = 42>
<cfparam name="validNum" type="numeric">
<cfscript>assert("cfparam type numeric valid", validNum, 42);</cfscript>

<!--- cfparam with type="boolean" on valid boolean --->
<cfset validBool = true>
<cfparam name="validBool" type="boolean">
<cfscript>assertTrue("cfparam type boolean valid", validBool);</cfscript>

<!--- cfparam with default and type --->
<cfparam name="typedDefault" type="string" default="typed">
<cfscript>assert("cfparam default with type", typedDefault, "typed");</cfscript>

<!--- cfparam quoted defaults with spaces/operators remain literal strings --->
<cfset quotedDefaultError = "">
<cftry>
    <cfparam name="classDefault" default="px-5 py-5">
    <cfcatch type="any">
        <cfset quotedDefaultError = cfcatch.message>
    </cfcatch>
</cftry>
<cfscript>
    classDefaultValue = structKeyExists(variables, "classDefault") ? variables.classDefault : "";
    assert("cfparam quoted default error", quotedDefaultError, "");
    assert("cfparam quoted default with spaces and hyphens", classDefaultValue, "px-5 py-5");
</cfscript>

<!--- A quoted default is ALWAYS literal: operator- and function-call-looking
      strings are NOT evaluated; only #...# segments interpolate (Lucee parity). --->
<cfparam name="opDefault" default="1+1">
<cfscript>assert("cfparam quoted operator default is literal", opDefault, "1+1");</cfscript>

<cfparam name="fnDefault" default="now()">
<cfscript>assert("cfparam quoted function-call default is literal", fnDefault, "now()");</cfscript>

<cfparam name="dotDefault" default="a.b.c">
<cfscript>assert("cfparam quoted dotted default is literal", dotDefault, "a.b.c");</cfscript>

<cfparam name="interpDefault" default="sum-#1+1#">
<cfscript>assert("cfparam hash segment in default interpolates", interpDefault, "sum-2");</cfscript>


<!--- cfparam with type="array" default --->
<cfparam name="arrDefault" type="array" default="#[]#">
<cfscript>assertTrue("cfparam array default is array", isArray(arrDefault));</cfscript>

<cfscript>suiteEnd();</cfscript>
