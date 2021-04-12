#[macro_use]
extern crate async_trait;
#[macro_use]
extern crate log;

mod packet;
mod tokio_ext;

use std::{io, net::SocketAddr, sync::Arc};

use packet::{InboundPacket, OutboundPacket};
use protocol::packets::handshake::{Handshake, NextState};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
};

pub struct Server {
    target_addrs: Vec<SocketAddr>,
    target_raw_addr: String,
    target_raw_port: u16,
}

#[tokio::main]
async fn main() -> io::Result<()> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    let addr = std::env::var("TARGET_ADDR").expect("missing TARGET_ADDR env");
    let (raw_addr, raw_port) = parse_addr(&addr);

    let server = Arc::new(Server {
        target_addrs: tokio::net::lookup_host(&addr).await?.collect(),
        target_raw_addr: raw_addr,
        target_raw_port: raw_port,
    });

    let listener =
        tokio::net::TcpListener::bind(std::env::var("BIND_ADDR").unwrap_or("0.0.0.0:25565".into()))
            .await?;

    info!("listening for connection on {}", listener.local_addr()?);

    while let Ok((stream, addr)) = listener.accept().await {
        tokio::spawn(handle_connection(server.clone(), stream, addr));
    }

    Ok(())
}

fn parse_addr(addr: &str) -> (String, u16) {
    let mut split = addr.splitn(2, ':');

    (
        split.next().expect("invalid target addr").into(),
        split
            .next()
            .map(|port| port.parse().expect("invalid port"))
            .unwrap_or(25565u16),
    )
}

async fn handle_connection(server: Arc<Server>, stream: TcpStream, addr: SocketAddr) {
    let (mut c_reader, mut c_writer) = tokio::io::split(stream);

    let server_stream = match TcpStream::connect(&server.target_addrs[..]).await {
        Ok(stream) => stream,
        Err(err) => {
            warn!("{}: failed to connect to target server: {}", addr, err);
            return;
        }
    };
    let (mut s_reader, mut s_writer) = tokio::io::split(server_stream);

    match handle_handshake(&server, &addr, &mut c_reader, &mut s_writer).await {
        Err(err) => {
            warn!("{}: failed to handle handshake: {}", addr, err);
            return;
        }
        _ => {}
    }

    tokio::spawn(async move {
        match tokio::io::copy(&mut c_reader, &mut s_writer).await {
            Err(err) => warn!("{}: connection (c -> s) ended: {}", addr, err),
            _ => {}
        }
    });

    match tokio::io::copy(&mut s_reader, &mut c_writer).await {
        Err(err) => warn!("{}: connection (s -> c) ended: {}", addr, err),
        _ => {}
    }
}

async fn handle_handshake<R, W>(
    server: &Arc<Server>,
    addr: &SocketAddr,
    c_reader: &mut R,
    s_writer: &mut W,
) -> io::Result<()>
where
    R: AsyncRead + Send + Unpin,
    W: AsyncWrite + Send + Unpin,
{
    let mut handshake = Handshake::read(c_reader).await?;
    trace!("read handshake");

    match handshake.next_state {
        NextState::Login => info!("{}: is logging in...", addr),
        NextState::Status => debug!("{}: is querying status...", addr),
    }

    handshake.server_address = server.target_raw_addr.clone();
    handshake.server_port = server.target_raw_port;

    handshake.write(s_writer).await.map(|_| ())
}
