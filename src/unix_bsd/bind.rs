#![allow(warnings)]
#[cfg(not(docsrs))]
#[cfg(feature = "bindgen")]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(all(target_os = "freebsd", any(docsrs, not(feature = "bindgen"))))]
include!("freebsd_bindings.rs");

#[cfg(all(target_os = "openbsd", any(docsrs, not(feature = "bindgen"))))]
include!("openbsd_bindings.rs");

#[cfg(all(target_os = "macos", any(docsrs, not(feature = "bindgen"))))]
include!("macos_bindings.rs");

#[cfg(all(target_os = "netbsd", any(docsrs, not(feature = "bindgen"))))]
include!("netbsd_bindings.rs");
