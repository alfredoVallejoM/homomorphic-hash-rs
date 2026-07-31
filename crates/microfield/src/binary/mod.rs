//! Internal algorithms for binary polynomial fields.

mod extension;
mod implementation;
mod invert;
mod portable;
mod reduction;
mod reference;
mod representation;
mod square;

pub(crate) use extension::{frobenius_binary, trace_binary};
pub(crate) use implementation::{
    BinaryFieldImpl, Polynomial128, Polynomial256, add_limbs, decode_limbs, encode_limbs,
    limbs_are_zero,
};
pub(crate) use invert::invert_binary;
