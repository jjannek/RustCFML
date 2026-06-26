<cfscript>
/**
 * Emit-from-anywhere demo (design principle P3): an ordinary HTTP page — no
 * socket handle, no channel CFC instance — pushes a frame to every connected
 * client. A cfthread, scheduled task, or queue worker would do the same.
 *
 * GET /broadcast.cfm?text=Hello&room=lobby
 */
param name="url.text" default="Server announcement";
param name="url.room" default="lobby";

// The flat BIF form — channel/event/data by name.
wsPublish(
      channel = "/demo"
    , event   = "system"
    , data    = { text = "📣 " & url.text }
    , to       = url.room          // omit `to` to hit the whole channel
);

// The fluent form does the same thing:
//   io( "/demo" ).to( url.room ).emit( "system", { text = url.text } );

writeOutput( "published to /demo → room '" & encodeForHtml( url.room ) & "'" );
</cfscript>
