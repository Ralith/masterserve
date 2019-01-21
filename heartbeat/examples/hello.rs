#[macro_use]
extern crate structopt;
#[macro_use]
extern crate failure;
extern crate tokio_current_thread;
extern crate futures;
extern crate quinn;
extern crate tokio;
extern crate masterserve_heartbeat as heartbeat;

use std::{io::{self, Write}, net::ToSocketAddrs};

use failure::{Error, err_msg};
use structopt::StructOpt;
use futures::{Future, Stream, stream};

type Result<T> = ::std::result::Result<T, Error>;

#[derive(StructOpt, Debug)]
#[structopt(name = "hello")]
struct Opt {
    /// Master server to connect to
    #[structopt(default_value = "localhost:4433")]
    master: String,
}

fn main() {
    let opt = Opt::from_args();
    let code = {
        if let Err(e) = run(opt) {
            eprintln!("ERROR: {}", e);
            1
        } else {
            0
        }
    };
    ::std::process::exit(code);
}

fn run(options: Opt) -> Result<()> {
    let mut runtime = tokio::runtime::current_thread::Runtime::new()?;

    let (endpoint, driver, _) = quinn::Endpoint::new().bind("[::]:0")?;
    runtime.spawn(driver.map_err(|e| eprintln!("IO error: {}", e)));

    let hostname = options.master.split(':').next().unwrap();

    let addr = options.master.to_socket_addrs().map_err(|_| err_msg("invalid master server address -- did you forget a port number?"))?
        .next().map_or_else(|| bail!("no such hostname"), Ok)?;

    let mut config = quinn::ClientConfigBuilder::new();
    config.set_protocols(&[heartbeat::PROTOCOL]);
    let config = config.build();

    print!("connecting to {}...", addr);
    io::stdout().flush()?;

    runtime.block_on(
        endpoint.connect_with(&config, &addr, hostname)?
            .map_err(|e| -> Error { e.into() })
            .and_then(|conn| {
                println!(" connected");
                let beats = stream::repeat(Vec::from(&b"hello, world"[..])).inspect(|_| { print!("."); io::stdout().flush().unwrap(); });
                heartbeat::run(conn, beats).map_err(Into::into)
            })
    )?;

    Ok(())
}
