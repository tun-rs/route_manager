use route_manager::{Route, RouteManager};
use std::thread;
use std::time::Duration;

pub fn main() {
    let mut route_listener = RouteManager::listener().unwrap();
    #[cfg(feature = "shutdown")]
    let shutdown_handle = route_listener.shutdown_handle().unwrap();
    thread::spawn(move || {
        loop {
            match route_listener.listen() {
                Ok(route) => {
                    println!("========= listen {route} =========");
                }
                Err(e) => {
                    println!("========= listen {e:?} =========");
                    break;
                }
            }
        }
        println!("========= listen end =========");
    });
    // Need to set up the correct gateway
    let route = Route::new("192.168.2.0".parse().unwrap(), 24).with_if_index(1);
    let mut manager = RouteManager::new().unwrap();

    let result = manager.add(&route);
    println!("route add {route} {result:?}");
    thread::sleep(Duration::from_secs(1));
    let result = manager.delete(&route);
    println!("route delete {route} {result:?}");
    thread::sleep(Duration::from_secs(3));
    #[cfg(feature = "shutdown")]
    {
        shutdown_handle.shutdown().unwrap();
    }
    thread::sleep(Duration::from_secs(100));
}
