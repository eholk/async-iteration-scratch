//! This one is impossible.

use std::{
    future::poll_fn,
    pin::Pin,
    task::{Context, Poll},
};

use crate::Either;

use super::{AsyncIterator, PinnedAsyncIterator};

enum PollState<F: Future> {
    Pending(F),
    Yielded,
    Complete,
}

impl<F, Item> Future for PollState<F>
where
    F: Future<Output = Option<Item>>,
{
    type Output = Option<Item>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<F::Output> {
        unsafe {
            match self.as_mut().get_unchecked_mut() {
                PollState::Pending(f) => match Pin::new_unchecked(f).poll(cx) {
                    Poll::Ready(Some(item)) => {
                        self.set(PollState::Yielded);
                        Poll::Ready(Some(item))
                    }
                    Poll::Ready(None) => {
                        self.set(PollState::Complete);
                        Poll::Ready(None)
                    }
                    Poll::Pending => Poll::Pending,
                },
                PollState::Yielded => panic!("no state to poll"),
                PollState::Complete => Poll::Ready(None),
            }
        }
    }
}

/// Polls two futures concurrently, returning the first one to complete
///
/// The future that did not complete is left in a partially-completed state.
/// The one that does complete is set to `None`.
async fn partial_race<
    A: Future<Output = Option<ItemA>>,
    B: Future<Output = Option<ItemB>>,
    ItemA,
    ItemB,
>(
    mut a: Pin<&mut PollState<A>>,
    mut b: Pin<&mut PollState<B>>,
    parity: bool,
) -> Option<Either<ItemA, ItemB>> {
    poll_fn(move |cx| {
        if parity {
            // poll a then b
            match a.as_mut().poll(cx) {
                Poll::Ready(Some(item)) => Poll::Ready(Some(Either::Left(item))),
                Poll::Ready(None) => match b.as_mut().poll(cx) {
                    Poll::Ready(Some(item)) => Poll::Ready(Some(Either::Right(item))),
                    Poll::Ready(None) => Poll::Ready(None),
                    Poll::Pending => Poll::Pending,
                },
                Poll::Pending => Poll::Pending,
            }
        } else {
            // poll b then a
            match b.as_mut().poll(cx) {
                Poll::Ready(Some(item)) => Poll::Ready(Some(Either::Right(item))),
                Poll::Ready(None) => match a.as_mut().poll(cx) {
                    Poll::Ready(Some(item)) => Poll::Ready(Some(Either::Left(item))),
                    Poll::Ready(None) => Poll::Ready(None),
                    Poll::Pending => Poll::Pending,
                },
                Poll::Pending => Poll::Pending,
            }
        }
    })
    .await
}

pub fn merge<A, B>(a: A, b: B) -> Merge<A, B>
where
    A: AsyncIterator,
    B: AsyncIterator,
{
    Merge {
        a,
        b,
        a_state: PollState::Yielded,
        b_state: PollState::Yielded,
        parity: false,
    }
}

pub struct Merge<A, B>
where
    A: AsyncIterator,
    B: AsyncIterator,
{
    a: A,
    b: B,

    // This is the impossible part.
    a_state: PollState<A::next()>,
    b_state: PollState<B::next()>,

    parity: bool,
}

impl<A, B> PinnedAsyncIterator for Merge<A, B>
where
    A: AsyncIterator,
    B: AsyncIterator,
{
    type Item = Either<A::Item, B::Item>;

    async fn next(mut self: Pin<&mut Self>) -> Option<Self::Item> {
        if self.a_state == PollState::Yielded {
            self.a_state.set(PollState::Pending(self.a.next()));
        }

        if self.b_state == PollState::Yielded {
            self.b_state.set(PollState::Pending(self.b.next()));
        }

        unsafe {
            self.as_mut()
                .map_unchecked_mut(|this| &mut this.parity)
                .set(!self.parity);
            match (&self.a_state, &self.b_state) {
                (PollState::Complete, PollState::Complete) => None,
                (PollState::Pending(_), PollState::Pending(_)) => {
                    partial_race(
                        self.as_mut().map_unchecked_mut(|this| &mut this.a_state),
                        self.as_mut().map_unchecked_mut(|this| &mut this.b_state),
                        self.parity,
                    )
                    .await
                }
                (PollState::Pending(_), _) => {
                    self.map_unchecked_mut(|this| &mut this.a_state).await
                }
                (_, PollState::Pending(_)) => {
                    self.map_unchecked_mut(|this| &mut this.b_state).await
                }
                (PollState::Yielded, _) | (_, PollState::Yielded) => unreachable!(),
            }
        }
    }
}
