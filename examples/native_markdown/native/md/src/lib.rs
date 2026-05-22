//! Markdown rendering as a native CFML module. Demonstrates a realistic
//! interop pattern (string in → string out via a third-party Rust crate)
//! rather than the toy state machines in the Counter/Tally example.

use pulldown_cmark::{html, Event, Options, Parser, Tag};
use rustcfml_cli::{CfmlResult, IndexMap, Value, Vm};

pub fn register(vm: &mut Vm) {
    vm.register_native_fn("rustMarkdown", render);
    vm.register_native_fn("rustMarkdownStats", stats);
}

/// `rustMarkdown(source)` — render a CommonMark string to HTML.
fn render(args: Vec<Value>) -> CfmlResult {
    let source = args.get(0).map(|v| v.as_string()).unwrap_or_default();
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(&source, opts);
    let mut out = String::with_capacity(source.len() + source.len() / 2);
    html::push_html(&mut out, parser);
    Ok(Value::String(out))
}

/// `rustMarkdownStats(source)` — return a struct of cheap counts. Shows
/// that a native fn can return a structured CFML value, not just a string.
fn stats(args: Vec<Value>) -> CfmlResult {
    let source = args.get(0).map(|v| v.as_string()).unwrap_or_default();
    let chars = source.chars().count() as i64;
    let words = source.split_whitespace().count() as i64;
    let lines = source.lines().count() as i64;

    let mut code_blocks: i64 = 0;
    let parser = Parser::new(&source);
    for ev in parser {
        if let Event::Start(Tag::CodeBlock(_)) = ev {
            code_blocks += 1;
        }
    }

    let mut s = IndexMap::new();
    s.insert("chars".into(), Value::Int(chars));
    s.insert("words".into(), Value::Int(words));
    s.insert("lines".into(), Value::Int(lines));
    s.insert("code_blocks".into(), Value::Int(code_blocks));
    Ok(Value::strukt(s))
}
