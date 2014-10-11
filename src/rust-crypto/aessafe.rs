// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

/*!

The `aessafe` module implements the AES algorithm completely in software without using any table
lookups or other timing dependant mechanisms. This module actually contains two seperate
implementations - an implementation that works on a single block at a time and a second
implementation that processes 8 blocks in parallel. Some block encryption modes really only work if
you are processing a single blocks (CFB, OFB, and CBC encryption for example) while other modes
are trivially parallelizable (CTR and CBC decryption). Processing more blocks at once allows for
greater efficiency, especially when using wide registers, such as the XMM registers available in
x86 processors.

## AES Algorithm

There are lots of places to go to on the internet for an involved description of how AES works. For
the purposes of this description, it sufficies to say that AES is just a block cipher that takes
a key of 16, 24, or 32 bytes and uses that to either encrypt or decrypt a block of 16 bytes. An
encryption or decryption operation consists of a number of rounds which involve some combination of
the following 4 basic operations:

* ShiftRows
* MixColumns
* SubBytes
* AddRoundKey

## Timing problems

Most software implementations of AES use a large set of lookup tables - generally at least the
SubBytes step is implemented via lookup tables; faster implementations generally implement the
MixColumns step this way as well. This is largely a design flaw in the AES implementation as it was
not realized during the NIST standardization process that table lookups can lead to security
problems [1]. The issue is that not all table lookups occur in constant time - an address that was
recently used is looked up much faster than one that hasn't been used in a while. A careful
adversary can measure the amount of time that each AES operation takes and use that information to
help determine the secret key or plain text information. More specifically, its not table lookups
that lead to these types of timing attacks - the issue is table lookups that use secret information
as part of the address to lookup. A table lookup that is performed the exact same way every time
regardless of the key or plaintext doesn't leak any information. This implementation uses no data
dependant table lookups.

## Bit Slicing

Bit Slicing is a technique that is basically a software emulation of hardware implementation
techniques. One of the earliest implementations of this technique was for a DES implementation [4].
In hardware, table lookups do not present the same timing problems as they do in software, however
they present other problems - namely that a 256 byte S-box table takes up a huge amount of space on
a chip. Hardware implementations, thus, tend to avoid table lookups and instead calculate the
contents of the S-Boxes as part of every operation. So, the key to an efficient Bit Sliced software
implementation is to re-arrange all of the bits of data to process into a form that can easily be
applied in much the same way that it would be in hardeware. It is fortunate, that AES was designed
such that these types of hardware implementations could be very efficient - the contents of the
S-boxes are defined by a mathematical formula.

A hardware implementation works on single bits at a time. Unlike adding variables in software,
however, that occur generally one at a time, hardware implementations are extremely parallel and
operate on many, many bits at once. Bit Slicing emulates that by moving all "equivalent" bits into
common registers and then operating on large groups of bits all at once. Calculating the S-box value
for a single bit is extremely expensive, but its much cheaper when you can amortize that cost over
128 bits (as in an XMM register). This implementation follows the same strategy as in [5] and that
is an excellent source for more specific details. However, a short description follows.

The input data is simply a collection of bytes. Each byte is comprised of 8 bits, a low order bit
(bit 0) through a high order bit (bit 7). Bit slicing the input data simply takes all of the low
order bits (bit 0) from the input data, and moves them into a single register (eg: XMM0). Next, all
of them 2nd lowest bits are moved into their own register (eg: XMM1), and so on. After completion,
we're left with 8 variables, each of which contains an equivalent set of bits. The exact order of
those bits is irrevent for the implementation of the SubBytes step, however, it is very important
for the MixColumns step. Again, see [5] for details. Due to the design of AES, its them possible to
execute the entire AES operation using just bitwise exclusive ors and rotates once we have Bit
Sliced the input data. After the completion of the AES operation, we then un-Bit Slice the data
to give us our output. Clearly, the more bits that we can process at once, the faster this will go -
thus, the version that processes 8 blocks at once is roughly 8 times faster than processing just a
single block at a time.

The ShiftRows step is fairly straight-forward to implement on the Bit Sliced state. The MixColumns
and especially the SubBytes steps are more complicated. This implementation draws heavily on the
formulas from [5], [6], and [7] to implement these steps.

## Implementation

Both implementations work basically the same way and share pretty much all of their code. The key
is first processed to create all of the round keys where each round key is just a 16 byte chunk of
data that is combined into the AES state by the AddRoundKey step as part of each encryption or
decryption round. Processing the round key can be expensive, so this is done before encryption or
decryption. Before encrypting or decrypting data, the data to be processed by be Bit Sliced into 8
seperate variables where each variable holds equivalent bytes from the state. This Bit Sliced state
is stored as a Bs8State<T>, where T is the type that stores each set of bits. The first
implementation stores these bits in a u32 which permits up to 8 * 32 = 1024 bits of data to be
processed at once. This implementation only processes a single block at a time, so, in reality, only
512 bits are processed at once and the remaining 512 bits of the variables are unused. The 2nd
implementation uses u32x4s - vectors of 4 u32s. Thus, we can process 8 * 128 = 4096 bits at once,
which corresponds exactly to 8 blocks.

The Bs8State struct implements the AesOps trait, which contains methods for each of the 4 main steps
of the AES algorithm. The types, T, each implement the AesBitValueOps trait, which containts methods
necessary for processing a collection or bit values and the AesOps trait relies heavily on this
trait to perform its operations.

The Bs4State and Bs2State struct implement operations of various subfields of the full GF(2^8)
finite field which allows for efficient computation of the AES S-Boxes. See [7] for details.

## References

[1] - "Cache-Collision Timing Attacks Against AES". Joseph Bonneau and Ilya Mironov.
      http://www.jbonneau.com/doc/BM06-CHES-aes_cache_timing.pdf
[2] - "Software mitigations to hedge AES against cache-based software side channel vulnerabilities".
      Ernie Brickell, et al. http://eprint.iacr.org/2006/052.pdf.
[3] - "Cache Attacks and Countermeasures: the Case of AES (Extended Version)".
      Dag Arne Osvik, et al. tau.ac.il/~tromer/papers/cache.pdf‎.
[4] - "A Fast New DES Implementation in Software". Eli Biham.
      http://citeseerx.ist.psu.edu/viewdoc/download?doi=10.1.1.52.5429&rep=rep1&type=pdf.
[5] - "Faster and Timing-Attack Resistant AES-GCM". Emilia K ̈asper and Peter Schwabe.
      http://www.chesworkshop.org/ches2009/presentations/01_Session_1/CHES2009_ekasper.pdf.
[6] - "FAST AES DECRYPTION". Vinit Azad. http://webcache.googleusercontent.com/
      search?q=cache:ld_f8pSgURcJ:csusdspace.calstate.edu/bitstream/handle/10211.9/1224/
      Vinit_Azad_MS_Report.doc%3Fsequence%3D2+&cd=4&hl=en&ct=clnk&gl=us&client=ubuntu.
[7] - "A Very Compact Rijndael S-box". D. Canright.
      http://www.dtic.mil/cgi-bin/GetTRDoc?AD=ADA434781.
*/

use std::default::Default;
use std::iter::range_step;

use cryptoutil::{read_u32v_le, write_u32_le};
use symmetriccipher::{BlockEncryptor, BlockEncryptorX8, BlockDecryptor, BlockDecryptorX8};

// Using std::unstable::simd::u32x4 results in issues creating static arrays of u32x4 values.
// Defining the type here avoids that problem. Additionally, we need to implement various trait from
// libstd which wouldn't be possible if we used that type directly.
#[simd]
#[deriving(Clone, Eq, PartialEq)]
#[allow(non_camel_case_types)]
pub struct u32x4(u32, u32, u32, u32);

// There are a variety of places where we need to use u32x4 types with either all bits set or not
// bits set. These macros make that more succinct.
macro_rules! o( () => ( u32x4(0, 0, 0, 0) ) )
macro_rules! x( () => ( u32x4(-1, -1, -1, -1) ) )

macro_rules! define_aes_struct(
    (
        $name:ident,
        $rounds:expr
    ) => (
        pub struct $name {
            sk: [Bs8State<u32>, ..$rounds + 1]
        }
    )
)

