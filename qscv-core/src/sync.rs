// For types with identical signatures that don't require runtime support,
// we can just arbitrarily pick one to use based on what's enabled.
//
// We'll generally lean towards Tokio's types as those are more featureful
// (including `tokio-console` support) and more widely deployed.

#[cfg(feature = "tokio")]
pub use tokio::sync::{Mutex as AsyncMutex, MutexGuard as AsyncMutexGuard};

pub struct AsyncSemaphore {
    // We use the semaphore from futures-intrusive as the one from async-std
    // is missing the ability to add arbitrary permits, and is not guaranteed to be fair:
    // * https://github.com/smol-rs/async-lock/issues/22
    // * https://github.com/smol-rs/async-lock/issues/23
    //
    // We're on the look-out for a replacement, however, as futures-intrusive is not maintained
    // and there are some soundness concerns (although it turns out any intrusive future is unsound
    // in MIRI due to the necessitated mutable aliasing):
    // https://github.com/launchbadge/sqlx/issues/1668
    #[cfg(feature = "tokio")]
    inner: tokio::sync::Semaphore,
}

impl AsyncSemaphore {
    #[track_caller]
    pub fn new(fair: bool, permits: usize) -> Self {
        if cfg!(not(feature = "tokio")) {
            drop((fair, permits));
            panic!("runtime disabled")
        }

        AsyncSemaphore {
            #[cfg(feature = "tokio")]
            inner: {
                debug_assert!(fair, "Tokio only has fair permits");
                tokio::sync::Semaphore::new(permits)
            },
        }
    }

    pub fn permits(&self) -> usize {
        #[cfg(feature = "tokio")]
        return self.inner.available_permits();

        #[cfg(not(feature = "tokio"))]
        panic!("runtime disabled")
    }

    pub async fn acquire(&self, permits: u32) -> AsyncSemaphoreReleaser<'_> {
        #[cfg(feature = "tokio")]
        return AsyncSemaphoreReleaser {
            inner: self
                .inner
                // Weird quirk: `tokio::sync::Semaphore` mostly uses `usize` for permit counts,
                // but `u32` for this and `try_acquire_many()`.
                .acquire_many(permits)
                .await
                .expect("BUG: we do not expose the `.close()` method"),
        };

        #[cfg(not(feature = "tokio"))]
        {
            drop(permits);
            panic!("runtime disabled")
        }
    }

    pub fn try_acquire(&self, permits: u32) -> Option<AsyncSemaphoreReleaser<'_>> {
        #[cfg(feature = "tokio")]
        return Some(AsyncSemaphoreReleaser {
            inner: self.inner.try_acquire_many(permits).ok()?,
        });

        #[cfg(not(feature = "tokio"))]
        {
            drop(permits);
            panic!("runtime disabled")
        }
    }

    pub fn release(&self, permits: usize) {
        #[cfg(feature = "tokio")]
        return self.inner.add_permits(permits);

        #[cfg(not(feature = "tokio"))]
        {
            drop(permits);
            panic!("runtime disabled")
        }
    }
}

pub struct AsyncSemaphoreReleaser<'a> {
    // We use the semaphore from futures-intrusive as the one from async-std
    // is missing the ability to add arbitrary permits, and is not guaranteed to be fair:
    // * https://github.com/smol-rs/async-lock/issues/22
    // * https://github.com/smol-rs/async-lock/issues/23
    //
    // We're on the look-out for a replacement, however, as futures-intrusive is not maintained
    // and there are some soundness concerns (although it turns out any intrusive future is unsound
    // in MIRI due to the necessitated mutable aliasing):
    // https://github.com/launchbadge/sqlx/issues/1668
    #[cfg(feature = "tokio")]
    inner: tokio::sync::SemaphorePermit<'a>,

    #[cfg(not(feature = "tokio"))]
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl AsyncSemaphoreReleaser<'_> {
    pub fn disarm(self) {
        #[cfg(feature = "tokio")]
        {
            self.inner.forget();
        }

        #[cfg(not(feature = "tokio"))]
        panic!("runtime disabled")
    }
}

