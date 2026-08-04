// Application demonstrations remain frozen until the graph model and exact
// canonization API are specified. These allowances document that boundary
// without weakening the new structural-signature module.
#![allow(
    clippy::clone_on_copy,
    clippy::manual_flatten,
    clippy::manual_is_multiple_of,
    clippy::needless_range_loop,
    clippy::new_without_default,
    clippy::unnecessary_sort_by
)]

pub mod chemistry;
pub mod logic;
pub mod network;
