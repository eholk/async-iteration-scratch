//! The point of this crate is to experiment with writing `map` and `merge`
//! combinators on the `poll_next` and `async fn next` versions of async
//! iterators.
//!
//! This module contains code that is used in common between the other two.

#![feature(async_iterator, noop_waker, async_for_loop, gen_blocks, async_closure)]
#![allow(unstable_features)]

use std::future::Future;
use std::pin::pin;
use std::task::{Context, Poll};

mod afit;
mod poll;

pub enum Either<A, B> {
    Left(A),
    Right(B),
}

fn block_on<F: Future>(f: F) -> F::Output {
    let waker = std::task::Waker::noop();
    let mut cx = Context::from_waker(&waker);

    let mut f = pin!(f);
    loop {
        match f.as_mut().poll(&mut cx) {
            Poll::Ready(val) => return val,
            Poll::Pending => (),
        }
    }
}
