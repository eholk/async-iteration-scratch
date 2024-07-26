use std::pin::Pin;

mod map;
// Merge is impossible currently.
// mod merge;

pub trait AsyncIterator {
    type Item;

    async fn next(&mut self) -> Option<Self::Item>;

    async fn for_each(mut self, mut f: impl async FnMut(Self::Item))
    where
        Self: Sized,
    {
        while let Some(item) = self.next().await {
            f(item).await;
        }
    }
}

pub trait PinnedAsyncIterator {
    type Item;

    async fn next(self: Pin<&mut Self>) -> Option<Self::Item>;
}

fn async_iter_from_iter<I: Iterator>(iter: I) -> impl AsyncIterator<Item = I::Item> {
    struct Iter<I: Iterator>(I);

    impl<I: Iterator> AsyncIterator for Iter<I> {
        type Item = I::Item;

        async fn next(&mut self) -> Option<Self::Item> {
            self.0.next()
        }
    }

    Iter(iter)
}
