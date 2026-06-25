<cfscript>
// Emit-from-anywhere: an ordinary page push to a connected channel/room.
// Hit over HTTP while a client is connected to /ws/echo to verify wsPublish
// reaches live sockets.
wsPublish( channel="/echo", event="announcement", data={ text=url.msg ?: "hi" } );
writeOutput( "published" );
</cfscript>
