//! Exact tracked snapshots, deliberately distinct from compact `MFSG` states.

use super::SignatureError;

const MAGIC: [u8; 4] = *b"MFTS";
const SCHEMA: u16 = 1;
const HEADER_BYTES: usize = 24;
pub(crate) const SEQUENCE_KIND: u8 = 1;
pub(crate) const MULTISET_KIND: u8 = 2;

pub(crate) struct DecodedTrackedSnapshot<'a> {
    pub(crate) compact: &'a [u8],
    pub(crate) entries: Vec<(Vec<u8>, u64)>,
}

/// Defensive limits applied before restoring source-retaining snapshots.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TrackedSnapshotLimits {
    maximum_items: u64,
    maximum_distinct_items: usize,
    maximum_item_bytes: usize,
    maximum_total_bytes: usize,
}

impl TrackedSnapshotLimits {
    /// Creates explicit restoration limits.
    #[must_use]
    pub const fn new(
        maximum_items: u64,
        maximum_distinct_items: usize,
        maximum_item_bytes: usize,
        maximum_total_bytes: usize,
    ) -> Self {
        Self {
            maximum_items,
            maximum_distinct_items,
            maximum_item_bytes,
            maximum_total_bytes,
        }
    }

    /// Maximum logical cardinality after expanding multiplicities.
    #[must_use]
    pub const fn maximum_items(self) -> u64 {
        self.maximum_items
    }

    /// Maximum number of framed entries.
    #[must_use]
    pub const fn maximum_distinct_items(self) -> usize {
        self.maximum_distinct_items
    }

    /// Maximum bytes in one source item.
    #[must_use]
    pub const fn maximum_item_bytes(self) -> usize {
        self.maximum_item_bytes
    }

    /// Maximum complete snapshot size.
    #[must_use]
    pub const fn maximum_total_bytes(self) -> usize {
        self.maximum_total_bytes
    }
}

impl Default for TrackedSnapshotLimits {
    fn default() -> Self {
        Self::new(1_000_000, 1_000_000, 16 * 1024 * 1024, 64 * 1024 * 1024)
    }
}