macro_rules! define_aes_impl(
    (
        $name:ident,
        $mode:ident,
        $rounds:expr,
        $key_size:expr
    ) => (
        impl $name {
            pub fn new(key: &[u8]) -> $name {
                let mut a =  $name {
                    sk: [Bs8State(0, 0, 0, 0, 0, 0, 0, 0), ..$rounds + 1]
                };
                let mut tmp = [[0u32, ..4], ..$rounds + 1];
                create_round_keys(key, $mode, tmp);
                for i in range(0u, $rounds + 1) {
                    a.sk[i] = bit_slice_4x4_with_u32(tmp[i][0], tmp[i][1], tmp[i][2], tmp[i][3]);
                }
                return a;
            }
        }
    )
)

macro_rules! define_aes_enc(
    (
        $name:ident,
        $rounds:expr
    ) => (
        impl BlockEncryptor for $name {
            fn block_size(&self) -> uint { 16 }
            fn encrypt_block(&self, input: &[u8], output: &mut [u8]) {
                let mut bs = bit_slice_1x16_with_u32(input);
                bs = encrypt_core(&bs, self.sk);
                un_bit_slice_1x16_with_u32(&bs, output);
            }
        }
    )
)

macro_rules! define_aes_dec(
    (
        $name:ident,
        $rounds:expr
    ) => (
        impl BlockDecryptor for $name {
            fn block_size(&self) -> uint { 16 }
            fn decrypt_block(&self, input: &[u8], output: &mut [u8]) {
                let mut bs = bit_slice_1x16_with_u32(input);
                bs = decrypt_core(&bs, self.sk);
                un_bit_slice_1x16_with_u32(&bs, output);
            }
        }
    )
)

define_aes_struct!(AesSafe128Encryptor, 10)
define_aes_struct!(AesSafe128Decryptor, 10)
define_aes_impl!(AesSafe128Encryptor, Encryption, 10, 16)
define_aes_impl!(AesSafe128Decryptor, Decryption, 10, 16)
define_aes_enc!(AesSafe128Encryptor, 10)
define_aes_dec!(AesSafe128Decryptor, 10)

define_aes_struct!(AesSafe192Encryptor, 12)
define_aes_struct!(AesSafe192Decryptor, 12)
define_aes_impl!(AesSafe192Encryptor, Encryption, 12, 24)
define_aes_impl!(AesSafe192Decryptor, Decryption, 12, 24)
define_aes_enc!(AesSafe192Encryptor, 12)
define_aes_dec!(AesSafe192Decryptor, 12)

define_aes_struct!(AesSafe256Encryptor, 14)
define_aes_struct!(AesSafe256Decryptor, 14)
define_aes_impl!(AesSafe256Encryptor, Encryption, 14, 32)
define_aes_impl!(AesSafe256Decryptor, Decryption, 14, 32)
define_aes_enc!(AesSafe256Encryptor, 14)
define_aes_dec!(AesSafe256Decryptor, 14)

macro_rules! define_aes_struct_x8(
    (
        $name:ident,
        $rounds:expr
    ) => (
        pub struct $name {
            sk: [Bs8State<u32x4>, ..$rounds + 1]
        }
    )
)

macro_rules! define_aes_impl_x8(
    (
        $name:ident,
        $mode:ident,
        $rounds:expr,
        $key_size:expr
    ) => (
        impl $name {
            pub fn new(key: &[u8]) -> $name {
                let mut a =  $name {
                    sk: [Bs8State(o!(), o!(), o!(), o!(), o!(), o!(), o!(), o!()), ..$rounds + 1]
                };
                let mut tmp = [[0u32, ..4], ..$rounds + 1];
                create_round_keys(key, $mode, tmp);
                for i in range(0u, $rounds + 1) {
                    a.sk[i] = bit_slice_fill_4x4_with_u32x4(
                        tmp[i][0],
                        tmp[i][1],
                        tmp[i][2],
                        tmp[i][3]);
                }
                return a;
            }
        }
    )
)

macro_rules! define_aes_enc_x8(
    (
        $name:ident,
        $rounds:expr
    ) => (
        impl BlockEncryptorX8 for $name {
            fn block_size(&self) -> uint { 16 }
            fn encrypt_block_x8(&self, input: &[u8], output: &mut [u8]) {
                let bs = bit_slice_1x128_with_u32x4(input);
                let bs2 = encrypt_core(&bs, self.sk);
                un_bit_slice_1x128_with_u32x4(&bs2, output);
            }
        }
    )
)

macro_rules! define_aes_dec_x8(
    (
        $name:ident,
        $rounds:expr
    ) => (
        impl BlockDecryptorX8 for $name {
            fn block_size(&self) -> uint { 16 }
            fn decrypt_block_x8(&self, input: &[u8], output: &mut [u8]) {
                let bs = bit_slice_1x128_with_u32x4(input);
                let bs2 = decrypt_core(&bs, self.sk);
                un_bit_slice_1x128_with_u32x4(&bs2, output);
            }
        }
    )
)

define_aes_struct_x8!(AesSafe128EncryptorX8, 10)
define_aes_struct_x8!(AesSafe128DecryptorX8, 10)
define_aes_impl_x8!(AesSafe128EncryptorX8, Encryption, 10, 16)
define_aes_impl_x8!(AesSafe128DecryptorX8, Decryption, 10, 16)
define_aes_enc_x8!(AesSafe128EncryptorX8, 10)
define_aes_dec_x8!(AesSafe128DecryptorX8, 10)

define_aes_struct_x8!(AesSafe192EncryptorX8, 12)
define_aes_struct_x8!(AesSafe192DecryptorX8, 12)
define_aes_impl_x8!(AesSafe192EncryptorX8, Encryption, 12, 24)
define_aes_impl_x8!(AesSafe192DecryptorX8, Decryption, 12, 24)
define_aes_enc_x8!(AesSafe192EncryptorX8, 12)
define_aes_dec_x8!(AesSafe192DecryptorX8, 12)

define_aes_struct_x8!(AesSafe256EncryptorX8, 14)
define_aes_struct_x8!(AesSafe256DecryptorX8, 14)
define_aes_impl_x8!(AesSafe256EncryptorX8, Encryption, 14, 32)
define_aes_impl_x8!(AesSafe256DecryptorX8, Decryption, 14, 32)
define_aes_enc_x8!(AesSafe256EncryptorX8, 14)
define_aes_dec_x8!(AesSafe256DecryptorX8, 14)

fn ffmulx(x: u32) -> u32 {
    let m1: u32 = 0x80808080;
    let m2: u32 = 0x7f7f7f7f;
    let m3: u32 = 0x0000001b;
    return ((x & m2) << 1) ^ (((x & m1) >> 7) * m3);
}

fn inv_mcol(x: u32) -> u32 {
    let f2 = ffmulx(x);
    let f4 = ffmulx(f2);
    let f8 = ffmulx(f4);
    let f9 = x ^ f8;

    return f2 ^ f4 ^ f8 ^ (f2 ^ f9).rotate_right(8) ^ (f4 ^ f9).rotate_right(16) ^ f9.rotate_right(24);
}

fn sub_word(x: u32) -> u32 {
    let bs = bit_slice_4x1_with_u32(x).sub_bytes();
    return un_bit_slice_4x1_with_u32(&bs);
}

enum KeyType {
    Encryption,
    Decryption
}

// This array is not accessed in any key-dependant way, so there are no timing problems inherent in
// using it.
static RCON: [u32, ..10] = [0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0x1b, 0x36];

