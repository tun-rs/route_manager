use libc::RTM_DELROUTE;
use netlink_packet_core::{
    NetlinkHeader, NetlinkMessage, NetlinkPayload, NLM_F_ACK, NLM_F_CREATE, NLM_F_DUMP, NLM_F_EXCL,
    NLM_F_REQUEST,
};
use netlink_packet_route::route::{
    RouteAddress, RouteAttribute, RouteMessage, RouteProtocol, RouteScope, RouteType,
};
use netlink_packet_route::{AddressFamily, RouteNetlinkMessage};
use netlink_sys::{protocols::NETLINK_ROUTE, Socket, SocketAddr};
use std::collections::VecDeque;
use std::io;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, RawFd};

use crate::{Route, RouteChange, RouteKind};
#[cfg(any(feature = "async", feature = "async_io"))]
pub(crate) mod async_route;
#[cfg(any(feature = "async", feature = "async_io"))]
pub use async_route::*;

/// RouteListener for receiving route change events.
pub struct RouteListener {
    list: VecDeque<RouteChange>,
    route_socket: RouteSocket,
    #[cfg(feature = "shutdown")]
    pub(crate) shutdown_handle: crate::RouteListenerShutdown,
}
impl AsRawFd for RouteListener {
    fn as_raw_fd(&self) -> RawFd {
        self.route_socket.as_raw_fd()
    }
}

