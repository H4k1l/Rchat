// importing modules
mod connections;
mod tui;
mod file_management;

// importing libraries
use clap::Parser;
use tokio;

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

    #[clap(long = "protected", help = "the password for the host")]
    protected: bool

}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    if args.connect && args.address.is_some() && !args.name.is_empty() && !args.port.is_empty(){
        connections::connect(&args.address.unwrap(), &args.port, &args.name).await;
    }
    else if args.host && !args.name.is_empty() && !args.port.is_empty(){
        connections::host(&args.port, &args.name, args.protected).await;
    }
}
