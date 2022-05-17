/// This benchmark tests different id access strategies.
/// First it creates partially pre-filled structure of ids.
/// Then it starts an updater thread which periodically adds more values to simulate writer load.
/// Then in measures read access time.

#[macro_use]
extern crate bencher;

use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, HashMap};
use std::hash::{BuildHasherDefault, Hasher};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock as StdRwLock};
use std::thread;
use std::time::Duration;

use bencher::Bencher;
use lockfree::map::Map as LockFreeMap;
use nohash_hasher::NoHashHasher;
use parking_lot::RwLock as ParkingLotRwLock;
use rand::prelude::*;
use rustc_hash::FxHasher;

type Id = i32;

const SIZE: usize = 1_000_000;
const FILLED_PERCENTAGE: usize = 90;
const LAST_FILLED_ID: Id = (SIZE * FILLED_PERCENTAGE / 100) as Id;
const UPDATER_PERIOD_MS: u64 = 100;
const READS_PER_BENCH_ITER: usize = 10_000;

fn prevent_opt<T: Default>(value: T) {
    let mut local = std::mem::MaybeUninit::new(T::default());
    let ptr = local.as_mut_ptr();
    unsafe { ptr.write_volatile(value) };
}

///////////////////////////////////////////////////////////////////////////////

struct RwLockBTreeMapUpdater {
    is_halt: Arc<AtomicBool>,
}

impl RwLockBTreeMapUpdater {
    fn start(ids: Arc<StdRwLock<BTreeMap<Id, usize>>>) -> Self {
        let is_halt = Arc::new(AtomicBool::new(false));
        let is_halt_clone = is_halt.clone();

        thread::spawn(move || {
            let mut rng = rand::thread_rng();

            while !is_halt_clone.load(Ordering::Relaxed) {
                let id = rng.gen_range(LAST_FILLED_ID..(SIZE as Id));
                ids.write().expect("Failed to get write lock").insert(id, 0);
                thread::sleep(Duration::from_millis(UPDATER_PERIOD_MS));
            }
        });

        Self { is_halt }
    }
}

impl Drop for RwLockBTreeMapUpdater {
    fn drop(&mut self) {
        self.is_halt.store(true, Ordering::SeqCst);
    }
}

fn id_index_rwlock_btree_map(bencher: &mut Bencher) {
    let mut ids = BTreeMap::new();

    for id in 0..LAST_FILLED_ID {
        ids.insert(id, 0);
    }

    let ids = Arc::new(StdRwLock::new(ids));
    let _updater = RwLockBTreeMapUpdater::start(ids.clone());
    let mut rng = rand::thread_rng();

    bencher.iter(|| {
        for _ in 0..READS_PER_BENCH_ITER {
            let id = rng.gen_range(1..(SIZE as Id));
            prevent_opt(ids.read().expect("Failed to get read lock").get(&id));
        }
    })
}

///////////////////////////////////////////////////////////////////////////////

struct StdRwLockHashUpdater {
    is_halt: Arc<AtomicBool>,
}

impl StdRwLockHashUpdater {
    fn start<H>(ids: Arc<StdRwLock<HashMap<Id, usize, BuildHasherDefault<H>>>>) -> Self
    where
        H: Hasher + Default + 'static,
    {
        let is_halt = Arc::new(AtomicBool::new(false));
        let is_halt_clone = is_halt.clone();

        thread::spawn(move || {
            let mut rng = rand::thread_rng();

            while !is_halt_clone.load(Ordering::Relaxed) {
                let id = rng.gen_range(LAST_FILLED_ID..(SIZE as Id));
                ids.write().expect("Failed to get write lock").insert(id, 0);
                thread::sleep(Duration::from_millis(UPDATER_PERIOD_MS));
            }
        });

        Self { is_halt }
    }
}

impl Drop for StdRwLockHashUpdater {
    fn drop(&mut self) {
        self.is_halt.store(true, Ordering::SeqCst);
    }
}

fn id_index_std_rwlock_hash<H: Hasher + Default + 'static>(bencher: &mut Bencher) {
    let hasher = BuildHasherDefault::<H>::default();
    let mut ids = HashMap::with_capacity_and_hasher(SIZE, hasher);

    for id in 0..LAST_FILLED_ID {
        ids.insert(id, 0);
    }

    let ids = Arc::new(StdRwLock::new(ids));
    let _updater = StdRwLockHashUpdater::start(ids.clone());
    let mut rng = rand::thread_rng();

    bencher.iter(|| {
        for _ in 0..READS_PER_BENCH_ITER {
            let id = rng.gen_range(1..(SIZE as Id));
            prevent_opt(ids.read().expect("Failed to get read lock").get(&id));
        }
    })
}

