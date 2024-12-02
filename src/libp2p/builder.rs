use std::marker::PhantomData;

use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

use crate::base::node::Config;
use crate::types::result::Result;
use crate::{ConfigState, InitState, NodeBuilder};

use super::node;

pub struct Logger {
    with_ansi: bool,
    level_filter: LevelFilter,
}

impl Default for Logger {
    fn default() -> Self {
        Self {
            with_ansi: true,
            level_filter: LevelFilter::INFO,
        }
    }
}

pub struct ConfigStateExt<'a> {
    inner: ConfigState<'a>,
    logger: Logger,
}

impl<'a> NodeBuilder<node::Node, InitState> {
    pub fn libp2p() -> NodeBuilder<node::Node, ConfigState<'a>> {
        NodeBuilder {
            phantom: PhantomData,
            state: ConfigState {
                config: Config::default(),
            },
        }
    }
}

impl<'a> NodeBuilder<node::Node, ConfigState<'a>> {
    pub fn with_logger(
        self,
        logger: Logger,
    ) -> Result<NodeBuilder<node::Node, ConfigStateExt<'a>>> {
        Ok(NodeBuilder {
            phantom: PhantomData,
            state: ConfigStateExt {
                inner: self.state,
                logger,
            },
        })
    }
}

impl<'a> NodeBuilder<node::Node, ConfigStateExt<'a>> {
    pub async fn build(self) -> Result<node::Node> {
        tracing_subscriber::fmt()
            .with_ansi(self.state.logger.with_ansi)
            .with_env_filter(
                EnvFilter::builder()
                    .with_default_directive(self.state.logger.level_filter.into())
                    .from_env()?,
            )
            .init();

        NodeBuilder {
            phantom: PhantomData,
            state: self.state.inner,
        }
        .build()
        .await
    }
}
