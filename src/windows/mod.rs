// See https://github.com/johnyburd/net-route/blob/main/src/platform_impl/windows.rs

use crate::common::Route;
use crate::RouteChange;
use flume::{Receiver, Sender};
use std::io;
use std::net::IpAddr;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::os::windows::raw::HANDLE;
use std::sync::{Arc, Mutex};
use windows_sys::Win32::Foundation::{BOOLEAN, ERROR_SUCCESS};
use windows_sys::Win32::NetworkManagement::IpHelper::{
    CancelMibChangeNotify2, CreateIpForwardEntry2, DeleteIpForwardEntry2, FreeMibTable,
    GetBestRoute2, GetIpForwardTable2, MibAddInstance, MibDeleteInstance, MibParameterNotification,
    NotifyRouteChange2, MIB_IPFORWARD_ROW2, MIB_IPFORWARD_TABLE2, MIB_NOTIFICATION_TYPE,
};
use windows_sys::Win32::Networking::WinSock::{AF_INET, AF_INET6, AF_UNSPEC, SOCKADDR_INET};
#[cfg(any(feature = "async", feature = "async_io"))]
pub(crate) mod async_route;
pub(crate) mod ffi;
#[cfg(any(feature = "async", feature = "async_io"))]
pub use async_route::*;
pub(crate) use ffi::*;

/// RouteListener for receiving route change events.
pub struct RouteListener {
    handle: Arc<Mutex<Option<RouteHandle>>>,
    receiver: Receiver<RouteChange>,
}
impl RouteListener {
    /// Creates a new RouteListener.
    pub fn new() -> io::Result<Self> {
        let mut handle: HANDLE = std::ptr::null_mut();
        let (sender, receiver) = flume::bounded::<RouteChange>(128);
        let mut sender = Box::new(sender);
        let ret = unsafe {
            NotifyRouteChange2(
                AF_UNSPEC,
                Some(callback),
                (sender.as_mut() as *mut _) as *mut _,
                BOOLEAN::from(false),
                &mut handle,
            )
        };
        if ret != ERROR_SUCCESS {
            return Err(io::Error::from_raw_os_error(ret as i32));
        }
        unsafe {
            Ok(RouteListener {
                handle: Arc::new(Mutex::new(Some((
                    OwnedHandle::from_raw_handle(handle),
                    sender,
                )))),
                receiver,
            })
        }
    }
    /// Listens for a route change event and returns a RouteChange.
    pub fn listen(&mut self) -> io::Result<RouteChange> {
        self.receiver
            .recv()
            .map_err(|_| io::Error::other("shutdown"))
    }

    /// Retrieves a shutdown handle for the RouteListener.
    #[cfg(feature = "shutdown")]
    pub fn shutdown_handle(&self) -> io::Result<RouteListenerShutdown> {
        Ok(RouteListenerShutdown {
            handle: self.handle.clone(),
        })
    }
}
fn shutdown(handle: &Mutex<Option<RouteHandle>>) {
    if let Some((handle, sender)) = handle.lock().unwrap().take() {
        unsafe {
            CancelMibChangeNotify2(handle.as_raw_handle());
        }
        drop(sender)
    }
}

