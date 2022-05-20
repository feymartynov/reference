mod array;
mod error;

use std::any::type_name;
use std::collections::HashMap;
use std::error::Error as StdError;
use std::fmt;
use std::hash::{BuildHasherDefault, Hash, Hasher};
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

use parking_lot::RwLock;
use rustc_hash::{FxHashMap, FxHasher};

use self::array::{Array, Iter as ArrayIter};
pub use self::error::Error;

///////////////////////////////////////////////////////////////////////////////

/// Entity identifier.
#[derive(Default)]
pub struct Id<T> {
    id: i32,
    _phantom: PhantomData<T>,
}

impl<T> Id<T> {
    pub fn new(id: i32) -> Self {
        Self {
            id,
            _phantom: PhantomData,
        }
    }

    pub fn as_i32(self) -> i32 {
        self.id
    }
}

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        Id::new(self.id)
    }
}

impl<T> Copy for Id<T> {}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T> Eq for Id<T> {}

impl<T> Hash for Id<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<T> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Id<{}>({})", type_name::<T>(), self.id)
    }
}

impl<T> fmt::Display for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.id)
    }
}

impl<T> From<i32> for Id<T> {
    fn from(id: i32) -> Self {
        Self::new(id)
    }
}

impl<T> Into<i32> for Id<T> {
    fn into(self) -> i32 {
        self.id
    }
}

/// An entity which can be identified by id.
pub trait Identifiable {
    fn id(&self) -> Id<Self>
    where
        Self: Sized;
}

///////////////////////////////////////////////////////////////////////////////

/// An entry of `Referential`.
/// Items of one `Referential` may refer to items of another by holding an entry in a field:
///
/// ```
/// # use reference::{Id, Identifiable, Entry, Reference};
/// #
/// struct Subject {
///     id: Id<Self>,
/// }
/// #
/// # impl Identifiable for Subject {
/// #     fn id(&self) -> Id<Self> {
/// #         self.id
/// #     }
/// # }
///
/// struct Product {
///     id: Id<Self>,
///     subject: Entry<Subject>,
/// }
/// #
/// # impl Identifiable for Product {
/// #     fn id(&self) -> Id<Self> {
/// #         self.id
/// #     }
/// # }
///
/// struct Ctx {
///     products: Reference<Product>,
///     subjects: Reference<Subject>,
/// }
/// ```
///
/// An entry can be dereferenced using `*` operator to access fields of the referred entity:
///
/// ```
/// # use reference::{Id, Identifiable, Entry, Reference};
/// #
/// # #[derive(Debug)]
/// # struct Subject {
/// #     id: Id<Self>,
/// # }
/// #
/// # impl Identifiable for Subject {
/// #     fn id(&self) -> Id<Self> {
/// #         self.id
/// #     }
/// # }
/// #
/// # #[derive(Debug)]
/// # struct Product {
/// #     id: Id<Self>,
/// #     subject: Entry<Subject>,
/// # }
/// #
/// # impl Identifiable for Product {
/// #     fn id(&self) -> Id<Self> {
/// #         self.id
/// #     }
/// # }
/// #
/// # let subjects = Reference::new(2);
/// # let subject_entry = subjects.insert(Subject { id: 1.into() }).unwrap();
/// # let products = Reference::new(2);
/// #
/// # let product_entry = products
/// #   .insert(Product {
/// #        id: 100.into(),
/// #        subject: subject_entry,
/// #   })
/// #   .unwrap();
/// #
/// let product = product_entry.as_ref().unwrap();
/// let subject = product.subject.as_ref().unwrap();
/// assert_eq!(subject.id, 1.into());
/// ```
///
/// Also entry can be used to modify the referred entity using `update` or `replace` methods.
pub struct Entry<T: 'static>(&'static mut Option<T>);

impl<T> Entry<T>
where
    T: Send + Sync + Identifiable + 'static,
{
    /// Updates the referred entity with a closure.
    /// The closure accepts a mutable reference to the referred entity as an `Option` and must
    /// return the `Result` of the update.
    pub fn update<F, E>(&mut self, f: F) -> Result<(), Error<T>>
    where
        F: Fn(&mut Option<T>) -> Result<(), E>,
        E: StdError + 'static,
    {
        f(self.0).map_err(|err| Error::UpdateError(Box::new(err)))
    }

    /// Sets or replaces the referred entity with the new one.
    pub fn replace(&mut self, item: T) {
        *self.0 = Some(item);
    }
}

