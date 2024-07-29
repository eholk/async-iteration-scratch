//! Common combinators on futures that we use to build async iterator
//! combinators.

use std::{future::poll_fn, pin::pin, task::Poll};

use crate::Either;

mod join;

pub use join::{join, JoinFuture, JoinRoot, JoinWith};

pub async fn race<A, B>(a: A, b: B) -> Either<A::Output, B::Output>
where
    A: IntoFuture,
    B: IntoFuture,
{
    let mut a = pin!(a.into_future());
    let mut b = pin!(b.into_future());
    let mut parity = false;
    poll_fn(|cx| {
        parity = !parity;

        if parity {
            // poll a then b
            match a.as_mut().poll(cx) {
                Poll::Ready(item) => Poll::Ready(Either::Left(item)),
                Poll::Pending => match b.as_mut().poll(cx) {
                    Poll::Ready(item) => Poll::Ready(Either::Right(item)),
                    Poll::Pending => Poll::Pending,
                },
            }
        } else {
            // poll b then a
            match b.as_mut().poll(cx) {
                Poll::Ready(item) => Poll::Ready(Either::Right(item)),
                Poll::Pending => match a.as_mut().poll(cx) {
                    Poll::Ready(item) => Poll::Ready(Either::Left(item)),
                    Poll::Pending => Poll::Pending,
                },
            }
        }
    })
    .await
}
