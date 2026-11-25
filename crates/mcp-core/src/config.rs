//! Layered configuration. Later layers win:
//!
//! 1. defaults of the server's config struct (`#[serde(default)]`)
//! 2. TOML file: `--config`, else `<PREFIX>_CONFIG`, else
//!    `<user config dir>/alpaca-mcp/<server>.toml` when it exists
//! 3. environment variables `<PREFIX>_<KEY>` (`__` separates nested keys)
//! 4. `--set key=value` (dots separate nested keys)

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use config::{Config, Environment, File, FileFormat};
use serde::de::DeserializeOwned;

use crate::ServerSpec;

/// Directory below the user's config directory that holds the server config files.
const CONFIG_DIR: &str = "alpaca-mcp";

/// Suffixes of `<PREFIX>_*` variables that control the runtime and are not config keys.
const RESERVED: [&str; 2] = ["CONFIG", "LOG"];

#[derive(Debug)]
pub struct LoadedConfig<C> {
    pub config: C,
    /// The configuration file that was read, if any.
    pub file: Option<PathBuf>,
}

/// Loads the configuration of a server. `env` is passed in so tests do not have
/// to mutate the process environment.
pub fn load_config<C: DeserializeOwned>(
    spec: &ServerSpec,
    explicit_file: Option<&Path>,
    mut env: HashMap<String, String>,
    overrides: &[(String, String)],
) -> Result<LoadedConfig<C>> {
    let prefix = spec.env_prefix();

    let env_file = env.get(&format!("{prefix}_CONFIG")).cloned();
    for suffix in RESERVED {
        env.remove(&format!("{prefix}_{suffix}"));
    }

    let requested = explicit_file
        .map(Path::to_path_buf)
        .or_else(|| env_file.filter(|path| !path.is_empty()).map(PathBuf::from));
    // A file the user asked for must exist; the default location is optional.
    let file = requested.or_else(|| default_file(spec).filter(|path| path.is_file()));

    let mut builder = Config::builder();
    if let Some(path) = &file {
        builder = builder.add_source(File::from(path.as_path()).format(FileFormat::Toml));
    }
    builder = builder.add_source(
        Environment::with_prefix(&prefix)
            .prefix_separator("_")
            .separator("__")
            .ignore_empty(true)
            .source(Some(env.into_iter().collect())),
    );
    for (key, value) in overrides {
        builder = builder
            .set_override(key.as_str(), value.as_str())
            .with_context(|| format!("invalid override key `{key}`"))?;
    }

    let config = builder
        .build()
        .and_then(Config::try_deserialize)
        .with_context(|| format!("invalid configuration for {}", spec.name))?;

    Ok(LoadedConfig { config, file })
}

fn default_file(spec: &ServerSpec) -> Option<PathBuf> {
    Some(
        dirs::config_dir()?
            .join(CONFIG_DIR)
            .join(format!("{}.toml", spec.name)),
    )
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    // A name no real installation uses, so a developer's own config file cannot
    // influence the tests.
    const SPEC: ServerSpec = ServerSpec {
        name: "core-test-mcp",
        version: "0.0.0",
        description: "test",
    };

    #[derive(Debug, Default, PartialEq, Deserialize)]
    #[serde(default, deny_unknown_fields)]
    struct Settings {
        name: String,
        limit: u32,
        verbose: bool,
        token: Option<String>,
        nested: Nested,
    }

    #[derive(Debug, Default, PartialEq, Deserialize)]
    #[serde(default, deny_unknown_fields)]
    struct Nested {
        url: String,
    }

    fn env(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    fn temp_file(name: &str, content: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("{name}-{}.toml", std::process::id()));
        std::fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn defaults_without_any_source() {
        let loaded = load_config::<Settings>(&SPEC, None, env(&[]), &[]).unwrap();
        assert_eq!(loaded.config, Settings::default());
        assert!(loaded.file.is_none());
    }

    #[test]
    fn later_layers_win() {
        let path = temp_file(
            "mcp-core-layers",
            "name = 'file'\nlimit = 1\n[nested]\nurl = 'file'\n",
        );
        let loaded = load_config::<Settings>(
            &SPEC,
            Some(&path),
            env(&[
                ("CORE_TEST_MCP_LIMIT", "2"),
                ("CORE_TEST_MCP_NESTED__URL", "env"),
            ]),
            &[("nested.url".into(), "set".into())],
        )
        .unwrap();
        std::fs::remove_file(path).unwrap();

        assert_eq!(loaded.config.name, "file");
        assert_eq!(loaded.config.limit, 2);
        assert_eq!(loaded.config.nested.url, "set");
    }

    #[test]
    fn environment_strings_are_converted_to_the_field_type() {
        let loaded = load_config::<Settings>(
            &SPEC,
            None,
            env(&[
                ("CORE_TEST_MCP_VERBOSE", "true"),
                ("CORE_TEST_MCP_TOKEN", "00123"),
                ("CORE_TEST_MCP_LOG", "debug"),
            ]),
            &[],
        )
        .unwrap();

        assert!(loaded.config.verbose);
        assert_eq!(loaded.config.token.as_deref(), Some("00123"));
    }

    #[test]
    fn config_variable_selects_the_file() {
        let path = temp_file("mcp-core-env-file", "name = 'from-env-file'\n");
        let loaded = load_config::<Settings>(
            &SPEC,
            None,
            env(&[("CORE_TEST_MCP_CONFIG", path.to_str().unwrap())]),
            &[],
        )
        .unwrap();
        std::fs::remove_file(path).unwrap();

        assert_eq!(loaded.config.name, "from-env-file");
    }

    #[test]
    fn unknown_keys_and_missing_files_are_errors() {
        let unknown = [("typo".to_string(), "1".to_string())];
        assert!(load_config::<Settings>(&SPEC, None, env(&[]), &unknown).is_err());

        let missing = Path::new("does-not-exist.toml");
        assert!(load_config::<Settings>(&SPEC, Some(missing), env(&[]), &[]).is_err());
    }
}
