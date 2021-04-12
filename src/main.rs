mod ext;

use std::{io, net::SocketAddr, sync::Arc};

use tokio::{io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt}, net::TcpStream};

pub struct Server {
    target_addrs: Vec<SocketAddr>,
    target_raw_addr: String,
    target_raw_port: u16,
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let addr = std::env::var("TARGET_ADDR").expect("missing TARGET_ADDR env");
    let (raw_addr, raw_port) = parse_addr(&addr);
    
    let server = Arc::new(Server {
        target_addrs: tokio::net::lookup_host(&addr).await?.collect(),
        target_raw_addr: raw_addr,
        target_raw_port: raw_port,
    });

    let listener = tokio::net::TcpListener::bind(
        std::env::var("BIND_ADDR").unwrap_or("0.0.0.0:25565".into())
    ).await?;

    println!("listening for connection on {}", listener.local_addr()?);

    while let Ok((stream, addr)) = listener.accept().await {
        let server = server.clone();
        tokio::spawn(handle_connection(server, stream, addr));
    }

    Ok(())
}

fn parse_addr(addr: &str) -> (String, u16) {
    let mut split = addr.splitn(2, ':');

    (
        split.next().expect("invalid target addr").into(), 
        split.next().map(|port| port.parse().expect("invalid port")).unwrap_or(25565u16)
    )
}

async fn handle_connection(server: Arc<Server>, stream: TcpStream, addr: SocketAddr) {
    let (mut c_reader, mut c_writer) = tokio::io::split(stream);

    let server_stream = match TcpStream::connect(&server.target_addrs[..]).await {
        Ok(stream) => stream,
        Err(err) => {
            eprintln!("{}: Failed to connect to target server: {}", addr, err);
            return;
        },
    };
    let (mut s_reader, mut s_writer) = tokio::io::split(server_stream);

    match handle_handshake(&server, &addr, &mut c_reader, &mut s_writer).await {
        Err(err) => {
            eprintln!("{}: Failed to handle handshake: {}", addr, err);
            return;
        },
        _ => {},
    }

    tokio::spawn(async move {
        match tokio::io::copy(&mut c_reader, &mut s_writer).await {
            Err(err) => eprintln!("{}: Connection (c -> s) ended: {}", addr, err),
            _ => {},
        }
    });

    match tokio::io::copy(&mut s_reader, &mut c_writer).await {
        Err(err) => eprintln!("{}: Connection (s -> c) ended: {}", addr, err),
        _ => {},
    }
}

async fn handle_handshake<R, W>(server: &Arc<Server>, addr: &SocketAddr, c_reader: &mut R, s_writer: &mut W) -> io::Result<()>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let size = ext::read_varint(c_reader).await? as usize;
    let mut payload = vec![0u8; size];
    if c_reader.read(&mut payload).await? != size {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid packet len, expected {}", size)
        ));
    }

    let next_state = *payload.last().ok_or(io::Error::new(
        io::ErrorKind::InvalidInput,
        format!("invalid handshake packet")
    ))?;

    if next_state == 2 {
        println!("{}: is logging in...", addr);
    }

    let host = server.target_raw_addr.as_bytes();

    let mut payload = Vec::with_capacity(6 + host.len());
    payload.push(0);
    payload.push(47);
    payload.push(host.len() as u8);
    payload.extend_from_slice(host);
    payload.extend_from_slice(&server.target_raw_port.to_be_bytes());
    payload.push(next_state);

    ext::write_varint(payload.len() as i32, s_writer).await?;
    s_writer.write(&payload).await.map(|_| ())
}
