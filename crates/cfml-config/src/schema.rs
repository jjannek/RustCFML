//! Strongly-typed schema for `.cfconfig.json`.
//!
//! Every field is `#[serde(default)]`. Unknown keys are silently ignored so a
//! config file authored for Lucee/BoxLang loads cleanly even when it carries
//! engine-specific sections RustCFML cannot use.
//!
//! All string fields support `${env.VAR:default}` placeholders, expanded in a
//! single pass by [`RustCfmlConfig::expand_env`] right after parse.

use indexmap::IndexMap;
use serde::Deserialize;
use std::path::PathBuf;

use crate::env::expand_env_vars;

// ─────────────────────────────────────────────
// Root
// ─────────────────────────────────────────────

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct RustCfmlConfig {
    pub server: ServerCfg,
    pub runtime: RuntimeCfg,
    pub datasources: IndexMap<String, DatasourceCfg>,
    pub mappings: IndexMap<String, String>,
    #[serde(rename = "customTagPaths")]
    pub custom_tag_paths: Vec<String>,
    #[serde(rename = "mailServers")]
    pub mail_servers: Vec<MailServerCfg>,
    pub caches: IndexMap<String, CacheCfg>,
    #[serde(rename = "sessionStorage")]
    pub session_storage: String,
    pub logging: LoggingCfg,
    pub debugging: DebuggingCfg,
    pub security: SecurityCfg,
    #[serde(rename = "urlRewriting")]
    pub url_rewriting: UrlRewritingCfg,

    /// Set by the loader; not part of the JSON schema. `None` when the config
    /// was synthesised from defaults (no file found) or parsed from a string.
    #[serde(skip)]
    pub source_path: Option<PathBuf>,
}

// ─────────────────────────────────────────────
// Server
// ─────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ServerCfg {
    pub host: String,
    pub port: u16,
    pub webroot: String,
    #[serde(rename = "welcomeFiles")]
    pub welcome_files: Vec<String>,
    #[serde(rename = "cfmlExtensions")]
    pub cfml_extensions: Vec<String>,
    #[serde(rename = "maxConcurrentRequests")]
    pub max_concurrent_requests: u32,
    /// Bytes. `0` = unlimited.
    #[serde(rename = "maxRequestBodySize")]
    pub max_request_body_size: u64,
    /// Seconds. `0` = no timeout.
    #[serde(rename = "requestTimeout")]
    pub request_timeout: u32,
    pub http2: bool,
}

impl Default for ServerCfg {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: 8500,
            webroot: String::new(),
            welcome_files: vec!["index.cfm".into(), "index.htm".into(), "index.html".into()],
            cfml_extensions: vec!["cfm".into(), "cfc".into()],
            max_concurrent_requests: 0,
            max_request_body_size: 10 * 1024 * 1024,
            request_timeout: 0,
            http2: false,
        }
    }
}

// ─────────────────────────────────────────────
// Runtime
// ─────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct RuntimeCfg {
    #[serde(rename = "nullSupport")]
    pub null_support: bool,
    #[serde(rename = "dotNotationUpperCase")]
    pub dot_notation_upper_case: bool,
    pub locale: String,
    pub timezone: String,
    #[serde(rename = "whitespaceCompressionEnabled")]
    pub whitespace_compression_enabled: bool,
    #[serde(rename = "trustedCache")]
    pub trusted_cache: bool,
    /// `"days,hours,minutes,seconds"`.
    #[serde(rename = "applicationTimeout")]
    pub application_timeout: String,
    #[serde(rename = "sessionTimeout")]
    pub session_timeout: String,
    #[serde(rename = "clientTimeout")]
    pub client_timeout: String,
}

impl Default for RuntimeCfg {
    fn default() -> Self {
        Self {
            null_support: false,
            dot_notation_upper_case: true,
            locale: String::new(),
            timezone: String::new(),
            whitespace_compression_enabled: false,
            trusted_cache: false,
            application_timeout: "1,0,0,0".into(),
            session_timeout: "0,0,30,0".into(),
            client_timeout: "7,0,0,0".into(),
        }
    }
}

