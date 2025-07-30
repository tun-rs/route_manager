#![allow(warnings)]
#[cfg(not(docsrs))]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(all(target_os = "freebsd", docsrs))]
include!("freebsd_bindings.rs");

#[cfg(all(target_os = "openbsd", docsrs))]
include!("openbsd_bindings.rs");
