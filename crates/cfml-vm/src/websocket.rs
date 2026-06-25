//! WebSocket / realtime core — registry, rooms, wire envelope, and the live
//! `socket` / `io()` NativeObjects.
//!
//! This module is deliberately **axum/tokio-free**. The only contact with the
//! async world is the [`FrameSink`] trait: the connection driver in
//! `crates/cli` implements it over a bounded `tokio::mpsc::Sender`, but nothing
//! here knows that. That keeps `cfml-vm` (and therefore `cfml-worker` /
//! `rustcfml-wasm`, which build for `wasm32` and never run a server) compiling
//! on every target — the registry is just data + trait-bounded sends.
//!
//! Design rationale and the cross-ecosystem principle catalog live in
//! `docs/websocket-design.md`; the build order and wire spec in
//! `docs/websocket-implementation-plan.md`. Cluster-readiness is baked in from
//! the start (decision 3): connection ids are **node-qualified**
//! (`{nodeId}:{ulid}`), per-channel message ids are monotonic, and all fan-out
//! routes through the registry so a distributed `Broker` (Phase 2) can slot in
//! at the same call sites with no id/wire changes.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use cfml_common::dynamic::{CfmlNative, CfmlStruct, CfmlValue, ValueMap};
use cfml_common::vm::{CfmlError, CfmlResult};
use parking_lot::RwLock;
/// `CfmlValue::NativeObject` wraps `std::sync::RwLock` (not parking_lot's),
/// so emitter/handle objects handed back to CFML must use this one.
use std::sync::RwLock as StdRwLock;

/// A node-qualified connection id: `"{nodeId}:{ulid}"`. Kept as a plain
/// `String` so it drops straight into `CfmlValue::String` for the CFML surface
/// (`socket.id()`) and into `HashMap` keys with no newtype friction. The node
/// prefix lets "send to connection X" route to the owning node once the
/// distributed `Broker` lands.
pub type ConnId = String;

/// The single outbound primitive. The connection driver task (in `cli`)
/// implements this over a bounded channel to the WebSocket sink; the registry
/// only ever calls `send`/`close`, so no async types leak into `cfml-vm`.
pub trait FrameSink: Send + Sync + std::fmt::Debug {
    /// Enqueue a frame for delivery to this connection. Implementations must be
    /// non-blocking (bounded channel; drop/close on overflow) — the registry is
    /// called from synchronous VM code.
    fn send(&self, frame: WireEnvelope);
    /// Request the connection be closed with the given code/reason.
    fn close(&self, code: u16, reason: String);
}

/// One realtime frame on the wire. Raw-WS transports serialize this to JSON;
/// the socket.io transport (Phase 3) maps the same fields onto Engine.IO
/// packets. Designed once so ids never have to change when the distributed
/// `Broker` switches on. `d` stays a live [`CfmlValue`] until the driver
/// serializes it, so `encoding="json"` round-trips structs/arrays unchanged.
#[derive(Clone, Debug)]
pub struct WireEnvelope {
    /// Frame type: `msg|ack|join|leave|presence|err|ping|pong`.
    pub t: String,
    /// Channel (handler + auth boundary), e.g. `/chat`.
    pub ch: String,
    /// Event name (routes to `on="message"` / `onMessage`). `None` for a raw
    /// `socket.send()` text/binary frame.
    pub ev: Option<String>,
    /// Payload — auto-(de)serialized when the channel declares `encoding="json"`.
    pub d: CfmlValue,
    /// Node-qualified, monotonic-per-channel id → resumability + routing.
    pub id: String,
    /// Ack correlation id (set when a reply is expected).
    pub ref_id: Option<String>,
}

impl WireEnvelope {
    /// Render the envelope as a CFML struct (for JSON serialization by the
    /// driver). Keys mirror the wire spec; absent optionals are omitted.
    pub fn to_value_map(&self) -> ValueMap {
        let mut m = ValueMap::default();
        m.insert("t".to_string(), CfmlValue::string(self.t.clone()));
        m.insert("ch".to_string(), CfmlValue::string(self.ch.clone()));
        if let Some(ev) = &self.ev {
            m.insert("ev".to_string(), CfmlValue::string(ev.clone()));
        }
        m.insert("d".to_string(), self.d.clone());
        m.insert("id".to_string(), CfmlValue::string(self.id.clone()));
        if let Some(r) = &self.ref_id {
            m.insert("ref".to_string(), CfmlValue::string(r.clone()));
        }
        m
    }
}

