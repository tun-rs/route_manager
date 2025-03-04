#[cfg(feature = "async")]
mod tokio;
#[cfg(feature = "async")]
pub(crate) use tokio::*;
#[cfg(all(feature = "async_io", not(feature = "async")))]
mod async_io;
#[cfg(all(feature = "async_io", not(feature = "async")))]
pub(crate) use async_io::*;
