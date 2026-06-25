/**
 * Resumability test channel (Phase 2, P12). URL `/ws/history`.
 * `history="50"` opts the channel into in-memory replay: the last 50
 * channel-wide frames are retained, and a client reconnecting with
 * `?lastEventId=<id>` is replayed everything it missed before live traffic
 * resumes. The `on="say"` handler broadcasts a `said` event to the whole
 * channel (io().emit) — those broadcasts are what gets retained.
 */
component socket="/history" encoding="json" history="50" {

    function broadcastSay( socket, data ) on="say" {
        io().emit( "said", data );
    }
}
