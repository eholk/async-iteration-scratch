//! This is a flexible, static join combinator that lets you build up a root
//! task with some number of side-effect tasks that run concurrently with the
//! root.
//! 
//! One downside is that when the root task completes, all the other tasks are
//! cancelled. Things also probably break if one of the other tasks finishes
//! first.

use std::{
    future::poll_fn,
    pin::{pin, Pin},
    task::{Context, Poll},
};

pub trait JoinFuture: IntoFuture {
    fn size(&self) -> usize;

    fn with<F: IntoFuture<Output = ()>>(self, f: F) -> impl JoinFuture<Output = Self::Output>
    where
        Self: Sized,
    {
        JoinWith {
            future: f.into_future(),
            next: self,
        }
    }

    fn poll_depth(self: Pin<&mut Self>, cx: &mut Context<'_>, depth: usize) -> Poll<Self::Output>;
}

pub fn join<F>(f: F) -> JoinRoot<F::IntoFuture>
where
    F: IntoFuture,
{
    JoinRoot {
        future: f.into_future(),
    }
}

struct JoinRoot<F: Future> {
    future: F,
}

impl<F> JoinFuture for JoinRoot<F>
where
    F: Future,
{
    fn size(&self) -> usize {
        1
    }

    fn poll_depth(self: Pin<&mut Self>, cx: &mut Context<'_>, depth: usize) -> Poll<Self::Output> {
        assert_eq!(depth, 0);

        unsafe { self.map_unchecked_mut(|this| &mut this.future) }.poll(cx)
    }
}

struct JoinWith<F: Future, G: JoinFuture> {
    future: F,
    next: G,
}

impl<F, G> JoinFuture for JoinWith<F, G>
where
    F: Future<Output = ()>,
    G: JoinFuture,
{
    fn size(&self) -> usize {
        1 + self.next.size()
    }

    fn poll_depth(self: Pin<&mut Self>, cx: &mut Context<'_>, depth: usize) -> Poll<Self::Output> {
        if depth == 0 {
            if let Poll::Ready(()) =
                unsafe { self.map_unchecked_mut(|this| &mut this.future).poll(cx) }
            {
                // Wake because while this future finished, we still need to
                // poll so the others can finish.
                cx.waker().wake_by_ref();
            }
            Poll::Pending
        } else {
            unsafe {
                self.map_unchecked_mut(|this| &mut this.next)
                    .poll_depth(cx, depth - 1)
            }
        }
    }
}

async fn run_join<F: JoinFuture>(f: F) -> F::Output {
    let mut i = f.size();
    let mut this = pin!(f);
    poll_fn(move |cx| {
        i -= 1;
        if i == 0 {
            i = this.size();
        }
        this.as_mut().poll_depth(cx, i - 1)
    })
    .await
}

impl<F> IntoFuture for JoinRoot<F>
where
    F: Future,
{
    type Output = F::Output;

    type IntoFuture = impl Future<Output = Self::Output>;

    fn into_future(self) -> Self::IntoFuture {
        run_join(self)
    }
}

impl<F, G> IntoFuture for JoinWith<F, G>
where
    F: Future<Output = ()>,
    G: JoinFuture,
{
    type Output = G::Output;

    type IntoFuture = impl Future<Output = Self::Output>;

    fn into_future(self) -> Self::IntoFuture {
        run_join(self)
    }
}

#[cfg(test)]
mod test {
    use std::{cell::RefCell, future::poll_fn, task::Poll};

    use crate::{block_on, poll};

    use super::{join, JoinFuture};

    #[test]
    fn join_futures() {
        let cell1 = RefCell::new(0);
        let cell2 = RefCell::new(0);
        let f = join(poll_fn(|cx| {
            if *cell1.borrow() < 10 && *cell2.borrow() < 10 {
                Poll::Pending
            } else {
                Poll::Ready(*cell1.borrow() + *cell2.borrow())
            }
        }))
        .with(poll_fn(|cx| {
            *cell1.borrow_mut() += 1;
            Poll::Ready(())
        }))
        .with(poll_fn(|cx| {
            *cell2.borrow_mut() += 1;
            Poll::Ready(())
        }));
        let result = block_on(f);
        assert_eq!(result, 20);
    }
}
