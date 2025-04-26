use crate::{transport::PgTransport, PgConnection};

/// A type that can returns a [`PgTransport`].
pub trait Executor {
    /// The returned transport.
    type Transport: PgTransport;

    /// Future that resolve to [`Executor::Transport`].
    type Future: Future<Output = Self::Transport>;

    /// Acquire the transport.
    fn connection(self) -> Self::Future;
}

impl Executor for &mut PgConnection {
    type Transport = Self;

    type Future = std::future::Ready<Self::Transport>;

    fn connection(self) -> Self::Future {
        std::future::ready(self)
    }
}

