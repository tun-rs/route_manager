use crate::unix_bsd::bind::*;
use crate::unix_bsd::{
    add_or_del_route_req, create_route_socket, deserialize_res, deserialize_res_change,
    list_routes, m_rtmsg,
};
use crate::Route;
use crate::{AsyncRoute, RouteChange};
use std::collections::VecDeque;
use std::io;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;

/// AsyncRouteListener for asynchronously receiving route change events.
pub struct AsyncRouteListener {
    list: VecDeque<RouteChange>,
    route_socket: AsyncRoute<UnixStream>,
}
impl AsyncRouteListener {
    /// Creates a new AsyncRouteListener.
    pub fn new() -> io::Result<Self> {
        let route_socket = create_route_socket()?;
        let route_socket = AsyncRoute::new(route_socket)?;
        Ok(AsyncRouteListener {
            list: Default::default(),
            route_socket,
        })
    }
    /// Asynchronously listens for a route change event and returns a RouteChange.
    pub async fn listen(&mut self) -> io::Result<RouteChange> {
        if let Some(route) = self.list.pop_front() {
            return Ok(route);
        }
        let mut buf = [0u8; 2048];
        let route_socket = &mut self.route_socket;
        loop {
            let read = route_socket.read_with(|s| s.read(&mut buf)).await?;

            deserialize_res_change(
                |route| {
                    self.list.push_back(route);
                },
                &buf[..read],
            )?;
            if let Some(route) = self.list.pop_front() {
                return Ok(route);
            }
        }
    }
}
/// AsyncRouteManager for asynchronously managing routes (adding, deleting, and listing).
pub struct AsyncRouteManager {
    _private: std::marker::PhantomData<()>,
}

impl AsyncRouteManager {
    /// Creates a new AsyncRouteManager.
    pub fn new() -> io::Result<AsyncRouteManager> {
        Ok(AsyncRouteManager {
            _private: std::marker::PhantomData,
        })
    }
    /// Retrieves a new instance of AsyncRouteListener.
    pub fn listener() -> io::Result<AsyncRouteListener> {
        AsyncRouteListener::new()
    }

    /// Asynchronously lists all current routes.
    /// **Note: On macOS and FreeBSD, this is not truly asynchronous.**
    pub async fn list(&mut self) -> io::Result<Vec<Route>> {
        list_routes()
    }
    /// Asynchronously adds a new route.
    pub async fn add(&mut self, route: &Route) -> io::Result<()> {
        add_route(route).await
    }
    /// Asynchronously deletes an existing route.
    pub async fn delete(&mut self, route: &Route) -> io::Result<()> {
        delete_route(route).await
    }
}

async fn add_route(route: &Route) -> io::Result<()> {
    add_or_del_route(route, RTM_ADD as u8).await
}
async fn delete_route(route: &Route) -> io::Result<()> {
    add_or_del_route(route, RTM_DELETE as u8).await
}

async fn add_or_del_route(route: &Route, rtm_type: u8) -> io::Result<()> {
    let rtmsg = add_or_del_route_req(route, rtm_type)?;
    let route_socket = create_route_socket()?;

    let mut route_socket = AsyncRoute::new(route_socket)?;

    route_socket
        .write_with(|s| s.write_all(rtmsg.slice()))
        .await?;

    let mut buf = [0u8; size_of::<m_rtmsg>()];
    let len = route_socket.read_with(|s| s.read(&mut buf)).await?;
    deserialize_res(|_, _| {}, &buf[..len])?;

    Ok(())
}
