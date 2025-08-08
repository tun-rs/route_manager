#![allow(warnings)]
#[cfg(not(docsrs))]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(all(target_os = "freebsd", docsrs))]
include!("freebsd_bindings.rs");

#[cfg(all(target_os = "openbsd", docsrs))]
include!("openbsd_bindings.rs");

#[cfg(all(target_os = "macos", docsrs))]
include!("macos_bindings.rs");

#[cfg(all(target_os = "netbsd", docsrs))]
include!("netbsd_bindings.rs");
