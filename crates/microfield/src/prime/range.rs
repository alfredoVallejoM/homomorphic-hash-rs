//! Internal range-state markers; none are canonical field elements.

#![allow(dead_code)]

pub(crate) struct Reduced<F>(F);
pub(crate) struct Lazy2<F>(F);
pub(crate) struct Lazy4<F>(F);

impl<F> Reduced<F> {
    pub(crate) const fn new(value: F) -> Self {
        Self(value)
    }

    pub(crate) fn into_inner(self) -> F {
        self.0
    }
}

impl<F> Lazy2<F> {
    pub(crate) const fn new(value: F) -> Self {
        Self(value)
    }

    pub(crate) fn into_inner(self) -> F {
        self.0
    }
}

impl<F> Lazy4<F> {
    pub(crate) const fn new(value: F) -> Self {
        Self(value)
    }

    pub(crate) fn into_inner(self) -> F {
        self.0
    }
}
