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
        let mut e = e.connection().await;
        let _ = query::<_, _, ()>("", &mut e);
        // TODO:
        // let _ = query::<_, _, ()>("", &mut e).fetch_all().await;
    }
}

