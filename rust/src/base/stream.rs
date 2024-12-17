use futures::{AsyncRead, AsyncWrite};

pub trait IncomingStream: AsyncRead + AsyncWrite + Send {}
pub trait OutgoingStream: AsyncRead + AsyncWrite + Send {}
