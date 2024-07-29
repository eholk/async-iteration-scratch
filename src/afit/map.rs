use super::AsyncIterator;

pub fn map<Iter: AsyncIterator, F: async Fn(Iter::Item) -> U, U>(
    iter: Iter,
    f: F,
) -> Map<Iter, F, U> {
    Map { iter, f }
}

pub struct Map<Iter: AsyncIterator, F, U>
where
    F: async Fn(Iter::Item) -> U,
{
    iter: Iter,
    f: F,
}

impl<Iter, F, U> AsyncIterator for Map<Iter, F, U>
where
    Iter: AsyncIterator,
    F: async Fn(Iter::Item) -> U,
{
    type Item = U;

    async fn next(&mut self) -> Option<Self::Item> {
        match self.iter.next().await {
            Some(item) => Some((self.f)(item).await),
            None => None,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{
        afit::{async_iter_from_iter, map::map, AsyncIterator},
        block_on,
    };

    #[test]
    fn simple_map() {
        block_on(async {
            let iter = async_iter_from_iter(0..3);
            let mut result = vec![];

            let mapped = map(iter, async |x| x + 1);

            mapped
                .for_each(async |x| {
                    result.push(x);
                })
                .await;

            assert_eq!(result, vec![1, 2, 3]);
        })
    }
}
