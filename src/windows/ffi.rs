use crate::Route;
use std::net::IpAddr;
use std::{io, mem};
use windows_sys::Win32::NetworkManagement::IpHelper::{
    ConvertInterfaceAliasToLuid, ConvertInterfaceIndexToLuid, ConvertInterfaceLuidToAlias,
    ConvertInterfaceLuidToIndex, InitializeIpForwardEntry, MIB_IPFORWARD_ROW2,
};
use windows_sys::Win32::NetworkManagement::Ndis::NET_LUID_LH;
use windows_sys::Win32::Networking::WinSock::{AF_INET, AF_INET6, IN6_ADDR, IN_ADDR};

pub(crate) fn encode_utf16(string: &str) -> Vec<u16> {
    use std::iter::once;
    string.encode_utf16().chain(once(0)).collect()
}

pub(crate) fn decode_utf16(string: &[u16]) -> String {
    let end = string.iter().position(|b| *b == 0).unwrap_or(string.len());
    String::from_utf16_lossy(&string[..end])
}
pub fn if_name_to_index(name: &str) -> io::Result<u32> {
    let luid = alias_to_luid(name)?;
    luid_to_index(&luid)
}
pub fn if_index_to_name(index: u32) -> io::Result<String> {
    let luid = index_to_luid(index)?;
    luid_to_alias(&luid)
}
pub(crate) fn alias_to_luid(alias: &str) -> io::Result<NET_LUID_LH> {
    let alias = encode_utf16(alias);
    let mut luid = unsafe { mem::zeroed() };
    match unsafe { ConvertInterfaceAliasToLuid(alias.as_ptr(), &mut luid) } {
        0 => Ok(luid),
        _err => Err(io::Error::last_os_error()),
    }
}

pub(crate) fn index_to_luid(index: u32) -> io::Result<NET_LUID_LH> {
    let mut luid = unsafe { mem::zeroed() };
    match unsafe { ConvertInterfaceIndexToLuid(index, &mut luid) } {
        0 => Ok(luid),
        _err => Err(io::Error::last_os_error()),
    }
}

pub(crate) fn luid_to_index(luid: &NET_LUID_LH) -> io::Result<u32> {
    let mut index = 0;
    match unsafe { ConvertInterfaceLuidToIndex(luid, &mut index) } {
        0 => Ok(index),
        _err => Err(io::Error::last_os_error()),
    }
}

pub(crate) fn luid_to_alias(luid: &NET_LUID_LH) -> io::Result<String> {
    // IF_MAX_STRING_SIZE + 1
    let mut alias = vec![0; 257];
    match unsafe { ConvertInterfaceLuidToAlias(luid, alias.as_mut_ptr(), alias.len()) } {
        0 => Ok(decode_utf16(&alias)),
        _err => Err(io::Error::last_os_error()),
    }
}

pub(crate) unsafe fn row_to_route(row: *const MIB_IPFORWARD_ROW2) -> Option<Route> {
    let dst_family = (*row).DestinationPrefix.Prefix.si_family;
    let dst = match dst_family {
        AF_INET => IpAddr::from(mem::transmute::<IN_ADDR, [u8; 4]>(
            (*row).DestinationPrefix.Prefix.Ipv4.sin_addr,
        )),
        AF_INET6 => IpAddr::from(mem::transmute::<IN6_ADDR, [u8; 16]>(
            (*row).DestinationPrefix.Prefix.Ipv6.sin6_addr,
        )),
        _ => panic!("Unexpected family {}", dst_family),
    };

    let dst_len = (*row).DestinationPrefix.PrefixLength;

    let nexthop_family = (*row).NextHop.si_family;

    let gateway = match nexthop_family {
        AF_INET => Some(IpAddr::from(std::mem::transmute::<IN_ADDR, [u8; 4]>(
            (*row).NextHop.Ipv4.sin_addr,
        ))),
        AF_INET6 => Some(IpAddr::from(std::mem::transmute::<IN6_ADDR, [u8; 16]>(
            (*row).NextHop.Ipv6.sin6_addr,
        ))),
        _ => None,
    };

    let mut route = Route::new(dst, dst_len)
        .with_if_index((*row).InterfaceIndex)
        .with_luid(std::mem::transmute::<NET_LUID_LH, u64>(
            (*row).InterfaceLuid,
        ))
        .with_metric((*row).Metric);
    route.if_name = if_index_to_name((*row).InterfaceIndex).ok();
    route.gateway = gateway;
    Some(route)
}

impl TryFrom<&Route> for MIB_IPFORWARD_ROW2 {
    type Error = io::Error;
    fn try_from(route: &Route) -> Result<Self, Self::Error> {
        route.check()?;
        let mut row: MIB_IPFORWARD_ROW2 = unsafe { std::mem::zeroed() };
        unsafe { InitializeIpForwardEntry(&mut row) };

        if let Some(ifindex) = route.get_index() {
            row.InterfaceIndex = ifindex;
        }

        if let Some(luid) = route.luid {
            row.InterfaceLuid = unsafe { std::mem::transmute::<u64, NET_LUID_LH>(luid) };
        }

        if let Some(gateway) = route.gateway {
            match gateway {
                IpAddr::V4(addr) => unsafe {
                    row.NextHop.si_family = AF_INET;
                    row.NextHop.Ipv4.sin_addr = mem::transmute::<[u8; 4], IN_ADDR>(addr.octets());
                },
                IpAddr::V6(addr) => unsafe {
                    row.NextHop.si_family = AF_INET6;
                    row.NextHop.Ipv6.sin6_addr =
                        mem::transmute::<[u8; 16], IN6_ADDR>(addr.octets());
                },
            }
        } else {
            // if we're not setting the gateway we need to explicitly set the family.
            row.NextHop.si_family = match route.destination {
                IpAddr::V4(_) => AF_INET,
                IpAddr::V6(_) => AF_INET6,
            };
        }

        row.DestinationPrefix.PrefixLength = route.prefix;
        match route.destination {
            IpAddr::V4(addr) => unsafe {
                row.DestinationPrefix.Prefix.si_family = AF_INET;
                row.DestinationPrefix.Prefix.Ipv4.sin_addr =
                    mem::transmute::<[u8; 4], IN_ADDR>(addr.octets());
            },
            IpAddr::V6(addr) => unsafe {
                row.DestinationPrefix.Prefix.si_family = AF_INET6;
                row.DestinationPrefix.Prefix.Ipv6.sin6_addr =
                    mem::transmute::<[u8; 16], IN6_ADDR>(addr.octets());
            },
        }

        if let Some(metric) = route.metric {
            row.Metric = metric;
        }

        Ok(row)
    }
}
