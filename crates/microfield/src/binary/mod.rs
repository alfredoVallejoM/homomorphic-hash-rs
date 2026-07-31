//! Internal algorithms for binary polynomial fields.

mod implementation;
mod invert;
mod portable;
mod reduction;
mod reference;
mod representation;
mod square;

pub(crate) use invert::invert_256;
pub(crate) use portable::wide_product_256;
pub(crate) use reduction::{mul_by_x_256, reduce_256};
pub(crate) use reference::reduce_polynomial_bytes_256;
pub(crate) use square::square_256;
