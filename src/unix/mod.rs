#[cfg(feature = "shutdown")]
mod shutdown;

#[cfg(any(feature = "async", feature = "async_io"))]
mod async_route;
#[cfg(any(feature = "async", feature = "async_io"))]
pub(crate) use async_route::*;
use libc::c_char;
#[cfg(feature = "shutdown")]
pub use shutdown::*;
use std::ffi::{CStr, CString};
use std::io;

pub fn if_name_to_index(name: &str) -> io::Result<u32> {
    let name = CString::new(name)?;
    let idx = unsafe { libc::if_nametoindex(name.as_ptr()) };
    if idx != 0 {
        Ok(idx)
    } else {
        Err(io::Error::last_os_error())
    }
}
pub fn if_index_to_name(index: u32) -> io::Result<String> {
    let mut ifname: [c_char; 256] = unsafe { std::mem::zeroed() };

    unsafe {
        if libc::if_indextoname(index as libc::c_uint, ifname.as_mut_ptr()).is_null() {
            Err(io::Error::last_os_error())
        } else {
            let ifname_str = CStr::from_ptr(ifname.as_ptr())
                .to_str()
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid UTF-8"))?;
            Ok(ifname_str.to_string())
        }
    }
}
