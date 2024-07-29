//! Merge with push-based streams

use std::{
    cell::RefCell,
    future::poll_fn,
    task::{Poll, Waker},
};

use crate::{
    future_combinators::{join, race, JoinFuture},
    Either,
};

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

    async fn exec(self, mut f: impl async FnMut(Option<Self::Item>) -> std::ops::ControlFlow<()>) {
        with_pipe(async |mut atx, mut arx| {
            with_pipe(async |mut btx, mut brx| {
                join(async {
                    let mut a_done = false;
                    let mut b_done = false;

                    loop {
                        match (a_done, b_done) {
                            (true, true) => break,
                            (false, false) => match race(arx.get(), brx.get()).await {
                                Either::Left(Some(i)) => match f(Some(Either::Left(i))).await {
                                    std::ops::ControlFlow::Break(()) => break,
                                    std::ops::ControlFlow::Continue(()) => {}
                                },
                                Either::Left(None) => {
                                    a_done = true;
                                }
                                Either::Right(Some(i)) => match f(Some(Either::Right(i))).await {
                                    std::ops::ControlFlow::Break(()) => break,
                                    std::ops::ControlFlow::Continue(()) => {}
                                },
                                Either::Right(None) => {
                                    b_done = true;
                                }
                            },
                            (false, true) => match arx.get().await {
                                Some(i) => match f(Some(Either::Left(i))).await {
                                    std::ops::ControlFlow::Break(()) => break,
                                    std::ops::ControlFlow::Continue(()) => {}
                                },
                                None => {
                                    a_done = true;
                                }
                            },
                            (true, false) => match brx.get().await {
                                Some(i) => match f(Some(Either::Right(i))).await {
                                    std::ops::ControlFlow::Break(()) => break,
                                    std::ops::ControlFlow::Continue(()) => {}
                                },
                                None => {
                                    b_done = true;
                                }
                            },
                        }
                    }
                })
                .with(async {
                    self.a
                        .exec(async |item| {
                            atx.put(item).await;
                            std::ops::ControlFlow::Continue(())
                        })
                        .await;
                })
                .with(async {
                    self.b
                        .exec(async |item| {
                            btx.put(item).await;
                            std::ops::ControlFlow::Continue(())
                        })
                        .await;
                })
                .await
            })
            .await
        })
        .await
    }
}
