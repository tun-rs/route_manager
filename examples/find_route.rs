use route_manager::{Route, RouteManager};
use std::net::IpAddr;
use std::thread;
use std::time::Duration;

pub fn main() {
    let net: IpAddr = "192.168.4.0".parse().unwrap();
    let ip: IpAddr = "192.168.4.10".parse().unwrap();
    let mut manager = RouteManager::new().unwrap();
    let find_route = manager.find_route(&ip).unwrap();
    println!("find route: {ip} -> {find_route:?}");
    // Need to set up the correct gateway
    let route = Route::new(net, 24).with_if_index(1);
    println!("route add {:?}", route);
    manager.add(&route).unwrap();
    thread::sleep(Duration::from_secs(1));

    let find_route = manager.find_route(&ip).unwrap();
    println!("find route: {ip} -> {find_route:?}");
    manager.delete(&route).unwrap();
}
