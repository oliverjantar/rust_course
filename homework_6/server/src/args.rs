use clap::{command, Parser};
use std::net::Ipv4Addr;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Server Host
    #[arg(long, default_value_t = Ipv4Addr::new(127, 0, 0, 1))]
    pub host: std::net::Ipv4Addr,

    /// Server Port
    #[arg(short, long, default_value_t = 11111)]
    pub port: u32,
}
