use std::collections::HashMap;
use std::error::Error as StdError;
use std::fmt;
use std::hash::BuildHasherDefault;
use std::ops::Deref;
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

use rustc_hash::{FxHashMap, FxHasher};

use crate::array::{Array, Iter as ArrayIter};
use crate::sync::RwLock;
use crate::{Enterable, Error, Id, Identifiable, Referential};

///////////////////////////////////////////////////////////////////////////////

pub struct Entry<T: 'static>(&'static mut Option<T>);

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

impl<T> Enterable<T> for Entry<T>
where
    T: Send + Sync + Identifiable + 'static,
{
    fn update<F, E>(&mut self, f: F) -> Result<(), Error<T>>
    where
        F: Fn(&mut Option<T>) -> Result<(), E>,
        E: StdError + 'static,
    {
        f(self.0).map_err(|err| Error::UpdateError(Box::new(err)))
    }

    fn replace(&mut self, item: T) {
        *self.0 = Some(item);
    }
}

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct Reference<T: Identifiable + 'static> {
    items: Array<Option<T>>,
    vids: RwLock<FxHashMap<Id, usize>>,
    effective_len: AtomicUsize,
}

impl<T: Identifiable + 'static> Reference<T> {
    fn add(&self, id: Id, maybe_item: Option<T>) -> Result<Entry<T>, Error<T>> {
        let vid = self.items.len();

        self.items
            .push(maybe_item)
            .map_err(|err| Error::Other(Box::new(err)))?;

        self.effective_len.fetch_add(1, AtomicOrdering::Relaxed);
        self.vids.write().insert(id, vid);
        Ok(Entry(self.items.get(vid).unwrap()))
    }
}

impl<T: Send + Sync + Identifiable + 'static> Referential<T> for Reference<T> {
    type Entry = Entry<T>;
    type Iterator = Iter<T>;

    fn new(capacity: usize) -> Self {
        let items = Array::new(capacity);
        let hasher = BuildHasherDefault::<FxHasher>::default();
        let mut vids = HashMap::with_capacity_and_hasher(capacity, hasher);

        items.push(None).expect("Failed to insert zero element");
        vids.insert(0, 0);

        Self {
            items,
            vids: RwLock::new(vids),
            effective_len: AtomicUsize::new(0),
        }
    }

    fn insert(&self, item: T) -> Result<Self::Entry, Error<T>> {
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
                let item_ref = self.items.get(vid).ok_or_else(|| {
                    Error::InsertError(format!("Index {} is out of bounds", vid,))
                })?;

                *item_ref = Some(item);
                self.effective_len.fetch_add(1, AtomicOrdering::Relaxed);
                Ok(Entry(item_ref))
            }
        }
    }

    fn get(&self, id: Id) -> Option<Self::Entry> {
        match self.vids.read().get(&id).copied() {
            None => None,
            Some(vid) => self.items.get(vid).map(|e| Entry(e)),
        }
    }

    fn get_or_reserve(&self, id: Id) -> Result<Self::Entry, Error<T>> {
        match self.get(id) {
            Some(entry) => Ok(entry),
            None => self.add(id, None),
        }
    }

    fn iter(&self) -> Self::Iterator {
        Iter::new(self.items.iter())
    }
}

///////////////////////////////////////////////////////////////////////////////

pub struct Iter<T: Identifiable + 'static> {
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
