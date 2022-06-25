pub extern crate pax_macro;
pub use pax_macro::*;

pub use pax_runtime_api as api;

pub use pax_runtime_api::log;
pub use pax_runtime_api::Property;

pub mod internal {
    pub use pax_compiler_api::message as message;
    pub use pax_compiler_api::PropertyManifestable as PropertyManifestable;
}