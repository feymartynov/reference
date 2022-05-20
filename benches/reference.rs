#[macro_use]
extern crate bencher;

use std::convert::Infallible;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

use bencher::Bencher;
use rand::prelude::*;
use reference::{Id, Identifiable, Reference};

const REFERENCE_SIZE: usize = 1_000_000;

///////////////////////////////////////////////////////////////////////////////

#[derive(Default)]
struct Foo {
    id: Id<Self>,
    name: String,
}

impl Foo {
    fn new(id: Id<Self>) -> Self {
        Self {
            id,
            ..Default::default()
        }
    }
}

impl Identifiable for Foo {
    fn id(&self) -> Id<Self> {
        self.id
    }
}

///////////////////////////////////////////////////////////////////////////////

struct Updater {
    is_halt: Arc<AtomicBool>,
}

impl Updater {
    fn start(reference: Arc<Reference<Foo>>) -> Self {
        let is_halt = Arc::new(AtomicBool::new(false));
        let is_halt_clone = is_halt.clone();

        thread::spawn(move || {
            let mut rng = rand::thread_rng();

            while !is_halt_clone.load(Ordering::Relaxed) {
                let id = rng.gen_range(1..(REFERENCE_SIZE as i32)).into();

                if let Some(mut entry) = reference.get(id) {
                    let _ = entry.update(|maybe_entity| {
                        if let Some(ref mut entity) = maybe_entity {
                            entity.name = format!("{}", rand::random::<i32>());
                        }

                        Ok(()) as Result<(), Infallible>
                    });
                }
            }
        });

        Self { is_halt }
    }
}

impl Drop for Updater {
    fn drop(&mut self) {
        self.is_halt.store(true, Ordering::SeqCst);
    }
}

///////////////////////////////////////////////////////////////////////////////

fn reference(bencher: &mut Bencher) {
    let reference = Arc::new(Reference::new(REFERENCE_SIZE));

    for id in 1..(REFERENCE_SIZE as i32) {
        reference
            .insert(Foo::new(id.into()))
            .expect("Failed to insert");
    }

    let _updater = Updater::start(reference.clone());

    bencher.iter(|| {
        for id in 1..(REFERENCE_SIZE as i32) {
            reference.get(id.into());
        }
    })
}

benchmark_group!(benches, reference);
benchmark_main!(benches);
