use std::sync::atomic::Ordering;

type AtomicId = std::sync::atomic::AtomicU16;

#[derive(Clone, PartialEq, Eq)]
pub struct Id([u8; 6]);

impl Id {
    pub(crate) fn unnamed() -> Self {
        Self([b'?'; 6])
    }

    pub(crate) fn next(atomic: &AtomicId) -> Self {
        let id = atomic.fetch_add(1, Ordering::SeqCst);
        let mut buf = [b'q', b'0',b'0',b'0',b'0',b'0'];
        let len = buf.len();

        let mut b = itoa::Buffer::new();
        let id = b.format(id);
        let i = id.as_bytes();
        buf[len - i.len()..].copy_from_slice(i);

        Self(buf)
    }

    pub fn as_str(&self) -> &str {
        if self.is_unnamed() {
            return "";
        }
        // SAFETY: string only construction and is immutable
        unsafe { std::str::from_utf8_unchecked(&self.0[..]) }
    }

    pub fn is_unnamed(&self) -> bool {
        self.0[0] == b'?'
    }
}

impl std::fmt::Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::fmt::Debug for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_tuple("Id").field(&self.as_str()).finish()
    }
}

impl AsRef<str> for Id {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

macro_rules! delegate {
    ($name:ident) => {
        #[derive(Clone, PartialEq, Eq)]
        pub struct $name(Id);

        impl $name {
            pub(crate) fn unnamed() -> Self {
                Self(Id::unnamed())
            }

            #[allow(unused, reason = "Portal `next` used later")]
            pub(crate) fn next() -> Self {
                static ID: AtomicId = AtomicId::new(0);
                Self(Id::next(&ID))
            }
        }

        impl std::ops::Deref for $name {
            type Target = Id;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.debug_tuple(stringify!($name)).field(&self.as_str()).finish()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.as_str())
            }
        }
    };
}

delegate!(StatementName);
delegate!(PortalName);

