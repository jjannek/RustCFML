<cfparam name="request.pageTitle" default="RustCFML on Cloudflare Workers">
<cfparam name="request.activeNav" default="">
<cfoutput>
<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>#request.pageTitle#</title>
<style>
:root {
    --bg: ##1a1a2e;
    --surface: ##16213e;
    --surface2: ##0f3460;
    --accent: ##e94560;
    --accent-hover: ##ff6b81;
    --text: ##eee;
    --text-dim: ##8892b0;
    --border: ##233554;
    --success: ##64ffda;
    --error: ##ff5370;
    --code-bg: ##0d1117;
    --font-mono: 'SF Mono', 'Fira Code', 'Cascadia Code', Consolas, monospace;
}
* { margin: 0; padding: 0; box-sizing: border-box; }
body {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    background: var(--bg);
    color: var(--text);
    min-height: 100vh;
}
.header {
    background: var(--surface);
    border-bottom: 1px solid var(--border);
    padding: 16px 24px;
    display: flex;
    align-items: center;
    gap: 16px;
}
.header h1 {
    font-size: 1.5rem;
    font-weight: 600;
}
.header h1 span { color: var(--accent); }
.header .subtitle {
    color: var(--text-dim);
    font-size: 0.85rem;
    margin-top: 2px;
}
.header .links {
    margin-left: auto;
    display: flex;
    gap: 16px;
}
.header .links a {
    color: var(--text-dim);
    text-decoration: none;
    font-size: 0.85rem;
    transition: color 0.2s;
}
.header .links a:hover { color: var(--accent); }
.container { max-width: 1100px; margin: 0 auto; padding: 24px; }
.nav-bar {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-bottom: 24px;
    align-items: center;
}
.nav-bar .label {
    color: var(--text-dim);
    font-size: 0.85rem;
    margin-right: 4px;
}
.nav-btn {
    background: var(--surface);
    color: var(--text-dim);
    border: 1px solid var(--border);
    padding: 6px 14px;
    border-radius: 6px;
    text-decoration: none;
    font-size: 0.8rem;
    font-family: var(--font-mono);
    transition: all 0.2s;
}
.nav-btn:hover {
    background: var(--surface2);
    color: var(--text);
    border-color: var(--accent);
}
.nav-btn.active {
    background: var(--surface2);
    color: var(--accent);
    border-color: var(--accent);
}
.panel {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 8px;
    margin-bottom: 16px;
    overflow: hidden;
}
.panel-header {
    background: var(--surface2);
    padding: 10px 16px;
    font-size: 0.8rem;
    font-weight: 600;
    color: var(--text-dim);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    border-bottom: 1px solid var(--border);
}
.panel-body { padding: 16px 20px; }
.panel-body p { margin-bottom: 12px; line-height: 1.6; }
.panel-body p:last-child { margin-bottom: 0; }
.cards {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
    gap: 16px;
    margin-bottom: 16px;
}
.card {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 20px;
    display: flex;
    flex-direction: column;
    transition: border-color 0.2s, transform 0.2s;
}
.card:hover {
    border-color: var(--accent);
    transform: translateY(-2px);
}
.card h3 {
    color: var(--accent);
    font-size: 1.05rem;
    margin-bottom: 8px;
}
.card .meta {
    color: var(--text-dim);
    font-size: 0.8rem;
    font-family: var(--font-mono);
    margin-bottom: 12px;
}
.card p {
    color: var(--text);
    font-size: 0.9rem;
    line-height: 1.55;
    margin-bottom: 16px;
    flex-grow: 1;
}
.card a.go {
    align-self: flex-start;
    background: var(--accent);
    color: white;
    text-decoration: none;
    padding: 8px 16px;
    border-radius: 6px;
    font-size: 0.85rem;
    font-weight: 600;
    transition: background 0.2s;
}
.card a.go:hover { background: var(--accent-hover); }
.kv {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 6px 16px;
    font-family: var(--font-mono);
    font-size: 0.85rem;
}
.kv dt { color: var(--text-dim); }
.kv dd { color: var(--success); word-break: break-all; }
pre.code {
    background: var(--code-bg);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 14px 16px;
    font-family: var(--font-mono);
    font-size: 0.85rem;
    line-height: 1.55;
    color: var(--text);
    overflow-x: auto;
    margin: 12px 0;
}
pre.output {
    background: var(--code-bg);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 14px 16px;
    font-family: var(--font-mono);
    font-size: 0.85rem;
    line-height: 1.55;
    color: var(--success);
    overflow-x: auto;
    margin: 12px 0;
    white-space: pre-wrap;
}
table.q {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.9rem;
    margin: 12px 0;
}
table.q th, table.q td {
    text-align: left;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
}
table.q th {
    color: var(--text-dim);
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.5px;
}
table.q td { color: var(--text); font-family: var(--font-mono); }
.footer {
    text-align: center;
    padding: 24px;
    color: var(--text-dim);
    font-size: 0.8rem;
}
.footer a { color: var(--accent); text-decoration: none; }
.footer a:hover { text-decoration: underline; }
.tag {
    display: inline-block;
    background: var(--surface2);
    color: var(--accent);
    padding: 2px 8px;
    border-radius: 4px;
    font-size: 0.7rem;
    font-family: var(--font-mono);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin-left: 8px;
    vertical-align: middle;
}
.greet-form { margin-top: 12px; display: flex; gap: 8px; flex-wrap: wrap; }
.forget-form { margin-top: 16px; }
.text-input {
    background: var(--code-bg);
    color: var(--text);
    border: 1px solid var(--border);
    padding: 10px 14px;
    border-radius: 6px;
    font-family: var(--font-mono);
    font-size: 0.9rem;
    min-width: 220px;
}
.btn {
    border: none;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.9rem;
    font-weight: 600;
    padding: 10px 24px;
    transition: background 0.2s;
}
.btn-primary { background: var(--accent); color: white; }
.btn-primary:hover { background: var(--accent-hover); }
.btn-quiet {
    background: var(--surface2);
    color: var(--text-dim);
    border: 1px solid var(--border);
    padding: 8px 16px;
    font-family: var(--font-mono);
    font-size: 0.85rem;
}
.btn-quiet:hover { color: var(--text); border-color: var(--accent); }
.hero {
    font-size: 1.4rem;
    color: var(--accent);
    margin-bottom: 16px;
}
</style>
</head>
<body>
<div class="header">
    <div>
        <h1>Rust<span>CFML</span> <span style="font-size: 0.85rem; color: var(--text-dim); font-weight: 400;">on Cloudflare Workers</span></h1>
        <div class="subtitle">#request.pageTitle#</div>
    </div>
    <div class="links">
        <a href="/">Dashboard</a>
        <a href="https://github.com/pixl8/RustCFML">GitHub</a>
    </div>
</div>
<div class="container">
<cfif len(request.activeNav)>
    <div class="nav-bar">
        <span class="label">Demo pages:</span>
        <a class="nav-btn <cfif request.activeNav eq "static">active</cfif>" href="/static.cfm">/static — no cookie</a>
        <a class="nav-btn <cfif request.activeNav eq "session">active</cfif>" href="/session.cfm">/session — writes</a>
        <a class="nav-btn <cfif request.activeNav eq "db">active</cfif>" href="/db.cfm">/db — D1 query</a>
        <a class="nav-btn <cfif request.activeNav eq "cfml">active</cfif>" href="/cfml.cfm">/cfml — language samples</a>
    </div>
</cfif>
</cfoutput>
