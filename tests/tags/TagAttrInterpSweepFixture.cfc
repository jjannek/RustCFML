<!---
    Gap fixture: a sweep of tag forms whose quoted attributes carry MIXED
    literal+#expr# interpolation (e.g. url="/page?id=#id#"). These arms routed
    attribute values through strip_hashes, which only handled a value that is
    entirely one #expr#; a mixed value emitted literal "#...#" into the generated
    script and failed to PARSE (surfacing at createObject() time as "Could not
    find the component"). Every tag is behind <cfif false> so it is compiled but
    never executed — this pins PARSE-time handling, not delivery.
--->
<cfcomponent output="false">
    <cffunction name="run" returntype="string" output="false">
        <cfargument name="id"   default="42" />
        <cfargument name="path" default="/tmp" />
        <cfif false>
            <cfheader name="X-Req-#arguments.id#" value="v-#arguments.id#" />
            <cfcontent type="text/#arguments.id#" />
            <cflocation url="/page?id=#arguments.id#" addtoken="false" />
            <cfcookie name="sess_#arguments.id#" value="tok-#arguments.id#" />
            <cfsetting requesttimeout="#arguments.id#" />
            <cfcache action="get" directory="#arguments.path#/cache" />
            <cfdirectory action="list" directory="#arguments.path#/sub" name="local.qd" />
            <cfzip action="zip" file="#arguments.path#/out.zip" source="#arguments.path#/src" />
            <cflock name="lock-#arguments.id#" timeout="10" type="exclusive">
                <cfset local.noop = 1 />
            </cflock>
            <cfthread name="t-#arguments.id#" action="run" label="job-#arguments.id#">
                <cfset thread.done = true />
            </cfthread>
        </cfif>
        <cfreturn "ok" />
    </cffunction>
</cfcomponent>
