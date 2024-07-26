use std::{async_iter::AsyncIterator, future::poll_fn, pin::Pin};

// map is impossible
// pub mod map;
pub mod merge;

pub trait AsyncIteratorExt: AsyncIterator {
    async fn next(mut self: Pin<&mut Self>) -> Option<Self::Item> {
        poll_fn(|cx| self.as_mut().poll_next(cx)).await
    }
}

impl<T: AsyncIterator> AsyncIteratorExt for T {}