pub(crate) fn encode_snapshot<'a, I>(
    kind: u8,
    compact: &[u8],
    entries: I,
    limits: TrackedSnapshotLimits,
) -> Result<Vec<u8>, SignatureError>
where
    I: IntoIterator<Item = (&'a [u8], u64)>,
{
    let entries = entries.into_iter();
    let (lower, upper) = entries.size_hint();
    let entry_count = upper.filter(|upper| *upper == lower).unwrap_or(lower);
    if entry_count > limits.maximum_distinct_items {
        return Err(SignatureError::SnapshotLimitExceeded("distinct items"));
    }
    let compact_len = u64::try_from(compact.len())
        .map_err(|_| SignatureError::SnapshotLimitExceeded("compact state"))?;
    let entry_count_u64 = u64::try_from(entry_count)
        .map_err(|_| SignatureError::SnapshotLimitExceeded("distinct items"))?;
    let initial_len = HEADER_BYTES
        .checked_add(compact.len())
        .ok_or(SignatureError::SnapshotLimitExceeded("total bytes"))?;
    if initial_len > limits.maximum_total_bytes {
        return Err(SignatureError::SnapshotLimitExceeded("total bytes"));
    }
    let mut output = Vec::new();
    output
        .try_reserve_exact(initial_len)
        .map_err(|_| SignatureError::AllocationFailed)?;
    output.extend_from_slice(&MAGIC);
    output.extend_from_slice(&SCHEMA.to_le_bytes());
    output.push(kind);
    output.push(0);
    output.extend_from_slice(&compact_len.to_le_bytes());
    output.extend_from_slice(&entry_count_u64.to_le_bytes());
    output.extend_from_slice(compact);

    let mut logical_items = 0_u64;
    let mut actual_entries = 0_usize;
    for (item, multiplicity) in entries {
        if item.len() > limits.maximum_item_bytes {
            return Err(SignatureError::SnapshotLimitExceeded("item bytes"));
        }
        if multiplicity == 0 || (kind == SEQUENCE_KIND && multiplicity != 1) {
            return Err(SignatureError::InvalidWireFormat(
                "tracked snapshot multiplicity",
            ));
        }
        logical_items = logical_items
            .checked_add(multiplicity)
            .ok_or(SignatureError::CounterOverflow)?;
        if logical_items > limits.maximum_items {
            return Err(SignatureError::SnapshotLimitExceeded("logical items"));
        }
        actual_entries = actual_entries
            .checked_add(1)
            .ok_or(SignatureError::SnapshotLimitExceeded("distinct items"))?;
        if actual_entries > limits.maximum_distinct_items {
            return Err(SignatureError::SnapshotLimitExceeded("distinct items"));
        }
        let item_len = u64::try_from(item.len())
            .map_err(|_| SignatureError::SnapshotLimitExceeded("item bytes"))?;
        let additional = 8_usize
            .checked_add(item.len())
            .and_then(|size| size.checked_add(usize::from(kind == MULTISET_KIND) * 8))
            .ok_or(SignatureError::SnapshotLimitExceeded("total bytes"))?;
        let next_len = output
            .len()
            .checked_add(additional)
            .ok_or(SignatureError::SnapshotLimitExceeded("total bytes"))?;
        if next_len > limits.maximum_total_bytes {
            return Err(SignatureError::SnapshotLimitExceeded("total bytes"));
        }
        output
            .try_reserve_exact(additional)
            .map_err(|_| SignatureError::AllocationFailed)?;
        output.extend_from_slice(&item_len.to_le_bytes());
        output.extend_from_slice(item);
        if kind == MULTISET_KIND {
            output.extend_from_slice(&multiplicity.to_le_bytes());
        }
    }
    if actual_entries != entry_count {
        return Err(SignatureError::InvalidWireFormat(
            "tracked snapshot entry count",
        ));
    }
    Ok(output)
}

pub(crate) fn decode_snapshot(
    bytes: &[u8],
    expected_kind: u8,
    limits: TrackedSnapshotLimits,
) -> Result<DecodedTrackedSnapshot<'_>, SignatureError> {
    if bytes.len() > limits.maximum_total_bytes {
        return Err(SignatureError::SnapshotLimitExceeded("total bytes"));
    }
    if bytes.len() < HEADER_BYTES {
        return Err(SignatureError::InvalidWireFormat("tracked snapshot header"));
    }
    if bytes[..4] != MAGIC
        || u16::from_le_bytes([bytes[4], bytes[5]]) != SCHEMA
        || bytes[6] != expected_kind
        || bytes[7] != 0
    {
        return Err(SignatureError::InvalidWireFormat(
            "tracked snapshot identity",
        ));
    }
    let compact_len = read_usize(&bytes[8..16], "tracked compact length")?;
    let entry_count = read_usize(&bytes[16..24], "tracked entry count")?;
    if entry_count > limits.maximum_distinct_items {
        return Err(SignatureError::SnapshotLimitExceeded("distinct items"));
    }
    let compact_end = HEADER_BYTES
        .checked_add(compact_len)
        .filter(|end| *end <= bytes.len())
        .ok_or(SignatureError::InvalidWireFormat(
            "tracked compact boundary",
        ))?;
    let compact = &bytes[HEADER_BYTES..compact_end];
    let mut cursor = compact_end;
    let mut entries = Vec::new();
    entries
        .try_reserve_exact(entry_count)
        .map_err(|_| SignatureError::AllocationFailed)?;
    let mut logical_items = 0_u64;
    for _ in 0..entry_count {
        let item_len_end = cursor
            .checked_add(8)
            .filter(|end| *end <= bytes.len())
            .ok_or(SignatureError::InvalidWireFormat("tracked item length"))?;
        let item_len = read_usize(&bytes[cursor..item_len_end], "tracked item length")?;
        if item_len > limits.maximum_item_bytes {
            return Err(SignatureError::SnapshotLimitExceeded("item bytes"));
        }
        cursor = item_len_end;
        let item_end = cursor
            .checked_add(item_len)
            .filter(|end| *end <= bytes.len())
            .ok_or(SignatureError::InvalidWireFormat("tracked item boundary"))?;
        let mut item = Vec::new();
        item.try_reserve_exact(item_len)
            .map_err(|_| SignatureError::AllocationFailed)?;
        item.extend_from_slice(&bytes[cursor..item_end]);
        cursor = item_end;
        let multiplicity = if expected_kind == MULTISET_KIND {
            let multiplicity_end = cursor
                .checked_add(8)
                .filter(|end| *end <= bytes.len())
                .ok_or(SignatureError::InvalidWireFormat("tracked multiplicity"))?;
            let value = read_u64(&bytes[cursor..multiplicity_end]);
            cursor = multiplicity_end;
            value
        } else {
            1
        };
        if multiplicity == 0 {
            return Err(SignatureError::InvalidWireFormat(
                "zero tracked multiplicity",
            ));
        }
        logical_items = logical_items
            .checked_add(multiplicity)
            .ok_or(SignatureError::CounterOverflow)?;
        if logical_items > limits.maximum_items {
            return Err(SignatureError::SnapshotLimitExceeded("logical items"));
        }
        entries.push((item, multiplicity));
    }
    if cursor != bytes.len() {
        return Err(SignatureError::InvalidWireFormat(
            "tracked snapshot trailing bytes",
        ));
    }
    Ok(DecodedTrackedSnapshot { compact, entries })
}

fn read_usize(bytes: &[u8], label: &'static str) -> Result<usize, SignatureError> {
    usize::try_from(read_u64(bytes)).map_err(|_| SignatureError::InvalidWireFormat(label))
}

fn read_u64(bytes: &[u8]) -> u64 {
    u64::from_le_bytes(bytes.try_into().expect("validated eight-byte range"))
}
