# RustCFML Java Shim Implementation Plan for ColdBox Compatibility

## Overview

This document outlines the implementation plan for Java shim handlers in RustCFML needed to make the ColdBox platform (minus WireBox, already ported) fully functional. The existing shims cover ~60% of the ColdBox Java interop surface. Seven phases address the remaining gaps.

---

## Existing RustCFML Shims (Already Done)

| Java Class | Handler |
|------------|---------|
| `java.lang.StringBuilder` / `StringBuffer` | `handle_java_stringbuilder` |
| `java.util.UUID` | `handle_java_uuid` |
| `java.lang.Thread` / `ThreadGroup` | `handle_java_thread` |
| `java.net.InetAddress` | `handle_java_inetaddress` |
| `java.io.File` | `handle_java_file` |
| `java.nio.file.Paths` / `Path` | `handle_java_paths` |
| `java.lang.System` | `handle_java_system` |
| `java.util.TreeMap` | `handle_java_treemap` |
| `java.util.LinkedHashMap` | `handle_java_linkedhashmap` |
| `java.util.concurrent.ConcurrentHashMap` | `handle_java_concurrenthashmap` |
| `java.util.concurrent.ConcurrentLinkedQueue` | `handle_java_concurrentlinkedqueue` |
| `java.util.Collections` | `handle_java_collections` |
| `java.util.regex.Pattern` / `Matcher` | `handle_java_pattern` / `java_matcher_step` |
| `java.security.MessageDigest` | `handle_java_messagedigest` |

### Registration (lib.rs)

All shims are registered in the `createObject("java", ...)` dispatch at `lib.rs:7030-7090` and in the method dispatch at `lib.rs:9232-9262`.

---

## Missing Shims: Implementation Phases

### Phase 1: java.time.* (HIGH — ~2 weeks)

**Unlocks:** DateTimeHelper, Scheduler, ScheduledTask, Duration, Period, TimeUnit, async time operations

**Crates to add:** `time` (or `chrono`), `tzdb`

| Java Class | Rust Strategy | Methods Required |
|------------|---------------|------------------|
| `java.time.ZoneId` | `time::UtcOffset` + IANA tz lookup via `tzdb` | `of(id: String)`, `systemDefault()`, `toString()` |
| `java.time.ZoneOffset` | `time::UtcOffset` | `ofHours(hours)`, `toString()` |
| `java.time.LocalDateTime` | `time::PrimitiveDateTime` | `parse(CharSequence)`, `format(DateTimeFormatter)`, `toLocalDate()` |
| `java.time.LocalDate` | `time::Date` | `now()`, `toString()`, `plusDays/plusMonths/plusYears()` |
| `java.time.Duration` | `time::Duration` | `ofDays/hours/minutes/seconds/millis/nanos`, `parse(CharSequence)`, `toDays/hours/minutes/seconds/millis/nanos`, `plus/minus/multipliedBy` |
| `java.time.Period` | Custom yrs/months/days struct | `of(y,m,d)`, `parse(CharSequence)`, `addTo/subtractFrom(LocalDate)`, `getYears/Months/Days` |
| `java.time.temporal.ChronoUnit` | Enum | `between(temporal1, temporal2)` |
| `java.time.temporal.TemporalAdjusters` | Static methods | `firstInMonth(DayOfWeek)`, `lastDayOfMonth()` |
| `java.time.DayOfWeek` | Enum | `MONDAY..SUNDAY`, `of(int)` |
| `java.time.Instant` | `time::OffsetDateTime` | `now()`, `toEpochMilli()`, `parse()` |

**Registration:**

