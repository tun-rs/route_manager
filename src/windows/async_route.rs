use crate::{Route, RouteChange, RouteListener, RouteManager};
use std::io;

/// AsyncRouteListener for asynchronously receiving route change events.
pub struct AsyncRouteListener {
    route_listener: RouteListener,
}
impl AsyncRouteListener {
    /// Creates a new AsyncRouteListener.
    pub fn new() -> io::Result<Self> {
        Ok(Self {
            route_listener: RouteListener::new()?,
        })
    }
    /// Asynchronously listens for a route change event and returns a RouteChange.
    pub async fn listen(&mut self) -> io::Result<RouteChange> {
        self.route_listener
            .receiver
            .recv_async()
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::Interrupted, "shutdown"))
    }
}
/// AsyncRouteManager for asynchronously managing routes (adding, deleting, and listing).
pub struct AsyncRouteManager {
    _private: std::marker::PhantomData<()>,
}
impl AsyncRouteManager {
    /// Creates a new AsyncRouteManager.
    pub fn new() -> io::Result<Self> {
        Ok(Self {
            _private: std::marker::PhantomData,
        })
    }
    /// Retrieves a new instance of AsyncRouteListener.
    pub fn listener() -> io::Result<AsyncRouteListener> {
        AsyncRouteListener::new()
    }
    /// Asynchronously lists all current routes.
    /// **Note: On Windows, this is not truly asynchronous.**
    pub async fn list(&mut self) -> io::Result<Vec<Route>> {
        RouteManager::new()?.list()
    }
    /// Asynchronously adds a new route.
    /// **Note: On Windows, this is not truly asynchronous.**
    pub async fn add(&mut self, route: &Route) -> io::Result<()> {
        RouteManager::new()?.add(route)
    }

    /// Asynchronously deletes an existing route.
    /// **Note: On Windows, this is not truly asynchronous.**
    pub async fn delete(&mut self, route: &Route) -> io::Result<()> {
        RouteManager::new()?.delete(route)
    }
}