///////////////////////////////////////////////////////////////////////////////

struct ParkingLotRwLockHashUpdater {
    is_halt: Arc<AtomicBool>,
}

impl ParkingLotRwLockHashUpdater {
    fn start<H>(ids: Arc<ParkingLotRwLock<HashMap<Id, usize, BuildHasherDefault<H>>>>) -> Self
    where
        H: Hasher + Default + 'static,
    {
        let is_halt = Arc::new(AtomicBool::new(false));
        let is_halt_clone = is_halt.clone();

        thread::spawn(move || {
            let mut rng = rand::thread_rng();

            while !is_halt_clone.load(Ordering::Relaxed) {
                let id = rng.gen_range(LAST_FILLED_ID..(SIZE as Id));
                ids.write().insert(id, 0);
                thread::sleep(Duration::from_millis(UPDATER_PERIOD_MS));
            }
        });

        Self { is_halt }
    }
}

impl Drop for ParkingLotRwLockHashUpdater {
    fn drop(&mut self) {
        self.is_halt.store(true, Ordering::SeqCst);
    }
}

fn id_index_parking_lot_rwlock_hash<H: Hasher + Default + 'static>(bencher: &mut Bencher) {
    let hasher = BuildHasherDefault::<H>::default();
    let mut ids = HashMap::with_capacity_and_hasher(SIZE, hasher);

    for id in 0..LAST_FILLED_ID {
        ids.insert(id, 0);
    }

    let ids = Arc::new(ParkingLotRwLock::new(ids));
    let _updater = ParkingLotRwLockHashUpdater::start(ids.clone());
    let mut rng = rand::thread_rng();

    bencher.iter(|| {
        for _ in 0..READS_PER_BENCH_ITER {
            let id = rng.gen_range(1..(SIZE as Id));
            prevent_opt(ids.read().get(&id));
        }
    })
}

///////////////////////////////////////////////////////////////////////////////

struct LockFreeMapUpdater {
    is_halt: Arc<AtomicBool>,
}

impl LockFreeMapUpdater {
    fn start<H>(ids: Arc<LockFreeMap<Id, usize, BuildHasherDefault<H>>>) -> Self
    where
        H: Hasher + Default + 'static,
    {
        let is_halt = Arc::new(AtomicBool::new(false));
        let is_halt_clone = is_halt.clone();

        thread::spawn(move || {
            let mut rng = rand::thread_rng();

            while !is_halt_clone.load(Ordering::Relaxed) {
                let id = rng.gen_range(LAST_FILLED_ID..(SIZE as Id));
                ids.insert(id, 0);
                thread::sleep(Duration::from_millis(UPDATER_PERIOD_MS));
            }
        });

        Self { is_halt }
    }
}

impl Drop for LockFreeMapUpdater {
    fn drop(&mut self) {
        self.is_halt.store(true, Ordering::SeqCst);
    }
}

fn id_index_lock_free_map<H: Hasher + Default + 'static>(bencher: &mut Bencher) {
    let ids = LockFreeMap::with_hasher(BuildHasherDefault::<H>::default());

    for id in 0..LAST_FILLED_ID {
        ids.insert(id, 0);
    }

    let ids = Arc::new(ids);
    let _updater = LockFreeMapUpdater::start(ids.clone());
    let mut rng = rand::thread_rng();

    bencher.iter(|| {
        for _ in 0..READS_PER_BENCH_ITER {
            let id = rng.gen_range(1..(SIZE as Id));
            prevent_opt(ids.get(&id).map(|i| *i.val()));
        }
    })
}

///////////////////////////////////////////////////////////////////////////////

benchmark_group!(
    benches,
    id_index_rwlock_btree_map,
    id_index_std_rwlock_hash::<DefaultHasher>,
    id_index_std_rwlock_hash::<FxHasher>,
    id_index_std_rwlock_hash::<NoHashHasher<Id>>,
    id_index_parking_lot_rwlock_hash::<DefaultHasher>,
    id_index_parking_lot_rwlock_hash::<FxHasher>,
    id_index_parking_lot_rwlock_hash::<NoHashHasher<Id>>,
    id_index_lock_free_map::<DefaultHasher>,
    id_index_lock_free_map::<FxHasher>,
    id_index_lock_free_map::<NoHashHasher<Id>>,
);

benchmark_main!(benches);
