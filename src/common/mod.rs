use std::cmp::Ordering;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::{fmt, io};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteChange {
    Add(Route),
    Delete(Route),
    Change(Route),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Route {
    pub(crate) destination: IpAddr,
    pub(crate) prefix: u8,
    pub(crate) gateway: Option<IpAddr>,
    pub(crate) if_name: Option<String>,
    pub(crate) if_index: Option<u32>,
    #[cfg(target_os = "linux")]
    pub(crate) table: u8,
    #[cfg(target_os = "linux")]
    pub(crate) source: Option<IpAddr>,
    #[cfg(target_os = "linux")]
    pub(crate) source_prefix: u8,
    #[cfg(target_os = "linux")]
    pub(crate) pref_source: Option<IpAddr>,
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    pub(crate) metric: Option<u32>,
    #[cfg(target_os = "windows")]
    pub(crate) luid: Option<u64>,
}
impl Route {
    pub fn destination(&self) -> IpAddr {
        self.destination
    }
    pub fn prefix(&self) -> u8 {
        self.prefix
    }
    pub fn gateway(&self) -> Option<IpAddr> {
        self.gateway
    }
    pub fn if_name(&self) -> Option<&String> {
        self.if_name.as_ref()
    }
    pub fn if_index(&self) -> Option<u32> {
        self.if_index
    }
    #[cfg(target_os = "linux")]
    pub fn table(&self) -> u8 {
        self.table
    }
    #[cfg(target_os = "linux")]
    pub fn source(&self) -> Option<IpAddr> {
        self.source
    }
    #[cfg(target_os = "linux")]
    pub fn source_prefix(&self) -> u8 {
        self.source_prefix
    }
    #[cfg(target_os = "linux")]
    pub fn pref_source(&self) -> Option<IpAddr> {
        self.pref_source
    }
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    pub fn metric(&self) -> Option<u32> {
        self.metric
    }
    #[cfg(target_os = "windows")]
    pub fn luid(&self) -> Option<u64> {
        self.luid
    }
}
impl Route {
    pub fn new(destination: IpAddr, prefix: u8) -> Self {
        Self {
            destination,
            prefix,
            gateway: None,
            if_name: None,
            if_index: None,
            #[cfg(target_os = "linux")]
            table: 0,
            #[cfg(target_os = "linux")]
            source: None,
            #[cfg(target_os = "linux")]
            source_prefix: 0,
            #[cfg(target_os = "linux")]
            pref_source: None,
            #[cfg(any(target_os = "windows", target_os = "linux"))]
            metric: None,
            #[cfg(target_os = "windows")]
            luid: None,
        }
    }
    /// Sets the gateway (next hop) for the route.
    pub fn with_gateway(mut self, gateway: IpAddr) -> Self {
        self.gateway = Some(gateway);
        self
    }
    /// Sets the network interface by name (e.g., "eth0").
    pub fn with_if_name(mut self, if_name: String) -> Self {
        self.if_name = Some(if_name);
        self
    }
    /// Sets the network interface by index.
    pub fn with_if_index(mut self, if_index: u32) -> Self {
        self.if_index = Some(if_index);
        self
    }
    /// (Linux only) Sets the routing table ID.
    #[cfg(target_os = "linux")]
    pub fn with_table(mut self, table: u8) -> Self {
        self.table = table;
        self
    }
    /// (Linux only) Sets the source address and prefix for policy-based routing.
    #[cfg(target_os = "linux")]
    pub fn with_source(mut self, source: IpAddr, prefix: u8) -> Self {
        self.source = Some(source);
        self.source_prefix = prefix;
        self
    }
    /// (Linux only) Sets the preferred source address for the route.
    #[cfg(target_os = "linux")]
    pub fn with_pref_source(mut self, pref_source: IpAddr) -> Self {
        self.pref_source = Some(pref_source);
        self
    }
    /// (Windows/Linux) Sets the route metric (priority).
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    pub fn with_metric(mut self, metric: u32) -> Self {
        self.metric = Some(metric);
        self
    }
    /// (Windows only) Sets the LUID (Local Unique Identifier) for the interface.
    #[cfg(target_os = "windows")]
    pub fn with_luid(mut self, luid: u64) -> Self {
        self.luid = Some(luid);
        self
    }
}
impl Route {
    pub fn check(&self) -> io::Result<()> {
        if self.destination.is_ipv4() {
            if self.prefix > 32 {
                return Err(io::Error::other("prefix error"));
            }
        } else if self.prefix > 128 {
            return Err(io::Error::other("prefix error"));
        }
        if let Some(index) = self.if_index {
            crate::if_index_to_name(index)?;
        }
        if let Some(gateway) = self.gateway {
            if gateway.is_ipv4() != self.destination.is_ipv4() {
                return Err(io::Error::other("gateway error"));
            }
        }
        if let Some(name) = self.if_name.as_ref() {
            let index = crate::if_name_to_index(name)?;
            if let Some(if_index) = self.if_index {
                if index != if_index {
                    return Err(io::Error::other("if_index mismatch"));
                }
            }
        }
        Ok(())
    }
    /// network address
    pub fn network(&self) -> IpAddr {
        Route::network_addr(self.destination, self.prefix)
    }
    fn network_addr(ip: IpAddr, prefix: u8) -> IpAddr {
        match ip {
            IpAddr::V4(ipv4) => {
                let ip = u32::from(ipv4);
                let mask = if prefix == 0 { 0 } else { !0 << (32 - prefix) };
                IpAddr::V4(Ipv4Addr::from(ip & mask))
            }
            IpAddr::V6(ipv6) => {
                let ip = u128::from(ipv6);
                let mask = if prefix == 0 {
                    0
                } else {
                    !0_u128 << (128 - prefix)
                };
                IpAddr::V6(Ipv6Addr::from(ip & mask))
            }
        }
    }
    /// Determine whether the target address is included in the route
    pub fn contains(&self, dest: &IpAddr) -> bool {
        if dest.is_ipv4() != self.destination.is_ipv4() {
            return false;
        }
        let route_network = self.network();
        let addr_network = Route::network_addr(*dest, self.prefix);
        route_network == addr_network
    }
    /// Subnet Mask
    pub fn mask(&self) -> IpAddr {
        match self.destination {
            IpAddr::V4(_) => IpAddr::V4(Ipv4Addr::from(
                u32::MAX.checked_shl(32 - self.prefix as u32).unwrap_or(0),
            )),
            IpAddr::V6(_) => IpAddr::V6(Ipv6Addr::from(
                u128::MAX.checked_shl(128 - self.prefix as u32).unwrap_or(0),
            )),
        }
    }
    #[allow(dead_code)]
    pub(crate) fn get_index(&self) -> Option<u32> {
        self.if_index.or_else(|| {
            if let Some(name) = &self.if_name {
                crate::if_name_to_index(name).ok()
            } else {
                None
            }
        })
    }
    #[allow(dead_code)]
    pub(crate) fn get_name(&self) -> Option<String> {
        self.if_name.clone().or_else(|| {
            if let Some(index) = &self.if_index {
                crate::if_index_to_name(*index).ok()
            } else {
                None
            }
        })
    }
}
impl PartialOrd<Self> for Route {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Route {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.prefix.cmp(&other.prefix) {
            #[cfg(any(target_os = "windows", target_os = "linux"))]
            Ordering::Equal => other.metric.cmp(&self.metric),
            v => v,
        }
    }
}
impl crate::RouteManager {
    /// Route Lookup by Destination Address
    #[cfg(not(target_os = "windows"))]
    pub fn find_route(&mut self, dest: &IpAddr) -> io::Result<Option<Route>> {
        let mut list = self.list()?;
        list.sort_by(|v1, v2| v2.cmp(v1));
        let rs = list
            .iter()
            .filter(|v| v.destination.is_ipv4() == dest.is_ipv4())
            .find(|v| v.contains(dest))
            .cloned();
        Ok(rs)
    }
}
impl fmt::Display for RouteChange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RouteChange::Add(route) => write!(f, "Add({})", route),
            RouteChange::Delete(route) => write!(f, "Delete({})", route),
            RouteChange::Change(route) => write!(f, "Change({})", route),
        }
    }
}
impl fmt::Display for Route {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Route {{ destination: {}/{}, gateway: ",
            self.destination, self.prefix
        )?;

        match self.gateway {
            Some(addr) => write!(f, "{}", addr),
            None => write!(f, "None"),
        }?;

        write!(f, ", if_index: ")?;

        match self.if_index {
            Some(index) => write!(f, "{}", index),
            None => write!(f, "None"),
        }?;
        write!(f, ", if_name: ")?;

        match &self.if_name {
            Some(if_name) => write!(f, "{}", if_name),
            None => write!(f, "None"),
        }?;

        #[cfg(any(target_os = "windows", target_os = "linux"))]
        {
            write!(f, ", metric: ")?;
            match self.metric {
                Some(m) => write!(f, "{}", m),
                None => write!(f, "None"),
            }?;
        }

        #[cfg(target_os = "windows")]
        {
            write!(f, ", luid: ")?;
            match self.luid {
                Some(l) => write!(f, "{}", l),
                None => write!(f, "None"),
            }?;
        }

        write!(f, " }}")
    }
}