/// Per-connection record held only by the owning node.
struct ConnEntry {
    channel: String,
    sink: Arc<dyn FrameSink>,
    rooms: HashSet<String>,
    /// Live, reference-typed per-connection state. The same handle is handed to
    /// CFML as `socket.data`, so mutations there persist for the connection's
    /// life with no get/set/write-back ceremony (design principle P7).
    data: CfmlStruct,
    /// Handshake query parameters (`?userId=42`), read by `socket.param(name)`.
    params: ValueMap,
    /// CFID from the handshake cookie — identity is resolved once at connect
    /// and then ambient on the socket (design principle P6).
    session_id: Option<String>,
    /// Presence keys this connection has tracked itself under (design principle
    /// P11). Recorded so disconnect can emit leave diffs and never leak a stale
    /// roster entry.
    presence_keys: HashSet<String>,
}

// `CfmlStruct` is not `Debug`, so derive can't reach through `data`. The other
// fields are all the registry's `{:?}` ever needs.
impl std::fmt::Debug for ConnEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConnEntry")
            .field("channel", &self.channel)
            .field("rooms", &self.rooms)
            .field("params", &self.params.keys().collect::<Vec<_>>())
            .field("session_id", &self.session_id)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Default)]
struct Inner {
    conns: HashMap<ConnId, ConnEntry>,
    /// `(channel, room)` → member connection ids. Local membership index; the
    /// distributed `Broker` merges remote membership on top (Phase 2).
    rooms: HashMap<(String, String), HashSet<ConnId>>,
    /// Discovered channel → CFC file path (bytecode-cached at dispatch time).
    channels: HashMap<String, String>,
    /// Presence roster: `(channel, key)` → `conn → meta`. A key (e.g. a user id)
    /// can have several metas — one per connection/device — exactly like Phoenix
    /// Presence. `BTreeMap` keeps the metas list deterministic. The distributed
    /// `Broker` merges remote presence on top (Phase 2).
    presence: HashMap<(String, String), BTreeMap<ConnId, CfmlValue>>,
}

/// The realtime connection registry. Lives on `ServerState` so it crosses
/// requests (emit-from-anywhere, design principle P1). Single `RwLock` for
/// Phase 1 — correctness over contention; a sharded/`DashMap` form is a drop-in
/// later if profiling demands it.
#[derive(Debug)]
pub struct WebSocketRegistry {
    node_id: Arc<str>,
    inner: RwLock<Inner>,
    seq: AtomicU64,
}

impl WebSocketRegistry {
    pub fn new(node_id: impl Into<Arc<str>>) -> Self {
        Self {
            node_id: node_id.into(),
            inner: RwLock::new(Inner::default()),
            seq: AtomicU64::new(1),
        }
    }

    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    /// Mint the next monotonic, node-qualified message id.
    pub fn next_id(&self) -> String {
        let n = self.seq.fetch_add(1, Ordering::Relaxed);
        format!("{}:{}", self.node_id, n)
    }

    fn new_conn_id(&self) -> ConnId {
        format!("{}:{}", self.node_id, uuid::Uuid::new_v4())
    }

    /// Build a `msg` envelope with a fresh id.
    pub fn msg(&self, channel: &str, event: Option<String>, data: CfmlValue) -> WireEnvelope {
        WireEnvelope {
            t: "msg".to_string(),
            ch: channel.to_string(),
            ev: event,
            d: data,
            id: self.next_id(),
            ref_id: None,
        }
    }

    // ── channel discovery cache ───────────────────────────────────────────

    pub fn set_channel(&self, channel: &str, cfc_path: &str) {
        self.inner
            .write()
            .channels
            .insert(channel.to_lowercase(), cfc_path.to_string());
    }

    pub fn channel_cfc(&self, channel: &str) -> Option<String> {
        self.inner.read().channels.get(&channel.to_lowercase()).cloned()
    }

    // ── connection lifecycle ──────────────────────────────────────────────

