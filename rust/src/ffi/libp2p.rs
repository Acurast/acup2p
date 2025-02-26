use tracing::level_filters::LevelFilter;

use crate::base::Node;
use crate::libp2p;
use crate::libp2p::LogConfig;
use crate::types::Result;

use super::{Config, LogLevel, FFI};

impl FFI<libp2p::Node> {
    pub async fn libp2p(config: Config) -> Result<Self> {
        let log = LogConfig {
            with_ansi: false,
            level_filter: config.log_level.into(),
        };

        let node = libp2p::Node::new(config.into_base(Some(log))).await?;

        Ok(FFI { node })
    }
}

impl From<LogLevel> for LevelFilter {
    fn from(value: LogLevel) -> Self {
        match value {
            LogLevel::Off => LevelFilter::OFF,
            LogLevel::Error => LevelFilter::ERROR,
            LogLevel::Warn => LevelFilter::WARN,
            LogLevel::Info => LevelFilter::INFO,
            LogLevel::Debug => LevelFilter::DEBUG,
            LogLevel::Trace => LevelFilter::TRACE,
        }
    }
}