/// Shutdown handle for the RouteListener, used to stop listening.
#[derive(Clone)]
#[cfg(feature = "shutdown")]
pub struct RouteListenerShutdown {
    handle: Arc<Mutex<Option<RouteHandle>>>,
}
type RouteHandle = (OwnedHandle, Box<Sender<RouteChange>>);
#[cfg(feature = "shutdown")]
impl RouteListenerShutdown {
    /// Shuts down the RouteListener.
    pub fn shutdown(&self) -> io::Result<()> {
        shutdown(&self.handle);
        Ok(())
    }
}
impl Drop for RouteListener {
    fn drop(&mut self) {
        shutdown(&self.handle);
    }
}
/// RouteManager is used for managing routes (adding, deleting, and listing).
pub struct RouteManager {
    _private: std::marker::PhantomData<()>,
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
    /// Lists all current routes.
    pub fn list(&mut self) -> io::Result<Vec<Route>> {
        let mut ptable: *mut MIB_IPFORWARD_TABLE2 = std::ptr::null_mut();

        let ret = unsafe { GetIpForwardTable2(AF_UNSPEC, &mut ptable as *mut _ as *mut _) };
        if ret != ERROR_SUCCESS {
            return Err(io::Error::from_raw_os_error(ret as i32));
        }

        let prows = unsafe {
            std::ptr::slice_from_raw_parts(
                &(*ptable).Table as *const _ as *const MIB_IPFORWARD_ROW2,
                (*ptable).NumEntries as usize,
            )
        };

        let entries = unsafe { (*ptable).NumEntries };
        let res = (0..entries)
            .map(|idx| unsafe { (*prows)[idx as usize] })
            .filter_map(|row| unsafe { row_to_route(&row) })
            .collect::<Vec<_>>();
        unsafe { FreeMibTable(ptable as *mut _ as *mut _) };
        Ok(res)
    }
    /// Route Lookup by Destination Address
    pub fn find_route(&mut self, dest_ip: &IpAddr) -> io::Result<Option<Route>> {
        unsafe {
            let mut row: MIB_IPFORWARD_ROW2 = std::mem::zeroed();
            let mut dest: SOCKADDR_INET = std::mem::zeroed();
            let mut best_source_address: SOCKADDR_INET = std::mem::zeroed();

            match dest_ip {
                IpAddr::V4(ipv4) => {
                    dest.si_family = AF_INET;
                    dest.Ipv4.sin_family = AF_INET;
                    dest.Ipv4.sin_addr.S_un.S_addr = u32::from(*ipv4).to_be();
                }
                IpAddr::V6(ipv6) => {
                    dest.si_family = AF_INET6;
                    dest.Ipv6.sin6_family = AF_INET6;
                    dest.Ipv6.sin6_addr.u.Byte = ipv6.octets();
                }
            }

            let err = GetBestRoute2(
                std::ptr::null_mut(),
                0,
                std::ptr::null(),
                &dest,
                0,
                &mut row,
                &mut best_source_address,
            );
            if err != ERROR_SUCCESS {
                return Err(io::Error::from_raw_os_error(err as i32));
            }
            Ok(row_to_route(&row))
        }
    }
    /// Adds a new route.
    pub fn add(&mut self, route: &Route) -> io::Result<()> {
        let row: MIB_IPFORWARD_ROW2 = route.try_into()?;

        let err = unsafe { CreateIpForwardEntry2(&row) };
        if err != ERROR_SUCCESS {
            return Err(io::Error::from_raw_os_error(err as i32));
        }
        Ok(())
    }
    /// Deletes an existing route.
    pub fn delete(&mut self, route: &Route) -> io::Result<()> {
        let row: MIB_IPFORWARD_ROW2 = route.try_into()?;
        let err = unsafe { DeleteIpForwardEntry2(&row) };
        if err != ERROR_SUCCESS {
            return Err(io::Error::from_raw_os_error(err as i32));
        }
        Ok(())
    }
}

unsafe extern "system" fn callback(
    callercontext: *const core::ffi::c_void,
    row: *const MIB_IPFORWARD_ROW2,
    notificationtype: MIB_NOTIFICATION_TYPE,
) {
    let tx = &*(callercontext as *const Sender<RouteChange>);

    if let Some(route) = ffi::row_to_route(row) {
        let event = match notificationtype {
            n if n == MibParameterNotification => RouteChange::Change(route),
            n if n == MibAddInstance => RouteChange::Add(route),
            n if n == MibDeleteInstance => RouteChange::Delete(route),
            _ => return,
        };
        _ = tx.send(event)
    }
}
