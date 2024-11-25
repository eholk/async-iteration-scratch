//! This module experiments with how generators (which must be pinned before
//! using) and iterators (which are not pinned) can be used together.

#![feature(pin_ergonomics, negative_impls, negative_bounds)]
#![allow(unstable_features, incomplete_features, internal_features)]

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
    fn next(self: &pin mut Self) -> Option<<Self as Generator>::Item>;
}

pub trait IntoGenerator {
    type Item;
    type IntoGen: Generator<Item = Self::Item>;
    fn into_gen(self) -> Self::IntoGen;
}

/// A generator-based for loop
#[allow(unused)]
macro_rules! for_gen {
    ($x:ident in $e:expr => $body:expr) => {
        let gn = core::pin::pin!($e.into_gen());
        loop {
            match gn.next() {
                Some($x) => $body,
                None => break,
            }
        }
    };
}

/// Iterators can be used as Generators since they don't need to be pinned.
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

// impl<T> Generator for T
// where
//     T: DerefMut + Unpin,
//     T::Target: Iterator,
// {
//     type Item = <T::Target as Iterator>::Item;
//     fn next(mut self: &pin mut Self) -> Option<Self::Item> {
//         self.deref_mut().next()
//     }
// }

impl<I: Iterator + !Generator> IntoIterator for I {
    type Item = <I as Iterator>::Item;
    type IntoIter = I;
    fn into_iter(self) -> Self::IntoIter {
        self
    }
}

impl <G: Generator + !Iterator> IntoGenerator for G {
    type Item = <G as Generator>::Item;
    type IntoGen = G;
    fn into_gen(self) -> Self::IntoGen {
        self
    }
}

// These don't actually accomplish what we want...
impl<G: Generator> !IntoIterator for G {}
impl<G: Generator> !Iterator for G {}


#[cfg(test)]
mod test {
    use super::*;
    use std::marker::PhantomPinned;
    use std::pin::pin;

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
    fn for_count_to_iterator() {
        let mut sum = 0;
        for_gen!(x in count_to(5) => {
            sum += x;
        });
        assert_eq!(sum, 15);
    }

    /// Like CountTo, but implements Generator and not Iterator.
    struct CountToGen {
        limit: usize,
        count: usize,
        _pinned: PhantomPinned,
    }

    impl Generator for CountToGen {
        type Item = usize;
        fn next(self: &pin mut Self) -> Option<usize> {
            // SAFETY: we're only pretending this needs to be pinned.
            let this = unsafe { self.get_unchecked_mut() };
            if this.count < this.limit {
                this.count += 1;
                Some(this.count)
            } else {
                None
            }
        }
    }
    impl !IntoIterator for CountToGen {}

    fn count_to_gen(limit: usize) -> CountToGen {
        CountToGen {
            limit,
            count: 0,
            _pinned: PhantomPinned,
        }
    }

    #[test]
    fn for_count_to_generator() {
        let mut sum = 0;
        for_gen!(x in count_to_gen(5) => {
            sum += x;
        });
        assert_eq!(sum, 15);
    }

    fn count_iterator(numbers: impl IntoIterator<Item = usize>) -> usize {
        let mut sum = 0;
        for_gen!(x in numbers => {
            sum += x;
        });
        sum
    }

    #[test]
    fn use_generator_as_iterator() {
        let sum = count_iterator(pin!(count_to_gen(5)));
        assert_eq!(sum, 15);
    }
}
