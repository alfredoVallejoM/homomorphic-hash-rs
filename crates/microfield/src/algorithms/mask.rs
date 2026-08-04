//! Compact invertibility masks with owned and caller-provided storage.

use core::fmt;

/// Failure while sizing or constructing a compact bit mask.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum BitMaskError {
    /// Computing the number of words overflowed `usize`.
    SizeOverflow,
    /// Caller-provided storage is too small.
    InsufficientStorage {
        /// Required number of `u64` words.
        required: usize,
        /// Supplied number of `u64` words.
        provided: usize,
    },
    /// Owned storage could not be reserved.
    AllocationFailed,
}

impl fmt::Display for BitMaskError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SizeOverflow => formatter.write_str("bit-mask size overflow"),
            Self::InsufficientStorage { required, provided } => write!(
                formatter,
                "insufficient bit-mask storage: required {required} words, provided {provided}"
            ),
            Self::AllocationFailed => formatter.write_str("bit-mask allocation failed"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for BitMaskError {}

/// Returns the number of `u64` words required for `len` logical bits.
///
/// # Errors
///
/// Returns [`BitMaskError::SizeOverflow`] if rounding the bit count overflows.
pub const fn required_mask_words(len: usize) -> Result<usize, BitMaskError> {
    match len.checked_add(63) {
        Some(rounded) => Ok(rounded / 64),
        None => Err(BitMaskError::SizeOverflow),
    }
}

/// Exclusive borrowed view over a compact logical bit mask.
pub struct BitMaskViewMut<'a> {
    words: &'a mut [u64],
    len: usize,
}

impl<'a> BitMaskViewMut<'a> {
    /// Constructs a view over the minimum required prefix of `words`.
    ///
    /// Construction does not modify storage. Derived algorithms initialize
    /// every logical and padding bit after all other arguments validate.
    ///
    /// # Errors
    ///
    /// Returns a sizing error when `words` cannot contain `len` bits.
    pub fn new(words: &'a mut [u64], len: usize) -> Result<Self, BitMaskError> {
        let required = required_mask_words(len)?;
        if words.len() < required {
            return Err(BitMaskError::InsufficientStorage {
                required,
                provided: words.len(),
            });
        }
        Ok(Self {
            words: &mut words[..required],
            len,
        })
    }

    /// Returns the logical number of bits.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Reports whether the logical mask is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns one logical bit, or `None` when `index` is out of bounds.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<bool> {
        (index < self.len).then(|| self.words[index / 64] & (1_u64 << (index % 64)) != 0)
    }

    /// Returns one logical bit.
    ///
    /// # Panics
    ///
    /// Panics when `index` is outside the logical mask.
    #[must_use]
    pub fn is_set(&self, index: usize) -> bool {
        self.get(index).expect("bit-mask index out of bounds")
    }

    /// Counts set logical bits.
    #[must_use]
    pub fn count_ones(&self) -> usize {
        count_logical_ones(self.words, self.len)
    }

    /// Clears all logical bits and all padding bits in the final word.
    pub fn clear(&mut self) {
        self.words.fill(0);
    }

    pub(crate) fn set(&mut self, index: usize) {
        debug_assert!(index < self.len);
        self.words[index / 64] |= 1_u64 << (index % 64);
    }
}

/// Owned compact mask, available when allocation support is enabled.
#[cfg(feature = "alloc")]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BitMask {
    words: alloc::vec::Vec<u64>,
    len: usize,
}

#[cfg(feature = "alloc")]
impl BitMask {
    /// Allocates a zero-initialized mask.
    ///
    /// # Errors
    ///
    /// Returns a sizing or allocation error without constructing a partial
    /// mask.
    pub fn new(len: usize) -> Result<Self, BitMaskError> {
        let required = required_mask_words(len)?;
        let mut words = alloc::vec::Vec::new();
        words
            .try_reserve_exact(required)
            .map_err(|_| BitMaskError::AllocationFailed)?;
        words.resize(required, 0);
        Ok(Self { words, len })
    }

    /// Returns the logical number of bits.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Reports whether the mask is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns one logical bit, or `None` when out of bounds.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<bool> {
        (index < self.len).then(|| self.words[index / 64] & (1_u64 << (index % 64)) != 0)
    }

    /// Returns one logical bit.
    ///
    /// # Panics
    ///
    /// Panics when `index` is outside the logical mask.
    #[must_use]
    pub fn is_set(&self, index: usize) -> bool {
        self.get(index).expect("bit-mask index out of bounds")
    }

    /// Counts set logical bits.
    #[must_use]
    pub fn count_ones(&self) -> usize {
        count_logical_ones(&self.words, self.len)
    }

    /// Clears every logical and padding bit.
    pub fn clear(&mut self) {
        self.words.fill(0);
    }

    /// Borrows the mask for an allocation-free algorithm call.
    #[must_use]
    pub fn as_view_mut(&mut self) -> BitMaskViewMut<'_> {
        BitMaskViewMut {
            words: &mut self.words,
            len: self.len,
        }
    }

    /// Returns the packed words for diagnostics or a custom wire adapter.
    #[must_use]
    pub fn words(&self) -> &[u64] {
        &self.words
    }
}

fn count_logical_ones(words: &[u64], len: usize) -> usize {
    let full_words = len / 64;
    let mut total = words[..full_words]
        .iter()
        .map(|word| word.count_ones() as usize)
        .sum();
    let tail_bits = len % 64;
    if tail_bits != 0 {
        let tail_mask = (1_u64 << tail_bits) - 1;
        total += (words[full_words] & tail_mask).count_ones() as usize;
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizing_checks_overflow_and_exact_boundaries() {
        assert_eq!(required_mask_words(0), Ok(0));
        assert_eq!(required_mask_words(1), Ok(1));
        assert_eq!(required_mask_words(64), Ok(1));
        assert_eq!(required_mask_words(65), Ok(2));
        assert_eq!(
            required_mask_words(usize::MAX),
            Err(BitMaskError::SizeOverflow)
        );
    }

    #[test]
    fn borrowed_view_uses_only_required_words_and_checks_bounds() {
        let mut storage = [u64::MAX; 3];
        let mut view = BitMaskViewMut::new(&mut storage, 65).expect("two words suffice");
        view.clear();
        view.set(0);
        view.set(64);
        assert_eq!(view.get(0), Some(true));
        assert_eq!(view.get(63), Some(false));
        assert_eq!(view.get(64), Some(true));
        assert_eq!(view.get(65), None);
        assert_eq!(view.count_ones(), 2);
        assert_eq!(storage[2], u64::MAX);
    }

    #[test]
    fn count_ignores_padding_owned_by_the_caller() {
        let mut storage = [u64::MAX; 1];
        let view = BitMaskViewMut::new(&mut storage, 1).expect("one word suffices");
        assert_eq!(view.count_ones(), 1);
    }

    #[test]
    fn insufficient_storage_is_reported_without_writing() {
        let mut storage = [0x55_u64; 1];
        assert!(matches!(
            BitMaskViewMut::new(&mut storage, 65),
            Err(BitMaskError::InsufficientStorage {
                required: 2,
                provided: 1
            })
        ));
        assert_eq!(storage, [0x55]);
    }
}