```rust
// lib.rs: createObject java dispatch
"java.time.zoneid" => handle_java_zoneid("init", empty_args, &CfmlValue::Null),
"java.time.zoneoffset" => handle_java_zoneoffset("init", empty_args, &CfmlValue::Null),
"java.time.localdatetime" => handle_java_localdatetime("init", empty_args, &CfmlValue::Null),
"java.time.duration" => handle_java_duration("init", empty_args, &CfmlValue::Null),
"java.time.period" => handle_java_period("init", empty_args, &CfmlValue::Null),
"java.time.instant" => handle_java_instant("init", empty_args, &CfmlValue::Null),
"java.time.chronounit" | "java.time.temporal.chronounit" => handle_java_chronounit("init", empty_args, &CfmlValue::Null),
"java.time.dayofweek" => handle_java_dayofweek("init", empty_args, &CfmlValue::Null),
"java.time.temporaladjusters" | "java.time.temporal.temporaladjusters" => handle_java_temporaladjusters("init", empty_args, &CfmlValue::Null),
```

**Files to create:**

```
crates/cfml-vm/src/java_shims/
├── handle_java_zoneid.rs
├── handle_java_zoneoffset.rs
├── handle_java_localdatetime.rs
├── handle_java_duration.rs
├── handle_java_period.rs
├── handle_java_instant.rs
├── handle_java_chronounit.rs
├── handle_java_dayofweek.rs
└── handle_java_temporaladjusters.rs
```

Or keep in a single `handle_java_time.rs` file for simplicity.

---

### Phase 2: java.util.concurrent.* (HIGH — ~3 weeks)

**Unlocks:** AsyncManager, ExecutorBuilder, Executor, ScheduledExecutor, Future, FutureTask, ScheduledFuture

**Crates to add:** `futures`, `async-trait` (tokio already in workspace)

| Java Class | Rust Strategy | Methods Required |
|------------|---------------|------------------|
| `java.util.concurrent.Executors` | Static factory returning pool shims | `newFixedThreadPool(nThreads)`, `newCachedThreadPool()`, `newSingleThreadExecutor()`, `newWorkStealingPool(parallelism)` |
| `java.util.concurrent.ExecutorService` | Trait on pool shim | `submit(Callable/Runnable)`, `invokeAll(Collection<Callable>)`, `shutdown()`, `shutdownNow()`, `isTerminated()` |
| `java.util.concurrent.ScheduledExecutorService` | Trait on scheduled pool shim | `schedule(Callable, delay, TimeUnit)`, `scheduleAtFixedRate(Runnable, initialDelay, period, TimeUnit)`, `scheduleWithFixedDelay(...)` |
| `java.util.concurrent.Future` | Trait | `get()`, `get(timeout, TimeUnit)`, `cancel(mayInterrupt)`, `isDone()`, `isCancelled()` |
| `java.util.concurrent.RunnableFuture` | Extends Future + Runnable | Above + `run()` |
| `java.util.concurrent.FutureTask` | Struct wrapping `Closure<CfmlResult>` + `JoinHandle` | `init(Callable)`, `get()`, `run()` |
| `java.util.concurrent.CompletableFuture` | Struct wrapping `tokio::sync::oneshot` + methods | `supplyAsync(Supplier)`, `thenApply(Function)`, `thenAccept(Consumer)`, `thenCombine(CompletableFuture, BiFunction)`, `exceptionally(Function)`, `allOf/anyOf`, `completedFuture(value)`, `get()`, `join()` |
| `java.util.concurrent.ScheduledFuture` | Future + getDelay | `getDelay(TimeUnit)` |
| `java.util.concurrent.TimeUnit` | Enum | `DAYS..NANOSECONDS`, `toMillis/days/hours/...`, `convert(sourceDuration, sourceUnit)`, `sleep(timed)`, `timedJoin(thread, timeout)` |
| `java.util.concurrent.Callable` | `Box<dyn FnOnce() -> CfmlResult + Send>` | `call()` |
| `java.lang.Runnable` | `Box<dyn FnOnce() + Send>` | `run()` |
| `java.util.concurrent.LinkedBlockingQueue` | `crossbeam::channel` or `tokio::sync::mpsc` | `init(capacity)`, `put/take` (blocking), `offer/poll` (non-blocking), `size`, `remainingCapacity()` |
| `java.util.concurrent.ForkJoinPool` | `rayon::ThreadPool` | `commonPool()`, `submit(Callable)`, `invoke(ForkJoinTask)`, `shutdown()` |

