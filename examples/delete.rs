use clap::Parser;
use route_manager::{Route, RouteManager};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// if_index
    #[arg(short, long)]
    index: u32,
    /// network
    #[arg(short, long)]
    network: String,
}

pub fn main() {
    // Need to set up the correct gateway
    let args = Args::parse();
    let route = Route::new(args.network.parse().unwrap(), 24).with_if_index(args.index);
    println!("route delete {route:?}");
    let result = RouteManager::new().unwrap().delete(&route);
    println!("{result:?}");
}
