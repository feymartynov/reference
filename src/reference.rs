use std::error::Error as StdError;
use std::ops::Deref;

use crate::Error;

pub type Id = i32;

pub trait Identifiable {
    fn id(&self) -> Id;
}

pub trait Enterable<'a, T>: Deref<Target = Option<T>> + Send + Sync
where
    T: Send + Sync + Identifiable + 'a,
{
    fn update<F, E>(&mut self, f: F) -> Result<(), Error<T>>
    where
        F: Fn(&mut Option<T>) -> Result<(), E>,
        E: StdError + 'static;

    fn replace(&mut self, item: T);
}

pub trait Referential<'a, T>: Send + Sync
where
    T: Send + Sync + Identifiable + 'a,
{
    type Entry: Enterable<'a, T>;
    type Iterator: Iterator<Item = Option<&'a T>> + 'a;

    fn new(capacity: usize) -> Self;
    fn insert(&'a self, item: T) -> Result<Self::Entry, Error<T>>;
    fn get(&'a self, id: Id) -> Option<Self::Entry>;
    fn get_or_reserve(&'a self, id: Id) -> Result<Self::Entry, Error<T>>;
    fn iter(&'a self) -> Self::Iterator;
}
