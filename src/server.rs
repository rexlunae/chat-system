//! [`ChatServer`] and [`ChatListener`] traits for server-side implementations.

use crate::message::Message;
use anyhow::Result;
use async_trait::async_trait;

/// A trait for network listeners that accept incoming connections for a
/// [`ChatServer`].
///
/// A single server may hold multiple listeners (e.g. different ports or
/// protocols).  Each listener is responsible only for accepting new connections
/// and handing them off to the server's event loop; protocol handling is done
/// by the server.
#[async_trait]
pub trait ChatListener: Send + Sync {
    /// The address this listener is (or will be) bound to.
    fn address(&self) -> &str;

    /// Shut down this listener, stopping the acceptance of new connections.
    async fn shutdown(&mut self) -> Result<()>;
}

/// A trait for implementing chat servers.
#[async_trait]
pub trait ChatServer: Send + Sync {
    async fn run<F, Fut>(&mut self, handler: F) -> Result<()>
    where
        F: Fn(Message) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<Option<String>>> + Send + 'static;

    /// The primary bind address of this server.
    ///
    /// For servers with multiple listeners, this returns the address of the
    /// first listener.  Use [`ChatServer::addresses`] to retrieve all of them.
    fn address(&self) -> &str;

    /// All addresses this server is (or will be) listening on.
    ///
    /// The default implementation returns a single-element slice containing
    /// [`ChatServer::address`], which is correct for servers with one listener.
    /// Servers that support multiple listeners should override this method.
    fn addresses(&self) -> Vec<&str> {
        vec![self.address()]
    }

    async fn shutdown(&mut self) -> Result<()>;
}
