#![allow(unused)] // utils
use std::mem::{self, MaybeUninit};

#[derive(Clone)]
pub struct InlineVec<T, const S: usize> {
    inner: Inner<T,S>,
}

enum Inner<T, const S: usize> {
    Array(usize,[MaybeUninit<T>;S]),
    Vec(Vec<T>),
}

impl<T, const S: usize> Clone for Inner<T, S>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        match self {
            Inner::Array(len, items) => {
                let mut items2 = [const { MaybeUninit::uninit() }; S];
                for (i1, i2) in items[..*len].iter().zip(&mut items2[..*len]) {
                    i2.write(unsafe { i1.assume_init_ref() }.clone());
                }
                Inner::Array(*len, items2)
            }
            Inner::Vec(items) => Inner::Vec(Vec::clone(items)),
        }
    }
}

impl<T, const S: usize> InlineVec<T, S> {
    pub const fn new() -> Self {
        Self {
            inner: Inner::Array(0, [const { MaybeUninit::uninit() }; S]),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: match capacity <= S {
                true => Inner::Array(0, [const { MaybeUninit::uninit() }; S]),
                false => Inner::Vec(Vec::with_capacity(capacity)),
            },
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn len(&self) -> usize {
        match &self.inner {
            Inner::Array(len, _) => *len,
            Inner::Vec(vec) => vec.len(),
        }
    }

    pub fn as_slice(&self) -> &[T] {
        match &self.inner {
            Inner::Array(len, items) => unsafe { mem::transmute(&items[..*len]) },
            Inner::Vec(vec) => vec.as_slice(),
        }
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        match &mut self.inner {
            Inner::Array(len, items) => unsafe { mem::transmute(&mut items[..*len]) },
            Inner::Vec(vec) => vec.as_mut_slice(),
        }
    }

    pub fn push(&mut self, value: T) {
        match &mut self.inner {
            Inner::Vec(vec) => vec.push(value),
            Inner::Array(len, items) => {
                if *len == S {
                    let mut vec = Vec::with_capacity(S + 1);
                    for item in &mut items[..*len] {
                        vec.push(unsafe { mem::replace(item, MaybeUninit::uninit()).assume_init() });
                    }
                    vec.push(value);
                } else {
                    items[*len].write(value);
                    *len += 1;
                }
            }
        }
    }

    #[must_use]
    pub fn pop(&mut self) -> Option<T> {
        match &mut self.inner {
            Inner::Vec(items) => items.pop(),
            Inner::Array(len, items) => {
                if *len == 0 {
                    return None;
                }
                *len -= 1;
                let item = mem::replace(&mut items[*len], MaybeUninit::uninit());
                Some(unsafe { item.assume_init() })
            }
        }
    }
}

impl<T, const S: usize> Drop for InlineVec<T, S> {
    fn drop(&mut self) {
        if let Inner::Array(len, items) = &mut self.inner {
            for item in &mut items[..*len] {
                unsafe { item.assume_init_drop() };
            }
        }
    }
}

impl<T, const S: usize> Default for InlineVec<T, S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const S: usize> std::ops::Deref for InlineVec<T, S> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<T, const S: usize> std::ops::DerefMut for InlineVec<T, S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}

impl<T: std::fmt::Debug, const S: usize> std::fmt::Debug for InlineVec<T, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self.as_slice(), f)
    }
}

// NOTE: IntoIter

impl<T, const S: usize> IntoIterator for InlineVec<T, S> {
    type Item = T;

    type IntoIter = IntoIter<T, S>;

    fn into_iter(mut self) -> Self::IntoIter {
        IntoIter {
            inner: match &mut self.inner {
                Inner::Array(oldlen, items) => {
                    let len = *oldlen;

                    // prevent `Drop` deallocating
                    *oldlen = 0;

                    let items = mem::replace(items, [const { MaybeUninit::uninit() };S]);
                    IntoIterInner::Array { read: 0, len, items, }
                },
                Inner::Vec(items) => IntoIterInner::Vec(mem::take(items).into_iter()),
            }
        }
    }
}

pub struct IntoIter<T, const S: usize> {
    inner: IntoIterInner<T, S>,
}

impl<T: std::fmt::Debug, const S: usize> std::fmt::Debug for IntoIter<T, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::fmt::Debug;
        match &self.inner {
            IntoIterInner::Array { read, len, items } => Debug::fmt(&items[*read..*len], f),
            IntoIterInner::Vec(vec) => Debug::fmt(vec.as_slice(), f),
        }
    }
}

enum IntoIterInner<T, const S: usize> {
    Array {
        read: usize,
        len: usize,
        items: [MaybeUninit<T>;S],
    },
    Vec(std::vec::IntoIter<T>),
}

impl<T, const S: usize> Drop for IntoIter<T, S> {
    fn drop(&mut self) {
        if let IntoIterInner::Array { read, len, items } = &mut self.inner {
            for item in &mut items[*read..*len] {
                unsafe { item.assume_init_drop() };
            }
        }
    }
}

impl<T, const S: usize> ExactSizeIterator for IntoIter<T, S> { }

impl<T, const S: usize> Iterator for IntoIter<T, S> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.inner {
            IntoIterInner::Vec(vec) => vec.next(),
            IntoIterInner::Array { read, len, items } => {
                if *read == *len {
                    return None;
                }
                let item = mem::replace(&mut items[*read], MaybeUninit::uninit());
                *read += 1;
                Some(unsafe { item.assume_init() })
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.inner {
            IntoIterInner::Array { read, len, .. } => {
                let len = len - read;
                (len,Some(len))
            }
            IntoIterInner::Vec(vec) => (vec.len(),Some(vec.len())),
        }
    }
}

