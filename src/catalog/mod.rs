//! Catalogo persistente: identità, copertura dei campi e query sullo stesso insieme.
mod query;
mod store;
pub mod sync;

pub use query::{CatalogPage, CatalogQuery, MovementMatch, ReleaseScope};
pub use store::{Catalog, Coverage, ExportIdentity, MovementFact};
