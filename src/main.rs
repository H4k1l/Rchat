// importing modules
mod connections;
mod tui;

// importing libraries
use clap::Parser;

#[derive(Parser, Debug)]
#[command(about = "Rchat is a encrypted, private and memory safe remote chat, built in rust", long_about = None)]
struct Args {
    #[clap(short = 'a', long = "address", help = "the address to connect to")]
    address: Option<String>,

    #[clap(short = 'p', long = "port", help = "the port to host/connect")]
    port: String,

    #[clap(long = "host", help = "host a connection")]
    host: bool,

    #[clap(long = "connect", help = "connect to an host")]
    connect: bool,

    #[clap(short = 'n', long = "name", help = "your name identifier")]
    name: String,

    #[clap(long = "protected", help = "the password to be protected be")]
    password: Option<String>

}

fn main() {
    let args = Args::parse();
    if args.connect && args.address.is_some() && !args.name.is_empty() && !args.port.is_empty(){
        connections::connect(&args.port, &args.address.unwrap(), &args.name);
    }
    else if args.host && !args.name.is_empty() && !args.port.is_empty(){
        connections::host(&args.port, &args.name);
    }
}