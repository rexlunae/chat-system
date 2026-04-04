//! [`ChatServer`] trait for server-side implementations.

use crate::message::Message;
use anyhow::Result;
use async_trait::async_trait;

/// A trait for implementing chat servers.
#[async_trait]
pub trait ChatServer: Send + Sync {
    async fn run<F, Fut>(&mut self, handler: F) -> Result<()>
    where
        F: Fn(Message) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<Option<String>>> + Send + 'static;

    fn address(&self) -> &str;
    async fn shutdown(&mut self) -> Result<()>;
}
