#[macro_use]
extern crate lazy_static;

use reference::v1::{Entry, Reference};
use reference::{Id, Identifiable, Referential};

const PRODUCTS_COUNT: usize = 10;
const SUBJECTS_COUNT: usize = 3;

lazy_static! {
    static ref CTX: Ctx = Ctx {
        products: Reference::new(PRODUCTS_COUNT),
        subjects: Reference::new(SUBJECTS_COUNT),
    };
}

///////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
struct Ctx {
    products: Reference<Product>,
    subjects: Reference<Subject>,
}

#[derive(Debug)]
struct Product {
    id: Id,
    name: String,
    subject: Entry<'static, Subject>,
}

impl Identifiable for Product {
    fn id(&self) -> Id {
        self.id
    }
}

#[derive(Debug)]
struct Subject {
    id: Id,
    name: String,
}

impl Identifiable for Subject {
    fn id(&self) -> Id {
        self.id
    }
}

///////////////////////////////////////////////////////////////////////////////

fn main() {
    for id in 1..(PRODUCTS_COUNT as i32) {
        let subject = CTX
            .subjects
            .get_or_reserve(id % 2 + 1)
            .expect("Failed to get or reserve subject");

        CTX.products
            .insert(Product {
                id,
                name: format!("Product {id}"),
                subject,
            })
            .expect("Failed to insert product");
    }

    for id in 1..(SUBJECTS_COUNT as i32) {
        CTX.subjects
            .insert(Subject {
                id,
                name: format!("Subject {id}"),
            })
            .expect("Failed to insert subject");
    }

    for entry in CTX.products.iter() {
        if let Some(ref product) = *entry {
            let subject = (*product.subject).as_ref().expect("Missing subject");

            println!(
                "id: {}, name: {}, subject id: {}, subject_name: {}",
                product.id, product.name, subject.id, subject.name,
            );
        }
    }
}
