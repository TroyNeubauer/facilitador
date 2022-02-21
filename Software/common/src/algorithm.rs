use crate::key::Key;
use core::marker::PhantomData;
use core::mem::size_of;
use core::ops::{BitXorAssign, Deref};

pub trait Index: core::ops::BitXor<Output = Self> + Sized + Copy {
    fn to_usize(self) -> usize;
}

#[repr(C, align(8))]
pub struct GenericCipherBlock<const N: usize>(pub [u8; N]);

impl<const N: usize> GenericCipherBlock<N> {
    pub fn new(buf: [u8; N]) -> Self {
        Self(buf)
    }
}

pub type CipherBlock = GenericCipherBlock<32>;

impl<const N: usize> Deref for GenericCipherBlock<N> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<const N: usize> AsRef<[u8]> for GenericCipherBlock<N> {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

pub trait Element: Copy + Default + BitXorAssign + crate::Word {}

struct GenericCipher<'k, Hash, IndexTy, const KEY_BYTES: usize, const BLOCK_BYTES: usize>
where
    Hash: Fn(IndexTy) -> IndexTy,
    IndexTy: Index,
{
    //hash: Hash,
    hash: Hash,
    key: &'k Key<KEY_BYTES>,
    index_key: IndexTy,
    _index: PhantomData<IndexTy>,
}

impl<'k, Hash, IndexTy, const KEY_BYTES: usize, const BLOCK_BYTES: usize>
    GenericCipher<'k, Hash, IndexTy, KEY_BYTES, BLOCK_BYTES>
where
    Hash: Fn(IndexTy) -> IndexTy,
    IndexTy: Index,
{
    /// Performs encryption or decryption of a single block.
    /// `L` determines many elements the u32 subkey has. Because N is in bytes, `L` should always
    /// be set to N / 4.
    ///
    /// Returns Err on error, and the bytes of `block` are not guaranteed to be encrypted or
    /// decrypted
    /// If Ok(()) is returned, all bytes of `block` have been encrypted/decrypted
    ///
    /// # Panics
    /// This function panics if L is the wrong size.
    /// If L is the correct size for N. This function will never panic, otherwise it will always
    /// panic
    pub fn cipher_block<const L: usize>(
        &self,
        index: IndexTy,
        block: &mut GenericCipherBlock<BLOCK_BYTES>,
    ) {
        if BLOCK_BYTES / size_of::<u32>() != L {
            // User choose wrong L for N
            panic!(
                "Wrong L ({}), for block bytes {}. Expected L to be {}",
                L,
                BLOCK_BYTES,
                BLOCK_BYTES / size_of::<u32>()
            );
        }

        // Perform Xor first, so that an attacker doesn't know the inputs to the hash function
        let index = index ^ self.index_key;
        let index = (self.hash)(index);
        let index = index.to_usize();

        let key = self.key.subkey::<u32, L>(index);

        // SAFETY: u8 is safe to transmute to u32. There are no invalid bit patterns
        let (before, buf, after) = unsafe { block.0.align_to_mut::<u32>() };

        // These lengths are guaranteed to be 0 because `CipherBlock` has explicit 4 byte alignment
        debug_assert!(before.is_empty());
        debug_assert!(after.is_empty());
        // Perform Xor encryption
        for i in 0..buf.len() {
            buf[i] ^= key[i];
        }
    }
}

fn identity_hash(index: u32) -> u32 {
    index
}

pub struct MainCipher<'k, Hash, const KEY_SIZE: usize>(GenericCipher<'k, Hash, u32, KEY_SIZE, 32>)
where
    Hash: Fn(u32) -> u32;

impl<'k, const KEY_BYTES: usize> MainCipher<'k, fn(u32) -> u32, KEY_BYTES> {
    pub fn new(key: &'k Key<KEY_BYTES>, index_key: u32) -> Self {
        Self(GenericCipher {
            hash: identity_hash,
            key,
            index_key,
            _index: Default::default(),
        })
    }

    /// Encrypts or decrypts a single block using `key` and `index`.
    /// Because Xor is used, the encryption and decryption operation is the same
    pub fn cipher_block(&self, index: u32, block: &mut GenericCipherBlock<32>) {
        self.0.cipher_block::<8>(index, block)
    }
}

impl Index for u32 {
    fn to_usize(self) -> usize {
        assert!(
            size_of::<usize>() >= size_of::<u32>(),
            "8 and 16 bit targets not supported yet!"
        );
        self.try_into().unwrap()
    }
}

impl Element for u32 {}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{RngCore, SeedableRng};

    #[test]
    fn encrypt_and_decrypt() {
        for i in 0..10 {
            let mut rng = rand::rngs::StdRng::seed_from_u64(i);

            let mut block_bytes = [0u8; 32];
            rng.fill_bytes(&mut block_bytes);
            let original_block = Clone::clone(&block_bytes);

            let mut key_bytes = [0u8; 64];
            rng.fill_bytes(&mut key_bytes);
            let key = Key::new(key_bytes);

            let mut index_key = [0u8; 4];
            rng.fill_bytes(&mut index_key);
            let index_key = u32::from_ne_bytes(index_key);

            let mut block = CipherBlock::new(block_bytes);
            let cipher = MainCipher::new(&key, index_key);

            //let index = rng.gen();
            let index = i as u32;
            cipher.cipher_block(index, &mut block);
            cipher.cipher_block(index, &mut block);
            assert_eq!(block.as_ref(), original_block.as_ref());
        }

        crate::key::print_freq();
    }
}
