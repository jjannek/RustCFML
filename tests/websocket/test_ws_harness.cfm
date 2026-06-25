<cfscript>
suiteBegin("WebSocket harness (connection-free)");

// emit-from-anywhere: wsPublish records into the per-VM broadcast log so
// realtime logic is testable with no live socket (design principle P14).
// assertBroadcast(channel, event[, predicate]) inspects that log.

// Named-arg form (the canonical call shape).
wsPublish( channel="/chat", event="message", data={ from="alice", text="hi" } );
assertTrue( "broadcast recorded for channel+event", assertBroadcast( "/chat", "message" ) );
assertFalse( "no broadcast for a different event", assertBroadcast( "/chat", "ping" ) );
assertFalse( "no broadcast for a different channel", assertBroadcast( "/other", "message" ) );

// Predicate closure receives the data payload.
assertTrue(
    "predicate matches payload",
    assertBroadcast( "/chat", "message", function( d ) { return d.text == "hi"; } )
);
assertFalse(
    "predicate rejects non-matching payload",
    assertBroadcast( "/chat", "message", function( d ) { return d.text == "bye"; } )
);

// Positional form also works.
wsPublish( "/room", "joined", "user-42" );
assertTrue( "positional wsPublish recorded", assertBroadcast( "/room", "joined" ) );

// Channel-only assertion (no event filter).
assertTrue( "channel-only match", assertBroadcast( "/chat" ) );

suiteEnd();
</cfscript>
