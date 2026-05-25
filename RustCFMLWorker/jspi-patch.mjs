// Post-build patch: wrap the wasm `fetch` + `scheduled` exports in
// WebAssembly.promising so that the JSPI Suspending imports
// (cfml_jspi_d1_query, cfml_jspi_do_fetch) can suspend the wasm stack.
//
// worker-build (0.8.x) does not do this itself. The generated index.js
// emits wrappers like:
//
//   function it(r,t,e){let n;return u(),n=s.fetch(o(r),o(t),o(e)),l(n)}
//
// We rewrite those into async wrappers that call the promising
// version, await its resolution, and feed the result through the
// jsval slot helper `l`.

import { readFileSync, writeFileSync } from "node:fs";

const path = "build/index.js";
let src = readFileSync(path, "utf8");

// Hoist a single promising wrapper per export.
//
// The init line we anchor to is `s.__wbindgen_start();` — present in
// every wasm-bindgen output. We insert promising wrappers right after.
const initAnchor = "s.__wbindgen_start();";
if (!src.includes(initAnchor)) {
  console.error("jspi-patch: anchor `s.__wbindgen_start();` not found");
  process.exit(1);
}
const wrappers =
  "var __jspi_fetch=WebAssembly.promising(s.fetch);" +
  "var __jspi_scheduled=WebAssembly.promising(s.scheduled);";
src = src.replace(initAnchor, initAnchor + wrappers);

// Find each `function NAME(r,...){let n;return u(),n=s.EXPORT(...),l(n)}`
// and rewrite to async + await + use the promising wrapper.
function rewriteExport(exportName, promisingVar) {
  // Use non-greedy `.*?` for the inner args so nested parens
  // (`o(r),o(t),o(e)`) don't break the match.
  const re = new RegExp(
    String.raw`function ([A-Za-z_$][\w$]*)\(([^)]*)\)\{let n;return u\(\),n=s\.` +
      exportName +
      String.raw`\((.*?)\),l\(n\)\}`,
    "g",
  );
  let count = 0;
  src = src.replace(re, (_m, fnName, params, args) => {
    count++;
    return `async function ${fnName}(${params}){u();return l(await ${promisingVar}(${args}))}`;
  });
  if (count === 0) {
    console.error(`jspi-patch: no wrapper found for s.${exportName}`);
    process.exit(1);
  }
  console.log(`jspi-patch: rewrote ${count} wrapper(s) for s.${exportName}`);
}

rewriteExport("fetch", "__jspi_fetch");
rewriteExport("scheduled", "__jspi_scheduled");

// ────────────────────────────────────────────────────────────────────
// Bypass the wasm-bindgen JS adapter for Suspending imports.
//
// wasm-bindgen emits, for each snippet import (even ones that take
// only primitives), a thin adapter:
//
//   __wbg_cfml_jspi_d1_query_HASH: function(t,e,n,i) {
//       return Q(t>>>0, e>>>0, n>>>0, i>>>0)
//   }
//
// where `Q` is the `new WebAssembly.Suspending(async ...)` object. The
// JS function call breaks JSPI: the wasm import sees a regular JS
// function, not the Suspending — so the await inside never has
// anywhere to suspend the wasm stack to. The whole request hangs.
//
// Fix: replace the adapter with a direct reference to the Suspending
// variable. The arg values flowing across are i32/u32 pointers; the
// >>> 0 conversions were defensive but the wasm side is already i32,
// so dropping them is safe.
// ────────────────────────────────────────────────────────────────────
function bypassAdapter(importName, { required = false } = {}) {
  const re = new RegExp(
    String.raw`(__wbg_` + importName + String.raw`_[0-9a-f]+):function\([^)]*\)\{return ([A-Za-z_$][\w$]*)\(.*?\)\}`,
    "g",
  );
  let count = 0;
  src = src.replace(re, (_m, fullName, varName) => {
    count++;
    return `${fullName}:${varName}`;
  });
  if (count === 0) {
    if (required) {
      console.error(`jspi-patch: adapter for ${importName} not found`);
      process.exit(1);
    }
    console.log(`jspi-patch: adapter for ${importName} absent (tree-shaken) — skipping`);
    return;
  }
  console.log(`jspi-patch: bypassed wasm-bindgen adapter for ${importName}`);
}

bypassAdapter("cfml_jspi_d1_query", { required: true });
bypassAdapter("cfml_jspi_do_fetch"); // optional — DO uses plain async

writeFileSync(path, src);
console.log("jspi-patch: build/index.js patched");
