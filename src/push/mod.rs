/// This module experiments with push-based iterators.
use std::{async_iter::AsyncIterator, marker::PhantomData, ops::ControlFlow, pin::pin};

use crate::poll::AsyncIteratorExt;

mod filter;

pub trait Stream {
    type Item;

    async fn exec(self, f: impl async FnMut(Self::Item) -> ControlFlow<()>);

    async fn for_each(self, mut f: impl async FnMut(Self::Item))
    where
        Self: Sized,
    {
        self.exec(async |item| {
            f(item).await;
            ControlFlow::Continue(())
        })
        .await;
    }
}

struct AsyncFnStream<Exec, F, Item> {
    exec: Exec,
    _phantom: PhantomData<(fn(F), fn(fn(Item)))>,
}

impl<Exec, F, Item> Stream for AsyncFnStream<Exec, F, Item>
where
    F: async FnMut(Item) -> ControlFlow<()>,
    Exec: async FnMut(F),
{
    type Item = Item;

    async fn exec(self, mut f: impl async FnMut(Self::Item) -> ControlFlow<()>) {
        (self.exec)(f).await;
    }
}

fn from_async_fn<Exec, F, Item>(exec: Exec) -> impl Stream<Item = Item>
where
    F: async FnMut(Item) -> ControlFlow<()>,
    Exec: async FnMut(F),
{
    AsyncFnStream {
        exec,
        _phantom: PhantomData,
    }
}

fn from_async_iter<I: AsyncIterator>(iter: I) -> impl Stream<Item = I::Item> {
    struct Iter<I: AsyncIterator>(I);

    impl<I: AsyncIterator> Stream for Iter<I> {
        type Item = I::Item;

        async fn exec(self, mut f: impl async FnMut(Self::Item) -> ControlFlow<()>) {
            let mut iter = pin!(self.0);
            while let Some(item) = iter.as_mut().next().await {
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
    from_async_fn(
        async move |mut f: impl async FnMut(I::Item) -> ControlFlow<()>| {
            for item in iter {
                if let ControlFlow::Break(()) = f(item).await {
                    break;
                }
            }
        },
    )
}
