// The fixture file the library's own unit tests use, shared rather than
// exported from the crate.
#[allow(dead_code)]
#[path = "../src/utility/mock_constants.rs"]
mod mock_constants;

mod bulk;
mod common;
mod generic_endpoints;
mod validation;
mod null_fields;
mod message_only;
mod html_page;
mod documented_examples;