// The round keys are created without bit-slicing the key data. The individual implementations bit
// slice the round keys returned from this function. This function, and the few functions above, are
// derived from the BouncyCastle AES implementation.
fn create_round_keys(key: &[u8], key_type: KeyType, round_keys: &mut [[u32, ..4]]) {
    let (key_words, rounds) = match key.len() {
        16 => (4, 10u),
        24 => (6, 12u),
        32 => (8, 14u),
        _ => fail!("Invalid AES key size.")
    };

    // The key is copied directly into the first few round keys
    let mut j = 0;
    for i in range_step(0u, key.len(), 4) {
        round_keys[j / 4][j % 4] =
            (key[i] as u32) |
            ((key[i+1] as u32) << 8) |
            ((key[i+2] as u32) << 16) |
            ((key[i+3] as u32) << 24);
        j += 1;
    };

    // Calculate the rest of the round keys
    for i in range(key_words, (rounds + 1) * 4) {
        let mut tmp = round_keys[(i - 1) / 4][(i - 1) % 4];
        if (i % key_words) == 0 {
            tmp = sub_word(tmp.rotate_right(8)) ^ RCON[(i / key_words) - 1];
        } else if (key_words == 8) && ((i % key_words) == 4) {
            // This is only necessary for AES-256 keys
            tmp = sub_word(tmp);
        }
        round_keys[i / 4][i % 4] = round_keys[(i - key_words) / 4][(i - key_words) % 4] ^ tmp;
    }

    // Decryption round keys require extra processing
    match key_type {
        Decryption => {
            for j in range(1u, rounds) {
                for i in range(0u, 4) {
                    round_keys[j][i] = inv_mcol(round_keys[j][i]);
                }
            }
        },
        Encryption => { }
    }
}

// This trait defines all of the operations needed for a type to be processed as part of an AES
// encryption or decryption operation.
trait AesOps {
    fn sub_bytes(&self) -> Self;
    fn inv_sub_bytes(&self) -> Self;
    fn shift_rows(&self) -> Self;
    fn inv_shift_rows(&self) -> Self;
    fn mix_columns(&self) -> Self;
    fn inv_mix_columns(&self) -> Self;
    fn add_round_key(&self, rk: &Self) -> Self;
}

fn encrypt_core<S: AesOps>(state: &S, sk: &[S]) -> S {
    // Round 0 - add round key
    let mut tmp = state.add_round_key(&sk[0]);

    // Remaining rounds (except last round)
    for i in range(1, sk.len() - 1) {
        tmp = tmp.sub_bytes();
        tmp = tmp.shift_rows();
        tmp = tmp.mix_columns();
        tmp = tmp.add_round_key(&sk[i]);
    }

    // Last round
    tmp = tmp.sub_bytes();
    tmp = tmp.shift_rows();
    tmp = tmp.add_round_key(&sk[sk.len() - 1]);

    return tmp;
}

fn decrypt_core<S: AesOps>(state: &S, sk: &[S]) -> S {
    // Round 0 - add round key
    let mut tmp = state.add_round_key(&sk[sk.len() - 1]);

    // Remaining rounds (except last round)
    for i in range(1, sk.len() - 1) {
        tmp = tmp.inv_sub_bytes();
        tmp = tmp.inv_shift_rows();
        tmp = tmp.inv_mix_columns();
        tmp = tmp.add_round_key(&sk[sk.len() - 1 - i]);
    }

    // Last round
    tmp = tmp.inv_sub_bytes();
    tmp = tmp.inv_shift_rows();
    tmp = tmp.add_round_key(&sk[0]);

    return tmp;
}


struct Bs8State<T>(T, T, T, T, T, T, T, T);

impl <T: Clone> Bs8State<T> {
    fn split(&self) -> (Bs4State<T>, Bs4State<T>) {
        let Bs8State(ref x0, ref x1, ref x2, ref x3, ref x4, ref x5, ref x6, ref x7) = *self;
        return (Bs4State(x0.clone(), x1.clone(), x2.clone(), x3.clone()),
            Bs4State(x4.clone(), x5.clone(), x6.clone(), x7.clone()));
    }
}

impl <T: BitXor<T, T>> Bs8State<T> {
    fn xor(&self, rhs: &Bs8State<T>) -> Bs8State<T> {
        let Bs8State(ref a0, ref a1, ref a2, ref a3, ref a4, ref a5, ref a6, ref a7) = *self;
        let Bs8State(ref b0, ref b1, ref b2, ref b3, ref b4, ref b5, ref b6, ref b7) = *rhs;
        return Bs8State(*a0 ^ *b0, *a1 ^ *b1, *a2 ^ *b2, *a3 ^ *b3,
            *a4 ^ *b4, *a5 ^ *b5, *a6 ^ *b6, *a7 ^ *b7);
    }
}

struct Bs4State<T>(T, T, T, T);

impl <T: Clone> Bs4State<T> {
    fn split(&self) -> (Bs2State<T>, Bs2State<T>) {
        let Bs4State(ref x0, ref x1, ref x2, ref x3) = *self;
        return (Bs2State(x0.clone(), x1.clone()), Bs2State(x2.clone(), x3.clone()));
    }

    fn join(&self, rhs: &Bs4State<T>) -> Bs8State<T> {
        let Bs4State(ref a0, ref a1, ref a2, ref a3) = *self;
        let Bs4State(ref b0, ref b1, ref b2, ref b3) = *rhs;
        return Bs8State(a0.clone(), a1.clone(), a2.clone(), a3.clone(),
            b0.clone(), b1.clone(), b2.clone(), b3.clone());
    }
}

impl <T: BitXor<T, T>> Bs4State<T> {
    fn xor(&self, rhs: &Bs4State<T>) -> Bs4State<T> {
        let Bs4State(ref a0, ref a1, ref a2, ref a3) = *self;
        let Bs4State(ref b0, ref b1, ref b2, ref b3) = *rhs;
        return Bs4State(*a0 ^ *b0, *a1 ^ *b1, *a2 ^ *b2, *a3 ^ *b3);
    }
}

struct Bs2State<T>(T, T);

impl <T: Clone> Bs2State<T> {
    fn split(&self) -> (T, T) {
        let Bs2State(ref x0, ref x1) = *self;
        return (x0.clone(), x1.clone());
    }

    fn join(&self, rhs: &Bs2State<T>) -> Bs4State<T> {
        let Bs2State(ref a0, ref a1) = *self;
        let Bs2State(ref b0, ref b1) = *rhs;
        return Bs4State(a0.clone(), a1.clone(), b0.clone(), b1.clone());
    }
}

impl <T: BitXor<T, T>> Bs2State<T> {
    fn xor(&self, rhs: &Bs2State<T>) -> Bs2State<T> {
        let Bs2State(ref a0, ref a1) = *self;
        let Bs2State(ref b0, ref b1) = *rhs;
        return Bs2State(*a0 ^ *b0, *a1 ^ *b1);
    }
}

// Pick the specified bit from the value x and shift it left by the specified amount.
fn pb(x: u32, bit: uint, shift: uint) -> u32 {
    ((x >> bit) & 1) << shift
}

// Bit Slice data in the form of 4 u32s in column-major order
fn bit_slice_4x4_with_u32(a: u32, b: u32, c: u32, d: u32) -> Bs8State<u32> {
    fn construct(a: u32, b: u32, c: u32, d: u32, bit: uint) -> u32 {
        pb(a, bit, 0)       | pb(b, bit, 1)       | pb(c, bit, 2)       | pb(d, bit, 3)       |
        pb(a, bit + 8, 4)   | pb(b, bit + 8, 5)   | pb(c, bit + 8, 6)   | pb(d, bit + 8, 7)   |
        pb(a, bit + 16, 8)  | pb(b, bit + 16, 9)  | pb(c, bit + 16, 10) | pb(d, bit + 16, 11) |
        pb(a, bit + 24, 12) | pb(b, bit + 24, 13) | pb(c, bit + 24, 14) | pb(d, bit + 24, 15)
    }

    let x0 = construct(a, b, c, d, 0);
    let x1 = construct(a, b, c, d, 1);
    let x2 = construct(a, b, c, d, 2);
    let x3 = construct(a, b, c, d, 3);
    let x4 = construct(a, b, c, d, 4);
    let x5 = construct(a, b, c, d, 5);
    let x6 = construct(a, b, c, d, 6);
    let x7 = construct(a, b, c, d, 7);

    return Bs8State(x0, x1, x2, x3, x4, x5, x6, x7);
}

// Bit slice a single u32 value - this is used to calculate the SubBytes step when creating the
// round keys.
fn bit_slice_4x1_with_u32(a: u32) -> Bs8State<u32> {
    return bit_slice_4x4_with_u32(a, 0, 0, 0);
}

// Bit slice a 16 byte array in column major order
fn bit_slice_1x16_with_u32(data: &[u8]) -> Bs8State<u32> {
    let mut n = [0u32, ..4];
    read_u32v_le(n, data);

    let a = n[0];
    let b = n[1];
    let c = n[2];
    let d = n[3];

    return bit_slice_4x4_with_u32(a, b, c, d);
}

