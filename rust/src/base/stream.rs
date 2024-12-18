use futures::{AsyncRead, AsyncWrite};

pub trait IncomingStream: AsyncRead + AsyncWrite + Send + Unpin {}
pub trait OutgoingStream: AsyncRead + AsyncWrite + Send + Unpin {}
