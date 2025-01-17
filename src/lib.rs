//! A library for scoped `Vec`s, allowing multi-level divergence from
//! the root element.
//!
//! This is useful for monitoring state within a de facto tree where
//! links to parents aren't necessarily needed. Consumers can keep
//! references to a specific parent if required and check the values
//! from the scope of their choosing, parents are free to be dropped if
//! they're no longer required.
//!
//! The full [std::vec::Vec] spec has not yet been implemented but as
//! the library stabilises, more and more of the `Vec` library will be
//! supported - however there will be some divergence from the API where
//! necessary given the structural differences of a `ScopedVec`.
//!
//! Example:
//! ```
//! # use scoped_vec::ScopedVec;
//! let mut root = ScopedVec::new();
//! root.push(3);
//!
//! {
//!     let mut scope1 = root.scope();
//!     scope1.push(4);
//!     {
//!         let mut scope1_scope1 = scope1.scope();
//!         scope1_scope1.push(5);
//!     }
//!
//!     let mut iter = scope1.iter();
//!     assert_eq!(iter.next(), Some(&4));
//!     assert_eq!(iter.next(), Some(&5));
//!     assert_eq!(iter.next(), None);
//! }
//!
//! {
//!     let mut scope2 = root.scope();
//!     scope2.push(6);
//! }
//!
//! let mut iter = root.iter();
//! assert_eq!(iter.next(), Some(&3));
//! assert_eq!(iter.next(), Some(&4));
//! assert_eq!(iter.next(), Some(&5));
//! assert_eq!(iter.next(), Some(&6));
//! assert_eq!(iter.next(), None);
//! ```

use std::sync::{Arc, RwLock, RwLockReadGuard};
use owning_ref::OwningHandle;

/// A `ScopedVec` instance can either represent the root element or a
/// divergence of it. Refer to the crate's documentation for usage
/// examples of the scoped-vec library.
///
/// Cloning a `ScopedVec` will result in a reference to the same scope,
/// and adding a value to one of the cloned instances will result in
/// the value being added to all instances and available for all the
/// parent instances to iterate over.
#[derive(Clone)]
pub struct ScopedVec<T: Clone> {
    inner: Arc<RwLock<Vec<T>>>,
    children: Arc<RwLock<Vec<ScopedVec<T>>>>,
}

impl<T: Clone> ScopedVec<T> {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::default()),
            children: Arc::new(RwLock::default())
        }
    }

    /// Create a new `ScopedVec` as a child of this one.
    pub fn scope(&mut self) -> ScopedVec<T> {
        let new = ScopedVec::new();
        //           .get_mut()?
        self.children.write().unwrap().push(new.clone());
        new
    }

    pub fn push(&mut self, val: T) {
        //        .get_mut()?
        self.inner.write().unwrap().push(val);
    }

    pub fn iter(&self) -> ScopedVecIterator<T> {
        ScopedVecIterator::new(self)
    }
}

impl<T: Clone + PartialEq> ScopedVec<T> {
    pub fn contains(&self, val: &T) -> bool {
        self.iter().any(|f| *f == *val)
    }
}

pub struct ScopedVecGuardHolder<'a, T: Clone> {
    inner: RwLockReadGuard<'a, Vec<T>>,
    children: RwLockReadGuard<'a, Vec<ScopedVec<T>>>,
}

pub struct ScopedVecIterator<'a, T: Clone> {
    iterator: OwningHandle<Box<ScopedVecGuardHolder<'a, T>>, Box<dyn Iterator<Item = &'a T> + 'a>>,
}
impl<'a, T: Clone> ScopedVecIterator<'a, T> {
    fn new(vec: &'a ScopedVec<T>) -> Self {
        Self {
            iterator: OwningHandle::new_with_fn(
                Box::new(ScopedVecGuardHolder {
                    inner: vec.inner.read().unwrap(),
                    children: vec.children.read().unwrap()
                }),
                |g| {
                    // the value behind the raw pointer `g` is boxed, so we're safe to dereference
                    let guards = unsafe { &*g };

                    Box::new(guards.inner.iter()
                        .chain(
                            guards.children.iter()
                                .map(ScopedVec::iter)
                                .flatten()
                        )) as Box<dyn Iterator<Item = &'a T>>
                }
            )
        }
    }
}
impl<'a, T: Clone> Iterator for ScopedVecIterator<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.iterator.next()
    }
}

