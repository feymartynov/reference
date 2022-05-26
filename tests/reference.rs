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

    let item0 = reference.get(0.into()).expect("Failed to get 0").load();
    assert!(item0.is_none());

    let item1 = reference.get(1.into()).expect("Failed to get 1").load();
    let entity = item1.expect("Entry 1 is empty");
    assert_eq!(entity.id, 1.into());

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
        .map(|maybe_entity| maybe_entity.load().map(|entity| entity.id))
        .collect::<Vec<_>>();

    assert_eq!(ids, [None, Some(1.into()), Some(4.into()), None]);
}

#[test]
fn set_and_replace() {
    let reference = Reference::new(2);

    let entry1 = reference
        .get_or_reserve(1.into())
        .expect("Failed to reserve");

    assert!(entry1.load().is_none());

    reference
        .insert(Foo::new(1.into()))
        .expect("Failed to set entity");

    let entry2 = reference.get(1.into()).expect("Entry not found");

    for entry in [&entry2, &entry1] {
        let entity = entry.load().expect("Entry is empty");
        assert_eq!(entity.id, 1.into());
    }

    let mut other = Foo::new(1.into());
    other.name = "other".to_string();
    reference.insert(other).expect("Failed to replace entity");

    let entry3 = reference.get(1.into()).expect("Entry not found");

    for entry in [entry3, entry2, entry1] {
        let entity = entry.load().expect("Entry is empty");
        assert_eq!(entity.name, "other");
    }
}
