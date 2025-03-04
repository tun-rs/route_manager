#[cfg(feature = "async")]
use route_manager::{AsyncRouteManager, Route};
#[cfg(feature = "async")]
use std::time::Duration;

#[cfg(feature = "async")]
#[tokio::main]
pub async fn main() {
    let mut route_listener = AsyncRouteManager::listener().unwrap();
    tokio::spawn(async move {
        while let Ok(route) = route_listener.listen().await {
            println!("listen {route}");
        }
        println!("========= listen end =========");
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

#[cfg(not(feature = "async"))]
#[tokio::main]
pub async fn main() {
    unimplemented!("This examples needs the 'async' feature.");
}
