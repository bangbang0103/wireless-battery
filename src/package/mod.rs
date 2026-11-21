//! Declarative package parsing and validation.

pub mod parser;
pub mod schema;

pub use parser::{load_package, parse_package, validate_package};
pub use schema::DevicePackage;
