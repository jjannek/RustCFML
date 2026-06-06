<cfsetting enablecfoutputonly="true" />
<!---
    Target for test_tags_cfhttp_multipart.cfm. Echoes the inbound request's
    Content-Type and the received form field so the caller can assert how the
    cfhttp request was encoded (multipart/form-data vs urlencoded).
--->
<cfoutput>ct=#cgi.content_type ?: ""#;a=#form.a ?: "(missing)"#</cfoutput>
