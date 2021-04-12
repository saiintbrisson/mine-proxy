use std::io;

use protocol::{PacketDeserializer, PacketSerializer};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

#[async_trait]
pub trait InboundPacket: PacketDeserializer + Sized {
    async fn read<R>(src: &mut R) -> io::Result<Self>
    where
        R: AsyncRead + Send + Unpin,
    {
        let size = crate::tokio_ext::MineAsyncReadExt::read_varint(src).await? as usize;
        let mut buf = vec![0u8; size];
        src.read_exact(&mut buf).await?;

        PacketDeserializer::deserialize(&mut io::Cursor::new(buf))
    }
}
impl<T> InboundPacket for T where T: PacketDeserializer {}

#[async_trait]
pub trait OutboundPacket: PacketSerializer {
    async fn write<W>(&self, dst: &mut W) -> io::Result<usize>
    where
        W: AsyncWrite + Send + Unpin,
    {
        let mut buf = Vec::with_capacity(PacketSerializer::calculate_len(self));
        PacketSerializer::serialize(self, &mut buf)?;

        crate::tokio_ext::MineAsyncWriteExt::write_varint(dst, buf.len() as i32).await?;
        dst.write(&buf[..]).await
    }
    // async fn write_compressed_format<W>(&self, dst: &mut W) -> io::Result<usize>
    // where
    //     W: AsyncWrite + Send + Unpin,
    // {
    //     let mut buf = Vec::with_capacity(PacketSerializer::calculate_len(self) + 1);
    //     buf.push(0);
    //     PacketSerializer::serialize(self, &mut buf)?;

    //     crate::io_ext::MineAsyncWriteExt::write_varint(dst, buf.len() as i32).await?;
    //     dst.write(&buf[..]).await
    // }
}
impl<T> OutboundPacket for T where T: PacketSerializer {}
