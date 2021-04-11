use std::io;

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

const NUM_SHIFT: [u8; 10] = [0, 7, 14, 21, 28, 35, 42, 49, 56, 63];

pub async fn read_varint<R>(src: &mut R) -> io::Result<i32>
where
    R: AsyncRead + Unpin,
{
    let mut result = 0i32;

    for i in &NUM_SHIFT[..5] {
        let byte = src.read_u8().await?;
        result |= ((byte & 0x7F) << i) as i32;

        if byte & 0x80 != 1 {
            return Ok(result);
        }
    }

    Err(io::Error::new(
        io::ErrorKind::InvalidInput,
        "varint is too big",
    ))
}

pub async fn write_varint<W>(value: i32, dst: &mut W) -> io::Result<()>
where
    W: AsyncWrite + Unpin,
{
    let mut temp = value;

    loop {
        let byte = (temp & 0x7F) as u8;
        temp >>= 7;

        if temp != 0 {
            dst.write_u8(byte | 0x80).await?;
        } else {
            dst.write_u8(byte).await?;
            break;
        }
    }

    Ok(())
}