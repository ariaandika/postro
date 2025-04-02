mod buf;
mod buf_mut;
mod decode;
mod encode;
mod read_buf;

pub use buf::BufExt;
pub use buf_mut::BufMutExt;
pub use decode::ProtocolDecode;
pub use encode::ProtocolEncode;
pub use read_buf::ReadBuf;

#[cfg(feature = "tokio")]
pub use tokio::io::{AsyncRead, AsyncReadExt};

#[cfg(not(feature = "tokio"))]
pub use futures_util::{AsyncRead, AsyncReadExt};

