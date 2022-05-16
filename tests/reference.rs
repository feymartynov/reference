use std::convert::Infallible;

use reference::{Enterable, Id, Identifiable, Referential, V1Reference};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Foo {
    id: Id,
    name: String,
}

impl Foo {
    fn new(id: Id) -> Self {
        Self {
            id,
            ..Default::default()
        }
    }
}

impl Identifiable for Foo {
    fn id(&self) -> Id {
        self.id
    }
}

#[test]
fn insert_and_get() {
    let reference = V1Reference::new(3);
    reference.insert(Foo::new(1)).expect("Failed to insert 1");
    assert!((*reference.get(0).expect("Failed to get 0")).is_none());
    let item1 = reference.get(1).expect("Failed to get 1");
    assert_eq!(*item1, Some(Foo::new(1)));
    assert!(reference.get(2).is_none());
    assert!(reference.get(3).is_none());
}

#[test]
fn iterate() {
    let reference = V1Reference::new(4);
    reference.insert(Foo::new(1)).expect("Failed to insert 1");
    reference.insert(Foo::new(4)).expect("Failed to insert 4");
    reference.get_or_reserve(3).expect("Failed to reserve 3");

    let ids = reference
        .iter()
        .map(|i| i.as_ref().map(|foo| foo.id))
        .collect::<Vec<_>>();

    assert_eq!(ids, [None, Some(1), Some(4), None]);
}

#[test]
fn replace() {
    let reference = V1Reference::new(2);
    let entry = reference.get_or_reserve(1).expect("Failed to reserve");
    assert!((*entry).is_none());

    entry.replace(Foo::new(1));
    assert_eq!(*entry, Some(Foo::new(1)));

    let mut other = Foo::new(1);
    other.name = "other".to_string();
    entry.replace(other.clone());
    assert_eq!(*entry, Some(other.clone()));
    assert_eq!(*(reference.get(1).expect("Failed to get 1")), Some(other));
}

#[test]
fn update() {
    let reference = V1Reference::new(2);
    let entry = reference.insert(Foo::new(1)).expect("Failed to insert");

    entry
        .update(|maybe_foo| match maybe_foo {
            None => panic!("Entry is empty"),
            Some(ref mut foo) => {
                foo.name.push_str("foo");
                Ok(()) as Result<(), Infallible>
            }
        })
        .expect("Failed to update");

    assert_eq!(entry.as_ref().expect("Entry is empty").name, "foo");
    let entry = reference.get(1).expect("Failed to get");
    assert_eq!(entry.as_ref().expect("Entry is empty").name, "foo");
}
