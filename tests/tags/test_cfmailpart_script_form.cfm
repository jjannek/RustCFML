<cfscript>
suiteBegin("Tags: cfmailpart script-statement form parses inside a script cfmail block");

// Background: inside a script cfmail(){} block, cfmailpart is callable as a
// script statement to declare a multipart (text + html) body — the same as the
// <cfmailpart> tag inside <cfmail>. Lucee/Adobe CF/BoxLang PARSE it (the
// function below compiles and is defined). RustCFML 0.161.0 failed to PARSE the
// script-form cfmailpart ("Expected RBrace, found Semicolon"); fixed so it
// lowers to the same runtime __cfmail_parts array the <cfmailpart> tag uses.
//
// Why it matters for Wheels: vendor/wheels/Global.cfc $mail() emits
// cfmailpart(attributeCollection=local.i){...} and cfmailparam(attributeCollection=local.i)
// in script form, so every multipart Wheels email (the default text+html
// mailer output) failed to COMPILE on RustCFML.

function cmpsfBuildMultipart() {
    cfmail(to = "a@example.com", from = "b@example.com", subject = "parts test") {
        cfmailpart(type = "text") { writeOutput("text-part-body"); }
        cfmailpart(type = "html") { writeOutput("<b>html-part-body</b>"); }
    }
    return "defined";
}

// Reaching here means the file PARSED (it previously parse-aborted before this).
assertTrue("script-form cfmailpart inside cfmail compiles (function is defined)",
    isCustomFunction(cmpsfBuildMultipart));

// cfmailparam script-statement form must also parse inside a script cfmail block.
function cmpsfBuildWithParam() {
    cfmail(to = "a@example.com", from = "b@example.com", subject = "param test") {
        cfmailparam(name = "Reply-To", value = "c@example.com");
        cfmailpart(type = "html") { writeOutput("<i>body</i>"); }
    }
    return "defined";
}
assertTrue("script-form cfmailparam inside cfmail compiles (function is defined)",
    isCustomFunction(cmpsfBuildWithParam));

suiteEnd();
</cfscript>