// Un Bit Slice into a set of 4 u32s
fn un_bit_slice_4x4_with_u32(bs: &Bs8State<u32>) -> (u32, u32, u32, u32) {
    fn deconstruct(bs: &Bs8State<u32>, bit: uint) -> u32 {
        let Bs8State(x0, x1, x2, x3, x4, x5, x6, x7) = *bs;

        pb(x0, bit, 0) | pb(x1, bit, 1) | pb(x2, bit, 2) | pb(x3, bit, 3) |
        pb(x4, bit, 4) | pb(x5, bit, 5) | pb(x6, bit, 6) | pb(x7, bit, 7) |

        pb(x0, bit + 4, 8)  | pb(x1, bit + 4, 9)  | pb(x2, bit + 4, 10) | pb(x3, bit + 4, 11) |
        pb(x4, bit + 4, 12) | pb(x5, bit + 4, 13) | pb(x6, bit + 4, 14) | pb(x7, bit + 4, 15) |

        pb(x0, bit + 8, 16) | pb(x1, bit + 8, 17) | pb(x2, bit + 8, 18) | pb(x3, bit + 8, 19) |
        pb(x4, bit + 8, 20) | pb(x5, bit + 8, 21) | pb(x6, bit + 8, 22) | pb(x7, bit + 8, 23) |

        pb(x0, bit + 12, 24) | pb(x1, bit + 12, 25) | pb(x2, bit + 12, 26) | pb(x3, bit + 12, 27) |
        pb(x4, bit + 12, 28) | pb(x5, bit + 12, 29) | pb(x6, bit + 12, 30) | pb(x7, bit + 12, 31)
    }

    let a = deconstruct(bs, 0);
    let b = deconstruct(bs, 1);
    let c = deconstruct(bs, 2);
    let d = deconstruct(bs, 3);

    return (a, b, c, d);
}

// Un Bit Slice into a single u32. This is used when creating the round keys.
fn un_bit_slice_4x1_with_u32(bs: &Bs8State<u32>) -> u32 {
    let (a, _, _, _) = un_bit_slice_4x4_with_u32(bs);
    return a;
}

// Un Bit Slice into a 16 byte array
fn un_bit_slice_1x16_with_u32(bs: &Bs8State<u32>, output: &mut [u8]) {
    let (a, b, c, d) = un_bit_slice_4x4_with_u32(bs);

    write_u32_le(output[mut 0..4], a);
    write_u32_le(output[mut 4..8], b);
    write_u32_le(output[mut 8..12], c);
    write_u32_le(output[mut 12..16], d);
}

// Bit Slice a 128 byte array of eight 16 byte blocks. Each block is in column major order.
fn bit_slice_1x128_with_u32x4(data: &[u8]) -> Bs8State<u32x4> {
    let bit0 = u32x4(0x01010101, 0x01010101, 0x01010101, 0x01010101);
    let bit1 = u32x4(0x02020202, 0x02020202, 0x02020202, 0x02020202);
    let bit2 = u32x4(0x04040404, 0x04040404, 0x04040404, 0x04040404);
    let bit3 = u32x4(0x08080808, 0x08080808, 0x08080808, 0x08080808);
    let bit4 = u32x4(0x10101010, 0x10101010, 0x10101010, 0x10101010);
    let bit5 = u32x4(0x20202020, 0x20202020, 0x20202020, 0x20202020);
    let bit6 = u32x4(0x40404040, 0x40404040, 0x40404040, 0x40404040);
    let bit7 = u32x4(0x80808080, 0x80808080, 0x80808080, 0x80808080);

    fn read_row_major(data: &[u8]) -> u32x4 {
        return u32x4(
            (data[0] as u32) |
            ((data[4] as u32) << 8) |
            ((data[8] as u32) << 16) |
            ((data[12] as u32) << 24),
            (data[1] as u32) |
            ((data[5] as u32) << 8) |
            ((data[9] as u32) << 16) |
            ((data[13] as u32) << 24),
            (data[2] as u32) |
            ((data[6] as u32) << 8) |
            ((data[10] as u32) << 16) |
            ((data[14] as u32) << 24),
            (data[3] as u32) |
            ((data[7] as u32) << 8) |
            ((data[11] as u32) << 16) |
            ((data[15] as u32) << 24));
    }

    let t0 = read_row_major(data[0..16]);
    let t1 = read_row_major(data[16..32]);
    let t2 = read_row_major(data[32..48]);
    let t3 = read_row_major(data[48..64]);
    let t4 = read_row_major(data[64..80]);
    let t5 = read_row_major(data[80..96]);
    let t6 = read_row_major(data[96..112]);
    let t7 = read_row_major(data[112..128]);

    let x0 = (t0 & bit0) | (t1.lsh(1) & bit1) | (t2.lsh(2) & bit2) | (t3.lsh(3) & bit3) |
        (t4.lsh(4) & bit4) | (t5.lsh(5) & bit5) | (t6.lsh(6) & bit6) | (t7.lsh(7) & bit7);
    let x1 = (t0.rsh(1) & bit0) | (t1 & bit1) | (t2.lsh(1) & bit2) | (t3.lsh(2) & bit3) |
        (t4.lsh(3) & bit4) | (t5.lsh(4) & bit5) | (t6.lsh(5) & bit6) | (t7.lsh(6) & bit7);
    let x2 = (t0.rsh(2) & bit0) | (t1.rsh(1) & bit1) | (t2 & bit2) | (t3.lsh(1) & bit3) |
        (t4.lsh(2) & bit4) | (t5.lsh(3) & bit5) | (t6.lsh(4) & bit6) | (t7.lsh(5) & bit7);
    let x3 = (t0.rsh(3) & bit0) | (t1.rsh(2) & bit1) | (t2.rsh(1) & bit2) | (t3 & bit3) |
        (t4.lsh(1) & bit4) | (t5.lsh(2) & bit5) | (t6.lsh(3) & bit6) | (t7.lsh(4) & bit7);
    let x4 = (t0.rsh(4) & bit0) | (t1.rsh(3) & bit1) | (t2.rsh(2) & bit2) | (t3.rsh(1) & bit3) |
        (t4 & bit4) | (t5.lsh(1) & bit5) | (t6.lsh(2) & bit6) | (t7.lsh(3) & bit7);
    let x5 = (t0.rsh(5) & bit0) | (t1.rsh(4) & bit1) | (t2.rsh(3) & bit2) | (t3.rsh(2) & bit3) |
        (t4.rsh(1) & bit4) | (t5 & bit5) | (t6.lsh(1) & bit6) | (t7.lsh(2) & bit7);
    let x6 = (t0.rsh(6) & bit0) | (t1.rsh(5) & bit1) | (t2.rsh(4) & bit2) | (t3.rsh(3) & bit3) |
        (t4.rsh(2) & bit4) | (t5.rsh(1) & bit5) | (t6 & bit6) | (t7.lsh(1) & bit7);
    let x7 = (t0.rsh(7) & bit0) | (t1.rsh(6) & bit1) | (t2.rsh(5) & bit2) | (t3.rsh(4) & bit3) |
        (t4.rsh(3) & bit4) | (t5.rsh(2) & bit5) | (t6.rsh(1) & bit6) | (t7 & bit7);

    return Bs8State(x0, x1, x2, x3, x4, x5, x6, x7);
}

// Bit slice a set of 4 u32s by filling a full 128 byte data block with those repeated values. This
// is used as part of bit slicing the round keys.
fn bit_slice_fill_4x4_with_u32x4(a: u32, b: u32, c: u32, d: u32) -> Bs8State<u32x4> {
    let mut tmp = [0u8, ..128];
    for i in range(0u, 8) {
        write_u32_le(tmp[mut i * 16..i * 16 + 4], a);
        write_u32_le(tmp[mut i * 16 + 4..i * 16 + 8], b);
        write_u32_le(tmp[mut i * 16 + 8..i * 16 + 12], c);
        write_u32_le(tmp[mut i * 16 + 12..i * 16 + 16], d);
    }
    return bit_slice_1x128_with_u32x4(tmp);
}