impl RouteListener {
    /// Creates a new RouteListener.
    pub fn new() -> io::Result<Self> {
        let mut route_socket = RouteSocket::new()?;
        route_socket.add_membership()?;
        #[cfg(feature = "shutdown")]
        route_socket.0.set_non_blocking(true)?;
        Ok(Self {
            list: Default::default(),
            route_socket,
            #[cfg(feature = "shutdown")]
            shutdown_handle: crate::RouteListenerShutdown::new()?,
        })
    }
    /// Listens for a route change event and returns a RouteChange.
    #[cfg(not(feature = "shutdown"))]
    pub fn listen(&mut self) -> io::Result<RouteChange> {
        if let Some(route) = self.list.pop_front() {
            return Ok(route);
        }
        let mut buf = vec![0; 4096];
        loop {
            let len = self.route_socket.recv(&mut buf)?;
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
impl RouteListener {
    /// Listens for a route change event and returns a RouteChange.
    #[cfg(feature = "shutdown")]
    pub fn listen(&mut self) -> io::Result<RouteChange> {
        if let Some(route) = self.list.pop_front() {
            return Ok(route);
        }
        let mut buf = vec![0; 4096];
        loop {
            self.wait()?;
            let len = match self.route_socket.recv(&mut buf) {
                Ok(list) => list,
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => continue,
                Err(e) => return Err(e),
            };
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
/// RouteManager is used for managing routes (adding, deleting, and listing).
pub struct RouteManager {
    _private: std::marker::PhantomData<()>,
}

pub(crate) struct RouteSocket(Socket);
impl AsRawFd for RouteSocket {
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_raw_fd()
    }
}
impl AsFd for RouteSocket {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.as_fd()
    }
}
impl RouteSocket {
    pub(crate) fn new() -> io::Result<Self> {
        Ok(Self(route_socket()?))
    }
    pub(crate) fn send(&self, buf: &[u8]) -> io::Result<usize> {
        self.0.send(buf, 0)
    }
    pub(crate) fn recv(&self, mut buf: &mut [u8]) -> io::Result<usize> {
        self.0.recv(&mut buf, 0)
    }
    pub(crate) fn add_membership(&mut self) -> io::Result<()> {
        self.0.add_membership(libc::RTNLGRP_IPV4_ROUTE)?;
        self.0.add_membership(libc::RTNLGRP_IPV6_ROUTE)?;
        Ok(())
    }
}

impl RouteManager {
    /// Creates a new RouteManager.
    pub fn new() -> io::Result<Self> {
        Ok(Self {
            _private: std::marker::PhantomData,
        })
    }
    /// Returns a new instance of RouteListener.
    pub fn listener() -> io::Result<RouteListener> {
        RouteListener::new()
    }

    /// Lists routes for a specific address family.
    fn list_family(socket: &RouteSocket, family: AddressFamily) -> io::Result<Vec<RouteChange>> {
        let mut buf = vec![0; 4096];
        let mut list = Vec::new();
        let req = list_route_req(family);
        socket.send(&req)?;
        loop {
            let len = socket.recv(&mut buf)?;
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

    /// Lists all current routes.
    pub fn list(&mut self) -> io::Result<Vec<Route>> {
        let socket = RouteSocket::new()?;

        // Query IPv4 routes
        let v4_result = Self::list_family(&socket, AddressFamily::Inet);

        // Query IPv6 routes
        let v6_result = Self::list_family(&socket, AddressFamily::Inet6);

        // Only fail if both queries failed. If at least one succeeded, return partial results.
        let list = match (v4_result, v6_result) {
            (Ok(v4), Ok(v6)) => [v4, v6].concat(),
            (Ok(v4), Err(_)) => v4,            // IPv4 succeeded
            (Err(_), Ok(v6)) => v6,            // IPv6 succeeded
            (Err(e), Err(_)) => return Err(e), // Both failed, return first error
        };
        Ok(convert_add_route(list))
    }
    /// Adds a new route.
    pub fn add(&mut self, route: &Route) -> io::Result<()> {
        let req = add_route_req(route)?;
        let socket = RouteSocket::new()?;
        socket.send(&req)?;
        let mut buf = vec![0; 4096];
        let len = socket.recv(&mut buf)?;
        deserialize_res(|_| {}, &buf[..len]).map(|_| ())
    }
    /// Deletes an existing route.
    pub fn delete(&mut self, route: &Route) -> io::Result<()> {
        let req = delete_route_req(route)?;
        let socket = RouteSocket::new()?;
        socket.send(&req)?;
        let mut buf = vec![0; 4096];
        let len = socket.recv(&mut buf)?;
        deserialize_res(|_| {}, &buf[..len]).map(|_| ())
    }
}
pub(crate) fn route_socket() -> io::Result<Socket> {
    let mut socket = Socket::new(NETLINK_ROUTE)?;
    let _port_number = socket.bind_auto()?.port_number();
    socket.connect(&SocketAddr::new(0, 0))?;
    Ok(socket)
}
pub(crate) fn convert_add_route(list: Vec<RouteChange>) -> Vec<Route> {
    list.into_iter()
        .filter_map(|v| {
            if let RouteChange::Add(route) = v {
                Some(route)
            } else {
                None
            }
        })
        .collect()
}

pub(crate) fn deserialize_res<F: FnMut(RouteChange)>(
    mut add_fn: F,
    receive_buffer: &[u8],
) -> io::Result<bool> {
    let mut offset = 0;
    loop {
        let bytes = &receive_buffer[offset..];
        if bytes.is_empty() {
            return Ok(false);
        }
        let rx_packet = <NetlinkMessage<RouteNetlinkMessage>>::deserialize(bytes)
            .map_err(|e| io::Error::other(format!("{e:?}")))?;
        match rx_packet.payload {
            NetlinkPayload::Done(_) => return Ok(true),
            NetlinkPayload::Error(e) => {
                if e.code.is_none() {
                    return Ok(true);
                }
                return Err(e.to_io());
            }
            NetlinkPayload::Noop => {}
            NetlinkPayload::Overrun(_) => {}
            NetlinkPayload::InnerMessage(msg) => match msg {
                RouteNetlinkMessage::NewRoute(msg) => add_fn(RouteChange::Add(msg.try_into()?)),
                RouteNetlinkMessage::DelRoute(msg) => add_fn(RouteChange::Delete(msg.try_into()?)),
                _ => {}
            },
            _ => {}
        }

        offset += rx_packet.header.length as usize;
        if rx_packet.header.length == 0 {
            return Ok(false);
        }
    }
}

impl TryFrom<RouteMessage> for Route {
    type Error = io::Error;

    fn try_from(msg: RouteMessage) -> Result<Self, Self::Error> {
        let mut destination = None;
        let mut gateway = None;
        let prefix = msg.header.destination_prefix_length;
        let source_prefix = msg.header.source_prefix_length;
        let mut source = None;
        let table = msg.header.table;
        let mut if_index = None;
        let mut metric = None;
        let mut pref_source = None;
        for x in msg.attributes {
            match x {
                RouteAttribute::Metrics(_) => {}
                RouteAttribute::MfcStats(_) => {}
                RouteAttribute::MultiPath(_) => {}
                RouteAttribute::CacheInfo(_) => {}
                RouteAttribute::Destination(addr) => {
                    destination = route_address_to_ip(addr);
                }
                RouteAttribute::Source(addr) => {
                    source = route_address_to_ip(addr);
                }
                RouteAttribute::Gateway(addr) => {
                    gateway = route_address_to_ip(addr);
                }
                RouteAttribute::PrefSource(addr) => {
                    pref_source = route_address_to_ip(addr);
                }
                RouteAttribute::Via(_) => {}
                RouteAttribute::NewDestination(_) => {}
                RouteAttribute::Preference(_) => {}
                RouteAttribute::EncapType(_) => {}
                RouteAttribute::Encap(_) => {}
                RouteAttribute::Expires(_) => {}
                RouteAttribute::MulticastExpires(_) => {}
                RouteAttribute::Uid(_) => {}
                RouteAttribute::TtlPropagate(_) => {}
                RouteAttribute::Iif(_) => {}
                RouteAttribute::Oif(v) => {
                    if_index = Some(v);
                }
                RouteAttribute::Priority(v) => metric = Some(v),
                RouteAttribute::Realm(_) => {}
                RouteAttribute::Table(_) => {}
                RouteAttribute::Mark(_) => {}
                RouteAttribute::Other(_) => {}
                _ => {}
            }
        }
        let destination = if let Some(destination) = destination {
            destination
        } else {
            match msg.header.address_family {
                AddressFamily::Inet => Ipv4Addr::UNSPECIFIED.into(),
                AddressFamily::Inet6 => Ipv6Addr::UNSPECIFIED.into(),
                _ => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "invalid destination family",
                    ))
                }
            }
        };
        let mut route = Route::new(destination, prefix).with_table(table);
        match msg.header.kind {
            RouteType::Unreachable => route = route.with_kind(RouteKind::Unreachable),
            RouteType::BlackHole => route = route.with_kind(RouteKind::Blackhole),
            RouteType::Prohibit => route = route.with_kind(RouteKind::Prohibit),
            _ => {}
        }
        if let Some(source) = source {
            route = route.with_source(source, source_prefix);
        }
        if let Some(if_index) = if_index {
            route = route.with_if_index(if_index);
            route.if_name = crate::unix::if_index_to_name(if_index).ok();
        }
        if let Some(gateway) = gateway {
            route = route.with_gateway(gateway);
        }
        if let Some(metric) = metric {
            route = route.with_metric(metric);
        }
        if let Some(pref_source) = pref_source {
            route = route.with_pref_source(pref_source);
        }
        Ok(route)
    }
}
impl TryFrom<&Route> for RouteMessage {
    type Error = io::Error;
    fn try_from(route: &Route) -> Result<Self, Self::Error> {
        route.check()?;
        let mut route_msg = RouteMessage::default();
        route_msg.header.address_family = if route.destination.is_ipv4() {
            AddressFamily::Inet
        } else {
            AddressFamily::Inet6
        };
        route_msg.header.destination_prefix_length = route.prefix;
        route_msg.header.protocol = RouteProtocol::Static;
        route_msg.header.scope = RouteScope::Universe;
        route_msg.header.kind = match route.kind {
            RouteKind::Unicast => RouteType::Unicast,
            RouteKind::Unreachable => RouteType::Unreachable,
            RouteKind::Blackhole => RouteType::BlackHole,
            RouteKind::Prohibit => RouteType::Prohibit,
        };
        route_msg.header.table = route.table;
        route_msg
            .attributes
            .push(RouteAttribute::Destination(route.destination.into()));
        // Only unicast routes have a next hop. When adding the other kinds,
        // IPv4 rejects a gateway or interface with EINVAL and IPv6 ignores them.
        if route.kind == RouteKind::Unicast {
            if let Some(gateway) = route.gateway {
                route_msg
                    .attributes
                    .push(RouteAttribute::Gateway(gateway.into()));
            }
            if let Some(if_index) = route.get_index() {
                route_msg.attributes.push(RouteAttribute::Oif(if_index));
            }
        }
        if let Some(metric) = route.metric {
            route_msg.attributes.push(RouteAttribute::Priority(metric));
        }
        if let Some(source) = route.source {
            route_msg.header.source_prefix_length = route.source_prefix;
            route_msg
                .attributes
                .push(RouteAttribute::Source(source.into()));
        }
        if let Some(pref_source) = route.pref_source {
            route_msg
                .attributes
                .push(RouteAttribute::PrefSource(pref_source.into()));
        }

        Ok(route_msg)
    }
}

pub(crate) fn list_route_req(family: AddressFamily) -> Vec<u8> {
    let mut nl_hdr = NetlinkHeader::default();
    nl_hdr.flags = NLM_F_REQUEST | NLM_F_DUMP;

    let mut route_msg = RouteMessage::default();
    route_msg.header.address_family = family;

    let mut packet = NetlinkMessage::new(
        nl_hdr,
        NetlinkPayload::from(RouteNetlinkMessage::GetRoute(route_msg)),
    );

    packet.finalize();

    let mut buf = vec![0; packet.header.length as usize];
    packet.serialize(&mut buf[..]);
    buf
}

pub(crate) fn add_route_req(route: &Route) -> io::Result<Vec<u8>> {
    let mut nl_hdr = NetlinkHeader::default();
    nl_hdr.flags = NLM_F_REQUEST | NLM_F_CREATE | NLM_F_EXCL | NLM_F_ACK;

    let mut packet = NetlinkMessage::new(
        nl_hdr,
        NetlinkPayload::from(RouteNetlinkMessage::NewRoute(route.try_into()?)),
    );

    packet.finalize();

    let mut buf = vec![0; packet.header.length as usize];
    packet.serialize(&mut buf[..]);
    Ok(buf)
}

pub(crate) fn delete_route_req(route: &Route) -> io::Result<Vec<u8>> {
    let mut nl_hdr = NetlinkHeader::default();
    nl_hdr.message_type = RTM_DELROUTE;
    nl_hdr.flags = NLM_F_REQUEST | NLM_F_ACK;

    let mut packet = NetlinkMessage::new(
        nl_hdr,
        NetlinkPayload::from(RouteNetlinkMessage::DelRoute(route.try_into()?)),
    );

    packet.finalize();

    let mut buf = vec![0; packet.header.length as usize];
    packet.serialize(&mut buf[..]);
    Ok(buf)
}

fn route_address_to_ip(addr: RouteAddress) -> Option<IpAddr> {
    match addr {
        RouteAddress::Inet(ip) => Some(IpAddr::V4(ip)),
        RouteAddress::Inet6(ip) => Some(IpAddr::V6(ip)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const REJECT_KINDS: [(RouteKind, RouteType); 3] = [
        (RouteKind::Unreachable, RouteType::Unreachable),
        (RouteKind::Blackhole, RouteType::BlackHole),
        (RouteKind::Prohibit, RouteType::Prohibit),
    ];

    fn v4_route() -> Route {
        Route::new("198.51.100.0".parse().unwrap(), 24)
    }

    fn v6_route() -> Route {
        Route::new("::".parse().unwrap(), 1)
    }

    fn to_msg(route: &Route) -> RouteMessage {
        RouteMessage::try_from(route).unwrap()
    }

    fn has_oif(msg: &RouteMessage) -> bool {
        msg.attributes
            .iter()
            .any(|a| matches!(a, RouteAttribute::Oif(_)))
    }

    fn expected_msg(
        route: &Route,
        kind: RouteType,
        attributes: Vec<RouteAttribute>,
    ) -> RouteMessage {
        let mut msg = RouteMessage::default();
        msg.header.address_family = if route.destination.is_ipv4() {
            AddressFamily::Inet
        } else {
            AddressFamily::Inet6
        };
        msg.header.destination_prefix_length = route.prefix;
        msg.header.protocol = RouteProtocol::Static;
        msg.header.scope = RouteScope::Universe;
        msg.header.kind = kind;
        msg.attributes = attributes;
        msg
    }

    fn parse(family: AddressFamily, destination: IpAddr, kind: RouteType) -> Route {
        let mut msg = RouteMessage::default();
        msg.header.address_family = family;
        msg.header.destination_prefix_length = 1;
        msg.header.kind = kind;
        msg.attributes
            .push(RouteAttribute::Destination(destination.into()));
        msg.attributes.push(RouteAttribute::Oif(1));
        Route::try_from(msg).unwrap()
    }

    #[test]
    fn unicast_is_default_and_message_unchanged() {
        let route = v4_route()
            .with_gateway("192.0.2.1".parse().unwrap())
            .with_if_index(1)
            .with_metric(100);
        assert_eq!(route.kind(), RouteKind::Unicast);

        let expected = expected_msg(
            &route,
            RouteType::Unicast,
            vec![
                RouteAttribute::Destination(route.destination.into()),
                RouteAttribute::Gateway(IpAddr::from([192, 0, 2, 1]).into()),
                RouteAttribute::Oif(1),
                RouteAttribute::Priority(100),
            ],
        );
        assert_eq!(to_msg(&route), expected);
    }

    #[test]
    fn kind_maps_to_route_type() {
        for (kind, route_type) in REJECT_KINDS {
            for route in [v4_route(), v6_route()] {
                let msg = to_msg(&route.with_kind(kind));
                assert_eq!(msg.header.kind, route_type, "{kind:?}");
            }
        }
    }

    #[test]
    fn non_unicast_omits_next_hop() {
        for (kind, route_type) in REJECT_KINDS {
            for route in [
                v4_route().with_if_index(1),
                v6_route().with_if_index(1),
                v6_route().with_if_name("lo".to_string()),
            ] {
                let route = route.with_kind(kind).with_metric(1024);
                let expected = expected_msg(
                    &route,
                    route_type,
                    vec![
                        RouteAttribute::Destination(route.destination.into()),
                        RouteAttribute::Priority(1024),
                    ],
                );
                assert_eq!(to_msg(&route), expected, "{route}");
            }
        }
    }

    #[test]
    fn check_rejects_gateway_on_non_unicast() {
        for (kind, _) in REJECT_KINDS {
            for (route, gateway) in [(v4_route(), "192.0.2.1"), (v6_route(), "fe80::1")] {
                let route = route.with_kind(kind).with_gateway(gateway.parse().unwrap());
                let err = route.check().unwrap_err();
                assert_eq!(err.to_string(), "only unicast routes can have a gateway");
                assert!(RouteMessage::try_from(&route).is_err());
            }

            assert!(v6_route().with_kind(kind).with_if_index(1).check().is_ok());
        }
    }

    #[test]
    fn parse_reject_route_types() {
        for (kind, route_type) in REJECT_KINDS {
            let route = parse(AddressFamily::Inet6, "::".parse().unwrap(), route_type);
            assert_eq!(route.kind(), kind);
            assert_eq!(route.if_index(), Some(1));

            let route = parse(
                AddressFamily::Inet,
                "198.51.100.0".parse().unwrap(),
                route_type,
            );
            assert_eq!(route.kind(), kind);
        }
    }

    #[test]
    fn parse_other_route_types_as_unicast() {
        for route_type in [RouteType::Unicast, RouteType::Local, RouteType::Throw] {
            let route = parse(AddressFamily::Inet6, "::".parse().unwrap(), route_type);
            assert_eq!(route.kind(), RouteKind::Unicast, "{route_type:?}");
        }
    }

    #[test]
    fn round_trip_keeps_kind() {
        let kinds = [
            RouteKind::Unicast,
            RouteKind::Unreachable,
            RouteKind::Blackhole,
            RouteKind::Prohibit,
        ];
        for kind in kinds {
            for (route, source, pref_source) in [
                (v4_route(), "192.0.2.0", "192.0.2.1"),
                (v6_route(), "2001:db8::", "2001:db8::1"),
            ] {
                let route = route
                    .with_table(254)
                    .with_metric(42)
                    .with_source(source.parse().unwrap(), 24)
                    .with_pref_source(pref_source.parse().unwrap())
                    .with_kind(kind);
                let parsed = Route::try_from(to_msg(&route)).unwrap();
                assert_eq!(parsed, route);
                assert_eq!(parsed.kind(), kind);
            }
        }
    }

    #[test]
    fn add_and_delete_requests_carry_kind() {
        let route = v4_route()
            .with_kind(RouteKind::Unreachable)
            .with_if_index(1);
        for req in [
            add_route_req(&route).unwrap(),
            delete_route_req(&route).unwrap(),
        ] {
            let packet = <NetlinkMessage<RouteNetlinkMessage>>::deserialize(&req).unwrap();
            let msg = match packet.payload {
                NetlinkPayload::InnerMessage(RouteNetlinkMessage::NewRoute(msg))
                | NetlinkPayload::InnerMessage(RouteNetlinkMessage::DelRoute(msg)) => msg,
                other => panic!("unexpected payload {other:?}"),
            };
            assert_eq!(msg.header.kind, RouteType::Unreachable);
            assert!(!has_oif(&msg));
        }
    }

    #[test]
    fn display_shows_kind_only_when_not_unicast() {
        let route = v6_route();
        assert_eq!(
            route.to_string(),
            "Route { destination: ::/1, gateway: None, if_index: None, if_name: None, \
             metric: None, table: 0, source: None, pref_source: None }"
        );
        assert_eq!(
            route.with_kind(RouteKind::Unreachable).to_string(),
            "Route { destination: ::/1, gateway: None, if_index: None, if_name: None, \
             metric: None, table: 0, source: None, kind: Unreachable, pref_source: None }"
        );
    }
}