**Registration:**

```rust
// lib.rs: createObject java dispatch
"java.util.concurrent.executors"
| "java.util.concurrent.executorservice" => handle_java_executors(...),
"java.util.concurrent.future"
| "java.util.concurrent.runnablefuture" => handle_java_future(...),
"java.util.concurrent.futuretask" => handle_java_futuretask(...),
"java.util.concurrent.completablefuture" => handle_java_completablefuture(...),
"java.util.concurrent.scheduledfuture" => handle_java_scheduledfuture(...),
"java.util.concurrent.scheduledexecutorservice"
| "java.util.concurrent.scheduledthreadpoolexecutor" => handle_java_scheduledexecutorservice(...),
"java.util.concurrent.executorcompletionservice" => handle_java_executorcompletionservice(...),
"java.util.concurrent.timeunit" => handle_java_timeunit(...),
"java.util.concurrent.callable" => handle_java_callable(...),
"java.lang.runnable" => handle_java_runnable(...),
"java.util.concurrent.linkedblockingqueue" => handle_java_linkedblockingqueue(...),
"java.util.concurrent.forkjoinpool" => handle_java_forkjoinpool(...),
"java.util.concurrent.threadpoolexecutor" => handle_java_threadpoolexecutor(...),
```

**Key design decisions:**

1. **Thread pool model:**
   - `ForkJoinPool.commonPool()` → `rayon::global::pool()`
   - `newFixedThreadPool(n)` → `threadpool::ThreadPool::new(n)` or tokio `spawn_blocking`
   - `newSingleThreadExecutor()` → single-producer, single-consumer channel
   - `newCachedThreadPool()` → spawn new thread per task, with idle-timeout reclamation

2. **CompletableFuture:**
   - `supplyAsync` → `tokio::spawn_blocking`
   - `thenApply` / `thenAccept` / `exceptionally` → chain via shared `Arc<Mutex<Option<CfmlResult>>>`
   - `allOf` / `anyOf` → `futures::future::join_all` / `select_all`

**Files to create:**

```
crates/cfml-vm/src/java_shims/
├── handle_java_executor.rs          # ExecutorService, ThreadPoolExecutor
├── handle_java_scheduledexecutor.rs # ScheduledExecutorService, ScheduledThreadPoolExecutor
├── handle_java_future.rs            # Future, RunnableFuture
├── handle_java_futuretask.rs        # FutureTask
├── handle_java_completablefuture.rs # CompletableFuture
├── handle_java_scheduledfuture.rs   # ScheduledFuture
├── handle_java_timeunit.rs          # TimeUnit
├── handle_java_callable.rs          # Callable + Runnable
└── handle_java_linkedblockingqueue.rs
```

---

### Phase 3: java.lang.ref.SoftReference + ReferenceQueue (HIGH — ~1 week)

**Unlocks:** ConcurrentSoftReferenceStore (cache eviction for memory-sensitive data)

**Challenge:** Rust has no GC. Must implement approximate soft-reference semantics.

**Design:**

```rust
struct SoftRefInner {
    referent: Mutex<Option<CfmlValue>>,
    last_access: Mutex<Instant>,
}

struct SoftReferenceShim {
    inner: Arc<SoftRefInner>,
    // Use crossbeam channel to match ReferenceQueue API
    enqueue_target: Option<crossbeam::channel::Sender<Arc<SoftRefInner>>>,
}

struct ReferenceQueueShim {
    rx: crossbeam::channel::Receiver<Arc<SoftRefInner>>,
    tx: crossbeam::channel::Sender<Arc<SoftRefInner>>,
}
```

**Background sweeper** (global, spawned on VM init):
- Runs every 30s
- Checks system memory via `sysinfo`
- If memory > threshold (e.g., 80%), clears `SoftRefInner.referent` for oldest-accessed refs
- Enqueues cleared refs to `ReferenceQueueShim`