// Un bit slice into a 128 byte buffer.
fn un_bit_slice_1x128_with_u32x4(bs: &Bs8State<u32x4>, output: &mut [u8]) {
    let Bs8State(t0, t1, t2, t3, t4, t5, t6, t7) = *bs;

    let bit0 = u32x4(0x01010101, 0x01010101, 0x01010101, 0x01010101);
    let bit1 = u32x4(0x02020202, 0x02020202, 0x02020202, 0x02020202);
    let bit2 = u32x4(0x04040404, 0x04040404, 0x04040404, 0x04040404);
    let bit3 = u32x4(0x08080808, 0x08080808, 0x08080808, 0x08080808);
    let bit4 = u32x4(0x10101010, 0x10101010, 0x10101010, 0x10101010);
    let bit5 = u32x4(0x20202020, 0x20202020, 0x20202020, 0x20202020);
    let bit6 = u32x4(0x40404040, 0x40404040, 0x40404040, 0x40404040);
    let bit7 = u32x4(0x80808080, 0x80808080, 0x80808080, 0x80808080);

    // decode the individual blocks, in row-major order
    // TODO: this is identical to the same block in bit_slice_1x128_with_u32x4
    let x0 = (t0 & bit0) | (t1.lsh(1) & bit1) | (t2.lsh(2) & bit2) | (t3.lsh(3) & bit3) |
        (t4.lsh(4) & bit4) | (t5.lsh(5) & bit5) | (t6.lsh(6) & bit6) | (t7.lsh(7) & bit7);
    let x1 = (t0.rsh(1) & bit0) | (t1 & bit1) | (t2.lsh(1) & bit2) | (t3.lsh(2) & bit3) |
        (t4.lsh(3) & bit4) | (t5.lsh(4) & bit5) | (t6.lsh(5) & bit6) | (t7.lsh(6) & bit7);
    let x2 = (t0.rsh(2) & bit0) | (t1.rsh(1) & bit1) | (t2 & bit2) | (t3.lsh(1) & bit3) |
        (t4.lsh(2) & bit4) | (t5.lsh(3) & bit5) | (t6.lsh(4) & bit6) | (t7.lsh(5) & bit7);
    let x3 = (t0.rsh(3) & bit0) | (t1.rsh(2) & bit1) | (t2.rsh(1) & bit2) | (t3 & bit3) |
        (t4.lsh(1) & bit4) | (t5.lsh(2) & bit5) | (t6.lsh(3) & bit6) | (t7.lsh(4) & bit7);
    let x4 = (t0.rsh(4) & bit0) | (t1.rsh(3) & bit1) | (t2.rsh(2) & bit2) | (t3.rsh(1) & bit3) |
        (t4 & bit4) | (t5.lsh(1) & bit5) | (t6.lsh(2) & bit6) | (t7.lsh(3) & bit7);
    let x5 = (t0.rsh(5) & bit0) | (t1.rsh(4) & bit1) | (t2.rsh(3) & bit2) | (t3.rsh(2) & bit3) |
        (t4.rsh(1) & bit4) | (t5 & bit5) | (t6.lsh(1) & bit6) | (t7.lsh(2) & bit7);
    let x6 = (t0.rsh(6) & bit0) | (t1.rsh(5) & bit1) | (t2.rsh(4) & bit2) | (t3.rsh(3) & bit3) |
        (t4.rsh(2) & bit4) | (t5.rsh(1) & bit5) | (t6 & bit6) | (t7.lsh(1) & bit7);
    let x7 = (t0.rsh(7) & bit0) | (t1.rsh(6) & bit1) | (t2.rsh(5) & bit2) | (t3.rsh(4) & bit3) |
        (t4.rsh(3) & bit4) | (t5.rsh(2) & bit5) | (t6.rsh(1) & bit6) | (t7 & bit7);

    fn write_row_major(block: &u32x4, output: &mut [u8]) {
        let u32x4(a0, a1, a2, a3) = *block;
        output[0] = a0 as u8;
        output[1] = a1 as u8;
        output[2] = a2 as u8;
        output[3] = a3 as u8;
        output[4] = (a0 >> 8) as u8;
        output[5] = (a1 >> 8) as u8;
        output[6] = (a2 >> 8) as u8;
        output[7] = (a3 >> 8) as u8;
        output[8] = (a0 >> 16) as u8;
        output[9] = (a1 >> 16) as u8;
        output[10] = (a2 >> 16) as u8;
        output[11] = (a3 >> 16) as u8;
        output[12] = (a0 >> 24) as u8;
        output[13] = (a1 >> 24) as u8;
        output[14] = (a2 >> 24) as u8;
        output[15] = (a3 >> 24) as u8;
    }

    write_row_major(&x0, output[mut 0..16]);
    write_row_major(&x1, output[mut 16..32]);
    write_row_major(&x2, output[mut 32..48]);
    write_row_major(&x3, output[mut 48..64]);
    write_row_major(&x4, output[mut 64..80]);
    write_row_major(&x5, output[mut 80..96]);
    write_row_major(&x6, output[mut 96..112]);
    write_row_major(&x7, output[mut 112..128])
}

// The Gf2Ops, Gf4Ops, and Gf8Ops traits specify the functions needed to calculate the AES S-Box
// values. This particuar implementation of those S-Box values is taken from [7], so that is where
// to look for details on how all that all works. This includes the transformations matrices defined
// below for the change_basis operation on the u32 and u32x4 types.

// Operations in GF(2^2) using normal basis (Omega^2,Omega)
trait Gf2Ops {
    // multiply
    fn mul(&self, y: &Self) -> Self;

    // scale by N = Omega^2
    fn scl_n(&self) -> Self;

    // scale by N^2 = Omega
    fn scl_n2(&self) -> Self;

    // square
    fn sq(&self) -> Self;

    // Same as sqaure
    fn inv(&self) -> Self;
}

impl <T: BitXor<T, T> + BitAnd<T, T> + Clone> Gf2Ops for Bs2State<T> {
    fn mul(&self, y: &Bs2State<T>) -> Bs2State<T> {
        let (b, a) = self.split();
        let (d, c) = y.split();
        let e = (a ^ b) & (c ^ d);
        let p = (a & c) ^ e;
        let q = (b & d) ^ e;
        return Bs2State(q, p);
    }

    fn scl_n(&self) -> Bs2State<T> {
        let (b, a) = self.split();
        let q = a ^ b;
        return Bs2State(q, b);
    }

    fn scl_n2(&self) -> Bs2State<T> {
        let (b, a) = self.split();
        let p = a ^ b;
        let q = a;
        return Bs2State(q, p);
    }

    fn sq(&self) -> Bs2State<T> {
        let (b, a) = self.split();
        return Bs2State(a, b);
    }

    fn inv(&self) -> Bs2State<T> {
        return self.sq();
    }
}

// Operations in GF(2^4) using normal basis (alpha^8,alpha^2)
trait Gf4Ops {
    // multiply
    fn mul(&self, y: &Self) -> Self;

    // square & scale by nu
    // nu = beta^8 = N^2*alpha^2, N = w^2
    fn sq_scl(&self) -> Self;

    // inverse
    fn inv(&self) -> Self;
}

impl <T: BitXor<T, T> + BitAnd<T, T> + Clone> Gf4Ops for Bs4State<T> {
    fn mul(&self, y: &Bs4State<T>) -> Bs4State<T> {
        let (b, a) = self.split();
        let (d, c) = y.split();
        let f = c.xor(&d);
        let e = a.xor(&b).mul(&f).scl_n();
        let p = a.mul(&c).xor(&e);
        let q = b.mul(&d).xor(&e);
        return q.join(&p);
    }

    fn sq_scl(&self) -> Bs4State<T> {
        let (b, a) = self.split();
        let p = a.xor(&b).sq();
        let q = b.sq().scl_n2();
        return q.join(&p);
    }

    fn inv(&self) -> Bs4State<T> {
        let (b, a) = self.split();
        let c = a.xor(&b).sq().scl_n();
        let d = a.mul(&b);
        let e = c.xor(&d).inv();
        let p = e.mul(&b);
        let q = e.mul(&a);
        return q.join(&p);
    }
}

