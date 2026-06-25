//! socket.io transport (Phase 3, native-only).
//!
//! Mounts a [`socketioxide`] tower layer on the existing axum router so a stock
//! socket.io client (Engine.IO v4: namespaces, acks, binary, polling↔ws
//! fallback) can talk to the very same channel CFCs the raw-WS driver serves.
//! socketioxide is *only* the transport — every fan-out (`wsPublish`, `io()`,
//! presence, broadcast, the distributed `Broker`) still funnels through our own
//! [`WebSocketRegistry`]. One socket.io connection therefore:
//!
//!   * resolves its namespace (`/chat`) to a channel CFC (shared
//!     [`crate::websocket::resolve_channel`]),
//!   * registers a [`FrameSink`] that re-emits registry frames as socket.io
//!     events, so server-initiated pushes reach it like any other connection,
//!   * dispatches inbound events through the same `dispatch_ws_event` path as
//!     the raw driver — the handler's return value rides back as the **native
//!     socket.io ack** (design principle P5), and
//!   * fires `onConnect`/`onMessage`/`onError`/`onDisconnect` identically.
//!
//! Because socketioxide 0.16 has no catch-all event handler, the concrete set
//! of `on="event"` names is scanned from the CFC at handshake
//! ([`crate::websocket::extract_event_names`]) and a `socket.on` registered for
//! each, plus the conventional `"message"` → `onMessage`.

use std::sync::Arc;

use cfml_common::dynamic::CfmlValue;
use cfml_vm::websocket::{FrameSink, WireEnvelope};
use socketioxide::extract::{AckSender, Data, SocketRef};
use socketioxide::layer::SocketIoLayer;
use socketioxide::SocketIo;

use crate::websocket::{dispatch, is_reject, resolve_channel};
use crate::AppState;

/// The [`FrameSink`] handed to the registry for one socket.io connection. A
/// frame the registry routes to this connection is re-emitted as a socket.io
/// event named by the frame's `ev` (a raw `socket.send` → `"message"`; a
/// presence frame → `"presence_state"`/`"presence_diff"`; otherwise the frame
/// type). socket.io owns its own outbound buffering/backpressure, so `emit` is
/// non-blocking and we just drop on a closed socket.
#[derive(Debug)]
struct SocketIoSink {
    socket: SocketRef,
}

impl FrameSink for SocketIoSink {
    fn send(&self, frame: WireEnvelope) {
        let event = frame.ev.clone().unwrap_or_else(|| {
            if frame.t == "msg" {
                "message".to_string()
            } else {
                frame.t.clone()
            }
        });
        // `emit` errors only if the socket is already closed → nothing to do.
        let _ = self.socket.emit(event.as_str(), &frame.d);
    }
    fn close(&self, _code: u16, _reason: String) {
        // socket.io has no close-code channel like raw WS; just disconnect.
        let _ = self.socket.clone().disconnect();
    }
}

/// Build the socket.io tower layer, wiring a single dynamic namespace handler
/// that resolves any `/<channel>` namespace to a CFC under
/// `<docroot>/websockets/`. The returned layer is mounted on the axum router;
/// the `SocketIo` handle is consumed here (namespaces live on the shared client
/// the layer holds, so nothing else needs it — fan-out goes through our
/// registry, never socketioxide's own rooms).
pub(crate) fn build_layer(state: Arc<AppState>) -> SocketIoLayer {
    let (layer, io) = SocketIo::new_layer();
    io.dyn_ns("/{ns}", move |socket: SocketRef| {
        let state = state.clone();
        async move {
            on_connect(socket, state).await;
        }
    })
    .expect("register socket.io dynamic namespace");
    layer
}

