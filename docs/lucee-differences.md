# RustCFML vs Lucee — cross-engine test differences

The test suite (`tests/runner.cfm`) runs on both RustCFML and Lucee 7.0.4 (pin
that version — `lucee@be` / `lucee@7` resolve to a broken 8.0.0-ALPHA whose
CommandBox cfconfig provider fails to load).

A small number of assertions cover RustCFML-specific features, deliberate
extensions, or by-design deltas. Those are wrapped in `if (isRustCFML())`
(see `tests/harness.cfm`) so they exercise RustCFML but are skipped on Lucee,
keeping a clean cross-engine bar. They are catalogued below for transparency,
followed by the **one genuinely unresolved divergence** that needs a decision.

`isRustCFML()` detects the engine via `server.coldfusion.productname`
(`"RustCFML"` here, `"Lucee"` on Lucee).

---

## Skipped-on-Lucee (intentional) — catalogue

These are *not* bugs; they are guarded only so the shared suite stays green.

### A. RustCFML-specific config / features the Lucee test server lacks
- **`disallowedFunctions` security policy** (`tests/config/test_cfconfig_security.cfm`)
  — enforced from RustCFML's `.cfconfig.json`; the CommandBox Lucee server has no
  equivalent loaded.
- **`this.datasources` in-memory sqlite datasources** (`tests/config/test_app_datasources.cfm`)
  — `rc_app_mem` / `rc_app_mem_str` / `rc_app_bad` are declared in
  `tests/Application.cfc` and backed by RustCFML's in-memory sqlite; they don't
  exist on Lucee.

### B. Deliberate RustCFML extensions beyond Lucee
- **`dateFormat()` single-quote literals** (`tests/stdlib/test_date_functions.cfm`)
  — RustCFML honours Java SimpleDateFormat-style `'...'` literals and `''` escapes
  in `dateFormat` masks (e.g. `dateFormat(d, "yyyy' year:'mmmm")` → `2026 year:May`).
  Lucee 7.0.4 honours them in `dateTimeFormat` but not `dateFormat`.
- **`createUniqueID("counter")`** (`tests/stdlib/test_create_unique_id.cfm`)
  — RustCFML adds a `"counter"` form returning an incrementing per-instance
  integer. Standard CF / Lucee ignore the argument.

### B1. Timezone display names — verified table, not full CLDR
RustCFML backs `getTimeZoneInfo()`, `setTimeZone()`/`getTimeZone()`,
`dateConvert()` and the `java.text.DateFormat` shim's `z`/`zzzz` fields with the
IANA database (`chrono-tz`). Offsets, DST transitions and instant↔wall-clock
conversion are faithful for **every** IANA zone. The four *display-name* fields
(`shortName`/`shortNameDST`/`name`/`nameDST`, and the `z`/`zzzz` pattern fields)
are CLDR data `chrono-tz` does not carry — Java even synthesises a *theoretical*
DST name for zones that never observe DST (e.g. `JDT` / "Japan Daylight Time").
RustCFML therefore serves these from a table captured **byte-for-byte from Lucee
7.0.4 / OpenJDK 21** (`crates/cfml-vm/src/tz.rs` `display_names`), covering the
common world zones. A valid zone that is **not** tabulated has full numeric
facts but no verified names, so name-bearing calls **fail loudly** (consistent
with the "Lucee-verified or fail loud" rule) rather than guess. Adding a zone is
a one-line table entry, ground-truthed against Lucee; the eventual full-coverage
path is `icu4x` (CLDR) behind an optional feature. `Z`/`X`/`O` numeric-offset
pattern fields are computed directly and need no table.

### C. Ordered-struct semantics (by design — `IndexMap` everywhere)
- **Auto-vivified struct key order** (`tests/core/test_subscript_autovivify.cfm`)
  and **struct-literal key order with a member-inc value**
  (`tests/core/test_member_index_incdec.cfm`) — RustCFML structs always preserve
  insertion order; Lucee's plain structs don't guarantee it in these cases.

### D. Implementation-defined
- **`csrfGenerateToken()` length** (`tests/config/test_cfconfig_security.cfm`)
  — RustCFML emits a 64-char hex token, Lucee 7.0.4 a 40-char one. cfdocs does
  not fix the length. (`csrfVerifyToken` round-trips on both.)

---

## E. UNRESOLVED DIVERGENCE — numeric-subscript auto-vivification

**Status:** open — needs a decision on which behaviour is correct. Guarded
RustCFML-only for now so it doesn't fail the Lucee run, but it is **not** settled.

**File:** `tests/core/test_subscript_autovivify.cfm`

**The case:**
```cfml
// rcfmlAutoVivArray is undefined here
rcfmlAutoVivArray[3] = "c";
```

**What each engine does:**

| Engine | `isArray(rcfmlAutoVivArray)` | length / shape |
|---|---|---|
| **RustCFML** | `true` | a 1-based, auto-growing **array** of length 3 (`[null, null, "c"]`) |
| **Lucee 7.0.4** | `false` | a **struct** with a single key `"3"` → `"c"` |

So assigning a numeric subscript into an *undefined* variable:
- **RustCFML** vivifies an **array** (and grows it to the index).
- **Lucee 7.0.4** vivifies a **struct** keyed by the numeric-as-string.

**Why this matters / why it's flagged:** the test's own comment claims this
behaviour is *"matching Lucee/ACF/BoxLang"* — but a live Lucee 7.0.4 run
**contradicts that**. One of these is true:
1. RustCFML is right and Lucee 7.0.4 differs (then the comment is fine but Lucee
   is the outlier), or
2. The test enshrines a RustCFML quirk that diverges from the reference engines
   (then RustCFML should arguably create a struct keyed `"3"`).

**To resolve (next session):**
- Check Adobe ColdFusion and BoxLang behaviour for `x[3] = "c"` on an undefined
  `x` (array vs struct, and whether it auto-grows). cfdocs / the BoxLang spec.
- If the reference engines make a **struct**, RustCFML's auto-viv-to-array is the
  bug — fix the vivification path (look for the subscript-assign-to-undefined
  handling in `crates/cfml-vm/src/lib.rs` / the codegen for `AssignTarget::ArrayAccess`
  on an undefined root) and update the test + comment.
- If they make an **array**, keep RustCFML's behaviour, correct the test comment
  (Lucee 7.0.4 is the outlier), and consider whether the guard can be removed
  (it can't while Lucee stays red, but the comment should say so).

**Note on `string`-key auto-viv:** the sibling case `x["alpha"] = 1` (string key
→ struct) is *not* in dispute — both engines make a struct; only the numeric case
diverges.
