use crate::transport::PgTransport;

/// A type that can returns a [`PgTransport`].
pub trait Executor {
    /// The returned transport.
    type Transport: PgTransport;

    /// Future that resolve to [`Executor::Transport`].
    type Future: Future<Output = Self::Transport>;

    /// Acquire the transport.
    fn connection(self) -> Self::Future;
}

