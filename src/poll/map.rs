//! Implement map for poll_next async iterators, using async closures.
//!
//! This one is actually impossible because we have no way to name the lifetime
//! in a future that results from calling an async closure.

use std::async_iter::AsyncIterator;

pub fn map<Iter: AsyncIterator, F: async Fn(Iter::Item) -> U, U>(
    iter: Iter,
    f: F,
) -> Map<Iter, F, U> {
    Map { iter, f }
}

pub struct Map<Iter: AsyncIterator, F, U>
where
    F: async Fn(Iter::Item) -> U,
{
    iter: Iter,
    f: F,
}
