use std::net::Ipv4Addr;

use clap::{command, Parser};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Server Host
    #[arg(long, default_value_t = Ipv4Addr::new(127, 0, 0, 1))]
    pub host: std::net::Ipv4Addr,

    /// Server Port
    #[arg(short, long, default_value_t = 11111)]
    pub port: u32,

    /// Directory to save incoming files and images
    #[arg(short, long, default_value = "./data")]
    pub output_dir: String,

    /// Directory to save tracing logs from client
    #[arg(short, long, default_value = "./logs")]
    pub logs_dir: String,

    /// Username
    #[arg(short, long, default_value = "anonymous")]
    pub username: String,
}
