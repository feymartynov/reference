use reference::{Entry, Id, Identifiable, Reference};

const PRODUCTS_COUNT: usize = 10;
const SUBJECTS_COUNT: usize = 3;

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
struct Ctx {
    products: Reference<Product>,
    subjects: Reference<Subject>,
}

#[derive(Debug)]
struct Product {
    id: Id<Self>,
    name: String,
    subject: Entry<Subject>,
}

impl<'a> Identifiable for Product {
    fn id(&self) -> Id<Self> {
        self.id
    }
}

#[derive(Debug)]
struct Subject {
    id: Id<Self>,
    name: String,
}

impl Identifiable for Subject {
    fn id(&self) -> Id<Self> {
        self.id
    }
}

///////////////////////////////////////////////////////////////////////////////

fn main() {
    let ctx = Ctx {
        products: Reference::new(PRODUCTS_COUNT),
        subjects: Reference::new(SUBJECTS_COUNT),
    };

    for id in 1..(PRODUCTS_COUNT as i32) {
        let subject = ctx
            .subjects
            .get_or_reserve(Id::new(id % 2 + 1))
            .expect("Failed to get or reserve subject");

        ctx.products
            .insert(Product {
                id: Id::new(id),
                name: format!("Product {id}"),
                subject,
            })
            .expect("Failed to insert product");
    }

    for id in 1..(SUBJECTS_COUNT as i32) {
        ctx.subjects
            .insert(Subject {
                id: Id::new(id),
                name: format!("Subject {id}"),
            })
            .expect("Failed to insert subject");
    }

    for product in ctx.products.iter().filter_map(|e| e.load()) {
        let subject = product.subject.load().expect("Missing subject");

        println!(
            "id: {}, name: {}, subject id: {}, subject_name: {}",
            product.id, product.name, subject.id, subject.name,
        );
    }
}
