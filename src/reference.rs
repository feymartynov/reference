use std::error::Error as StdError;
use std::ops::Deref;

use crate::Error;

pub type Id = i32;

/// An entity which can be identified by id.
pub trait Identifiable {
    fn id(&self) -> Id;
}

/// An entry of `Referential`.
/// Items of one `Referential` may refer to items of another by holding an entry in a field:
///
/// ```rust
/// struct Subject {
///     id: Id,
/// }
///
/// struct Product {
///     id: Id,
///     subject: Entry<Subject>,
/// }
///
/// struct Ctx {
///     products: Reference<Product>,
///     subjects: Reference<Subject>,
/// }
/// ```
///
/// An entry can be dereferenced using `*` operator to access fields of the referred entity:
///
/// ```rust
/// (*product.subject).unwrap().id
/// ```
///
/// Also entry can be used to modify the referred entity using `update` or `replace` methods.
pub trait Enterable<T>: Deref<Target = Option<T>> + Send + Sync
where
    T: Send + Sync + Identifiable + 'static,
{
    /// Update the referred entity with a closure.
    /// The closure accept a mutable reference to the referred entity as an `Option` and must return
    /// the `Result` of the update.
    fn update<F, E>(&mut self, f: F) -> Result<(), Error<T>>
    where
        F: Fn(&mut Option<T>) -> Result<(), E>,
        E: StdError + 'static;

    /// Sets or replaces the referred entity with the new one.
    fn replace(&mut self, item: T);
}

/// Entity storage of `T`.
pub trait Referential<T>: Send + Sync
where
    T: Send + Sync + Identifiable + 'static,
{
    type Entry: Enterable<T>;
    type Iterator: Iterator<Item = Option<&'static T>>;

    /// Creates a `Referential<T>` with the given capacity and zero element as `None`.
    fn new(capacity: usize) -> Self;

    /// Adds a new element to the storage.
    fn insert(&self, item: T) -> Result<Self::Entry, Error<T>>;

    /// Gets an entry with the given `id`. Returns `None` if there's no item with this `id`.
    fn get(&self, id: Id) -> Option<Self::Entry>;

    /// Like `get` but if the item is not found it initializes an `Entry` with `None` value
    /// for the given `id`. The `Entry` may be set later using `replace` method.
    /// This method is useful when you want to fill the reference of dependent items first
    /// and add referred entities into another reference later.
    fn get_or_reserve(&self, id: Id) -> Result<Self::Entry, Error<T>>;

    /// Creates a reader iterator over items.
    fn iter(&self) -> Self::Iterator;
}