**ColdBox usage pattern** (from `ConcurrentSoftReferenceStore.cfc`):
```coldbox
// init
refQueue = createObject("java", "java.lang.ref.ReferenceQueue").init()
// get object
target = ref.get()  // returns value or null if cleared
// enqueue (GC does this automatically in Java)
// poll
cleared = refQueue.poll()  // returns SoftReference or null
```

**Methods required:**

| Method | Implementation |
|--------|----------------|
| `SoftReference.init(referent[, queue])` | Create shim with `Arc<SoftRefInner>` |
| `SoftReference.get()` | Clone inner value or Null |
| `SoftReference.clear()` | Drop inner referent |
| `SoftReference.enqueue()` | Send to ReferenceQueue |
| `SoftReference.isEnqueued()` | Check if already enqueued |
| `ReferenceQueue.init()` | Create channel pair |
| `ReferenceQueue.poll()` | `try_recv()` → value or Null |
| `ReferenceQueue.remove([timeout])` | `recv_timeout(Duration)` |

**Files to create:**

```
crates/cfml-vm/src/java_shims/
├── handle_java_softreference.rs
├── handle_java_referencequeue.rs
```

**Registration:**

```rust
"java.lang.ref.softreference" => handle_java_softreference("init", empty_args, &CfmlValue::Null),
"java.lang.ref.referencequeue" => handle_java_referencequeue("init", empty_args, &CfmlValue::Null),
```

---

### Phase 4: java.io.* Serialization (MEDIUM — ~1 week)

**Unlocks:** ObjectMarshaller (cluster replication, session serialization)

**Crates to add:** `bincode` or `postcard` (for CFML-native serialization format)

| Java Class | Methods Required | Strategy |
|------------|------------------|----------|
| `java.io.ByteArrayOutputStream` | `init()`, `write(byte[])`, `toByteArray()`, `size()`, `toString()`, `close()` | `bytes::BytesMut` wrapper |
| `java.io.ByteArrayInputStream` | `init(byte[])`, `read()`, `read(byte[])`, `available()`, `close()` | `std::io::Cursor<Vec<u8>>` |
| `java.io.ObjectOutputStream` | `writeObject(Object)`, `flush()`, `close()` | **Custom CFML serializer** |
| `java.io.ObjectInputStream` | `readObject()`, `close()` | **Custom CFML deserializer** |

**Critical limitation:** Java serialization is **not portable**. You cannot serialize a CFML struct to Java `ObjectOutputStream` format and have Java read it. Solutions:

| Option | Pros | Cons | Recommendation |
|--------|------|------|----------------|
| **1. CBOR/bincode** | Simple, fast, Rust-native | Breaks cross-engine cluster replication | **v2 default** |
| **2. No-op stub** | Trivial | Breaks ObjectMarshaller entirely | Fallback |
| **3. Feature-flag** | Choose per deployment | User must know what to pick | **Recommended** |

```rust
// Design: ObjectOutputStream wrapper
// - writeObject -> serialize to CBOR/bincode and store in inner ByteArrayOutputStream
// - flush/close -> no-op
//
// ObjectInputStream wrapper
// - init with ByteArray containing CBOR/bincode
// - readObject -> deserialize
```

**Files to create:**

```
crates/cfml-vm/src/java_shims/
├── handle_java_bytearrayoutputstream.rs
├── handle_java_bytearrayinputstream.rs
├── handle_java_objectoutputstream.rs
└── handle_java_objectinputstream.rs
```

**Registration:**

```rust
"java.io.bytearrayoutputstream" => handle_java_bytearrayoutputstream("init", empty_args, &CfmlValue::Null),
"java.io.bytearrayinputstream" => handle_java_bytearrayinputstream("init", empty_args, &CfmlValue::Null),
"java.io.objectoutputstream" => handle_java_objectoutputstream("init", empty_args, &CfmlValue::Null),
"java.io.objectinputstream" => handle_java_objectinputstream("init", empty_args, &CfmlValue::Null),
```

