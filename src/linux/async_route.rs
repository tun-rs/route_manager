use crate::linux::{
    add_route_req, convert_add_route, delete_route_req, deserialize_res, list_route_req,
    RouteSocket,
};
use crate::AsyncRoute;
use crate::{Route, RouteChange};
use netlink_packet_route::AddressFamily;
use std::collections::VecDeque;
use std::io;
/// AsyncRouteListener for asynchronously receiving route change events.
pub struct AsyncRouteListener {
    list: VecDeque<RouteChange>,
    socket: AsyncRoute<RouteSocket>,
}
impl AsyncRouteListener {
    /// Creates a new AsyncRouteListener.
    pub fn new() -> io::Result<Self> {
        let mut route_socket = RouteSocket::new()?;
        route_socket.add_membership()?;
        let socket = AsyncRoute::new(route_socket)?;
        Ok(Self {
            list: Default::default(),
            socket,
        })
    }
    /// Asynchronously listens for a route change event and returns a RouteChange.
    pub async fn listen(&mut self) -> io::Result<RouteChange> {
        if let Some(route) = self.list.pop_front() {
            return Ok(route);
        }
        let mut buf = vec![0; 4096];
        loop {
            let len = self.socket.read_with(|s| s.recv(&mut buf[..])).await?;
            deserialize_res(
                |route| {
                    self.list.push_back(route);
                },
                &buf[..len],
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

    /// Asynchronously lists routes for a specific address family.
    async fn list_family(
        socket: &mut AsyncRoute<RouteSocket>,
        family: AddressFamily,
    ) -> io::Result<Vec<RouteChange>> {
        let mut buf = vec![0; 4096];
        let mut list = Vec::new();
        let req = list_route_req(family);
        socket.write_with(|s| s.send(&req)).await?;
        loop {
            let len = socket.read_with(|s| s.recv(&mut buf)).await?;
            let rs = deserialize_res(
                |route| {
                    list.push(route);
                },
                &buf[..len],
            )?;
            if rs {
                break;
            }
        }
        Ok(list)
    }

    /// Asynchronously lists all current routes.
    pub async fn list(&mut self) -> io::Result<Vec<Route>> {
        let mut socket = AsyncRoute::new(RouteSocket::new()?)?;

        // Query IPv4 routes
        let v4_result = Self::list_family(&mut socket, AddressFamily::Inet).await;

        // Query IPv6 routes
        let v6_result = Self::list_family(&mut socket, AddressFamily::Inet6).await;

        // Only fail if both queries failed. If at least one succeeded, return partial results.
        let list = match (v4_result, v6_result) {
            (Ok(v4), Ok(v6)) => [v4, v6].concat(),
            (Ok(v4), Err(_)) => v4,            // IPv4 succeeded
            (Err(_), Ok(v6)) => v6,            // IPv6 succeeded
            (Err(e), Err(_)) => return Err(e), // Both failed, return first error
        };
        Ok(convert_add_route(list))
    }
    /// Asynchronously adds a new route.
    pub async fn add(&mut self, route: &Route) -> io::Result<()> {
        let req = add_route_req(route)?;
        let mut socket = AsyncRoute::new(RouteSocket::new()?)?;
        socket.write_with(|s| s.send(&req)).await?;
        let mut buf = vec![0; 4096];
        let len = socket.read_with(|s| s.recv(&mut buf)).await?;
        deserialize_res(|_| {}, &buf[..len]).map(|_| ())
    }
    /// Asynchronously deletes an existing route.
    pub async fn delete(&mut self, route: &Route) -> io::Result<()> {
        let req = delete_route_req(route)?;
        let mut socket = AsyncRoute::new(RouteSocket::new()?)?;
        socket.write_with(|s| s.send(&req)).await?;
        let mut buf = vec![0; 4096];
        let len = socket.read_with(|s| s.recv(&mut buf)).await?;
        deserialize_res(|_| {}, &buf[..len]).map(|_| ())
    }
}