impl RuntimeCfg {
    /// Convert a `"d,h,m,s"` timeout string to total seconds. Returns `None`
    /// on parse failure so callers can fall back to a hard-coded default.
    pub fn parse_timeout_seconds(spec: &str) -> Option<u64> {
        let mut parts = spec.split(',').map(str::trim).map(str::parse::<u64>);
        let d = parts.next()?.ok()?;
        let h = parts.next()?.ok()?;
        let m = parts.next()?.ok()?;
        let s = parts.next()?.ok()?;
        Some(d * 86_400 + h * 3_600 + m * 60 + s)
    }
}

// ─────────────────────────────────────────────
// Datasources
// ─────────────────────────────────────────────

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct DatasourceCfg {
    pub driver: String,
    pub host: String,
    pub port: String,
    pub database: String,
    pub username: String,
    pub password: String,
    #[serde(rename = "connectionString")]
    pub connection_string: String,
    #[serde(rename = "connectionLimit")]
    pub connection_limit: i32,
    #[serde(rename = "connectionTimeout")]
    pub connection_timeout: u32,
    #[serde(rename = "idleTimeout")]
    pub idle_timeout: u32,
    pub timezone: String,
    pub default: bool,
}

impl DatasourceCfg {
    /// Build a connection string that the cfml-stdlib query driver layer can
    /// consume. Honors `connectionString` verbatim when provided; otherwise
    /// synthesises a URL from `driver` + host/port/database/credentials.
    /// Returns `None` for an unsupported / unrecognised driver.
    pub fn connection_url(&self) -> Option<String> {
        if !self.connection_string.is_empty() {
            return Some(self.connection_string.clone());
        }
        let driver = self.driver.to_ascii_lowercase();
        let creds = if self.username.is_empty() && self.password.is_empty() {
            String::new()
        } else if self.password.is_empty() {
            format!("{}@", self.username)
        } else {
            format!("{}:{}@", self.username, self.password)
        };
        let port = if self.port.is_empty() {
            String::new()
        } else {
            format!(":{}", self.port)
        };
        match driver.as_str() {
            "mysql" | "mariadb" => Some(format!(
                "mysql://{}{}{}/{}",
                creds, self.host, port, self.database
            )),
            "postgresql" | "postgres" => Some(format!(
                "postgresql://{}{}{}/{}",
                creds, self.host, port, self.database
            )),
            "mssql" | "sqlserver" => Some(format!(
                "mssql://{}{}{}/{}",
                creds, self.host, port, self.database
            )),
            "sqlite" => Some(format!("sqlite://{}", self.database)),
            _ => None,
        }
    }
}

// ─────────────────────────────────────────────
// Mail
// ─────────────────────────────────────────────

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct MailServerCfg {
    pub smtp: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub tls: bool,
    pub ssl: bool,
    pub timeout: u32,
}

// ─────────────────────────────────────────────
// Caches
// ─────────────────────────────────────────────

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct CacheCfg {
    pub provider: String,
    pub properties: CacheProperties,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct CacheProperties {
    #[serde(rename = "maxObjects")]
    pub max_objects: u64,
    #[serde(rename = "defaultTimeout")]
    pub default_timeout: u64,
    #[serde(rename = "evictionPolicy")]
    pub eviction_policy: String,
}

impl Default for CacheProperties {
    fn default() -> Self {
        Self {
            max_objects: 1000,
            default_timeout: 3600,
            eviction_policy: "LRU".into(),
        }
    }
}

// ─────────────────────────────────────────────
// Logging
// ─────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct LoggingCfg {
    #[serde(rename = "logsDirectory")]
    pub logs_directory: String,
    pub level: String,
    pub format: String,
    pub loggers: IndexMap<String, LoggerCfg>,
}

