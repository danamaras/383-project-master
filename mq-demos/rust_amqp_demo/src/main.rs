mod client;
mod config;
mod server;
mod types;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let main = &args[1];

    match &main[..] {
        "client" => client::main(),
        "server" => server::main(),
        _ => panic!("unknown main"),
    }
}
