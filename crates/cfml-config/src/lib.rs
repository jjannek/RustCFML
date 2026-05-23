//! RustCFML configuration (`.cfconfig.json`).
//!
//! Engine-agnostic schema sharing the Ortus CFConfig filename convention with
//! the BoxLang-style flat, declarative layout. Unknown keys are silently
//! ignored so the same file can be consumed by Lucee, BoxLang, and RustCFML.
//!
//! Load entry points:
//!   * [`RustCfmlConfig::load`] — resolve the first matching file and parse it
//!   * [`RustCfmlConfig::from_str`] — parse JSON directly (used by tests and the
//!     embedded-VFS path for `--build` binaries)
//!
//! After parse, [`RustCfmlConfig::expand_env`] walks every string field and
//! resolves `${env.VAR:default}` placeholders against the process environment.

pub mod env;
pub mod resolve;
pub mod schema;

pub use env::expand_env_vars;
pub use resolve::{resolve_config_path, LoadMode};
pub use schema::{
    CacheCfg, DatasourceCfg, DebuggingCfg, LoggerCfg, LoggingCfg, MailServerCfg, RuntimeCfg,
    RustCfmlConfig, SecurityCfg, ServerCfg, UrlRewritingCfg,
};

use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read config file {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse config file {path}: {source}")]
    Parse {
        path: String,
        #[source]
        source: serde_json::Error,
    },
}

impl RustCfmlConfig {
    /// Search the supplied paths in order and parse the first file that exists.
    /// Returns the default config if no file is found.
    pub fn load(search_paths: &[std::path::PathBuf]) -> Result<Self, ConfigError> {
        let Some(found) = resolve::resolve_config_path(search_paths) else {
            log::debug!("no .cfconfig.json found in {} search paths", search_paths.len());
            return Ok(Self::default());
        };
        log::info!("loading cfconfig from {}", found.display());
        Self::from_file(&found)
    }

    pub fn from_file(path: &Path) -> Result<Self, ConfigError> {
        let bytes = std::fs::read(path).map_err(|source| ConfigError::Io {
            path: path.display().to_string(),
            source,
        })?;
        let mut cfg: Self = serde_json::from_slice(&bytes).map_err(|source| ConfigError::Parse {
            path: path.display().to_string(),
            source,
        })?;
        cfg.expand_env();
        cfg.source_path = Some(path.to_path_buf());
        Ok(cfg)
    }

    pub fn from_str(json: &str) -> Result<Self, ConfigError> {
        let mut cfg: Self =
            serde_json::from_str(json).map_err(|source| ConfigError::Parse {
                path: "<string>".to_string(),
                source,
            })?;
        cfg.expand_env();
        Ok(cfg)
    }
}