    /// Register a freshly-upgraded connection. Returns its node-qualified id.
    /// The connection auto-joins a room named after its own id (design
    /// principle P2: "send to a user" and "send to a room" are one primitive).
    pub fn register(
        &self,
        channel: &str,
        sink: Arc<dyn FrameSink>,
        session_id: Option<String>,
        params: ValueMap,
    ) -> ConnId {
        let channel = channel.to_lowercase();
        let conn_id = self.new_conn_id();
        let mut rooms = HashSet::new();
        rooms.insert(conn_id.clone());
        let mut inner = self.inner.write();
        inner
            .rooms
            .entry((channel.clone(), conn_id.clone()))
            .or_default()
            .insert(conn_id.clone());
        inner.conns.insert(
            conn_id.clone(),
            ConnEntry {
                channel,
                sink,
                rooms,
                data: CfmlStruct::empty(),
                params,
                session_id,
                presence_keys: HashSet::new(),
            },
        );
        conn_id
    }

    /// Remove a connection from the registry and every room it belonged to.
    /// Returns `(channel, rooms)` so the driver can fire `onDisconnect` and any
    /// presence diffs. Cleanup is unconditional (design principle P10: the #1
    /// realtime leak is impossible by default).
    pub fn unregister(&self, conn_id: &str) -> Option<(String, Vec<String>)> {
        // Mutate under the write lock, then release it *before* broadcasting the
        // presence-leave diffs — `broadcast` takes a read lock and parking_lot's
        // RwLock is not reentrant.
        let (channel, rooms, leaves) = {
            let mut inner = self.inner.write();
            let entry = inner.conns.remove(conn_id)?;
            let channel = entry.channel.clone();
            let rooms: Vec<String> = entry.rooms.iter().cloned().collect();
            for room in &rooms {
                if let Some(set) = inner.rooms.get_mut(&(channel.clone(), room.clone())) {
                    set.remove(conn_id);
                    if set.is_empty() {
                        inner.rooms.remove(&(channel.clone(), room.clone()));
                    }
                }
            }
            // Drop every presence entry this connection held, capturing the
            // removed metas so the channel learns it left (design principle P10).
            let mut leaves = Vec::new();
            for key in entry.presence_keys.iter() {
                let map_key = (channel.clone(), key.clone());
                if let Some(map) = inner.presence.get_mut(&map_key) {
                    if let Some(meta) = map.remove(conn_id) {
                        leaves.push((key.clone(), meta));
                    }
                    if map.is_empty() {
                        inner.presence.remove(&map_key);
                    }
                }
            }
            (channel, rooms, leaves)
        };
        for (key, meta) in leaves {
            let frame = self.presence_diff_frame(&channel, false, &key, &meta);
            self.broadcast(&channel, frame, None);
        }
        Some((channel, rooms))
    }

    pub fn join(&self, conn_id: &str, room: &str) {
        let room = room.to_lowercase();
        let mut inner = self.inner.write();
        let channel = match inner.conns.get(conn_id) {
            Some(e) => e.channel.clone(),
            None => return,
        };
        if let Some(e) = inner.conns.get_mut(conn_id) {
            e.rooms.insert(room.clone());
        }
        inner
            .rooms
            .entry((channel, room))
            .or_default()
            .insert(conn_id.to_string());
    }

    pub fn leave(&self, conn_id: &str, room: &str) {
        let room = room.to_lowercase();
        let mut inner = self.inner.write();
        let channel = match inner.conns.get(conn_id) {
            Some(e) => e.channel.clone(),
            None => return,
        };
        if let Some(e) = inner.conns.get_mut(conn_id) {
            e.rooms.remove(&room);
        }
        if let Some(set) = inner.rooms.get_mut(&(channel.clone(), room.clone())) {
            set.remove(conn_id);
            if set.is_empty() {
                inner.rooms.remove(&(channel, room));
            }
        }
    }

