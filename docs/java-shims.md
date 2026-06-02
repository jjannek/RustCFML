# Java Shim Support

[← Back to README](../README.md)

RustCFML has **no JVM under the hood**, so `createObject("java", …)` and `<cfobject type="java">` are served by hand-written **shims** — pure-Rust emulations of a small, curated set of Java classes that real-world CFML frameworks (ColdBox, Preside, Taffy, etc.) reach for. The goal is to run those libraries, not to reimplement the JDK.

```cfml
// Static method
now = createObject("java", "java.lang.System").currentTimeMillis();

// Constructor + chaining
sb = createObject("java", "java.lang.StringBuilder").init("Hello");
sb.append(", World").append("!");
writeOutput(sb.toString());   // Hello, World!

// Hashing
md = createObject("java", "java.security.MessageDigest").getInstance("SHA-256");
md.update("data");
digest = md.digest();         // Binary
```

> **Expect differences.** These are emulations, not the real classes. Anything outside the lists below is **not** shimmed. Class names are matched case-insensitively, so `java.lang.System` and `java.lang.system` both work.

## Important caveats

- **Unsupported classes return `null` silently.** `createObject("java", "java.util.HashMap")` does not throw — it returns null, and subsequent method calls fail in confusing ways. Stick to the supported list below.
- **Unsupported *methods* on a supported class also return `null`** (no error). For example `File.canRead()` is not implemented and yields null.
- **Regex uses the Rust [`regex`](https://docs.rs/regex) crate**, whose syntax is a close superset of common Java `Pattern` usage but is **not identical** (no backreferences/lookaround). Invalid patterns throw a clear `java.util.regex.Pattern: invalid pattern …` error.
- **Threads are stubs.** `Thread.sleep()` is a no-op; there is no real concurrency.
- **No true immutability/thread-safety.** `Collections.unmodifiableList()` / `synchronizedMap()` are identity operations (matching Lucee's practical behaviour), and the concurrent collections are single-threaded emulations.
- **`System.getProperty("java.version")` returns `"rustcfml"`** — a deliberate tell that there is no JVM.

## Shimmed classes

| Class (and aliases) | Supported methods |
|---|---|
| `java.security.MessageDigest` | `getInstance(algorithm)`, `update(data)`, `digest()`, `reset()`, static `isEqual(a, b)` |
| `java.util.UUID` | static `randomUUID()`, `toString()`, `getVersion()` (→ 4), `getVariant()` (→ 2) |
| `java.lang.Thread` | static `currentThread()`, `getName()`, `getThreadGroup()`, `getPriority()` (→ 5), `isDaemon()` (→ false), `sleep()` (no-op) |
| `java.lang.ThreadGroup` *(via `Thread.getThreadGroup()`)* | `getName()` |
| `java.net.InetAddress` | static `getLocalHost()`, static `getByName(host)`, `getHostName()`, `getHostAddress()`, `getCanonicalHostName()`, `toString()` |
| `java.io.File` | `init(path)`, `toString()`, `getAbsolutePath()`, `getCanonicalPath()`, `isAbsolute()`, `exists()`, `isFile()`, `isDirectory()`, `getName()`, `lastModified()`, `length()`, `toPath()` |
| `java.lang.System` | static `currentTimeMillis()`, `nanoTime()`, `getProperty(key)`, `getenv([name])`, and `System.out.println(...)` |
| `java.lang.StringBuilder` / `java.lang.StringBuffer` | `init([s])`, `append(v)`, `toString()`, `length()`, `clear()` |
| `java.util.TreeMap` | `init([struct])`, `put(k, v)`, `get(k)`, `keySet()`/`keys()` (sorted), `size()`, `containsKey(k)`, `isEmpty()` |
| `java.util.LinkedHashMap` | `init([struct])`, `put(k, v)`, `get(k)`, `keySet()`/`keys()` (insertion order), `size()`, `containsKey(k)`, `isEmpty()` |
| `java.util.concurrent.ConcurrentHashMap` | `init()`, `put(k, v)`, `putIfAbsent(k, v)`, `get(k)`, `remove(k)`, `containsKey(k)`, `keys()`/`keySet()`/`values()`, `size()`, `isEmpty()`, `clear()` |
| `java.util.concurrent.ConcurrentLinkedQueue` *(alias `…LinkedQueue`)* | `init()`, `offer(v)`, `poll()`, `peek()`, `size()`, `isEmpty()` |
| `java.util.Collections` | `list(e)`, `emptyList()`/`emptySet()`/`emptyMap()`, `sort(list)`, `reverse(list)`, and identity `unmodifiable*`/`synchronized*` wrappers |
| `java.nio.file.Paths` / `java.nio.file.Path` *(also via `File.toPath()`)* | static `get(s)`, `getParent()`, `isAbsolute()`, `toString()`, `toAbsolutePath()` |
| `java.util.regex.Pattern` | static/instance `compile(regex)`, `pattern()`/`toString()`, `matcher(input)` |
| `java.util.regex.Matcher` *(via `Pattern.matcher()`)* | `find()`, `matches()`, `lookingAt()`, `group([n])`, `groupCount()` |

## Known gaps

These are the most likely things to trip you up — methods on otherwise-supported classes that are **not** implemented (they return `null`):

- **`java.io.File`** — `canRead()`/`canWrite()`/`canExecute()`, `getParent()`/`getParentFile()`, `delete()`, `mkdir()`, `renameTo()`, `listFiles()`.
- **`java.lang.StringBuilder`** — `insert()`, `reverse()`, `delete()`, `substring()`, `replace()`.
- **`TreeMap` / `LinkedHashMap`** — `values()`, `entrySet()`, `clear()`, and (except on `ConcurrentHashMap`) `remove()`.
- **`java.nio.file.Paths`** — `getFileName()`, `getNameCount()`, `normalize()`, `relativize()`.
- **`java.util.regex`** — `Matcher.replaceAll()`/`replaceFirst()`/`split()`, `Pattern.matches()` (static), `Pattern.quote()`.

Whole classes commonly requested but **not** shimmed include `java.util.HashMap`/`ArrayList`, `java.lang.reflect.*`, `java.sql.*`, `java.io.InputStream`/`OutputStream`/`Reader`/`Writer`, and `javax.crypto.*` (use CFML's built-in `hash()`, `encrypt()`, `hmac()` instead).

If you hit a missing class or method that a framework needs, please [open an Issue](https://github.com/RustCFML/RustCFML/issues) with the call and the framework context — the shim set grows from real-world demand.

## Tests

The shims are exercised by the CFML suite under [`tests/java_shims/`](../tests/java_shims/) (e.g. `test_all.cfm`, `test_security.cfm`, `test_concurrent_map.cfm`, `test_stringbuilder.cfm`, `test_file.cfm`), which is also run against Lucee to confirm the emulated behaviour matches the reference engine. See **[Testing](testing.md)**.
