<cfscript>
// Tiny static-site-style renderer driven entirely from CFML — the markdown
// engine itself is the Rust pulldown_cmark crate, exposed as a single BIF.
//
// Try editing the markdown below or piping a file in:
//   ./markdown_demo < your_file.md

// Note: a single `#` inside a CFML double-quoted string starts variable
// interpolation. Use `##` to emit a literal `#` — markdown headings
// below are therefore `## ` in the source even though they render as
// `# heading` to pulldown_cmark.
markdown = "
## RustCFML + Rust
This is **bold**, this is *italic*, and this is `code`.

- one
- two
- three

```
let x = 42;
```

[Project repo](https://github.com/RustCFML/RustCFML)
";

// Default to the inline sample; if stdin has data, render that instead.
// (Reading from cli.stdin if provided.)
input = markdown;

writeOutput("=== Source ===" & chr(10));
writeOutput(input & chr(10));
writeOutput("=== Rendered HTML ===" & chr(10));
writeOutput(rustMarkdown(input) & chr(10));

writeOutput("=== Stats ===" & chr(10));
stats = rustMarkdownStats(input);
writeOutput("characters: " & stats.chars & chr(10));
writeOutput("words: " & stats.words & chr(10));
writeOutput("lines: " & stats.lines & chr(10));
writeOutput("code blocks: " & stats.code_blocks & chr(10));
</cfscript>