---

### Phase 5: java.net.* Networking (MEDIUM — ~1 week)

**Unlocks:** SocketAppender, RequestContext URI, RemotingUtil

**Crates to add:** `url` (for URI), `tokio::net` (for Socket)

| Java Class | Methods Required | Strategy |
|------------|------------------|----------|
| `java.net.Socket` | `init(String host, int port)`, `getOutputStream()`, `getInputStream()`, `close()`, `isConnected()`, `connect(SocketAddress)`, `setSoTimeout(int)` | `tokio::net::TcpStream` wrapper, blocking mode |

| `java.net.PrintWriter` | `init(OutputStream)`, `print(String)`, `println(String)`, `flush()`, `close()`, `write(String)` | `tokio::io::BufWriter<TcpStream>` wrapper |
| `java.net.URI` | `init(String str)`, `getHost()`, `getPath()`, `getPort()`, `getScheme()`, `getQuery()`, `toString()`, `getRawPath()`, `resolve(String)`, `normalize()` | `url::Url` wrapper |
| `java.net.ServerSocket` | `init(int port)`, `accept()`, `close()` | `tokio::net::TcpListener` (future work, not needed by ColdBox core) |
| `java.net.InetSocketAddress` | `init(String host, int port)`, `getHostName()`, `getPort()`, `getAddress()` | `std::net::SocketAddr` wrapper |

**Files to create:**

```
crates/cfml-vm/src/java_shims/
├── handle_java_socket.rs
├── handle_java_printwriter.rs
├── handle_java_uri.rs
└── handle_java_inetsocketaddress.rs
```

**Registration:**

```rust
"java.net.socket" => handle_java_socket("init", empty_args, &CfmlValue::Null),
"java.io.printwriter" => handle_java_printwriter("init", empty_args, &CfmlValue::Null),
"java.net.uri" => handle_java_uri("init", empty_args, &CfmlValue::Null),
"java.net.inetsocketaddress" => handle_java_inetsocketaddress("init", empty_args, &CfmlValue::Null),
```

---

### Phase 6: java.lang.Runtime + java.lang.String (MEDIUM — ~3 days)

**Unlocks:** CacheBoxProvider memory stats, ReportHandler, RemotingUtil workaround

**Crates to add:** `sysinfo`, `num_cpus`

| Java Class | Methods Required | Strategy |
|------------|------------------|----------|
| `java.lang.Runtime` | `getRuntime()`, `freeMemory()`, `totalMemory()`, `maxMemory()`, `gc()`, `exec(String[])`, `availableProcessors()` | Singleton shim using `sysinfo` |
| `java.lang.String` (as factory) | `init(String)` — string constructor (`RemotingUtil.cfc:41`) | Already works — return input as-is |

**Existing gap in `handle_java_system`:** `Runtime` methods like `freeMemory()` don't exist yet.

**Extend handle_java_system.rs with:**

```rust
// Access via createObject("java","java.lang.Runtime").getRuntime()
// Returns a shim struct (same pattern as System shim)
"getruntime" => {
    let mut shim = IndexMap::new();
    shim.insert("__java_class", CfmlValue::String("java.lang.runtime"));
    shim.insert("__java_shim", CfmlValue::Bool(true));
    // Pre-populate memory values
    shim.insert("__free_memory", CfmlValue::Double(free_memory));
    shim.insert("__total_memory", CfmlValue::Double(total_memory));
    Ok(CfmlValue::strukt(shim))
}
```

**Methods:**

| Method | Implementation |
|--------|----------------|
| `Runtime.freeMemory()` | `sysinfo::System::free_memory()` |
| `Runtime.totalMemory()` | `sysinfo::System::total_memory()` |
| `Runtime.maxMemory()` | `sysinfo::System::total_memory()` (no JVM limit) |
| `Runtime.gc()` | No-op (log or count calls) |
| `Runtime.availableProcessors()` | `num_cpus::get()` |
| `Runtime.exec(String[])` | `std::process::Command` (blocking) or `tokio::process::Command` |

