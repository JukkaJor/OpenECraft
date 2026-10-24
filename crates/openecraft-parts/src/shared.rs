//! Shared, immutable payloads: heavy values (meshes, embedded files) that whole
//! project snapshots are compared and cloned with. Two [`Shared`] values that point to
//! the same allocation compare equal without looking inside, cloning copies a pointer,
//! and the JSON form is that of the value itself.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::Deref;
use std::sync::Arc;

#[derive(Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Shared<T>(Arc<T>);

impl<T> Shared<T> {
    pub fn new(value: T) -> Shared<T> {
        Shared(Arc::new(value))
    }

    /// Mutable access; copies the value first if it is shared with another holder.
    pub fn make_mut(&mut self) -> &mut T
    where
        T: Clone,
    {
        Arc::make_mut(&mut self.0)
    }

    /// True if both point to the same allocation.
    pub fn ptr_eq(&self, other: &Shared<T>) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl<T> Clone for Shared<T> {
    fn clone(&self) -> Self {
        Shared(self.0.clone())
    }
}

impl<T> Deref for Shared<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> AsRef<T> for Shared<T> {
    fn as_ref(&self) -> &T {
        &self.0
    }
}

impl<T> From<T> for Shared<T> {
    fn from(value: T) -> Self {
        Shared::new(value)
    }
}

impl<T: PartialEq> PartialEq for Shared<T> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0) || *self.0 == *other.0
    }
}

impl<T: fmt::Debug> fmt::Debug for Shared<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<T: fmt::Display> fmt::Display for Shared<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests;
