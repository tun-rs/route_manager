use route_manager::RouteManager;

pub fn main() {
    let vec = RouteManager::new().unwrap().list().unwrap();
    for x in vec {
        println!("{x}");
    }
}
