use std::alloc::Layout;
use std::error::Error as StdError;
use std::fmt::{self, Debug};
use std::ops::{Deref, DerefMut};
use std::ptr::NonNull;
use std::sync::atomic::{AtomicUsize, Ordering};

///////////////////////////////////////////////////////////////////////////////

pub struct Array<T> {
    ptr: NonNull<T>,
    capacity: usize,
    len: AtomicUsize,
}

impl<T: 'static> Array<T> {
    pub fn new(capacity: usize) -> Self {
        let layout = Layout::array::<T>(capacity).unwrap();
        let ptr = unsafe { std::alloc::alloc(layout) };

        let ptr = match NonNull::new(ptr as *mut T) {
            Some(ptr) => ptr,
            None => std::alloc::handle_alloc_error(layout),
        };

        Self {
            ptr,
            capacity,
            len: AtomicUsize::new(0),
        }
    }

    pub fn push(&self, item: T) -> Result<&mut T, Error> {
        let len = self.len();

        if len >= self.capacity {
            return Err(Error::CapacityExceeded {
                capacity: self.capacity,
            });
        }

        let ptr = unsafe {
            let ptr = self.ptr.as_ptr().add(len);
            std::ptr::write(ptr, item);
            &mut *ptr
        };

        self.len.fetch_add(1, Ordering::Relaxed);
        Ok(ptr)
    }

    pub fn get(&self, idx: usize) -> Option<&'static mut T> {
        if idx < self.len() {
            Some(unsafe { self.get_mut_unchecked(idx) })
        } else {
            None
        }
    }

    #[allow(clippy::mut_from_ref)]
    pub unsafe fn get_mut_unchecked(&self, idx: usize) -> &'static mut T {
        &mut *self.ptr.as_ptr().add(idx)
    }

    pub fn iter(&self) -> Iter<T> {
        Iter::new(self)
    }

    pub fn len(&self) -> usize {
        self.len.load(Ordering::Relaxed)
    }
}

unsafe impl<T: Send> Send for Array<T> {}
unsafe impl<T: Sync> Sync for Array<T> {}

impl<T: 'static> Deref for Array<T> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.len()) }
    }
}

impl<T: 'static> DerefMut for Array<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len()) }
    }
}

impl<T: fmt::Debug + 'static> fmt::Debug for Array<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<T: 'static> From<Vec<T>> for Array<T> {
    fn from(items: Vec<T>) -> Self {
        let array = Self::new(items.len() + 1);

        for item in items {
            if let Err(err) = array.push(item) {
                panic!("Failed to add an item to array: {err:#}");
            }
        }

        array
    }
}

///////////////////////////////////////////////////////////////////////////////

pub struct Iter<T: 'static> {
    array: &'static Array<T>,
    len: usize,
    idx: usize,
}

impl<T: 'static> Iter<T> {
    fn new(array: &Array<T>) -> Self {
        let len = array.len();
        let ptr = array as *const Array<T>;
        let array = unsafe { ptr.as_ref::<'static>() }.unwrap();
        Self { array, len, idx: 0 }
    }
}

impl<T> Iterator for Iter<T> {
    type Item = &'static T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx < self.len {
            let item = unsafe { self.array.get_unchecked(self.idx) };
            self.idx += 1;
            Some(item)
        } else {
            None
        }
    }
}

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub enum Error {
    CapacityExceeded { capacity: usize },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CapacityExceeded { capacity } => write!(f, "Capacity exceeded ({})", capacity),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::CapacityExceeded { .. } => None,
        }
    }
}