impl Default for LoggingCfg {
    fn default() -> Self {
        Self {
            logs_directory: String::new(),
            level: "warn".into(),
            format: "text".into(),
            loggers: IndexMap::new(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct LoggerCfg {
    pub level: String,
    pub appender: String,
}

// ─────────────────────────────────────────────
// Debugging
// ─────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct DebuggingCfg {
    pub enabled: bool,
    #[serde(rename = "errorTemplate")]
    pub error_template: String,
    #[serde(rename = "errorStatusCode")]
    pub error_status_code: bool,
    #[serde(rename = "showExecutionTime")]
    pub show_execution_time: bool,
}

impl Default for DebuggingCfg {
    fn default() -> Self {
        Self {
            enabled: false,
            error_template: String::new(),
            error_status_code: true,
            show_execution_time: false,
        }
    }
}

// ─────────────────────────────────────────────
// Security
// ─────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct SecurityCfg {
    pub sandbox: bool,
    #[serde(rename = "disallowedFunctions")]
    pub disallowed_functions: Vec<String>,
    #[serde(rename = "disallowedImports")]
    pub disallowed_imports: Vec<String>,
    #[serde(rename = "blockedPaths")]
    pub blocked_paths: Vec<String>,
    #[serde(rename = "csrfEnabled")]
    pub csrf_enabled: bool,
    #[serde(rename = "secureJSON")]
    pub secure_json: bool,
    #[serde(rename = "secureJSONPrefix")]
    pub secure_json_prefix: String,
}

impl Default for SecurityCfg {
    fn default() -> Self {
        Self {
            sandbox: false,
            disallowed_functions: Vec::new(),
            disallowed_imports: Vec::new(),
            blocked_paths: vec![
                "*.cfm.bak".into(),
                "*.cfm~".into(),
                "Application.cfc".into(),
                "*.config.cfm".into(),
            ],
            csrf_enabled: true,
            secure_json: false,
            secure_json_prefix: "//".into(),
        }
    }
}

// ─────────────────────────────────────────────
// URL rewriting
// ─────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct UrlRewritingCfg {
    #[serde(rename = "configFile")]
    pub config_file: String,
    pub enabled: bool,
}

impl Default for UrlRewritingCfg {
    fn default() -> Self {
        Self {
            config_file: "urlrewrite.xml".into(),
            enabled: true,
        }
    }
}

// ─────────────────────────────────────────────
// Env expansion
// ─────────────────────────────────────────────

impl RustCfmlConfig {
    /// Walk every string field and expand `${env.VAR:default}` placeholders
    /// in place. Called automatically after parse.
    pub fn expand_env(&mut self) {
        // server
        expand(&mut self.server.host);
        expand(&mut self.server.webroot);
        for s in &mut self.server.welcome_files {
            expand(s);
        }
        for s in &mut self.server.cfml_extensions {
            expand(s);
        }
        // runtime
        expand(&mut self.runtime.locale);
        expand(&mut self.runtime.timezone);
        expand(&mut self.runtime.application_timeout);
        expand(&mut self.runtime.session_timeout);
        expand(&mut self.runtime.client_timeout);
        // datasources
        for ds in self.datasources.values_mut() {
            expand(&mut ds.driver);
            expand(&mut ds.host);
            expand(&mut ds.port);
            expand(&mut ds.database);
            expand(&mut ds.username);
            expand(&mut ds.password);
            expand(&mut ds.connection_string);
            expand(&mut ds.timezone);
        }
        // mappings: keys are virtual paths (rarely templated), values are physical
        let new_mappings: IndexMap<String, String> = self
            .mappings
            .iter()
            .map(|(k, v)| (k.clone(), expand_env_vars(v)))
            .collect();
        self.mappings = new_mappings;
        // custom tag paths
        for s in &mut self.custom_tag_paths {
            expand(s);
        }
        // mail
        for m in &mut self.mail_servers {
            expand(&mut m.smtp);
            expand(&mut m.username);
            expand(&mut m.password);
        }
        // caches
        for c in self.caches.values_mut() {
            expand(&mut c.provider);
            expand(&mut c.properties.eviction_policy);
        }
        expand(&mut self.session_storage);
        // logging
        expand(&mut self.logging.logs_directory);
        expand(&mut self.logging.level);
        expand(&mut self.logging.format);
        for l in self.logging.loggers.values_mut() {
            expand(&mut l.level);
            expand(&mut l.appender);
        }
        // debugging
        expand(&mut self.debugging.error_template);
        // security
        for s in &mut self.security.disallowed_functions {
            expand(s);
        }
        for s in &mut self.security.disallowed_imports {
            expand(s);
        }
        for s in &mut self.security.blocked_paths {
            expand(s);
        }
        expand(&mut self.security.secure_json_prefix);
        // url rewriting
        expand(&mut self.url_rewriting.config_file);
    }
}

fn expand(s: &mut String) {
    if s.contains("${") {
        *s = expand_env_vars(s);
    }
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_object_uses_defaults() {
        let cfg: RustCfmlConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(cfg.server.host, "127.0.0.1");
        assert_eq!(cfg.server.port, 8500);
        assert_eq!(cfg.runtime.session_timeout, "0,0,30,0");
        assert!(cfg.security.csrf_enabled);
        assert!(cfg.url_rewriting.enabled);
    }

    #[test]
    fn unknown_keys_are_ignored() {
        let json = r#"{
            "server": {"port": 9000, "luceeOnlyKey": 42},
            "extensions": [{"id": "lucee-thing"}],
            "adminPassword": "secret"
        }"#;
        let cfg: RustCfmlConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.server.port, 9000);
        assert_eq!(cfg.server.host, "127.0.0.1"); // default preserved
    }

    #[test]
    fn datasource_parses() {
        let json = r#"{
            "datasources": {
                "myDSN": {
                    "driver": "mysql",
                    "host": "localhost",
                    "port": "3306",
                    "database": "mydb",
                    "default": true
                }
            }
        }"#;
        let cfg: RustCfmlConfig = serde_json::from_str(json).unwrap();
        let ds = cfg.datasources.get("myDSN").expect("missing dsn");
        assert_eq!(ds.driver, "mysql");
        assert!(ds.default);
    }

    #[test]
    fn timeout_parser_handles_typical_inputs() {
        assert_eq!(RuntimeCfg::parse_timeout_seconds("0,0,30,0"), Some(1800));
        assert_eq!(RuntimeCfg::parse_timeout_seconds("1,0,0,0"), Some(86_400));
        assert_eq!(RuntimeCfg::parse_timeout_seconds("0, 1, 0, 0"), Some(3600));
        assert_eq!(RuntimeCfg::parse_timeout_seconds("bad"), None);
        assert_eq!(RuntimeCfg::parse_timeout_seconds("1,2,3"), None);
    }

    #[test]
    fn datasource_connection_url_mysql() {
        let mut ds = DatasourceCfg::default();
        ds.driver = "mysql".into();
        ds.host = "db.example.com".into();
        ds.port = "3306".into();
        ds.database = "app".into();
        ds.username = "u".into();
        ds.password = "p".into();
        assert_eq!(
            ds.connection_url().unwrap(),
            "mysql://u:p@db.example.com:3306/app"
        );
    }

    #[test]
    fn datasource_connection_url_sqlite_path() {
        let mut ds = DatasourceCfg::default();
        ds.driver = "sqlite".into();
        ds.database = "./data/dev.db".into();
        assert_eq!(ds.connection_url().unwrap(), "sqlite://./data/dev.db");
    }

    #[test]
    fn datasource_connection_url_passthrough() {
        let mut ds = DatasourceCfg::default();
        ds.driver = "mysql".into();
        ds.connection_string = "mysql://override/db".into();
        assert_eq!(ds.connection_url().unwrap(), "mysql://override/db");
    }

    #[test]
    fn datasource_connection_url_unknown_driver() {
        let mut ds = DatasourceCfg::default();
        ds.driver = "h2".into();
        assert!(ds.connection_url().is_none());
    }

    #[test]
    fn env_expansion_runs_after_parse() {
        std::env::set_var("RUSTCFML_TEST_HOST_VAL", "db.internal");
        let json = r#"{
            "datasources": {
                "x": {
                    "driver": "mysql",
                    "host": "${env.RUSTCFML_TEST_HOST_VAL}",
                    "database": "${env.RUSTCFML_MISSING_DB:fallback_db}"
                }
            }
        }"#;
        let mut cfg: RustCfmlConfig = serde_json::from_str(json).unwrap();
        cfg.expand_env();
        let ds = cfg.datasources.get("x").unwrap();
        assert_eq!(ds.host, "db.internal");
        assert_eq!(ds.database, "fallback_db");
        std::env::remove_var("RUSTCFML_TEST_HOST_VAL");
    }
}
