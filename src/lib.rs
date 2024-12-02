use std::marker::PhantomData;

pub mod base;
pub mod types;

#[cfg(feature = "libp2p")]
pub mod libp2p;

pub use crate::base::node::{Config, Node};
pub use crate::types::*;

use crate::types::result::Result;

pub struct NodeBuilder<T, State>
where
    T: Node,
{
    phantom: PhantomData<T>,
    state: State,
}

pub struct InitState {}

pub struct ConfigState<'a> {
    config: Config<'a>,
}

impl<'a, T> NodeBuilder<T, ConfigState<'a>>
where
    T: Node,
{
    pub fn with_config(self, config: Config<'a>) -> NodeBuilder<T, ConfigState<'a>> {
        NodeBuilder {
            phantom: PhantomData,
            state: ConfigState { config },
        }
    }

    pub async fn build(self) -> Result<T> {
        Ok(T::new(self.state.config).await?)
    }
}
