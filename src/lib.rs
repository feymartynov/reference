mod array;
mod error;
pub mod reference;
pub mod v1;

pub use self::error::Error;
pub use self::reference::{Enterable, Id, Identifiable, Referential};
pub use self::v1::Reference as V1Reference;
