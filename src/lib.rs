//! Extension traits for dealing with [`Option`]s as [`Future`]s or [`Stream`]s.
//!
//! # Examples
//!
//! ```rust
//! #![feature(async_await)]
//!
//! use futures::future::{self, FusedFuture as _};
//! use futures_option::OptionExt as _;
//! # futures::executor::block_on(async {
//! let mut f = Some(future::ready::<u32>(1));
//! assert!(f.is_some());
//! assert_eq!(f.current().await, 1);
//! assert!(f.is_none());
//! assert!(f.current().is_terminated());
//! # });
//! ```
//!
//! This is useful when you want to implement optional branches using the
//! `select!` macro.
//!
//! ```rust
//! #![feature(async_await)]
//! #![recursion_limit="128"]
//!
//! use futures::{future, stream, StreamExt as _};
//! use futures_option::OptionExt as _;
//! # futures::executor::block_on(async {
//! let mut value = None;
//! let mut values = Some(stream::iter(vec![1u32, 2u32, 4u32].into_iter()).fuse());
//! let mut parked = None;
//!
//! let mut sum = 0;
//!
//! loop {
//!     futures::select! {
//!         value = value.current() => {
//!             sum += value;
//!             std::mem::swap(&mut parked, &mut values);
//!         }
//!         v = values.next() => {
//!             match v {
//!                 Some(v) => {
//!                     value = Some(future::ready(v));
//!                     std::mem::swap(&mut parked, &mut values);
//!                 },
//!                 None => break,
//!             }
//!         }
//!     }
//! }
//!
//! assert_eq!(7, sum);
//! # });
//! ```
//!
//! [`Option`]: Option
//! [`Stream`]: futures_core::stream::Stream
//! [`Future`]: futures_core::future::Future

use futures::{
    future::{FusedFuture, FutureExt as _},
    ready,
    stream::{Stream, StreamExt as _},
};
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

impl<T> OptionExt<T> for Option<T> {
    fn next(&mut self) -> Next<'_, T>
    where
        T: Stream + Unpin,
    {
        Next { stream: self }
    }

    fn select_next_some(&mut self) -> SelectNextSome<'_, T>
    where
        T: Stream + Unpin,
    {
        SelectNextSome { stream: self }
    }

    fn current(&mut self) -> Current<'_, T>
    where
        T: Future + Unpin,
    {
        Current { future: self }
    }
}

/// Extension methods for [`Option`] of [`Stream`]s or [`Future`]s.
///
/// [`Option`]: std::option::Option
/// [`Stream`]: Stream
/// [`Future`]: Future
pub trait OptionExt<T>: Sized {
    /// Convert [`Option`] into a Future that resolves to the next item in the [`Stream`].
    ///
    /// If the [`Option`] is [`None`], the returned future also resolves to [`None`].
    ///
    /// [`Option`]: Option
    /// [`None`]: Option::None
    /// [`Stream`]: Stream
    fn next(&mut self) -> Next<'_, T>
    where
        T: Stream + Unpin;

    /// Returns a [Future] that resolves when the next item in this stream is ready.
    ///
    /// If the [`Option`] is [`None`], the returned future will always be pending.
    ///
    /// [Future]: Future
    fn select_next_some(&mut self) -> SelectNextSome<'_, T>
    where
        T: Stream + Unpin;

    /// Convert [`Option`] into a Future that resolves to the same value as the stored [`Future`].
    ///
    /// If the [`Option`] is [`None`], the returned future also resolves to [`None`].
    ///
    /// [`Option`]: Option
    /// [`None`]: Option::None
    /// [`Future`]: Future
    fn current(&mut self) -> Current<'_, T>
    where
        T: Future + Unpin;
}

/// Adapter future for `Option` to get the next value of the stored future.
///
/// Resolves to `None` if no future is present.
#[derive(Debug)]
pub struct Next<'a, T> {
    pub(crate) stream: &'a mut Option<T>,
}

impl<'a, T> Future for Next<'a, T>
where
    T: Stream + Unpin,
{
    type Output = Option<T::Item>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        assert!(self.stream.is_some(), "Next polled after terminated");

        if let Some(stream) = self.stream.as_mut() {
            if let Some(result) = ready!(stream.poll_next_unpin(cx)) {
                return Poll::Ready(Some(result));
            }
        }

        *self.stream = None;
        Poll::Ready(None)
    }
}

impl<'a, T> FusedFuture for Next<'a, T> {
    fn is_terminated(&self) -> bool {
        self.stream.is_none()
    }
}

/// Adapter future for `Option` to get the next value of the stored future.
///
/// Resolves to `None` if no future is present.
#[derive(Debug)]
pub struct SelectNextSome<'a, T> {
    pub(crate) stream: &'a mut Option<T>,
}

impl<'a, T> Future for SelectNextSome<'a, T>
where
    T: Stream + Unpin,
{
    type Output = T::Item;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        assert!(
            self.stream.is_some(),
            "SelectNextSome polled after terminated"
        );

        if let Some(stream) = self.stream.as_mut() {
            if let Some(result) = ready!(stream.poll_next_unpin(cx)) {
                return Poll::Ready(result);
            }
        }

        *self.stream = None;
        cx.waker().wake_by_ref();
        return Poll::Pending;
    }
}

impl<'a, T> FusedFuture for SelectNextSome<'a, T> {
    fn is_terminated(&self) -> bool {
        self.stream.is_none()
    }
}

/// Adapter future for `Option` to get the next value of the stored stream.
///
/// Resolves to `None` if no stream is present.
#[derive(Debug)]
pub struct Current<'a, T> {
    pub(crate) future: &'a mut Option<T>,
}

impl<'a, T> Future for Current<'a, T>
where
    T: Future + Unpin,
{
    type Output = T::Output;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let future = self
            .future
            .as_mut()
            .expect("Current polled after terminated");
        let result = ready!(future.poll_unpin(cx));
        // NB: we do this to mark the future as terminated.
        *self.future = None;
        Poll::Ready(result)
    }
}

impl<'a, T> FusedFuture for Current<'a, T> {
    fn is_terminated(&self) -> bool {
        self.future.is_none()
    }
}
