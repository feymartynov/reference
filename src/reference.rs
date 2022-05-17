use std::error::Error as StdError;
use std::ops::Deref;

use crate::Error;

pub type Id = i32;

pub trait Identifiable {
    fn id(&self) -> Id;
}

pub trait Enterable<T>: Deref<Target = Option<T>> + Send + Sync
where
    T: Send + Sync + Identifiable + 'static,
{
    fn update<F, E>(&mut self, f: F) -> Result<(), Error<T>>
    where
        F: Fn(&mut Option<T>) -> Result<(), E>,
        E: StdError + 'static;

    fn replace(&mut self, item: T);
}

pub trait Referential<T>: Send + Sync
where
    T: Send + Sync + Identifiable + 'static,
{
    type Entry: Enterable<T>;
    type Iterator: Iterator<Item = Option<&'static T>>;

    fn new(capacity: usize) -> Self;
    fn insert(&self, item: T) -> Result<Self::Entry, Error<T>>;
    fn get(&self, id: Id) -> Option<Self::Entry>;
    fn get_or_reserve(&self, id: Id) -> Result<Self::Entry, Error<T>>;
    fn iter(&self) -> Self::Iterator;
}
