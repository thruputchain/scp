use clap::Parser;

mod server;
use server::*;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = ":30803")]
    peer_addr: String,

    #[arg(short, long, default_value = ":11055")]
    control_addr: String,
}

fn main() {
    let args = Args::parse();

    println!(
        "peer_addr={} control_addr={}",
        args.peer_addr, args.control_addr
    );
}
