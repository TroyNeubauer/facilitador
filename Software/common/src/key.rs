#[cfg(feature = "std")]
use lazy_static::lazy_static;
#[cfg(feature = "std")]
use std::{collections::HashMap, sync::Mutex};

/// Represents an `N` element key of type `T`
pub struct Key<T, const N: usize>([T; N])
where
    T: Copy + Default;

/// The symmetric key used for both encryption and decryption
pub const KEY: Key<u32, 53280> = Key::new(include_bytes!("../../private/key.bin"));

#[cfg(feature = "std")]
lazy_static! {
    static ref FREQ: Mutex<HashMap<usize, usize>> = Mutex::new(HashMap::new());
}

pub fn print_freq() {
    let lock = FREQ.lock().unwrap();
    for (k, v) in lock.iter() {
        println!("{}: {}", k, v);
    }
}

impl<T, const N: usize> Key<T, N>
where
    T: Copy + Default,
{
    /// Creates a new key.
    ///
    /// # Panics
    ///
    /// If `key.len()` is not a multiple of four
    pub fn new(key: &[u8]) -> Self {
        let elements = [T::default(); N];
        Self(elements)
    }

    pub fn new_with_elements(key: [T; N]) -> Self {
        Self(key)
    }

    /// Returns a slice len `key_len` of this key based on word offset module the key length
    /// `L` is the number of 32 bit elements returned
    pub fn subkey<const L: usize>(&self, word_offset: usize) -> &[T; L] {
        if L > N {
            panic!(
                "Subkey larger than main key! Main key elements: {}, requested elements: {}",
                N, L
            );
        }

        // We need to find `L` contiguous elements, so the maximum index (exclusive) is `L`
        // less than the total length of the key
        let max_index = (N + 1) - L;

        // Ensure offset is in range
        let offset = word_offset % max_index;

        // SAFETY:
        // Self is aligned to a 4 byte boundaries, so self.0 is aligned, so the resulting pointer is aligned
        let ptr: *const T = self.0.as_ptr() as *const T;

        #[cfg(feature = "std")]
        {
            dbg!(L, N, max_index, offset, max_index);
            let mut lock = FREQ.lock().unwrap();
            let count = lock.entry(offset).or_insert_with(|| 0);
            *count += 1;
        }

        // SAFETY:
        // 1. Offset is in range by the calculation of `max_index` above
        // 2. At least `L` elements are readable by the `max_index` calculation
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
        unsafe { &*(subkey_start as *const [T; L]) }
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
        assert_eq!(KEY.0.len() - KEY.0.len(), 2usize.pow(15) * 13 / 8);
    }

    #[test]
    fn subkey() {
        const KEY_LEN: usize = 32;
        let key = (0..KEY_LEN).collect();

        let key = Key::new_with_elements(key);
        for i in 0..32 {
            let subkey = key.subkey::<1>(i);
            let b = (i * 4 % KEY_LEN) as u8;
            let expected = [b, b + 1, b + 2, b + 3];
            assert_eq!(subkey[0], u32::from_ne_bytes(expected));
        }

        for i in 0..32 {
            let subkey = key.subkey::<4>(i);
            // The numbers are a bit strange here because when getting a 16 byte subkey from a 32
            // byte key, there are only 5 positions we can go to to get a unique key. We always
            // check the last 32 bit word of the subkey, so we need to add 12 because 3 * 4. We do
            // mod 20 because of 5 possible alignments we can have, 5 * 4 == 12
            let b = (i * 4 % 20) as u8 + 12;
            let expected = [b, b + 1, b + 2, b + 3];
            assert_eq!(subkey[3], u32::from_ne_bytes(expected));
        }
        //Make sure this works for zero sized types
        let zst = key.subkey::<0>(0);
        assert!(zst.is_empty());
    }
}
