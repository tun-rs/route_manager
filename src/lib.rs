/*!
## Example:
### Asynchronous API
```rust,no_run
use route_manager::{AsyncRouteManager, Route};
use std::time::Duration;
#[cfg(feature = "async")]
#[tokio::main]
pub async fn main() {
    let mut route_listener = AsyncRouteManager::listener().unwrap();
    tokio::spawn(async move {
        while let Ok(route) = route_listener.listen().await {
            println!("listen {route}");
        }
    });
    // Need to set up the correct gateway
    let route = Route::new("192.168.2.0".parse().unwrap(), 24).with_if_index(1);
    let mut manager = AsyncRouteManager::new().unwrap();
    let result = manager.add(&route).await;
    println!("route add {route} {result:?}");
    tokio::time::sleep(Duration::from_secs(1)).await;
    let result = manager.delete(&route).await;
    println!("route delete {route} {result:?}");
    tokio::time::sleep(Duration::from_secs(1)).await;
}
```
### Synchronous API
```rust,no_run
use route_manager::{Route, RouteManager};
use std::thread;
use std::time::Duration;
let mut route_listener = RouteManager::listener().unwrap();
#[cfg(feature = "shutdown")]
let shutdown_handle = route_listener.shutdown_handle().unwrap();
thread::spawn(move || {
    while let Ok(route) = route_listener.listen() {
        println!("listen {route}");
    }
    println!("========= end =========");
});
// Need to set up the correct gateway
let route = Route::new("192.168.2.0".parse().unwrap(), 24).with_if_index(1);
let mut manager = RouteManager::new().unwrap();

let result = manager.add(&route);
println!("route add {route} {result:?}");
thread::sleep(Duration::from_secs(1));
let result = manager.delete(&route);
println!("route delete {route} {result:?}");
thread::sleep(Duration::from_secs(1));
#[cfg(feature = "shutdown")]
shutdown_handle.shutdown().unwrap();
thread::sleep(Duration::from_secs(100));
```
 */
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
#[cfg(all(
    any(target_os = "freebsd", target_os = "openbsd", target_os = "macos"),
    not(docsrs)
))]
mod unix_bsd;
#[cfg(all(
    any(target_os = "freebsd", target_os = "openbsd", target_os = "macos"),
    not(docsrs)
))]
pub use unix_bsd::*;
#[cfg(all(unix, not(docsrs)))]
mod unix;
#[cfg(all(unix, not(docsrs)))]
#[allow(unused_imports)]
pub use crate::unix::*;
