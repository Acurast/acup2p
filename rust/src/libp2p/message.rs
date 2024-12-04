use libp2p_request_response as request_response;

pub(super) type Behaviour = request_response::Behaviour<codec::Codec>;

pub(super) mod codec {
    use crate::libp2p::message::request_response;
    use std::io;

    use async_trait::async_trait;
    use futures::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
    use libp2p::StreamProtocol;

    pub struct Codec {}

    const REQUEST_SIZE_MAXIMUM: u64 = 1024 * 1024;
    const RESPONSE_SIZE_MAXIMUM: u64 = 10 * 1024 * 1024;

    impl Default for Codec {
        fn default() -> Self {
            Codec {}
        }
    }

    impl Clone for Codec {
        fn clone(&self) -> Self {
            Self::default()
        }
    }

    impl Codec {
        async fn read_bytes<T>(&mut self, io: &mut T, len: u64) -> io::Result<Vec<u8>>
        where
            T: AsyncRead + Unpin + Send,
        {
            let mut vec = Vec::new();

            io.take(len).read_to_end(&mut vec).await?;

            Ok(vec)
        }

        async fn write_bytes<T>(&mut self, io: &mut T, bytes: Vec<u8>) -> io::Result<()>
        where
            T: AsyncWrite + Unpin + Send,
        {
            io.write_all(bytes.as_ref()).await?;

            Ok(())
        }
    }

    #[async_trait]
    impl request_response::Codec for Codec {
        type Protocol = StreamProtocol;
        type Request = Vec<u8>;
        type Response = Vec<u8>;

        async fn read_request<T>(&mut self, _: &Self::Protocol, io: &mut T) -> io::Result<Vec<u8>>
        where
            T: AsyncRead + Unpin + Send,
        {
            self.read_bytes(io, REQUEST_SIZE_MAXIMUM).await
        }

        async fn read_response<T>(&mut self, _: &Self::Protocol, io: &mut T) -> io::Result<Vec<u8>>
        where
            T: AsyncRead + Unpin + Send,
        {
            self.read_bytes(io, RESPONSE_SIZE_MAXIMUM).await
        }

        async fn write_request<T>(
            &mut self,
            _: &Self::Protocol,
            io: &mut T,
            req: Self::Request,
        ) -> io::Result<()>
        where
            T: AsyncWrite + Unpin + Send,
        {
            self.write_bytes(io, req).await
        }

        async fn write_response<T>(
            &mut self,
            _: &Self::Protocol,
            io: &mut T,
            resp: Self::Response,
        ) -> io::Result<()>
        where
            T: AsyncWrite + Unpin + Send,
        {
            self.write_bytes(io, resp).await
        }
    }
}