// Operations in GF(2^8) using normal basis (d^16,d)
trait Gf8Ops<T> {
    // inverse
    fn inv(&self) -> Self;

    // change the basis using the provided array
    fn change_basis(&self, arr: &[[T, ..8], ..8]) -> Self;
}

impl <T: BitXor<T, T> + BitAnd<T, T> + Clone + Default> Gf8Ops<T> for Bs8State<T> {
    fn inv(&self) -> Bs8State<T> {
        let (b, a) = self.split();
        let c = a.xor(&b).sq_scl();
        let d = a.mul(&b);
        let e = c.xor(&d).inv();
        let p = e.mul(&b);
        let q = e.mul(&a);
        return q.join(&p);
    }

    fn change_basis(&self, arr: &[[T, ..8], ..8]) -> Bs8State<T> {
        let Bs8State(ref x0, ref x1, ref x2, ref x3, ref x4, ref x5, ref x6, ref x7) = *self;

        let mut x0_out: T = Default::default();
        let mut x1_out: T = Default::default();
        let mut x2_out: T = Default::default();
        let mut x3_out: T = Default::default();
        let mut x4_out: T = Default::default();
        let mut x5_out: T = Default::default();
        let mut x6_out: T = Default::default();
        let mut x7_out: T = Default::default();

        /*
        // FIXME - #XXXX: This is prettier, but ICEs

        macro_rules! helper( ($x:ident, $idx:expr) => (
                {
                    x0_out = x0_out ^ (*($x) & arr[7 - $idx][0]);
                    x1_out = x1_out ^ (*($x) & arr[7 - $idx][1]);
                    x2_out = x2_out ^ (*($x) & arr[7 - $idx][2]);
                    x3_out = x3_out ^ (*($x) & arr[7 - $idx][3]);
                    x4_out = x4_out ^ (*($x) & arr[7 - $idx][4]);
                    x5_out = x5_out ^ (*($x) & arr[7 - $idx][5]);
                    x6_out = x6_out ^ (*($x) & arr[7 - $idx][6]);
                    x7_out = x7_out ^ (*($x) & arr[7 - $idx][7]);
                }
            )
        )

        helper!(x0, 0);
        helper!(x1, 1);
        helper!(x2, 2);
        helper!(x3, 3);
        helper!(x4, 4);
        helper!(x5, 5);
        helper!(x6, 6);
        helper!(x7, 7);
        */

        x0_out = x0_out ^ (*x0 & arr[7][0]);
        x1_out = x1_out ^ (*x0 & arr[7][1]);
        x2_out = x2_out ^ (*x0 & arr[7][2]);
        x3_out = x3_out ^ (*x0 & arr[7][3]);
        x4_out = x4_out ^ (*x0 & arr[7][4]);
        x5_out = x5_out ^ (*x0 & arr[7][5]);
        x6_out = x6_out ^ (*x0 & arr[7][6]);
        x7_out = x7_out ^ (*x0 & arr[7][7]);

        x0_out = x0_out ^ (*x1 & arr[6][0]);
        x1_out = x1_out ^ (*x1 & arr[6][1]);
        x2_out = x2_out ^ (*x1 & arr[6][2]);
        x3_out = x3_out ^ (*x1 & arr[6][3]);
        x4_out = x4_out ^ (*x1 & arr[6][4]);
        x5_out = x5_out ^ (*x1 & arr[6][5]);
        x6_out = x6_out ^ (*x1 & arr[6][6]);
        x7_out = x7_out ^ (*x1 & arr[6][7]);

        x0_out = x0_out ^ (*x2 & arr[5][0]);
        x1_out = x1_out ^ (*x2 & arr[5][1]);
        x2_out = x2_out ^ (*x2 & arr[5][2]);
        x3_out = x3_out ^ (*x2 & arr[5][3]);
        x4_out = x4_out ^ (*x2 & arr[5][4]);
        x5_out = x5_out ^ (*x2 & arr[5][5]);
        x6_out = x6_out ^ (*x2 & arr[5][6]);
        x7_out = x7_out ^ (*x2 & arr[5][7]);

        x0_out = x0_out ^ (*x3 & arr[4][0]);
        x1_out = x1_out ^ (*x3 & arr[4][1]);
        x2_out = x2_out ^ (*x3 & arr[4][2]);
        x3_out = x3_out ^ (*x3 & arr[4][3]);
        x4_out = x4_out ^ (*x3 & arr[4][4]);
        x5_out = x5_out ^ (*x3 & arr[4][5]);
        x6_out = x6_out ^ (*x3 & arr[4][6]);
        x7_out = x7_out ^ (*x3 & arr[4][7]);

        x0_out = x0_out ^ (*x4 & arr[3][0]);
        x1_out = x1_out ^ (*x4 & arr[3][1]);
        x2_out = x2_out ^ (*x4 & arr[3][2]);
        x3_out = x3_out ^ (*x4 & arr[3][3]);
        x4_out = x4_out ^ (*x4 & arr[3][4]);
        x5_out = x5_out ^ (*x4 & arr[3][5]);
        x6_out = x6_out ^ (*x4 & arr[3][6]);
        x7_out = x7_out ^ (*x4 & arr[3][7]);

        x0_out = x0_out ^ (*x5 & arr[2][0]);
        x1_out = x1_out ^ (*x5 & arr[2][1]);
        x2_out = x2_out ^ (*x5 & arr[2][2]);
        x3_out = x3_out ^ (*x5 & arr[2][3]);
        x4_out = x4_out ^ (*x5 & arr[2][4]);
        x5_out = x5_out ^ (*x5 & arr[2][5]);
        x6_out = x6_out ^ (*x5 & arr[2][6]);
        x7_out = x7_out ^ (*x5 & arr[2][7]);

        x0_out = x0_out ^ (*x6 & arr[1][0]);
        x1_out = x1_out ^ (*x6 & arr[1][1]);
        x2_out = x2_out ^ (*x6 & arr[1][2]);
        x3_out = x3_out ^ (*x6 & arr[1][3]);
        x4_out = x4_out ^ (*x6 & arr[1][4]);
        x5_out = x5_out ^ (*x6 & arr[1][5]);
        x6_out = x6_out ^ (*x6 & arr[1][6]);
        x7_out = x7_out ^ (*x6 & arr[1][7]);

        x0_out = x0_out ^ (*x7 & arr[0][0]);
        x1_out = x1_out ^ (*x7 & arr[0][1]);
        x2_out = x2_out ^ (*x7 & arr[0][2]);
        x3_out = x3_out ^ (*x7 & arr[0][3]);
        x4_out = x4_out ^ (*x7 & arr[0][4]);
        x5_out = x5_out ^ (*x7 & arr[0][5]);
        x6_out = x6_out ^ (*x7 & arr[0][6]);
        x7_out = x7_out ^ (*x7 & arr[0][7]);

        return Bs8State(x0_out, x1_out, x2_out, x3_out, x4_out, x5_out, x6_out, x7_out);
    }
}

