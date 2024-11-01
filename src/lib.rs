//! The point of this crate is to experiment with writing `map` and `merge`
//! combinators on the `poll_next` and `async fn next` versions of async
//! iterators.
//!
//! This module contains code that is used in common between the other two.

#![feature(pin_ergonomics)]
#![allow(unstable_features, incomplete_features)]

use std::ops::{Deref, DerefMut};

pub trait Iterator {
    type Item;
    fn next(&mut self) -> Option<Self::Item>;
}

pub trait IntoIterator {
    type Item;
    type IntoIter: Iterator<Item = Self::Item>;
    fn into_iter(self) -> Self::IntoIter;
}

pub trait Generator {
    type Item;
    fn next(self: &pin mut Self) -> Option<Self::Item>;
}

pub trait IntoGenerator {
    type Item;
    type IntoGen: Generator<Item = Self::Item>;
    fn into_gen(self) -> Self::IntoGen;
}

macro_rules! for_gen {
    ($x:ident in $e:expr => $body:expr) => {
        let mut gn = core::pin::pin!($e.into_gen());
        loop {
            match gn.next() {
                Some($x) => $body,
                None => break,
            }
        }
    };
}

impl<I: IntoIterator> IntoGenerator for I {
    type Item = I::Item;
    type IntoGen = ForceUnpin<I::IntoIter>;
    fn into_gen(self) -> Self::IntoGen {
        ForceUnpin(self.into_iter())
    }
}

pub struct ForceUnpin<T>(T);

impl<T> Unpin for ForceUnpin<T> {}

impl<T> Deref for ForceUnpin<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for ForceUnpin<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> Generator for T
where
    T: DerefMut + Unpin,
    T::Target: Iterator,
{
    type Item = <T::Target as Iterator>::Item;
    fn next(mut self: &pin mut Self) -> Option<Self::Item> {
        self.deref_mut().next()
    }
}

impl<I: Iterator> IntoIterator for I {
    type Item = I::Item;
    type IntoIter = I;
    fn into_iter(self) -> Self::IntoIter {
        self
    }
}

#[cfg(test)]
mod test {
    use super::*;

    struct CountTo {
        limit: usize,
        count: usize,
    }

    impl Iterator for CountTo {
        type Item = usize;
        fn next(&mut self) -> Option<Self::Item> {
            if self.count < self.limit {
                self.count += 1;
                Some(self.count)
            } else {
                None
            }
        }
    }

    fn count_to(limit: usize) -> CountTo {
        CountTo { limit, count: 0 }
    }

    #[test]
    fn for_count_to() {
        let mut sum = 0;
        for_gen!(x in count_to(5) => {
            sum += x;
        });
        assert_eq!(sum, 15);
    }
}