**Registration:**

```rust
"java.lang.runtime" => handle_java_system("getruntime", empty_args, &CfmlValue::Null),
// Extend handle_java_system to route Runtime.sub_method to Runtime handler
```

---

### Phase 7: Engine-Specific Feature Flags (LOW — ~1 day)

**Unlocks:** Graceful degradation for Hibernate/EHCache-dependent features

| Java Class | Strategy |
|------------|----------|
| `org.hibernate.*` | Add `#[cfg(feature = "hibernate")]` guard. Without feature: throw `CfmlError::runtime("Hibernate not supported on RustCFML")` |
| `net.sf.ehcache.*` | Same — throw "Use native CacheBox providers (CacheBoxProvider) on RustCFML" |

**Registration:**

```rust
// At top-level createObject dispatch — BEFORE java match
// Route hibernate/ehcache to error stubs:
if class_name.starts_with("org.hibernate.") {
    return if cfg!(feature = "hibernate") {
        // unimplemented — could use actual Hibernate via JNI
        Err(CfmlError::runtime("Hibernate support is not yet implemented"))
    } else {
        Err(CfmlError::runtime("Hibernate is not available on RustCFML"))
    };
}
```

---

### Phase 8 (Bonus): ColdBox-Specific Shims (LOW — ~3 days)

These are Java classes that ColdBox creates but aren't in the JDK — can't be shimmed, must be replaced in ColdBox source:

| Reference | File | Strategy |
|-----------|------|----------|
| `createObject("java","org.hibernate.Version")` | `system/core/util/Util.cfc:507` | Add `server.keyExists("rustcfml")` guard |
| `createObject("java", mapping.getPath())` in Builder | `system/ioc/Builder.cfc:367-376` | WireBox already ported — no-op |
| `isInstanceOf(cacheSession, "net.sf.ehcache.Cache")` | `system/cache/providers/CFProvider.cfc:494` | Adobe CF only — will not execute on RustCFML |
| `catch (org.hibernate.TransientObjectException)` | `system/core/dynamic/ObjectPopulator.cfc:683` | ORM → wrap in `catch(any)` |

---

## Implementation Architecture

### 1. File Organization

```
crates/cfml-vm/src/
├── java_shims.rs                    # Existing — keeps all handle_java_* functions
├── java_shims/                      # NEW — modular subdirectory (optional, keep flat if preferred)
│   ├── mod.rs                       # Re-exports
│   ├── handle_java_zoneid.rs
│   ├── handle_java_executor.rs
│   ├── handle_java_softreference.rs
│   └── ...
```

**Recommendation:** Keep all shims in `java_shims.rs` until it exceeds ~3000 lines, then split by module. Start new work in `java_shims.rs` for consistency.

### 2. Shared State

Add to `CfmlVm` struct in `lib.rs`:

```rust
pub struct CfmlVm {
    // ... existing fields ...

    /// Shared state for Java shims that need background tasks or global pools
    pub java_shim_state: JavaShimState,
}

pub struct JavaShimState {
    /// SoftReference sweeper state — used by Phase 3
    pub soft_ref_registry: Arc<Mutex<Vec<SoftRefEntry>>>,
    pub soft_ref_sweeper_tx: Option<crossbeam::channel::Sender<()>>,

    /// Executor pools — used by Phase 2
    pub executor_pools: Arc<Mutex<HashMap<String, Box<dyn ExecutorPool + Send>>>>,

    /// Reference queue channel
    pub ref_queue_registry: Arc<Mutex<HashMap<String, crossbeam::channel::Receiver<...>>>>,
}
```

### 3. Background Tasks

Spawned once during `CfmlVm::init`:

