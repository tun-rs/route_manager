# route_manager

[![Crates.io](https://img.shields.io/crates/v/route_manager.svg)](https://crates.io/crates/route_manager)
[![route_manager](https://docs.rs/route_manager/badge.svg)](https://docs.rs/route_manager)

Used for adding, deleting, and querying routes,
with support for asynchronous or synchronous monitoring of route changes.

## Supported Platforms

| Platform |   |
|----------|---|
| Windows  | ✅ |
| Linux    | ✅ |
| macOS    | ✅ |
| FreeBSD  | ✅ |

## Features:

1. Supporting Synchronous and Asynchronous API
2. Supports choosing between Tokio and async-io for asynchronous I/O operations.

## Example:

```rust
use route_manager::{AsyncRouteManager, Route};
use std::time::Duration;
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

## Reference project

- [net-route](https://github.com/johnyburd/net-route)