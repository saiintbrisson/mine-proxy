mod ext;

use std::{io, net::SocketAddr};

use tokio::{io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt}, net::TcpStream};

lazy_static::lazy_static!{
    static ref TARGET_ADDR: String = std::env::var("TARGET_ADDR").expect("missing TARGET_ADDR env");
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let listener = tokio::net::TcpListener::bind(
        std::env::var("BIND_ADDR").unwrap_or("0.0.0.0:25565".into())
    ).await?;

    while let Ok((stream, addr)) = listener.accept().await {
        tokio::spawn(handle_connection(stream, addr));
    }

    Ok(())
}

async fn handle_connection(stream: TcpStream, addr: SocketAddr) {
    let (mut c_reader, mut c_writer) = tokio::io::split(stream);

    let server = match TcpStream::connect(TARGET_ADDR.as_str()).await {
        Ok(stream) => stream,
        Err(err) => {
            eprintln!("{}: Failed to connect to target server: {}", addr, err);
            return;
        },
    };
    let server_addr = match server.peer_addr() {
        Ok(addr) => addr,
        Err(err) => {
            eprintln!("{}: Failed to get remote addr: {}", addr, err);
            return;
        },
    };
    let (mut s_reader, mut s_writer) = tokio::io::split(server);

    match handle_handshake(&mut c_reader, &mut s_writer, &server_addr).await {
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

async fn handle_handshake<R, W>(c_reader: &mut R, s_writer: &mut W, remote_addr: &SocketAddr) -> io::Result<()>
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

    let addr = TARGET_ADDR.clone();
    let host = addr.split(':').next().unwrap().as_bytes();

    let mut payload = Vec::with_capacity(6 + host.len());
    payload.push(0);
    payload.push(47);
    payload.push(host.len() as u8);
    payload.extend_from_slice(host);
    payload.extend_from_slice(&remote_addr.port().to_be_bytes());
    payload.push(next_state);

    ext::write_varint(payload.len() as i32, s_writer).await?;
    s_writer.write(&payload).await.map(|_| ())
}
