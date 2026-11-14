//! A tiny deterministic generator (SplitMix64). The benchmark needs
//! reproducible schedules, not statistical quality or secrecy.

const GAMMA: u64 = 0x9E37_79B9_7F4A_7C15;

#[derive(Debug, Clone)]
pub struct SplitMix64(u64);

impl SplitMix64 {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(GAMMA);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A value in `0..bound`. `bound` must be nonzero. The slight modulo
    /// bias is irrelevant at the sizes used here.
    pub fn below(&mut self, bound: u64) -> u64 {
        self.next_u64() % bound
    }

    /// A value in `low..=high`.
    pub fn range(&mut self, low: u64, high: u64) -> u64 {
        low + self.below(high - low + 1)
    }

    /// Fisher-Yates shuffle. The slight modulo bias is irrelevant here.
    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = (self.next_u64() % (i as u64 + 1)) as usize;
            items.swap(i, j);
        }
    }
}

/// Derive an independent value from a seed and an index.
pub fn mix(seed: u64, index: u64) -> u64 {
    SplitMix64::new(seed ^ index.wrapping_mul(GAMMA)).next_u64()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_reference_vector() {
        // First outputs of SplitMix64 seeded with 0, from the reference C code.
        let mut rng = SplitMix64::new(0);
        assert_eq!(rng.next_u64(), 0xE220_A839_7B1D_CDAF);
        assert_eq!(rng.next_u64(), 0x6E78_9E6A_A1B9_65F4);
        assert_eq!(rng.next_u64(), 0x06C4_5D18_8009_454F);
    }

    #[test]
    fn same_seed_same_stream() {
        let mut a = SplitMix64::new(42);
        let mut b = SplitMix64::new(42);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn shuffle_is_a_seeded_permutation() {
        let original: Vec<u32> = (0..48).collect();
        let shuffled = |seed| {
            let mut items = original.clone();
            SplitMix64::new(seed).shuffle(&mut items);
            items
        };
        assert_eq!(shuffled(5), shuffled(5));
        assert_ne!(shuffled(5), shuffled(6));
        assert_ne!(shuffled(5), original);

        let mut sorted = shuffled(5);
        sorted.sort_unstable();
        assert_eq!(sorted, original);
    }

    #[test]
    fn mix_varies_with_index() {
        assert_ne!(mix(1, 0), mix(1, 1));
        assert_eq!(mix(1, 5), mix(1, 5));
    }
}
