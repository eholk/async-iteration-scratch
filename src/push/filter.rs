//! Implementation of filter combinator for push-based iterators.
//!
//! Using filter because it requires more capabilities than just `map`.

use std::ops::ControlFlow;

use super::Stream;

pub struct Filter<S, F>
where
    S: Stream,
    F: async FnMut(&S::Item) -> bool,
{
    stream: S,
    predicate: F,
}

impl<S, F> Stream for Filter<S, F>
where
    S: Stream,
    F: async FnMut(&S::Item) -> bool,
{
    type Item = S::Item;

    async fn exec(mut self, mut f: impl async FnMut(Self::Item) -> ControlFlow<()>) {
        self.stream
            .exec(async |item| {
                if (self.predicate)(&item).await {
                    f(item).await?;
                }
                ControlFlow::Continue(())
            })
            .await;
    }
}

pub fn filter<S, F>(stream: S, predicate: F) -> Filter<S, F>
where
    S: Stream,
    F: async FnMut(&S::Item) -> bool,
{
    Filter { stream, predicate }
}

#[cfg(test)]
mod test {
    use crate::{
        block_on,
        push::{self, filter::filter, Stream},
    };

    #[test]
    fn simple_filter() {
        block_on(async {
            let iter = push::from_iter(0..10);
            let mut result = vec![];

            filter(iter, async |x| x % 2 == 0)
                .for_each(async |x| {
                    result.push(x);
                })
                .await;

            assert_eq!(result, vec![0, 2, 4, 6, 8]);
        })
    }
}
