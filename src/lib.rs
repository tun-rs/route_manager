mod common;
#[cfg(windows)]
mod windows;
pub use common::*;
#[cfg(windows)]
pub use windows::*;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::*;
#[cfg(any(target_os = "freebsd", target_os = "macos"))]
mod unix_bsd;
#[cfg(any(target_os = "freebsd", target_os = "macos"))]
pub use unix_bsd::*;
#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use crate::unix::*;
