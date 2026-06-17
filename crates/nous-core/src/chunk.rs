//! Content-defined chunking (gear-hash / FastCDC-style).
//!
//! Splits a byte slice into variable-length chunks whose boundaries are chosen
//! by content, not offset. Inserting or removing bytes only re-chunks the
//! locally affected region, so unchanged regions keep their chunk ids and
//! deduplicate across versions.
//!
//! The boundary function is fully deterministic: the same input always yields
//! the same chunk boundaries (required for content addressing).

/// Minimum chunk size in bytes. No boundary is emitted before this.
pub const MIN_SIZE: usize = 2 * 1024;
/// Target average chunk size (controls the boundary mask).
pub const AVG_SIZE: usize = 8 * 1024;
/// Maximum chunk size in bytes. A boundary is forced here.
pub const MAX_SIZE: usize = 64 * 1024;

/// Number of low bits in the boundary mask, ~log2(AVG_SIZE).
const MASK_BITS: u32 = 13; // 2^13 = 8192 = AVG_SIZE
const MASK: u64 = (1u64 << MASK_BITS) - 1;

/// Per-byte gear table (256 deterministic 64-bit values via splitmix64).
const GEAR: [u64; 256] = gear_table();

const fn gear_table() -> [u64; 256] {
    let mut t = [0u64; 256];
    let mut x: u64 = 0x2545_F491_4F6C_DD1D;
    let mut i = 0;
    while i < 256 {
        // splitmix64
        x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = x;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        t[i] = z;
        i += 1;
    }
    t
}

/// Find the next chunk boundary in `data`, returning the chunk length.
///
/// Scans from `MIN_SIZE` up to `MAX_SIZE`, emitting a boundary at the first
/// position where the rolling gear hash matches `MASK`, or at `MAX_SIZE` /
/// end-of-data otherwise.
fn next_boundary(data: &[u8]) -> usize {
    let len = data.len();
    if len <= MIN_SIZE {
        return len;
    }
    let end = len.min(MAX_SIZE);
    let mut hash: u64 = 0;
    let mut i = MIN_SIZE;
    while i < end {
        hash = (hash << 1).wrapping_add(GEAR[data[i] as usize]);
        if hash & MASK == 0 {
            return i + 1;
        }
        i += 1;
    }
    end
}

/// Split `data` into content-defined chunks.
///
/// The returned slices are contiguous and in order; concatenating them
/// reproduces `data` exactly. An empty input yields no chunks.
pub fn chunks(data: &[u8]) -> Vec<&[u8]> {
    let mut out = Vec::new();
    let mut rest = data;
    while !rest.is_empty() {
        let n = next_boundary(rest);
        out.push(&rest[..n]);
        rest = &rest[n..];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pattern(len: usize) -> Vec<u8> {
        // A non-trivial, reproducible byte pattern.
        (0..len).map(|i| ((i * 31 + 7) ^ (i >> 3)) as u8).collect()
    }

    #[test]
    fn empty_input_no_chunks() {
        assert!(chunks(&[]).is_empty());
    }

    #[test]
    fn small_input_single_chunk() {
        let data = pattern(100);
        let c = chunks(&data);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0], &data[..]);
    }

    #[test]
    fn chunks_concatenate_to_input() {
        let data = pattern(500_000);
        let c = chunks(&data);
        let joined: Vec<u8> = c.concat();
        assert_eq!(joined, data);
    }

    #[test]
    fn chunking_is_deterministic() {
        let data = pattern(300_000);
        let a: Vec<usize> = chunks(&data).iter().map(|c| c.len()).collect();
        let b: Vec<usize> = chunks(&data).iter().map(|c| c.len()).collect();
        assert_eq!(a, b);
    }

    #[test]
    fn chunk_sizes_within_bounds() {
        let data = pattern(1_000_000);
        let c = chunks(&data);
        // every chunk is non-empty and <= MAX_SIZE
        for (idx, ch) in c.iter().enumerate() {
            assert!(!ch.is_empty());
            assert!(ch.len() <= MAX_SIZE, "chunk {idx} exceeds MAX_SIZE");
        }
        // produced more than one chunk for a large input
        assert!(c.len() > 1);
    }

    #[test]
    fn shared_prefix_shares_leading_chunks() {
        // Two inputs with a long identical prefix should share leading chunk
        // boundaries (content-defined: the prefix chunks are identical).
        let base = pattern(200_000);
        let mut longer = base.clone();
        longer.extend_from_slice(&pattern(50_000));

        let ca = chunks(&base);
        let cb = chunks(&longer);
        let shared = ca.len().min(cb.len());
        let mut identical = 0;
        for i in 0..shared {
            if ca[i] == cb[i] {
                identical += 1;
            } else {
                break;
            }
        }
        assert!(identical >= 1, "expected shared leading chunks, got {identical}");
    }
}
