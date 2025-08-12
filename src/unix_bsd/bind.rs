#![allow(warnings)]
#[cfg(not(docsrs))]
#[cfg(feature = "build-bindings")]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(all(target_os = "freebsd", any(docsrs, not(feature = "build-bindings"))))]
include!("freebsd_bindings.rs");

#[cfg(all(target_os = "openbsd", any(docsrs, not(feature = "build-bindings"))))]
include!("openbsd_bindings.rs");

#[cfg(all(target_os = "macos", any(docsrs, not(feature = "build-bindings"))))]
include!("macos_bindings.rs");

#[cfg(all(target_os = "netbsd", any(docsrs, not(feature = "build-bindings"))))]
include!("netbsd_bindings.rs");
