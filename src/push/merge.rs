//! Merge with push-based streams

use std::{
    cell::RefCell,
    future::poll_fn,
    task::{Poll, Waker},
};

use crate::Either;

use super::Stream;

async fn with_pipe<T, F, U>(f: F) -> U
where
    F: async FnOnce(SendPipe<T>, ReceivePipe<T>) -> U,
{
    let pipe = RefCell::new(OnePipeInner::new());
    f(SendPipe { pipe: &pipe }, ReceivePipe { pipe: &pipe }).await
}

struct OnePipeInner<T> {
    item: Option<T>,
    waker: Option<Waker>,
}

impl<T> OnePipeInner<T> {
    fn new() -> Self {
        OnePipeInner {
            item: None,
            waker: None,
        }
    }
}

struct SendPipe<'a, T> {
    pipe: &'a RefCell<OnePipeInner<T>>,
}

impl<T> SendPipe<'_, T> {
    async fn put(&mut self, item: T) {
        let mut item = Some(item);
        poll_fn(|cx| {
            let mut this = self.pipe.borrow_mut();
            // make sure the slot is empty
            if this.item.is_some() {
                this.waker = Some(cx.waker().clone());
                return Poll::Pending;
            }
            this.item = item.take();
            this.waker.take().map(|w| w.wake());
            Poll::Ready(())
        })
        .await;
    }
}

struct ReceivePipe<'a, T> {
    pipe: &'a RefCell<OnePipeInner<T>>,
}

impl<T> ReceivePipe<'_, T> {
    async fn get(&mut self) -> T {
        poll_fn(|cx| {
            let mut this = self.pipe.borrow_mut();
            // make sure the slot is full
            if this.item.is_none() {
                this.waker = Some(cx.waker().clone());
                return Poll::Pending;
            }
            let item = this.item.take().unwrap();
            this.waker.take().map(|w| w.wake());
            Poll::Ready(item)
        })
        .await
    }
}

pub struct Merge<A, B> {
    a: A,
    b: B,
}

impl<A, B> Merge<A, B>
where
    A: Stream,
    B: Stream,
{
    pub fn new(a: A, b: B) -> Self {
        Merge { a, b }
    }
}

impl<A, B> Stream for Merge<A, B>
where
    A: Stream,
    B: Stream,
{
    type Item = Either<A::Item, B::Item>;

    // async fn exec(self, f: impl async FnMut(Self::Item) -> std::ops::ControlFlow<()>) {
    //     with_pipe(async |atx, arx| {
    //         with_pipe(async |btx, brx| {
    //             let mut a_done = false;
    //             let mut b_done = false;

                

    //             // TODO: create a task that reads from the first stream, then
    //             // another for the second task, and a root task that races
    //             // reading the other two until both are complete.
    //         })
    //         .await
    //     })
    //     .await
    // }
}
