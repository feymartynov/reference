#[macro_use]
extern crate bencher;

use std::sync::{Arc, RwLock as StdRwLock};

use arc_swap::ArcSwap;
use bencher::Bencher;
use crossbeam_utils::sync::ShardedLock;
use parking_lot::RwLock as ParkingLotRwLock;

const N: usize = 1000;

fn prevent_opt<T: Default>(value: T) {
    let mut local = std::mem::MaybeUninit::new(T::default());
    let ptr = local.as_mut_ptr();
    unsafe { ptr.write_volatile(value) };
}

struct Entity {
    id: i32,
}

////////////////////////////////////////////////////////////////////////////////

fn entry_std_rwlock_arc(bencher: &mut Bencher) {
    let entry = StdRwLock::new(Arc::new(Some(Entity { id: 123 })));

    bencher.iter(|| {
        for _ in 0..N {
            let entry_lock = entry.read().unwrap();
            let entity = (**entry_lock).as_ref().unwrap();
            prevent_opt(entity.id);
        }
    });
}

fn entry_parking_lot_rwlock_arc(bencher: &mut Bencher) {
    let entry = ParkingLotRwLock::new(Arc::new(Some(Entity { id: 123 })));

    bencher.iter(|| {
        for _ in 0..N {
            let entry_lock = entry.read();
            let entity = (**entry_lock).as_ref().unwrap();
            prevent_opt(entity.id);
        }
    });
}

fn entry_sharded_lock_arc(bencher: &mut Bencher) {
    let entry = ShardedLock::new(Arc::new(Some(Entity { id: 123 })));

    bencher.iter(|| {
        for _ in 0..N {
            let entry_lock = entry.read().unwrap();
            let entity = (**entry_lock).as_ref().unwrap();
            prevent_opt(entity.id);
        }
    });
}

fn entry_arc_swap(bencher: &mut Bencher) {
    let entry = ArcSwap::from(Arc::new(Some(Entity { id: 123 })));

    bencher.iter(|| {
        for _ in 0..N {
            let guard = entry.load();
            let entity = (**guard).as_ref().unwrap();
            prevent_opt(entity.id);
        }
    });
}

struct UnsafeEntry<'a>(&'a mut Option<Entity>);

impl<'a> UnsafeEntry<'a> {
    fn new(entity: &'a mut Option<Entity>) -> Self {
        let inner = unsafe {
            let ptr = entity as *mut Option<Entity>;
            ptr.as_mut::<'static>()
        };

        Self(inner.unwrap())
    }
}

impl<'a> Clone for UnsafeEntry<'a> {
    fn clone(&self) -> Self {
        let inner = unsafe {
            let ptr = self.0 as *const Option<Entity> as *mut Option<Entity>;
            ptr.as_mut::<'a>()
        };

        Self(inner.unwrap())
    }
}

fn entry_unsafe_mut(bencher: &mut Bencher) {
    let mut entity = Some(Entity { id: 123 });
    let entry = UnsafeEntry::new(&mut entity);

    bencher.iter(|| {
        for _ in 0..N {
            let entry_clone = entry.clone();
            let entity = (*entry_clone.0).as_ref().unwrap();
            prevent_opt(entity.id);
        }
    });
}

////////////////////////////////////////////////////////////////////////////////

benchmark_group!(
    benches,
    entry_std_rwlock_arc,
    entry_parking_lot_rwlock_arc,
    entry_sharded_lock_arc,
    entry_arc_swap,
    entry_unsafe_mut
);

benchmark_main!(benches);
