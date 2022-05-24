#[macro_use]
extern crate bencher;

use std::sync::Arc;

use bencher::Bencher;
use parking_lot::RwLock;

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

struct SafeEntry(Option<Entity>);

fn entry_arc_rwlock(bencher: &mut Bencher) {
    let entry = Arc::new(RwLock::new(SafeEntry(Some(Entity { id: 123 }))));

    bencher.iter(|| {
        for _ in 0..N {
            let entry_clone = entry.clone();
            let entry_lock = entry_clone.read();
            let entity = (*entry_lock).0.as_ref().unwrap();
            prevent_opt(entity.id);
        }
    });
}

////////////////////////////////////////////////////////////////////////////////

struct UnsafeEntry(&'static mut Option<Entity>);

impl UnsafeEntry {
    fn new(entity: &mut Option<Entity>) -> Self {
        let inner = unsafe {
            let ptr = entity as *mut Option<Entity>;
            ptr.as_mut::<'static>()
        };

        Self(inner.unwrap())
    }
}

impl Clone for UnsafeEntry {
    fn clone(&self) -> Self {
        let inner = unsafe {
            let ptr = self.0 as *const Option<Entity> as *mut Option<Entity>;
            ptr.as_mut::<'static>()
        };

        Self(inner.unwrap())
    }
}

fn entry_unsafe_static_mut(bencher: &mut Bencher) {
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

benchmark_group!(benches, entry_arc_rwlock, entry_unsafe_static_mut);
benchmark_main!(benches);
