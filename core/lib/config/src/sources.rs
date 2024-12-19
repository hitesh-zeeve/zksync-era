use std::{
    fs, io,
    path::{Path, PathBuf},
};

use anyhow::Context;
use smart_config::{ConfigSources, Environment, Prefixed, Yaml};

// FIXME: consensus
#[derive(Debug, Default)]
pub struct ConfigFilePaths {
    pub general: Option<PathBuf>,
    pub secrets: Option<PathBuf>,
    pub contracts: Option<PathBuf>,
    pub genesis: Option<PathBuf>,
    pub wallets: Option<PathBuf>,
}

impl ConfigFilePaths {
    fn read_yaml(path: &Path) -> anyhow::Result<Yaml> {
        let file =
            fs::File::open(path).with_context(|| format!("failed opening config file {path:?}"))?;
        let raw: serde_yaml::Mapping = serde_yaml::from_reader(io::BufReader::new(file))
            .with_context(|| format!("failed reading YAML map from {path:?}"))?;
        let filename = path.as_os_str().to_string_lossy();
        Yaml::new(&filename, raw)
            .with_context(|| format!("failed digesting YAML map from {path:?}"))
    }

    /// **Important.** This method is blocking.
    pub fn into_config_sources(self, env_prefix: &str) -> anyhow::Result<ConfigSources> {
        let mut sources = ConfigSources::default();
        sources.push(Environment::prefixed(env_prefix));

        if let Some(path) = &self.general {
            sources.push(Self::read_yaml(path)?);
        }
        if let Some(path) = &self.secrets {
            sources.push(Self::read_yaml(path)?);
        }

        // Prefixed sources
        if let Some(path) = &self.contracts {
            sources.push(Prefixed::new(Self::read_yaml(path)?, "contracts"));
        }
        if let Some(path) = &self.genesis {
            sources.push(Prefixed::new(Self::read_yaml(path)?, "genesis"));
        }
        if let Some(path) = &self.wallets {
            sources.push(Prefixed::new(Self::read_yaml(path)?, "wallets"));
        }

        Ok(sources)
    }
}
