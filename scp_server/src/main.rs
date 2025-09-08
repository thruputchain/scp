use clap::Parser;

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

    println!("Hello, world! {} {}", args.peer_addr, args.control_addr);
}
