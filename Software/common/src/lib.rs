use core::{mem::size_of, slice};

#[repr(C, align(4))]
pub struct Key<'k>(&'k [u8]);

/// The symmetric key used for both encryption and decryption
pub const KEY: Key = Key(include_bytes!("../../private/key.bin"));

/// The number of bytes in a normal subkey
pub const SUB_KEY_BYTES: usize = 32;

/// The number of 32 bit elements in a normal subkey
pub const SUB_KEY_ELEMENTS: usize = SUB_KEY_BYTES / size_of::<u32>();

/// The number of bytes used to store the index at the beginning of the packet.
/// The high bit of the index is always 0. 1 is reserved for future protocol expansion
pub const INDEX_BYTES: usize = 2;

impl<'k> Key<'k> {
    /// Creates a new key.
    ///
    /// # Panics
    ///
    /// If `key.len()` is not a multiple of four
    pub fn new(key: &'k [u8]) -> Self {
        assert_eq!(key.len() % size_of::<u32>(), 0);
        // SAFETY: Just ran length check
        unsafe { Self::new_unchecked(key) }
    }

    /// Creates a new key.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `key.len()` is a multiple of four
    pub unsafe fn new_unchecked(key: &'k [u8]) -> Self {
        Self(key)
    }

    /// Returns a slice len `key_len` of `KEY` based on word offset module the key length
    pub fn subkey(&self, word_offset: usize, key_len: usize) -> &[u32] {
        let ptr: *const u32 = KEY.0.as_ptr() as *const u32;
        let len = self.0.len() / size_of::<u32>();
        // SAFETY:
        // 1. `KEY` is aligned to a 4 byte boundaries, so the resulting slice is aligned
        // 2. Dividing by the size of a u32 rounds down, so every element of the slice is safe to read
        // 3. The lifetime of `word_slice` is 'k because that is the lifetime of this key
        let word_slice: &'k [u32] = unsafe { slice::from_raw_parts(ptr, len) };
        println!("{:?}", word_slice);

        // We need to find `key_len` contiguous elements, so the maximum index (exclusive) is `key_len`
        // less than the total length of the key
        let max_index = word_slice.len() - key_len;

        let offset = word_offset % max_index;
        println!("{}", offset);
        println!(
            "Word {:x} and bytes {:?}",
            word_slice[offset],
            &self.0[offset * 4..offset * 4 + 4]
        );

        /*
        // SAFETY:
        // 1. Offset is in range by the calculation of `max_index` above
        // 2. At least `key_len` elements are readable by the `max_index` calculation
        // 3. The result is aligned because `word_slice` is aligned
        // 4. The lifetime of the result is 'k because `word_slice` is 'k
        unsafe { slice::from_raw_parts(word_slice.as_ptr().add(offset), key_len) }
        */
        &word_slice[offset..offset + key_len]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_len() {
        // 53280 is a special number.
        // It is equal to `2^15*(1+5/8)+32`
        // We want this so that when the optimizer looks at the calculation of `max_index` in `subkey`,
        // it sees that its value is `2^15*(1+5/8)` which should reduce the modulus to bitwise
        // instructions (please compiler)
        assert_eq!(KEY.0.len(), 53280);
        assert_eq!(KEY.0.len() - SUB_KEY_BYTES, 2usize.pow(15) * 13 / 8);
    }

    #[test]
    fn subkey() {
        let mut key = [0u8; 32];
        for (i, val) in key.iter_mut().enumerate() {
            *val = i as u8;
        }
        println!("{:?}", &key[4..8]);
        let key = Key::new(&key);
        let subkey = u32::to_ne_bytes(key.subkey(1, 1)[0]);
        let expected = [4, 5, 6, 7];
        assert_eq!(subkey, expected);
    }
}
