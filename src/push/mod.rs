//! This module experiments with push-based iterators.

use std::{async_iter::AsyncIterator, ops::ControlFlow, pin::pin};

use crate::poll::AsyncIteratorExt as _;

mod filter;
mod merge;

pub trait Stream {
    type Item;

    async fn exec(self, f: impl async FnMut(Option<Self::Item>) -> ControlFlow<()>);

    async fn for_each(self, mut f: impl async FnMut(Self::Item))
    where
        Self: Sized,
    {
        self.exec(async |item| {
            match item {
                Some(item) => f(item).await,
                None => {}
            }
            ControlFlow::Continue(())
        })
        .await;
    }
}

fn from_async_iter<I: AsyncIterator>(iter: I) -> impl Stream<Item = I::Item> {
    struct Iter<I: AsyncIterator>(I);

    impl<I: AsyncIterator> Stream for Iter<I> {
        type Item = I::Item;

        async fn exec(self, mut f: impl async FnMut(Option<Self::Item>) -> ControlFlow<()>) {
            let mut iter = pin!(self.0);
            let mut done = false;
            while !done {
                let item = iter.as_mut().next().await;
                done = item.is_none();
                if let ControlFlow::Break(()) = f(item).await {
                    break;
                }
            }
        }
    }

    Iter(iter)
}

fn from_iter<I>(iter: I) -> impl Stream<Item = I::Item>
where
    I: Iterator,
{
    struct Iter<I>(I);

    impl<I> Stream for Iter<I>
    where
        I: Iterator,
    {
        type Item = I::Item;

        async fn exec(self, mut f: impl async FnMut(Option<Self::Item>) -> ControlFlow<()>) {
            let mut iter = self.0;
            while let Some(item) = iter.next() {
                if let ControlFlow::Break(()) = f(Some(item)).await {
                    return;
                }
            }
            f(None).await;
        }
    }

    Iter(iter)
}
