use core::{mem::size_of, slice};

#[repr(C, align(4))]
pub struct Key<const N: usize>([u8; N]);

/// The symmetric key used for both encryption and decryption
pub const KEY: Key<53280> = Key(*include_bytes!("../../private/key.bin"));

/// The number of bytes in a normal subkey
pub const SUB_KEY_BYTES: usize = 32;

/// The number of 32 bit elements in a normal subkey
pub const SUB_KEY_ELEMENTS: usize = SUB_KEY_BYTES / size_of::<u32>();

/// The number of bytes used to store the index at the beginning of the packet.
/// The high bit of the index is always 0. 1 is reserved for future protocol expansion
pub const INDEX_BYTES: usize = 2;

impl<const N: usize> Key<N> {
    /// Creates a new key.
    ///
    /// # Panics
    ///
    /// If `key.len()` is not a multiple of four
    pub fn new(key: [u8; N]) -> Self {
        Self(key)
    }

    /// Returns a slice len `key_len` of this key based on word offset module the key length
    pub fn subkey<const L: usize>(&self, word_offset: usize) -> &[u32; L] {
        // Dividing by the size of a u32 rounds truncates the remainder, so a 32 bit slice of self.0
        // has at most `len` elements
        let len = self.0.len() / size_of::<u32>();

        // We need to find `Ly_len` contiguous elements, so the maximum index (exclusive) is `L`
        // less than the total length of the key
        let max_index = (len + 1) - L;

        // Ensure offset is in range
        let offset = word_offset % max_index;

        // SAFETY:
        // Self is aligned to a 4 byte boundaries, so self.0 is aligned, so the resulting pointer is aligned
        let ptr: *const u32 = self.0.as_ptr() as *const u32;
        // SAFETY:
        // 1. Offset is in range by the calculation of `max_index` above
        // 2. At least `key_len` elements are readable by the `max_index` calculation
        // 3. The result is aligned because `word_slice` is aligned
        // 4. The lifetime of the result is 'k because `word_slice` is 'k
        let subkey_start = unsafe { ptr.add(offset) }; 

        // SAFETY:
        // 1. The layout of a array type ([T; N]) has the same alignment requirements of T, and has
        //    the size of size_of::<T>() * N
        // 2. The lifetime of `self.0` is 'self, so the lifetime elision knows that the returned
        //    lifetime is 'self
        // 
        // See: https://doc.rust-lang.org/reference/type-layout.html#array-layout
        unsafe { &*(subkey_start as *const [u32; L]) }
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
        const KEY_LEN: usize = 32;
        let mut key = [0u8; KEY_LEN];
        for (i, val) in key.iter_mut().enumerate() {
            *val = i as u8;
        }

        let key = Key::new(key);
        for i in 0..2048 {
            let subkey = key.subkey::<1>(i);
            let b = (i * 4 % KEY_LEN) as u8;
            let expected = [b, b + 1, b + 2, b + 3];
            assert_eq!(subkey[0], u32::from_ne_bytes(expected));
        }

        for i in 0..2048 {
            let subkey = key.subkey::<4>(i);
            // The numbers are a bit strange here because when getting a 16 byte subkey from a 32
            // byte key, there are only 5 positions we can go to to get a unique key. We always
            // check the last 32 bit word of the subkey, so we need to add 12 because 3 * 4. We do
            // mod 20 because of 5 possible alignments we can have, 5 * 4 == 12
            let b = (i * 4 % 20) as u8 + 12;
            let expected = [b, b + 1, b + 2, b + 3];
            println!("i {}, b {}, subkey {:?}, expected {:?}", i, b, expected, subkey[3].to_ne_bytes());
            assert_eq!(subkey[3], u32::from_ne_bytes(expected));
        }
    }
}