//! The [`Executor`] trait.
use std::future::Ready;

use crate::{transport::PgTransport, Result};

/// A type that can returns a [`PgTransport`].
pub trait Executor {
    /// The returned transport.
    type Transport: PgTransport;

    /// Future that resolve to [`Executor::Transport`].
    type Future: Future<Output = Result<Self::Transport>>;

    /// Acquire the transport.
    fn connection(self) -> Self::Future;
}

impl<T: PgTransport> Executor for &mut T {
    type Transport = Self;

    type Future = Ready<Result<Self>>;

    fn connection(self) -> Self::Future {
        std::future::ready(Ok(self))
    }
}

#[cfg(test)]
mod test {
    use crate::query::query;

    use super::Executor;

    #[allow(unused)] // type assertion
    async fn assert_type<E: Executor>(e: E) {
        let _ = query::<_, _, ()>("", e).fetch_all().await;
    }

    #[allow(unused)] // type assertion
    async fn assert_type2<E: Executor>(e: E) {
        let mut e = e.connection().await.unwrap();
        // TODO:
        let _ = query::<_, _, ()>("", &mut e).fetch_all().await;
        let _ = query::<_, _, ()>("", &mut e).fetch_all().await;
    }
}