```rust
// Phase 3: SoftReference sweeper
let (sweeper_tx, sweeper_rx) = crossbeam::channel::bounded::<()>(1);
let registry = state.soft_ref_registry.clone();
std::thread::spawn(move || {
    let mut sys = sysinfo::System::new();
    loop {
        // Wait 30s or until shutdown
        if sweeper_rx.recv_timeout(Duration::from_secs(30)).is_ok() {
            break; // shutdown signal
        }
        sys.refresh_memory();
        let usage = sys.used_memory() as f64 / sys.total_memory() as f64;
        if usage > 0.8 {
            sweep_oldest_refs(&registry, (usage - 0.8) * 1000.0);
        }
    }
});
```

### 4. Registration Pattern

All shims follow the existing pattern in `lib.rs`:

```rust
// createObject("java", "fully.qualified.ClassName") → init dispatch
"java.time.duration" => handle_java_duration("init", empty_args, &CfmlValue::Null),

// Method dispatch on shim objects
"java.time.duration" => handle_java_duration(&m, all_args, object),
```

---

## Effort Summary

| Phase | Description | Effort | Lines of Code | Unlocks |
|-------|-------------|--------|---------------|---------|
| 1 | java.time.* | 2 weeks | ~800 | DateTimeHelper, Scheduler, async time ops |
| 2 | java.util.concurrent.* | 3 weeks | ~2000 | AsyncManager, ExecutorBuilder, Futures |
| 3 | java.lang.ref.* | 1 week | ~500 | ConcurrentSoftReferenceStore |
| 4 | java.io.* serialization | 1 week | ~600 | ObjectMarshaller |
| 5 | java.net.* networking | 1 week | ~700 | SocketAppender, URI, Remoting |
| 6 | java.lang.Runtime + String | 3 days | ~200 | CacheBox memory stats |
| 7 | Engine feature flags | 1 day | ~50 | Graceful hibernate/ehcache errors |
| 8 | ColdBox source guards | 3 days | ~100 | Full compatibility |
| **Total** | | **~9 weeks** | **~5000** | |

---

## Dependencies to Add to Cargo.toml

```toml
[dependencies]
# Phase 1
time = { version = "0.3", features = ["parsing", "serde", "formatting"] }
tzdb = "0.5"

# Phase 2
futures = "0.3"
async-trait = "0.1"

# Phase 3
crossbeam = "0.8"
sysinfo = "0.30"

# Phase 4
bincode = "1.3"
serde = { version = "1", features = ["derive"] }

# Phase 5
url = "2"

# Phase 6
num_cpus = "1"

# Already in workspace
tokio = { version = "1", features = ["full"] }
```

---

## Verification Plan

After each phase, verify by running the relevant ColdBox test suite on RustCFML:

| Phase | ColdBox Test Runner | Tests Directory |
|-------|---------------------|-----------------|
| 1 | `tests/runner-core.cfm` | Async manager, scheduler tests |
| 2 | `tests/runner-async.cfm` | `tests/specs/async/` |
| 3 | `tests/runner-cachebox.cfm` | Cache soft reference store tests |
| 4 | — | Manual test of ObjectMarshaller |
| 5 | — | Manual test of socket appender |
| 6 | `tests/runner-core.cfm` | CacheBox stats report |
| 7 | `tests/runner-integration.cfm` | Full integration suite |
| 8 | `tests/runner-integration.cfm` | Full integration suite |

---

## Quick Start (Week 1 Sprint)

For fastest ColdBox compatibility, implement these in order:

1. **`TimeUnit`** — ~50 lines, trivial enum, unblocks Scheduler
2. **`ZoneId` / `ZoneOffset`** — ~100 lines, unblocks DateTimeHelper
3. **`Duration` + `Period`** — ~150 lines, unblocks delay/period scheduling
4. **`ExecutorService` stub** — ~200 lines, unblocks basic AsyncManager
5. **`Future`/`CompletableFuture`** — ~300 lines, unblocks async execution
6. **`SoftReference`** — ~200 lines, unblocks cache soft reference store

These six give **~80% coverage** of the remaining ColdBox Java interop with minimal code.
