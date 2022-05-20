use std::convert::Infallible;

use reference::{Id, Identifiable, Reference};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
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

#[test]
fn insert_and_get() {
    let reference = Reference::new(3);
    let item = Foo::new(1.into());
    reference.insert(item).expect("Failed to insert 1");
    assert!((*reference.get(0.into()).expect("Failed to get 0")).is_none());
    let item1 = reference.get(1.into()).expect("Failed to get 1");
    assert_eq!(*item1, Some(Foo::new(1.into())));
    assert!(reference.get(2.into()).is_none());
    assert!(reference.get(3.into()).is_none());
}

#[test]
fn iterate() {
    let reference = Reference::new(4);
    reference
        .insert(Foo::new(1.into()))
        .expect("Failed to insert 1");
    reference
        .insert(Foo::new(4.into()))
        .expect("Failed to insert 4");
    reference
        .get_or_reserve(3.into())
        .expect("Failed to reserve 3");

    let ids = reference
        .iter()
        .map(|maybe_entity| maybe_entity.as_ref().map(|entity| entity.id))
        .collect::<Vec<_>>();

    assert_eq!(ids, [None, Some(1.into()), Some(4.into()), None]);
}

#[test]
fn replace() {
    let reference = Reference::new(2);

    let mut entry = reference
        .get_or_reserve(1.into())
        .expect("Failed to reserve");

    assert!((*entry).is_none());

    entry.replace(Foo::new(1.into()));
    assert_eq!(*entry, Some(Foo::new(1.into())));

    let mut other = Foo::new(1.into());
    other.name = "other".to_string();
    entry.replace(other.clone());
    assert_eq!(*entry, Some(other.clone()));

    assert_eq!(
        *(reference.get(1.into()).expect("Failed to get 1")),
        Some(other)
    );
}

#[test]
fn update() {
    let reference = Reference::new(2);

    let mut entry = reference
        .insert(Foo::new(1.into()))
        .expect("Failed to insert");

    entry
        .update(|maybe_foo| match maybe_foo {
            None => panic!("Entry is empty"),
            Some(ref mut entity) => {
                entity.name.push_str("foo");
                Ok(()) as Result<(), Infallible>
            }
        })
        .expect("Failed to update");

    assert_eq!(entry.as_ref().expect("Entry is empty").name, "foo");
    let entry = reference.get(1.into()).expect("Failed to get");
    assert_eq!(entry.as_ref().expect("Entry is empty").name, "foo");
}