/// One socket.io connection: resolve the channel, register with the registry,
/// run the `onConnect` reject gate (+ auto-join + resumability replay), then
/// wire up the per-event handlers and the disconnect teardown.
async fn on_connect(socket: SocketRef, state: Arc<AppState>) {
    // Namespace `/chat` → channel CFC `chat`. An unknown channel is refused.
    let ns = socket.ns().to_string();
    let name = ns.trim_start_matches('/');
    let info = match resolve_channel(&state, name) {
        Some(i) => i,
        None => {
            let _ = socket.disconnect();
            return;
        }
    };

    // Handshake identity (CFID cookie) + query params (P6), read off the
    // upgrade request socketioxide preserved on the socket.
    let parts = socket.req_parts();
    let session_id = crate::websocket::session_id_from_headers(&parts.headers);
    let params = crate::websocket::parse_query(parts.uri.query());

    let registry = state.server_state.websocket.clone();
    let sink: Arc<dyn FrameSink> = Arc::new(SocketIoSink { socket: socket.clone() });
    let conn_id = registry.register(&info.channel, sink, session_id.clone(), params);
    registry.set_history_cap(&info.channel, info.history);

    // Register the per-event handlers *before* the onConnect gate. socketioxide
    // spawns this connect handler (so inbound packets are processed
    // concurrently), and it has no catch-all — registering up front means the
    // `socket.on` handlers exist the instant the socket is connected, closing
    // the window where a client that emits immediately after connecting would
    // have its first event silently dropped. (A message racing *ahead* of the
    // onConnect side effects is a sub-millisecond window; real clients await the
    // `connect` event first.) `on_disconnect` is registered only after accept,
    // below, so a rejected connection never fires `onDisconnect`.
    for event in &info.events {
        let event_name = event.clone();
        // `"message"` maps to `onMessage` with no event; everything else routes
        // to the matching `on="<event>"` handler (resolved VM-side).
        let routed_event = if event_name == "message" {
            None
        } else {
            Some(event_name.clone())
        };
        let state = state.clone();
        let info = info.clone();
        let conn_id = conn_id.clone();
        let session_id = session_id.clone();
        socket.on(
            event_name,
            move |_s: SocketRef, Data::<CfmlValue>(payload): Data<CfmlValue>, ack: AckSender| {
                let state = state.clone();
                let info = info.clone();
                let conn_id = conn_id.clone();
                let session_id = session_id.clone();
                let routed_event = routed_event.clone();
                async move {
                    let r = dispatch(
                        &state,
                        &info,
                        &conn_id,
                        session_id.clone(),
                        "onMessage",
                        routed_event.as_deref(),
                        Some(payload),
                    )
                    .await;
                    match r {
                        // Non-null return → the client's ack (P5). A null/void
                        // return sends no ack (matches the raw driver).
                        Ok(Some(v)) if !matches!(v, CfmlValue::Null) => {
                            let _ = ack.send(&v);
                        }
                        Ok(_) => {}
                        // A throw surfaces to onError, never killing the socket.
                        Err(msg) => {
                            let mut err = cfml_common::dynamic::ValueMap::default();
                            err.insert("message".to_string(), CfmlValue::string(msg));
                            err.insert(
                                "type".to_string(),
                                CfmlValue::string("Application".to_string()),
                            );
                            let _ = dispatch(
                                &state,
                                &info,
                                &conn_id,
                                session_id,
                                "onError",
                                None,
                                Some(CfmlValue::strukt(err)),
                            )
                            .await;
                        }
                    }
                }
            },
        );
    }

    // onConnect reject gate (same semantics as the raw driver): `false` → reject;
    // an array return → the rooms to auto-join.
    let connect =
        dispatch(&state, &info, &conn_id, session_id.clone(), "onConnect", None, None).await;
    let mut rejected = false;
    match connect {
        Ok(Some(v)) => {
            if is_reject(&v) {
                rejected = true;
            } else if let CfmlValue::Array(rooms) = &v {
                for r in rooms.iter() {
                    registry.join(&conn_id, &r.as_string());
                }
            }
        }
        Err(_) => rejected = true,
        Ok(None) => {}
    }
    if rejected {
        registry.unregister(&conn_id);
        let _ = socket.disconnect();
        return;
    }

    // Resumability replay (P12): a reconnecting client passes `?lastEventId=…`
    // (socket.io `query` option); queue the missed channel frames ahead of live
    // traffic, after the auto-join so rooms are re-established first.
    if info.history > 0 {
        if let Some(last) = registry.params_of(&conn_id).and_then(|p| {
            p.iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("lastEventId"))
                .map(|(_, v)| v.as_string())
        }) {
            if !last.is_empty() {
                registry.replay_since(&info.channel, &last, &conn_id);
            }
        }
    }

    // Teardown: fire onDisconnect then unregister (rooms + presence cleanup is
    // unconditional in the registry, P10).
    {
        let state = state.clone();
        let info = info.clone();
        let conn_id = conn_id.clone();
        let session_id = session_id.clone();
        socket.on_disconnect(move |_s: SocketRef| {
            let state = state.clone();
            let info = info.clone();
            let conn_id = conn_id.clone();
            let session_id = session_id.clone();
            async move {
                let _ = dispatch(
                    &state,
                    &info,
                    &conn_id,
                    session_id,
                    "onDisconnect",
                    None,
                    Some(CfmlValue::string("client closed".to_string())),
                )
                .await;
                state.server_state.websocket.unregister(&conn_id);
            }
        });
    }
}
