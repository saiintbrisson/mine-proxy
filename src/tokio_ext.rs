use std::io;

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

const NUM_SHIFT: [u8; 10] = [0, 7, 14, 21, 28, 35, 42, 49, 56, 63];

#[async_trait]
pub trait MineAsyncReadExt: AsyncRead + Unpin {
    async fn read_varint(&mut self) -> io::Result<i32> {
        let mut result = 0i32;

        for i in &NUM_SHIFT[..5] {
            let byte = self.read_u8().await?;
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
}

#[async_trait]
impl<T> MineAsyncReadExt for T
where T: AsyncRead + Unpin {}

#[async_trait]
pub trait MineAsyncWriteExt: AsyncWrite + Unpin {
    async fn write_varint(&mut self, mut temp: i32) -> io::Result<()> {
        loop {
            let byte = (temp & 0x7F) as u8;
            temp >>= 7;
    
            if temp != 0 {
                self.write_u8(byte | 0x80).await?;
            } else {
                self.write_u8(byte).await?;
                break;
            }
        }
    
        Ok(())
    }
}

#[async_trait]
impl<T> MineAsyncWriteExt for T
where T: AsyncWrite + Unpin {}
