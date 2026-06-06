<cfscript>
suiteBegin("Tags: cfhttp multipart");

// ============================================================
// Background
// ============================================================
// <cfhttp multipart="true"> must encode its form data as multipart/form-data
// (rather than application/x-www-form-urlencoded), and <cfhttpparam type="file">
// must attach a file part. On Lucee/ACF this is how a multipart upload is sent.
//
// On RustCFML the `multipart` attribute is dropped by the tag preprocessor (it
// is not among the attributes the cfhttp handler passes through), and the cfhttp
// builtin has no multipart path nor `type="file"` handling -- it only builds an
// application/x-www-form-urlencoded body from formfield params. So a multipart
// request is silently sent as urlencoded and any file param is discarded.
//
// This is a behavioral (not parse) gap, so -- like the other cfhttp round-trip
// tests -- it runs only when served (cgi.server_port present) and POSTs to a
// local target that echoes the inbound Content-Type. When run without a server
// it skips. The control assertion (a urlencoded POST whose form field arrives)
// proves the round-trip wiring; the multipart assertion is expected to fail on
// current upstream until multipart encoding is implemented.
//
// Why it matters for Moopa: code/moopa/lib/cloudflare_stream.cfc uploadCaptionVtt
// uploads WebVTT via <cfhttp ... multipart="true"> with <cfhttpparam type="file">.
// ============================================================

serverPort = structKeyExists(cgi, "server_port") ? trim(cgi.server_port) : "";
skip = serverPort == "" || serverPort == "0";

if (skip) {
    assertTrue("cfhttp multipart skipped (no cgi.server_port)", true);
} else {
    baseUrl = "http://127.0.0.1:" & serverPort;
    target = "/tests/tags/cfhttp_multipart_target.cfm";
    plainError = "";
    multipartError = "";
}
</cfscript>

<cfif NOT skip>
    <!--- control: a plain POST — form field round-trips as urlencoded (works today) --->
    <cftry>
        <cfhttp url="#baseUrl##target#" method="POST" result="plainResult">
            <cfhttpparam type="formfield" name="a" value="hello" />
        </cfhttp>
        <cfcatch type="any"><cfset plainError = cfcatch.message></cfcatch>
    </cftry>

    <!--- gap: multipart="true" must send multipart/form-data --->
    <cftry>
        <cfhttp url="#baseUrl##target#" method="POST" result="multipartResult" multipart="true">
            <cfhttpparam type="formfield" name="a" value="hello" />
        </cfhttp>
        <cfcatch type="any"><cfset multipartError = cfcatch.message></cfcatch>
    </cftry>

    <cfscript>
        assert("control: plain POST form field round-trips",
            (plainError == "" && structKeyExists(variables, "plainResult"))
                ? (findNoCase("a=hello", plainResult.filecontent) GT 0) : false,
            true);
        assert("cfhttp multipart=true sends multipart/form-data",
            (multipartError == "" && structKeyExists(variables, "multipartResult"))
                ? (findNoCase("multipart/form-data", multipartResult.filecontent) GT 0) : false,
            true);
    </cfscript>
</cfif>

<cfscript>
suiteEnd();
</cfscript>
