//! Typed workspaces for derived algorithms.

use core::fmt;

use crate::Field;

/// Failure while constructing an owned typed workspace.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum WorkspaceError {
    /// The requested allocation could not be reserved.
    AllocationFailed,
}

impl fmt::Display for WorkspaceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AllocationFailed => formatter.write_str("workspace allocation failed"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for WorkspaceError {}

/// Caller-provided field-element storage used by batch inversion.
///
/// The workspace is fully typed and therefore introduces no raw-byte casts,
/// alignment claims or additional `unsafe` boundary.
pub struct BatchInvertWorkspace<'a, F: Field> {
    prefixes: &'a mut [F],
}

impl<'a, F: Field> BatchInvertWorkspace<'a, F> {
    /// Borrows reusable prefix-product storage.
    #[must_use]
    pub fn new(prefixes: &'a mut [F]) -> Self {
        Self { prefixes }
    }

    /// Returns the available number of field-element slots.
    #[must_use]
    pub const fn capacity(&self) -> usize {
        self.prefixes.len()
    }

    pub(crate) fn prefix_storage(&mut self, len: usize) -> &mut [F] {
        debug_assert!(self.prefixes.len() >= len);
        &mut self.prefixes[..len]
    }
}

/// Owned reusable batch-inversion workspace.
#[cfg(feature = "alloc")]
#[derive(Clone, Debug)]
pub struct OwnedBatchInvertWorkspace<F: Field> {
    prefixes: alloc::vec::Vec<F>,
}

#[cfg(feature = "alloc")]
impl<F: Field> OwnedBatchInvertWorkspace<F> {
    /// Allocates `len` initialized field-element slots.
    ///
    /// # Errors
    ///
    /// Returns [`WorkspaceError::AllocationFailed`] when capacity cannot be
    /// reserved.
    pub fn new(len: usize) -> Result<Self, WorkspaceError> {
        let mut prefixes = alloc::vec::Vec::new();
        prefixes
            .try_reserve_exact(len)
            .map_err(|_| WorkspaceError::AllocationFailed)?;
        prefixes.resize(len, F::ZERO);
        Ok(Self { prefixes })
    }

    /// Returns the available number of slots.
    #[must_use]
    pub const fn capacity(&self) -> usize {
        self.prefixes.len()
    }

    /// Borrows the owned allocation for one algorithm call.
    #[must_use]
    pub fn as_workspace(&mut self) -> BatchInvertWorkspace<'_, F> {
        BatchInvertWorkspace::new(&mut self.prefixes)
    }
}