impl <T: AesBitValueOps + 'static> AesOps for Bs8State<T> {
    // This is necessary to work around RUST #17906 and should
    // be removed once that issue is resolved.
    #[inline(never)]
    fn sub_bytes(&self) -> Bs8State<T> {
        let nb: Bs8State<T> = self.change_basis(AesBitValueOps::a2x());
        let inv = nb.inv();
        let nb2: Bs8State<T> = inv.change_basis(AesBitValueOps::x2s());
        let x63: Bs8State<T> = AesBitValueOps::x63();
        return nb2.xor(&x63);
    }

    // This is necessary to work around RUST #17906 and should
    // be removed once that issue is resolved.
    #[inline(never)]
    fn inv_sub_bytes(&self) -> Bs8State<T> {
        let x63: Bs8State<T> = AesBitValueOps::x63();
        let t = self.xor(&x63);
        let nb: Bs8State<T> = t.change_basis(AesBitValueOps::s2x());
	let inv = nb.inv();
        return inv.change_basis(AesBitValueOps::x2a());
    }

    fn shift_rows(&self) -> Bs8State<T> {
        let Bs8State(ref x0, ref x1, ref x2, ref x3, ref x4, ref x5, ref x6, ref x7) = *self;
        return Bs8State(
            x0.shift_row(),
            x1.shift_row(),
            x2.shift_row(),
            x3.shift_row(),
            x4.shift_row(),
            x5.shift_row(),
            x6.shift_row(),
            x7.shift_row());
    }

    fn inv_shift_rows(&self) -> Bs8State<T> {
        let Bs8State(ref x0, ref x1, ref x2, ref x3, ref x4, ref x5, ref x6, ref x7) = *self;
        return Bs8State(
            x0.inv_shift_row(),
            x1.inv_shift_row(),
            x2.inv_shift_row(),
            x3.inv_shift_row(),
            x4.inv_shift_row(),
            x5.inv_shift_row(),
            x6.inv_shift_row(),
            x7.inv_shift_row());
    }

    // Formula from [5]
    fn mix_columns(&self) -> Bs8State<T> {
        let Bs8State(ref x0, ref x1, ref x2, ref x3, ref x4, ref x5, ref x6, ref x7) = *self;

        let x0out = *x7 ^ x7.ror1() ^ x0.ror1() ^ (*x0 ^ x0.ror1()).ror2();
        let x1out = *x0 ^ x0.ror1() ^ *x7 ^ x7.ror1() ^ x1.ror1() ^ (*x1 ^ x1.ror1()).ror2();
        let x2out = *x1 ^ x1.ror1() ^ x2.ror1() ^ (*x2 ^ x2.ror1()).ror2();
        let x3out = *x2 ^ x2.ror1() ^ *x7 ^ x7.ror1() ^ x3.ror1() ^ (*x3 ^ x3.ror1()).ror2();
        let x4out = *x3 ^ x3.ror1() ^ *x7 ^ x7.ror1() ^ x4.ror1() ^ (*x4 ^ x4.ror1()).ror2();
        let x5out = *x4 ^ x4.ror1() ^ x5.ror1() ^ (*x5 ^ x5.ror1()).ror2();
        let x6out = *x5 ^ x5.ror1() ^ x6.ror1() ^ (*x6 ^ x6.ror1()).ror2();
        let x7out = *x6 ^ x6.ror1() ^ x7.ror1() ^ (*x7 ^ x7.ror1()).ror2();

        return Bs8State(x0out, x1out, x2out, x3out, x4out, x5out, x6out, x7out);
    }

    // Formula from [6]
    fn inv_mix_columns(&self) -> Bs8State<T> {
        let Bs8State(ref x0, ref x1, ref x2, ref x3, ref x4, ref x5, ref x6, ref x7) = *self;

        let x0out = *x5 ^ *x6 ^ *x7 ^
            (*x5 ^ *x7 ^ *x0).ror1() ^
            (*x0 ^ *x5 ^ *x6).ror2() ^
            (*x5 ^ *x0).ror3();
        let x1out = *x5 ^ *x0 ^
            (*x6 ^ *x5 ^ *x0 ^ *x7 ^ *x1).ror1() ^
            (*x1 ^ *x7 ^ *x5).ror2() ^
            (*x6 ^ *x5 ^ *x1).ror3();
        let x2out = *x6 ^ *x0 ^ *x1 ^
            (*x7 ^ *x6 ^ *x1 ^ *x2).ror1() ^
            (*x0 ^ *x2 ^ *x6).ror2() ^
            (*x7 ^ *x6 ^ *x2).ror3();
        let x3out = *x0 ^ *x5 ^ *x1 ^ *x6 ^ *x2 ^
            (*x0 ^ *x5 ^ *x2 ^ *x3).ror1() ^
            (*x0 ^ *x1 ^ *x3 ^ *x5 ^ *x6 ^ *x7).ror2() ^
            (*x0 ^ *x5 ^ *x7 ^ *x3).ror3();
        let x4out = *x1 ^ *x5 ^ *x2 ^ *x3 ^
            (*x1 ^ *x6 ^ *x5 ^ *x3 ^ *x7 ^ *x4).ror1() ^
            (*x1 ^ *x2 ^ *x4 ^ *x5 ^ *x7).ror2() ^
            (*x1 ^ *x5 ^ *x6 ^ *x4).ror3();
        let x5out = *x2 ^ *x6 ^ *x3 ^ *x4 ^
            (*x2 ^ *x7 ^ *x6 ^ *x4 ^ *x5).ror1() ^
            (*x2 ^ *x3 ^ *x5 ^ *x6).ror2() ^
            (*x2 ^ *x6 ^ *x7 ^ *x5).ror3();
        let x6out =  *x3 ^ *x7 ^ *x4 ^ *x5 ^
            (*x3 ^ *x7 ^ *x5 ^ *x6).ror1() ^
            (*x3 ^ *x4 ^ *x6 ^ *x7).ror2() ^
            (*x3 ^ *x7 ^ *x6).ror3();
        let x7out = *x4 ^ *x5 ^ *x6 ^
            (*x4 ^ *x6 ^ *x7).ror1() ^
            (*x4 ^ *x5 ^ *x7).ror2() ^
            (*x4 ^ *x7).ror3();

        Bs8State(x0out, x1out, x2out, x3out, x4out, x5out, x6out, x7out)
    }

    fn add_round_key(&self, rk: &Bs8State<T>) -> Bs8State<T> {
        return self.xor(rk);
    }
}

trait AesBitValueOps: BitXor<Self, Self> + BitAnd<Self, Self> + Clone + Default {
    fn a2x() -> &'static [[Self, ..8], ..8];
    fn x2s() -> &'static [[Self, ..8], ..8];
    fn s2x() -> &'static [[Self, ..8], ..8];
    fn x2a() -> &'static [[Self, ..8], ..8];
    fn x63() -> Bs8State<Self>;

    fn shift_row(&self) -> Self;
    fn inv_shift_row(&self) -> Self;
    fn ror1(&self) -> Self;
    fn ror2(&self) -> Self;
    fn ror3(&self) -> Self;
}

// Arrays to convert to and from a polynomial basis and a normal basis. The affine transformation
// step is included in these matrices as well, so that doesn't have to be done seperately.
static A2X_U32: [[u32, ..8], ..8] = [
    [ 0,  0,  0, -1, -1,  0,  0, -1],
    [-1, -1,  0,  0, -1, -1, -1, -1],
    [ 0, -1,  0,  0, -1, -1, -1, -1],
    [ 0,  0,  0, -1,  0,  0, -1,  0],
    [-1,  0,  0, -1,  0,  0,  0,  0],
    [-1,  0,  0,  0,  0,  0,  0, -1],
    [-1,  0,  0, -1,  0, -1,  0, -1],
    [-1, -1, -1, -1, -1, -1, -1, -1]
];

static X2A_U32: [[u32, ..8], ..8] = [
    [ 0,  0, -1,  0,  0, -1, -1,  0],
    [ 0,  0,  0, -1, -1, -1, -1,  0],
    [ 0, -1, -1, -1,  0, -1, -1,  0],
    [ 0,  0, -1, -1,  0,  0,  0, -1],
    [ 0,  0,  0, -1,  0, -1, -1,  0],
    [-1,  0,  0, -1,  0, -1,  0,  0],
    [ 0, -1, -1, -1, -1,  0, -1, -1],
    [ 0,  0,  0,  0,  0, -1, -1,  0],
];

static X2S_U32: [[u32, ..8], ..8] = [
    [ 0,  0,  0, -1, -1,  0, -1,  0],
    [-1,  0, -1, -1,  0, -1,  0,  0],
    [ 0, -1, -1, -1, -1,  0,  0, -1],
    [-1, -1,  0, -1,  0,  0,  0,  0],
    [ 0,  0, -1, -1, -1,  0, -1, -1],
    [ 0,  0, -1,  0,  0,  0,  0,  0],
    [-1, -1,  0,  0,  0,  0,  0,  0],
    [ 0,  0, -1,  0,  0, -1,  0,  0],
];

static S2X_U32: [[u32, ..8], ..8] = [
    [0, 0 ,  -1, -1,  0,  0,  0, -1],
    [-1,  0,  0, -1, -1, -1, -1,  0],
    [-1,  0, -1,  0,  0,  0,  0,  0],
    [-1, -1,  0, -1,  0, -1, -1, -1],
    [0,  -1,  0,  0, -1,  0,  0,  0],
    [0,   0, -1,  0,  0,  0,  0,  0],
    [-1,  0,  0,  0, -1,  0, -1,  0],
    [-1, -1,  0,  0, -1,  0, -1,  0],
];

