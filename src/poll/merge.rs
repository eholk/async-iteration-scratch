//! This modules does `merge` and `map` using the `poll_next` formulation.
//!
//! Since this is currently implemented in nightly, we'll use that version
//! instead of defining our own.

use std::{
    async_iter::AsyncIterator,
    pin::Pin,
    task::{Context, Poll},
};

use crate::Either;

pub fn merge<A, B>(a: A, b: B) -> Merge<A, B>
where
    A: AsyncIterator,
    B: AsyncIterator,
{
    Merge {
        a: Fused {
            inner: a,
            done: false,
        },
        b: Fused {
            inner: b,
            done: false,
        },
        parity: true,
    }
}

pub struct Merge<A, B>
where
    A: AsyncIterator,
    B: AsyncIterator,
{
    a: Fused<A>,
    b: Fused<B>,
    /// Specifies whether to poll a or b first
    ///
    /// We do this to prevent starvation if one stream is faster than the other.
    parity: bool,
}

impl<A, B> AsyncIterator for Merge<A, B>
where
    A: AsyncIterator,
    B: AsyncIterator,
{
    type Item = Either<A::Item, B::Item>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        unsafe {
            if self.parity {
                // Poll a then b
                match self
                    .as_mut()
                    .map_unchecked_mut(|this| &mut this.a)
                    .poll_next(cx)
                {
                    Poll::Ready(Some(a)) => Poll::Ready(Some(Either::Left(a))),
                    Poll::Ready(None) => match self
                        .as_mut()
                        .map_unchecked_mut(|this| &mut this.b)
                        .poll_next(cx)
                    {
                        Poll::Ready(Some(b)) => Poll::Ready(Some(Either::Right(b))),
                        Poll::Ready(None) => Poll::Ready(None),
                        Poll::Pending => Poll::Pending,
                    },
                    Poll::Pending => Poll::Pending,
                }
            } else {
                // Poll b then a
                match self
                    .as_mut()
                    .map_unchecked_mut(|this| &mut this.b)
                    .poll_next(cx)
                {
                    Poll::Ready(Some(b)) => Poll::Ready(Some(Either::Right(b))),
                    Poll::Ready(None) => match self
                        .as_mut()
                        .map_unchecked_mut(|this| &mut this.a)
                        .poll_next(cx)
                    {
                        Poll::Ready(Some(a)) => Poll::Ready(Some(Either::Left(a))),
                        Poll::Ready(None) => Poll::Ready(None),
                        Poll::Pending => Poll::Pending,
                    },
                    Poll::Pending => Poll::Pending,
                }
            }
        }
    }
}

struct Fused<T: AsyncIterator> {
    inner: T,
    done: bool,
}

impl<T: AsyncIterator> AsyncIterator for Fused<T> {
    type Item = T::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        unsafe {
            if self.done {
                Poll::Ready(None)
            } else {
                match self
                    .as_mut()
                    .map_unchecked_mut(|this| &mut this.inner)
                    .poll_next(cx)
                {
                    Poll::Ready(None) => {
                        *self.map_unchecked_mut(|this| &mut this.done) = true;
                        Poll::Ready(None)
                    }
                    otherwise => otherwise,
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::vec;

    use super::merge;
    use crate::block_on;
    use crate::Either::{Left, Right};

    #[test]
    fn simple_merge() {
        block_on(async {
            let mut result = vec![];

            let a = async gen {
                yield 1;
                yield 2;
                yield 3;
            };
            let b = async gen {
                yield 4;
                yield 5;
                yield 6;
            };

            for await item in merge(a, b) {
                result.push(match item {
                    Left(a) => a,
                    Right(b) => b,
                });
            }

            assert_eq!(result.len(), 6);
            for i in [1, 2, 3, 4, 5, 6] {
                assert!(result.contains(&i), "result does not contain {i}");
            }
        })
    }
}
