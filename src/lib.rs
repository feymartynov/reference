mod array;
mod error;

use std::any::type_name;
use std::collections::HashMap;
use std::fmt;
use std::hash::{BuildHasherDefault, Hash, Hasher};
use std::marker::PhantomData;
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
use std::sync::Arc;

use arc_swap::ArcSwapOption;
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

impl<T> From<Id<T>> for i32 {
    fn from(id: Id<T>) -> Self {
        id.id
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
/// let product = product_entry.load().unwrap();
/// let subject = product.subject.load().unwrap();
/// assert_eq!(subject.id, 1.into());
/// ```
pub struct Entry<T: 'static>(&'static ArcSwapOption<T>);

impl<T: 'static> Entry<T> {
    pub fn load(&self) -> Option<Arc<T>> {
        (*self.0.load()).as_ref().cloned()
    }
}

impl<T: fmt::Debug> fmt::Debug for Entry<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Entry({:?})", self.0)
    }
}

///////////////////////////////////////////////////////////////////////////////

/// Entity storage of `T`.
#[derive(Debug)]
pub struct Reference<T: Identifiable + 'static> {
    items: Array<Arc<ArcSwapOption<T>>>,
    vids: RwLock<FxHashMap<Id<T>, usize>>,
    effective_len: AtomicUsize,
}

impl<T: Identifiable + 'static> Reference<T> {
    /// Creates a `Referential<T>` with the given capacity and zero element as `None`.
    pub fn new(capacity: usize) -> Self {
        let items = Array::new(capacity);
        let hasher = BuildHasherDefault::<FxHasher>::default();
        let mut vids = HashMap::with_capacity_and_hasher(capacity, hasher);

        items
            .push(Arc::new(ArcSwapOption::const_empty()))
            .expect("Failed to insert zero element");

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
                let existing_item = self.items.get(vid).ok_or_else(|| {
                    Error::InsertError(format!("Index {} is out of bounds", vid,))
                })?;

                existing_item.store(Some(Arc::new(item)));
                self.effective_len.fetch_add(1, AtomicOrdering::Relaxed);
                Ok(Entry(existing_item))
            }
        }
    }

    fn add(&self, id: Id<T>, maybe_item: Option<T>) -> Result<Entry<T>, Error<T>> {
        let vid = self.items.len();

        self.items
            .push(Arc::new(ArcSwapOption::from_pointee(maybe_item)))
            .map_err(|err| Error::Other(Box::new(err)))?;

        self.effective_len.fetch_add(1, AtomicOrdering::Relaxed);
        self.vids.write().insert(id, vid);
        Ok(Entry(self.items.get(vid).unwrap()))
    }

    /// Gets an entry with the given `id`. Returns `None` if there's no item with this `id`.
    pub fn get(&self, id: Id<T>) -> Option<Entry<T>> {
        match self.vids.read().get(&id).copied() {
            None => None,
            Some(vid) => self.items.get(vid).map(|e| Entry(e)),
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
    pub fn iter(&self) -> impl Iterator<Item = Entry<T>> {
        Iter::new(self.items.iter())
    }
}

///////////////////////////////////////////////////////////////////////////////

struct Iter<T: Identifiable + 'static> {
    inner: ArrayIter<Arc<ArcSwapOption<T>>>,
}

impl<T: Identifiable + 'static> fmt::Debug for Iter<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Iter").finish()
    }
}

impl<T: Identifiable + 'static> Iter<T> {
    fn new(inner: ArrayIter<Arc<ArcSwapOption<T>>>) -> Self {
        Self { inner }
    }
}

impl<T: Identifiable + 'static> Iterator for Iter<T> {
    type Item = Entry<T>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|e| Entry(e))
    }
}