impl AesBitValueOps for u32 {
    fn a2x() -> &'static [[u32, ..8], ..8] { &A2X_U32 }
    fn x2s() -> &'static [[u32, ..8], ..8] { &X2S_U32 }
    fn s2x() -> &'static [[u32, ..8], ..8] { &S2X_U32 }
    fn x2a() -> &'static [[u32, ..8], ..8] { &X2A_U32 }
    fn x63() -> Bs8State<u32> { Bs8State(-1, -1, 0, 0, 0, -1, -1, 0) }

    fn shift_row(&self) -> u32 {
        // first 4 bits represent first row - don't shift
        (*self & 0x000f) |
        // next 4 bits represent 2nd row - left rotate 1 bit
        ((*self & 0x00e0) >> 1) | ((*self & 0x0010) << 3) |
        // next 4 bits represent 3rd row - left rotate 2 bits
        ((*self & 0x0c00) >> 2) | ((*self & 0x0300) << 2) |
        // next 4 bits represent 4th row - left rotate 3 bits
        ((*self & 0x8000) >> 3) | ((*self & 0x7000) << 1)
    }

    fn inv_shift_row(&self) -> u32 {
        // first 4 bits represent first row - don't shift
        (*self & 0x000f) |
        // next 4 bits represent 2nd row - right rotate 1 bit
        ((*self & 0x0080) >> 3) | ((*self & 0x0070) << 1) |
        // next 4 bits represent 3rd row - right rotate 2 bits
        ((*self & 0x0c00) >> 2) | ((*self & 0x0300) << 2) |
        // next 4 bits represent 4th row - right rotate 3 bits
        ((*self & 0xe000) >> 1) | ((*self & 0x1000) << 3)
    }

    fn ror1(&self) -> u32 {
        ((*self >> 4) & 0x0fff) | (*self << 12)
    }

    fn ror2(&self) -> u32 {
        ((*self >> 8) & 0x00ff) | (*self << 8)
    }

    fn ror3(&self) -> u32 {
        ((*self >> 12) & 0x000f) | (*self << 4)
    }
}

impl u32x4 {
    fn lsh(&self, s: uint) -> u32x4 {
        let u32x4(a0, a1, a2, a3) = *self;
        return u32x4(
            a0 << s,
            (a1 << s) | (a0 >> (32 - s)),
            (a2 << s) | (a1 >> (32 - s)),
            (a3 << s) | (a2 >> (32 - s)));
    }

    fn rsh(&self, s: uint) -> u32x4 {
        let u32x4(a0, a1, a2, a3) = *self;
        return u32x4(
            (a0 >> s) | (a1 << (32 - s)),
            (a1 >> s) | (a2 << (32 - s)),
            (a2 >> s) | (a3 << (32 - s)),
            a3 >> s);
    }
}

impl BitXor<u32x4, u32x4> for u32x4 {
    fn bitxor(&self, rhs: &u32x4) -> u32x4 {
        return *self ^ *rhs;
    }
}

impl BitAnd<u32x4, u32x4> for u32x4 {
    fn bitand(&self, rhs: &u32x4) -> u32x4 {
        return *self & *rhs;
    }
}

impl Default for u32x4 {
    fn default() -> u32x4 {
        return u32x4(0, 0, 0, 0);
    }
}

// Arrays to convert to and from a polynomial basis and a normal basis. The affine transformation
// step is included in these matrices as well, so that doesn't have to be done seperately.
static A2X_U32X4: [[u32x4, ..8], ..8] = [
    [o!(), o!(), o!(), x!(), x!(), o!(), o!(), x!()],
    [x!(), x!(), o!(), o!(), x!(), x!(), x!(), x!()],
    [o!(), x!(), o!(), o!(), x!(), x!(), x!(), x!()],
    [o!(), o!(), o!(), x!(), o!(), o!(), x!(), o!()],
    [x!(), o!(), o!(), x!(), o!(), o!(), o!(), o!()],
    [x!(), o!(), o!(), o!(), o!(), o!(), o!(), x!()],
    [x!(), o!(), o!(), x!(), o!(), x!(), o!(), x!()],
    [x!(), x!(), x!(), x!(), x!(), x!(), x!(), x!()]
];

static X2A_U32X4: [[u32x4, ..8], ..8] = [
    [o!(), o!(), x!(), o!(), o!(), x!(), x!(), o!()],
    [o!(), o!(), o!(), x!(), x!(), x!(), x!(), o!()],
    [o!(), x!(), x!(), x!(), o!(), x!(), x!(), o!()],
    [o!(), o!(), x!(), x!(), o!(), o!(), o!(), x!()],
    [o!(), o!(), o!(), x!(), o!(), x!(), x!(), o!()],
    [x!(), o!(), o!(), x!(), o!(), x!(), o!(), o!()],
    [o!(), x!(), x!(), x!(), x!(), o!(), x!(), x!()],
    [o!(), o!(), o!(), o!(), o!(), x!(), x!(), o!()],
];

static X2S_U32X4: [[u32x4, ..8], ..8] = [
    [o!(), o!(), o!(), x!(), x!(), o!(), x!(), o!()],
    [x!(), o!(), x!(), x!(), o!(), x!(), o!(), o!()],
    [o!(), x!(), x!(), x!(), x!(), o!(), o!(), x!()],
    [x!(), x!(), o!(), x!(), o!(), o!(), o!(), o!()],
    [o!(), o!(), x!(), x!(), x!(), o!(), x!(), x!()],
    [o!(), o!(), x!(), o!(), o!(), o!(), o!(), o!()],
    [x!(), x!(), o!(), o!(), o!(), o!(), o!(), o!()],
    [o!(), o!(), x!(), o!(), o!(), x!(), o!(), o!()],
];

static S2X_U32X4: [[u32x4, ..8], ..8] = [
    [o!(), o!(), x!(), x!(), o!(), o!(), o!(), x!()],
    [x!(), o!(), o!(), x!(), x!(), x!(), x!(), o!()],
    [x!(), o!(), x!(), o!(), o!(), o!(), o!(), o!()],
    [x!(), x!(), o!(), x!(), o!(), x!(), x!(), x!()],
    [o!(), x!(), o!(), o!(), x!(), o!(), o!(), o!()],
    [o!(), o!(), x!(), o!(), o!(), o!(), o!(), o!()],
    [x!(), o!(), o!(), o!(), x!(), o!(), x!(), o!()],
    [x!(), x!(), o!(), o!(), x!(), o!(), x!(), o!()],
];

impl AesBitValueOps for u32x4 {
    fn a2x() -> &'static [[u32x4, ..8], ..8] { &A2X_U32X4 }
    fn x2s() -> &'static [[u32x4, ..8], ..8] { &X2S_U32X4 }
    fn s2x() -> &'static [[u32x4, ..8], ..8] { &S2X_U32X4 }
    fn x2a() -> &'static [[u32x4, ..8], ..8] { &X2A_U32X4 }
    fn x63() -> Bs8State<u32x4> { Bs8State(x!(), x!(), o!(), o!(), o!(), x!(), x!(), o!()) }

    fn shift_row(&self) -> u32x4 {
        let u32x4(a0, a1, a2, a3) = *self;
        return u32x4(a0, a1 >> 8 | a1 << 24, a2 >> 16 | a2 << 16, a3 >> 24 | a3 << 8);
    }

    fn inv_shift_row(&self) -> u32x4 {
        let u32x4(a0, a1, a2, a3) = *self;
        return u32x4(a0, a1 >> 24 | a1 << 8, a2 >> 16 | a2 << 16, a3 >> 8 | a3 << 24);
    }

    fn ror1(&self) -> u32x4 {
        let u32x4(a0, a1, a2, a3) = *self;
        return u32x4(a1, a2, a3, a0);
    }

    fn ror2(&self) -> u32x4 {
        let u32x4(a0, a1, a2, a3) = *self;
        return u32x4(a2, a3, a0, a1);
    }

    fn ror3(&self) -> u32x4 {
        let u32x4(a0, a1, a2, a3) = *self;
        return u32x4(a3, a0, a1, a2);
    }
}