    pub fn rooms_of(&self, conn_id: &str) -> Vec<String> {
        self.inner
            .read()
            .conns
            .get(conn_id)
            .map(|e| e.rooms.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// The live per-connection `data` struct (reference-typed handle).
    pub fn data_of(&self, conn_id: &str) -> Option<CfmlStruct> {
        self.inner.read().conns.get(conn_id).map(|e| e.data.clone())
    }

    pub fn channel_of(&self, conn_id: &str) -> Option<String> {
        self.inner.read().conns.get(conn_id).map(|e| e.channel.clone())
    }

    /// Handshake query params for a connection.
    pub fn params_of(&self, conn_id: &str) -> Option<ValueMap> {
        self.inner.read().conns.get(conn_id).map(|e| e.params.clone())
    }

    /// CFID captured at the handshake (identity is ambient on the socket, P6).
    pub fn session_id_of(&self, conn_id: &str) -> Option<String> {
        self.inner.read().conns.get(conn_id).and_then(|e| e.session_id.clone())
    }

    // ── delivery ──────────────────────────────────────────────────────────

    /// Deliver to a single connection (by id). Local-only in Phase 1; remote
    /// ids route through the `Broker` once it lands.
    pub fn emit_to(&self, conn_id: &str, frame: WireEnvelope) {
        let inner = self.inner.read();
        if let Some(entry) = inner.conns.get(conn_id) {
            entry.sink.send(frame);
        }
    }

    /// Deliver to every connection on `channel`, optionally excluding one id
    /// (self-echo control — design principle P4).
    pub fn broadcast(&self, channel: &str, frame: WireEnvelope, except: Option<&str>) {
        let channel = channel.to_lowercase();
        let inner = self.inner.read();
        for (id, entry) in inner.conns.iter() {
            if entry.channel != channel {
                continue;
            }
            if Some(id.as_str()) == except {
                continue;
            }
            entry.sink.send(frame.clone());
        }
    }

    /// Deliver to every member of `(channel, room)`, optionally excluding one id.
    pub fn to_room(&self, channel: &str, room: &str, frame: WireEnvelope, except: Option<&str>) {
        let channel = channel.to_lowercase();
        let room = room.to_lowercase();
        let inner = self.inner.read();
        if let Some(set) = inner.rooms.get(&(channel, room)) {
            for id in set.iter() {
                if Some(id.as_str()) == except {
                    continue;
                }
                if let Some(entry) = inner.conns.get(id) {
                    entry.sink.send(frame.clone());
                }
            }
        }
    }

    pub fn close_conn(&self, conn_id: &str, code: u16, reason: String) {
        let inner = self.inner.read();
        if let Some(entry) = inner.conns.get(conn_id) {
            entry.sink.close(code, reason);
        }
    }

    // ── presence (design principle P11) ───────────────────────────────────

    /// Track `conn_id` in its channel's presence roster under `key`, carrying
    /// `meta`. The tracking connection immediately gets the full `presence_state`
    /// snapshot (so it sees who is already here); everyone else gets a
    /// `presence_diff` join. Re-tracking under the same key replaces the meta
    /// (an update). Cluster-correctness comes free once the distributed `Broker`
    /// fans these through the shared-session cluster.
    pub fn track(&self, conn_id: &str, key: &str, meta: CfmlValue) {
        let channel = {
            let mut inner = self.inner.write();
            let channel = match inner.conns.get(conn_id) {
                Some(e) => e.channel.clone(),
                None => return,
            };
            if let Some(e) = inner.conns.get_mut(conn_id) {
                e.presence_keys.insert(key.to_string());
            }
            inner
                .presence
                .entry((channel.clone(), key.to_string()))
                .or_default()
                .insert(conn_id.to_string(), meta.clone());
            channel
        };
        // Snapshot to the tracking connection, join diff to the rest.
        let state = self.presence_state(&channel);
        let state_frame = WireEnvelope {
            t: "presence".to_string(),
            ch: channel.clone(),
            ev: Some("presence_state".to_string()),
            d: state,
            id: self.next_id(),
            ref_id: None,
        };
        self.emit_to(conn_id, state_frame);
        let join = self.presence_diff_frame(&channel, true, key, &meta);
        self.broadcast(&channel, join, Some(conn_id));
    }

    /// Remove `conn_id`'s presence under `key` and broadcast a `presence_diff`
    /// leave to the channel. No-op if it wasn't tracked.
    pub fn untrack(&self, conn_id: &str, key: &str) {
        let removed = {
            let mut inner = self.inner.write();
            let channel = match inner.conns.get(conn_id) {
                Some(e) => e.channel.clone(),
                None => return,
            };
            if let Some(e) = inner.conns.get_mut(conn_id) {
                e.presence_keys.remove(key);
            }
            let map_key = (channel.clone(), key.to_string());
            let meta = inner.presence.get_mut(&map_key).and_then(|m| m.remove(conn_id));
            if inner.presence.get(&map_key).is_some_and(|m| m.is_empty()) {
                inner.presence.remove(&map_key);
            }
            meta.map(|m| (channel, m))
        };
        if let Some((channel, meta)) = removed {
            let leave = self.presence_diff_frame(&channel, false, key, &meta);
            self.broadcast(&channel, leave, None);
        }
    }

    /// The full presence roster for a channel as a CFML struct:
    /// `{ "<key>": { metas: [ meta, … ] }, … }`. Same shape as a
    /// `presence_state` frame's payload.
    pub fn presence_state(&self, channel: &str) -> CfmlValue {
        let channel = channel.to_lowercase();
        let inner = self.inner.read();
        let mut out = ValueMap::default();
        for ((ch, key), metas) in inner.presence.iter() {
            if ch != &channel {
                continue;
            }
            let arr: Vec<CfmlValue> = metas.values().cloned().collect();
            let mut entry = ValueMap::default();
            entry.insert("metas".to_string(), CfmlValue::array(arr));
            out.insert(key.clone(), CfmlValue::strukt(entry));
        }
        CfmlValue::strukt(out)
    }

    /// Build a `presence_diff` frame. `join == true` puts the affected key under
    /// `joins`, otherwise under `leaves` — the side not in play is an empty struct
    /// (stable shape for clients).
    fn presence_diff_frame(
        &self,
        channel: &str,
        join: bool,
        key: &str,
        meta: &CfmlValue,
    ) -> WireEnvelope {
        let mut side = ValueMap::default();
        let mut entry = ValueMap::default();
        entry.insert("metas".to_string(), CfmlValue::array(vec![meta.clone()]));
        side.insert(key.to_string(), CfmlValue::strukt(entry));
        let mut d = ValueMap::default();
        if join {
            d.insert("joins".to_string(), CfmlValue::strukt(side));
            d.insert("leaves".to_string(), CfmlValue::strukt(ValueMap::default()));
        } else {
            d.insert("joins".to_string(), CfmlValue::strukt(ValueMap::default()));
            d.insert("leaves".to_string(), CfmlValue::strukt(side));
        }
        WireEnvelope {
            t: "presence".to_string(),
            ch: channel.to_string(),
            ev: Some("presence_diff".to_string()),
            d: CfmlValue::strukt(d),
            id: self.next_id(),
            ref_id: None,
        }
    }

    // ── introspection ─────────────────────────────────────────────────────

    pub fn channel_count(&self, channel: &str) -> usize {
        let channel = channel.to_lowercase();
        self.inner
            .read()
            .conns
            .values()
            .filter(|e| e.channel == channel)
            .count()
    }

    pub fn room_count(&self, channel: &str, room: &str) -> usize {
        let channel = channel.to_lowercase();
        let room = room.to_lowercase();
        self.inner
            .read()
            .rooms
            .get(&(channel, room))
            .map(|s| s.len())
            .unwrap_or(0)
    }

    pub fn channel_sockets(&self, channel: &str) -> Vec<String> {
        let channel = channel.to_lowercase();
        self.inner
            .read()
            .conns
            .iter()
            .filter(|(_, e)| e.channel == channel)
            .map(|(id, _)| id.clone())
            .collect()
    }

    pub fn room_sockets(&self, channel: &str, room: &str) -> Vec<String> {
        let channel = channel.to_lowercase();
        let room = room.to_lowercase();
        self.inner
            .read()
            .rooms
            .get(&(channel, room))
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default()
    }
}

// ──────────────────────────────────────────────────────────────────────────
// CFML-facing NativeObjects
// ──────────────────────────────────────────────────────────────────────────

/// The live `socket` handle passed to channel lifecycle/event methods.
/// Wraps `(conn_id, channel, registry)` and maps `id/send/emit/broadcast/
/// join/leave/rooms/to/close/data` onto registry ops. Same surface for raw-WS
/// and socket.io.
#[derive(Debug)]
pub struct SocketHandle {
    pub conn_id: ConnId,
    pub channel: String,
    pub registry: Arc<WebSocketRegistry>,
}

impl SocketHandle {
    pub fn new(conn_id: ConnId, channel: String, registry: Arc<WebSocketRegistry>) -> Self {
        Self { conn_id, channel, registry }
    }
}

fn arg_string(args: &[CfmlValue], idx: usize) -> String {
    args.get(idx).map(|v| v.as_string()).unwrap_or_default()
}

fn arg_value(args: &[CfmlValue], idx: usize) -> CfmlValue {
    args.get(idx).cloned().unwrap_or(CfmlValue::Null)
}

impl CfmlNative for SocketHandle {
    fn class_name(&self) -> &str {
        "Socket"
    }

    fn call_method(&mut self, name: &str, args: Vec<CfmlValue>) -> CfmlResult {
        match name.to_ascii_lowercase().as_str() {
            "id" => Ok(CfmlValue::string(self.conn_id.clone())),
            "channel" => Ok(CfmlValue::string(self.channel.clone())),
            "send" => {
                let frame = self.registry.msg(&self.channel, None, arg_value(&args, 0));
                self.registry.emit_to(&self.conn_id, frame);
                Ok(CfmlValue::Null)
            }
            "emit" => {
                let frame =
                    self.registry
                        .msg(&self.channel, Some(arg_string(&args, 0)), arg_value(&args, 1));
                self.registry.emit_to(&self.conn_id, frame);
                Ok(CfmlValue::Null)
            }
            "broadcast" => {
                // To everyone in the channel except the sender.
                let frame =
                    self.registry
                        .msg(&self.channel, Some(arg_string(&args, 0)), arg_value(&args, 1));
                self.registry.broadcast(&self.channel, frame, Some(&self.conn_id));
                Ok(CfmlValue::Null)
            }
            "join" => {
                self.registry.join(&self.conn_id, &arg_string(&args, 0));
                Ok(CfmlValue::Null)
            }
            "leave" => {
                self.registry.leave(&self.conn_id, &arg_string(&args, 0));
                Ok(CfmlValue::Null)
            }
            "rooms" => Ok(CfmlValue::array(
                self.registry
                    .rooms_of(&self.conn_id)
                    .into_iter()
                    .map(CfmlValue::string)
                    .collect(),
            )),
            // Presence (P11). `track(meta)` keys on the connection id;
            // `track(key, meta)` groups several connections (e.g. a user's tabs)
            // under one key. The tracking client gets a `presence_state`
            // snapshot, others a `presence_diff` join.
            "track" => {
                let (key, meta) = match args.as_slice() {
                    [CfmlValue::Struct(_), ..] => (self.conn_id.clone(), arg_value(&args, 0)),
                    [k, m, ..] => (k.as_string(), m.clone()),
                    [k] => (k.as_string(), CfmlValue::strukt(ValueMap::default())),
                    [] => (self.conn_id.clone(), CfmlValue::strukt(ValueMap::default())),
                };
                self.registry.track(&self.conn_id, &key, meta);
                Ok(CfmlValue::Null)
            }
            "untrack" => {
                let key = if args.is_empty() {
                    self.conn_id.clone()
                } else {
                    arg_string(&args, 0)
                };
                self.registry.untrack(&self.conn_id, &key);
                Ok(CfmlValue::Null)
            }
            "presence" => Ok(self.registry.presence_state(&self.channel)),
            "to" | "in" => {
                let emitter = ServerEmitter {
                    channel: self.channel.clone(),
                    room: Some(arg_string(&args, 0)),
                    except: None,
                    registry: self.registry.clone(),
                };
                Ok(CfmlValue::NativeObject(Arc::new(StdRwLock::new(emitter))))
            }
            "close" => {
                let code = args
                    .first()
                    .map(|v| v.as_string().parse::<u16>().unwrap_or(1000))
                    .unwrap_or(1000);
                let reason = arg_string(&args, 1);
                self.registry.close_conn(&self.conn_id, code, reason);
                Ok(CfmlValue::Null)
            }
            "data" => Ok(self
                .registry
                .data_of(&self.conn_id)
                .map(CfmlValue::Struct)
                .unwrap_or(CfmlValue::Null)),
            // Handshake query parameter, e.g. socket.param("userId") for ?userId=42.
            "param" => {
                let key = arg_string(&args, 0);
                let val = self
                    .registry
                    .params_of(&self.conn_id)
                    .and_then(|p| {
                        p.iter()
                            .find(|(k, _)| k.eq_ignore_ascii_case(&key))
                            .map(|(_, v)| v.clone())
                    })
                    .unwrap_or(CfmlValue::Null);
                Ok(val)
            }
            "params" => Ok(self
                .registry
                .params_of(&self.conn_id)
                .map(CfmlValue::strukt)
                .unwrap_or_else(|| CfmlValue::strukt(ValueMap::default()))),
            // Session identity resolved at the handshake (P6). The full live
            // session scope + Preside auth helpers (isWebUser/...) ride the
            // socket.io compat layer in a later phase.
            "sessionid" => Ok(self
                .registry
                .session_id_of(&self.conn_id)
                .map(CfmlValue::string)
                .unwrap_or(CfmlValue::Null)),
            other => Err(CfmlError::runtime(format!("Socket has no method [{}]", other))),
        }
    }

    fn get_property(&self, name: &str) -> Option<CfmlValue> {
        match name.to_ascii_lowercase().as_str() {
            "id" => Some(CfmlValue::string(self.conn_id.clone())),
            "channel" => Some(CfmlValue::string(self.channel.clone())),
            "data" => self.registry.data_of(&self.conn_id).map(CfmlValue::Struct),
            "sessionid" => Some(
                self.registry
                    .session_id_of(&self.conn_id)
                    .map(CfmlValue::string)
                    .unwrap_or(CfmlValue::Null),
            ),
            _ => None,
        }
    }
}

/// A scoped emitter — what `io(channel)`, `io(channel).to(room)`, and
/// `socket.to(room)` return. The fluent target chain (design principle P4):
/// `io("/chat").to("lobby").except(id).emit(event, data)`.
#[derive(Debug)]
pub struct ServerEmitter {
    pub channel: String,
    pub room: Option<String>,
    pub except: Option<ConnId>,
    pub registry: Arc<WebSocketRegistry>,
}

impl ServerEmitter {
    pub fn new(channel: String, registry: Arc<WebSocketRegistry>) -> Self {
        Self { channel, room: None, except: None, registry }
    }

    fn deliver(&self, frame: WireEnvelope) {
        match &self.room {
            Some(room) => self.registry.to_room(&self.channel, room, frame, self.except.as_deref()),
            None => self.registry.broadcast(&self.channel, frame, self.except.as_deref()),
        }
    }
}

impl CfmlNative for ServerEmitter {
    fn class_name(&self) -> &str {
        "WsEmitter"
    }

    fn call_method(&mut self, name: &str, args: Vec<CfmlValue>) -> CfmlResult {
        match name.to_ascii_lowercase().as_str() {
            "to" | "in" => {
                let emitter = ServerEmitter {
                    channel: self.channel.clone(),
                    room: Some(arg_string(&args, 0)),
                    except: self.except.clone(),
                    registry: self.registry.clone(),
                };
                Ok(CfmlValue::NativeObject(Arc::new(StdRwLock::new(emitter))))
            }
            "except" | "exclude" => {
                let emitter = ServerEmitter {
                    channel: self.channel.clone(),
                    room: self.room.clone(),
                    except: Some(arg_string(&args, 0)),
                    registry: self.registry.clone(),
                };
                Ok(CfmlValue::NativeObject(Arc::new(StdRwLock::new(emitter))))
            }
            "emit" => {
                let frame =
                    self.registry
                        .msg(&self.channel, Some(arg_string(&args, 0)), arg_value(&args, 1));
                self.deliver(frame);
                Ok(CfmlValue::Null)
            }
            "send" => {
                let frame = self.registry.msg(&self.channel, None, arg_value(&args, 0));
                self.deliver(frame);
                Ok(CfmlValue::Null)
            }
            "count" => {
                let n = match &self.room {
                    Some(room) => self.registry.room_count(&self.channel, room),
                    None => self.registry.channel_count(&self.channel),
                };
                Ok(CfmlValue::Int(n as i64))
            }
            "sockets" => {
                let ids = match &self.room {
                    Some(room) => self.registry.room_sockets(&self.channel, room),
                    None => self.registry.channel_sockets(&self.channel),
                };
                Ok(CfmlValue::array(ids.into_iter().map(CfmlValue::string).collect()))
            }
            // The full presence roster for the channel (room scoping is ignored —
            // presence is channel-level in this phase).
            "presence" => Ok(self.registry.presence_state(&self.channel)),
            other => Err(CfmlError::runtime(format!("WsEmitter has no method [{}]", other))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Debug, Default)]
    struct CapturingSink {
        frames: Arc<Mutex<Vec<WireEnvelope>>>,
        closed: Arc<Mutex<Option<(u16, String)>>>,
    }

    impl FrameSink for CapturingSink {
        fn send(&self, frame: WireEnvelope) {
            self.frames.lock().unwrap().push(frame);
        }
        fn close(&self, code: u16, reason: String) {
            *self.closed.lock().unwrap() = Some((code, reason));
        }
    }

    fn sink() -> (Arc<CapturingSink>, Arc<dyn FrameSink>) {
        let s = Arc::new(CapturingSink::default());
        let dyn_s: Arc<dyn FrameSink> = s.clone();
        (s, dyn_s)
    }

    #[test]
    fn conn_id_is_node_qualified() {
        let reg = WebSocketRegistry::new("n1");
        let (_s, sink) = sink();
        let id = reg.register("/chat", sink, None, ValueMap::default());
        assert!(id.starts_with("n1:"), "conn id should be node-qualified: {id}");
    }

    #[test]
    fn broadcast_excludes_sender() {
        let reg = Arc::new(WebSocketRegistry::new("n1"));
        let (a, a_dyn) = sink();
        let (b, b_dyn) = sink();
        let id_a = reg.register("/chat", a_dyn, None, ValueMap::default());
        let _id_b = reg.register("/chat", b_dyn, None, ValueMap::default());
        let frame = reg.msg("/chat", Some("hi".into()), CfmlValue::string("x"));
        reg.broadcast("/chat", frame, Some(&id_a));
        assert_eq!(a.frames.lock().unwrap().len(), 0, "sender excluded");
        assert_eq!(b.frames.lock().unwrap().len(), 1, "other receives");
    }

    #[test]
    fn rooms_fan_out_and_cleanup() {
        let reg = Arc::new(WebSocketRegistry::new("n1"));
        let (a, a_dyn) = sink();
        let id_a = reg.register("/chat", a_dyn, None, ValueMap::default());
        reg.join(&id_a, "lobby");
        assert_eq!(reg.room_count("/chat", "lobby"), 1);
        let frame = reg.msg("/chat", Some("e".into()), CfmlValue::Null);
        reg.to_room("/chat", "lobby", frame, None);
        assert_eq!(a.frames.lock().unwrap().len(), 1);
        // Disconnect removes from every room unconditionally.
        let (ch, rooms) = reg.unregister(&id_a).unwrap();
        assert_eq!(ch, "/chat");
        assert!(rooms.contains(&"lobby".to_string()));
        assert_eq!(reg.room_count("/chat", "lobby"), 0);
        assert_eq!(reg.channel_count("/chat"), 0);
    }

    #[test]
    fn presence_track_snapshot_and_leave_on_disconnect() {
        let reg = Arc::new(WebSocketRegistry::new("n1"));
        let (a, a_dyn) = sink();
        let (b, b_dyn) = sink();
        let id_a = reg.register("/chat", a_dyn, None, ValueMap::default());
        let id_b = reg.register("/chat", b_dyn, None, ValueMap::default());

        // A tracks → A gets a presence_state, B gets a join diff.
        let mut meta_a = ValueMap::default();
        meta_a.insert("user".into(), CfmlValue::string("alice"));
        reg.track(&id_a, "alice", CfmlValue::strukt(meta_a));
        assert_eq!(a.frames.lock().unwrap().last().unwrap().ev.as_deref(), Some("presence_state"));
        assert_eq!(b.frames.lock().unwrap().last().unwrap().ev.as_deref(), Some("presence_diff"));

        // Roster lists alice.
        let state = reg.presence_state("/chat");
        let CfmlValue::Struct(s) = state else { panic!("state is a struct") };
        assert!(s.get_ci("alice").is_some(), "alice is present");

        // A disconnects → B gets a leave diff and the roster empties.
        b.frames.lock().unwrap().clear();
        reg.unregister(&id_a);
        let last = b.frames.lock().unwrap().last().unwrap().clone();
        assert_eq!(last.ev.as_deref(), Some("presence_diff"));
        if let CfmlValue::Struct(d) = &last.d {
            let CfmlValue::Struct(leaves) = d.get_ci("leaves").unwrap() else { panic!() };
            assert!(leaves.get_ci("alice").is_some(), "leave diff names alice");
        } else {
            panic!("diff payload is a struct");
        }
        let CfmlValue::Struct(empty) = reg.presence_state("/chat") else { panic!() };
        assert_eq!(empty.keys().len(), 0, "roster empty after disconnect");
    }

    #[test]
    fn self_room_named_after_id() {
        let reg = Arc::new(WebSocketRegistry::new("n1"));
        let (_a, a_dyn) = sink();
        let id_a = reg.register("/chat", a_dyn, None, ValueMap::default());
        // Auto-joined a room named after its own id.
        assert_eq!(reg.room_count("/chat", &id_a), 1);
    }
}