impl<'a, T> fmt::Debug for Entry<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Entry").finish()
    }
}

impl<'a, T> Deref for Entry<T> {
    type Target = Option<T>;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

///////////////////////////////////////////////////////////////////////////////

/// Entity storage of `T`.
#[derive(Debug)]
pub struct Reference<T: Identifiable + 'static> {
    items: Array<Option<T>>,
    vids: RwLock<FxHashMap<Id<T>, usize>>,
    effective_len: AtomicUsize,
}

impl<T: Identifiable + 'static> Reference<T> {
    /// Creates a `Referential<T>` with the given capacity and zero element as `None`.
    pub fn new(capacity: usize) -> Self {
        let items = Array::new(capacity);
        let hasher = BuildHasherDefault::<FxHasher>::default();
        let mut vids = HashMap::with_capacity_and_hasher(capacity, hasher);

        items.push(None).expect("Failed to insert zero element");
        vids.insert(Id::from(0), 0);

        Self {
            items,
            vids: RwLock::new(vids),
            effective_len: AtomicUsize::new(0),
        }
    }

    /// Adds a new element to the storage or replaces existing one.
    pub fn insert(&self, item: T) -> Result<Entry<T>, Error<T>> {
        let id = item.id();

        let maybe_existing_vid = {
            let vids = self.vids.read();
            let maybe_vid = vids.get(&id).copied();

            if maybe_vid.is_none() && vids.contains_key(&id) {
                return Err(Error::InsertError(format!(
                    "Failed to add id {} because it already exists",
                    id,
                )));
            }

            maybe_vid
        };

        match maybe_existing_vid {
            None => self.add(id, Some(item)),
            Some(vid) => {
                let item_ref = self.items.get_mut(vid).ok_or_else(|| {
                    Error::InsertError(format!("Index {} is out of bounds", vid,))
                })?;

                *item_ref = Some(item);
                self.effective_len.fetch_add(1, AtomicOrdering::Relaxed);
                Ok(Entry(item_ref))
            }
        }
    }

    fn add(&self, id: Id<T>, maybe_item: Option<T>) -> Result<Entry<T>, Error<T>> {
        let vid = self.items.len();

        self.items
            .push(maybe_item)
            .map_err(|err| Error::Other(Box::new(err)))?;

        self.effective_len.fetch_add(1, AtomicOrdering::Relaxed);
        self.vids.write().insert(id, vid);
        Ok(Entry(self.items.get_mut(vid).unwrap()))
    }

    /// Gets an entry with the given `id`. Returns `None` if there's no item with this `id`.
    pub fn get(&self, id: Id<T>) -> Option<Entry<T>> {
        match self.vids.read().get(&id).copied() {
            None => None,
            Some(vid) => self.items.get_mut(vid).map(|e| Entry(e)),
        }
    }

    /// Like `get` but if the item is not found it initializes an `Entry` with `None` value
    /// for the given `id`. The `Entry` may be set later using `replace` method.
    /// This method is useful when you want to fill the reference of dependent items first
    /// and add referred entities into another reference later.
    pub fn get_or_reserve(&self, id: Id<T>) -> Result<Entry<T>, Error<T>> {
        match self.get(id) {
            Some(entry) => Ok(entry),
            None => self.add(id, None),
        }
    }

    /// Creates a reader iterator over items.
    pub fn iter(&self) -> impl Iterator<Item = Option<&'static T>> {
        Iter::new(self.items.iter())
    }
}

///////////////////////////////////////////////////////////////////////////////

struct Iter<T: Identifiable + 'static> {
    inner: ArrayIter<Option<T>>,
}

impl<T: Identifiable> fmt::Debug for Iter<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Iter").finish()
    }
}

impl<T: Identifiable> Iter<T> {
    fn new(inner: ArrayIter<Option<T>>) -> Self {
        Self { inner }
    }
}

impl<T: Identifiable> Iterator for Iter<T> {
    type Item = Option<&'static T>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|item| item.as_ref())
    }
}