#[cfg(test)]
mod tests {
    use crate::ScopedVec;

    #[test]
    fn unscoped_standard() {
        let mut root = ScopedVec::new();
        root.push(3);
        let mut iter = root.iter();
        assert_eq!(iter.next(), Some(&3));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn scoped_cant_read_root() {
        let mut root = ScopedVec::new();
        root.push(3);

        let scoped = root.scope();
        let mut iter = scoped.iter();
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn root_can_read_scoped() {
        let mut root = ScopedVec::new();
        root.push(3);

        let mut scoped = root.scope();
        scoped.push(4);

        let mut iter = root.iter();
        assert_eq!(iter.next(), Some(&3));
        assert_eq!(iter.next(), Some(&4));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn root_can_read_nested_scoped() {
        let mut root = ScopedVec::new();
        root.push(3);

        let mut scoped = root.scope();
        scoped.push(4);

        let mut nested_scoped = scoped.scope();
        nested_scoped.push(5);

        let mut iter = root.iter();
        assert_eq!(iter.next(), Some(&3));
        assert_eq!(iter.next(), Some(&4));
        assert_eq!(iter.next(), Some(&5));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn scoped_can_read_nested_scoped() {
        let mut root = ScopedVec::new();
        root.push(3);

        let mut scoped = root.scope();
        scoped.push(4);

        let mut nested_scoped = scoped.scope();
        nested_scoped.push(5);

        let mut iter = scoped.iter();
        assert_eq!(iter.next(), Some(&4));
        assert_eq!(iter.next(), Some(&5));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn nested_scoped_cant_read_backwards() {
        let mut root = ScopedVec::new();
        root.push(3);

        let mut scoped = root.scope();
        scoped.push(4);

        let mut nested_scoped = scoped.scope();
        nested_scoped.push(5);

        let mut iter = nested_scoped.iter();
        assert_eq!(iter.next(), Some(&5));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn can_drop_scopes() {
        let mut root = ScopedVec::new();
        root.push(3);

        let mut scoped = root.scope();
        scoped.push(4);

        drop(root);

        let mut nested_scoped = scoped.scope();
        nested_scoped.push(5);

        {
            let mut iter = scoped.iter();
            assert_eq!(iter.next(), Some(&4));
            assert_eq!(iter.next(), Some(&5));
            assert_eq!(iter.next(), None);
        }

        drop(scoped);

        {
            let mut iter = nested_scoped.iter();
            assert_eq!(iter.next(), Some(&5));
            assert_eq!(iter.next(), None);
        }
    }

    #[test]
    fn diverged_scopes_can_be_read() {
        let mut root = ScopedVec::new();
        root.push(3);

        let mut scoped = root.scope();
        scoped.push(4);

        let mut nested_scoped1 = scoped.scope();
        nested_scoped1.push(5);

        let mut nested_scoped2 = scoped.scope();
        nested_scoped2.push(6);

        let mut iter = root.iter();
        assert_eq!(iter.next(), Some(&3));
        assert_eq!(iter.next(), Some(&4));
        assert_eq!(iter.next(), Some(&5));
        assert_eq!(iter.next(), Some(&6));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn diverged_adjacent_scopes_cant_interact() {
        let mut root = ScopedVec::new();
        root.push(3);

        let mut scoped1 = root.scope();
        scoped1.push(4);

        let mut scoped2 = root.scope();
        scoped2.push(5);

        let mut iter = scoped1.iter();
        assert_eq!(iter.next(), Some(&4));
        assert_eq!(iter.next(), None);

        let mut iter = scoped2.iter();
        assert_eq!(iter.next(), Some(&5));
        assert_eq!(iter.next(), None);
    }
}
