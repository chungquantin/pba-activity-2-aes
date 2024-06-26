//! In Module 1, we discussed Block ciphers like AES. Block ciphers have a fixed length input.
//! Real wold data that we wish to encrypt _may_ be exactly the right length, but is probably not.
//! When your data is too short, you can simply pad it up to the correct length.
//! When your data is too long, you have some options.
//!
//! In this exercise, we will explore a few of the common ways that large pieces of data can be
//! broken up and combined in order to encrypt it with a fixed-length block cipher.
//!
//! WARNING: ECB MODE IS NOT SECURE.
//! Seriously, ECB is NOT secure. Don't use it irl. We are implementing it here to understand _why_
//! it is not secure and make the point that the most straight-forward approach isn't always the
//! best, and can sometimes be trivially broken.

use aes::{
    cipher::{generic_array::GenericArray, BlockDecrypt, BlockEncrypt, KeyInit},
    Aes128,
};
use rand::Rng;
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};

///We're using AES 128 which has 16-byte (128 bit) blocks.
const BLOCK_SIZE: usize = 16;

fn main() {
    todo!("Maybe this should be a library crate. TBD");
}

#[test]
fn test_ecb() {
    let plain_text = "Polkadot Blockchain Academy";
    let plain_text_bytes = plain_text.as_bytes().to_vec();

    let mut rng = rand::thread_rng();
    let key: [u8; BLOCK_SIZE] = array_init::array_init(|_| rng.gen::<u8>());
    let encrypted = ecb_encrypt(plain_text_bytes.clone(), key);
    let expected = ecb_decrypt(encrypted, key);

    let expected_str = String::from_utf8(expected[..plain_text.len()].to_vec()).unwrap();
    assert!(expected_str == plain_text);
}

#[test]
fn test_cbc() {
    let plain_text = "Polkadot Blockchain Academy";
    let plain_text_bytes = plain_text.as_bytes().to_vec();

    let mut rng = rand::thread_rng();
    let key: [u8; BLOCK_SIZE] = array_init::array_init(|_| rng.gen::<u8>());
    let encrypted = cbc_encrypt(plain_text_bytes.clone(), key);
    let expected = cbc_decrypt(encrypted.clone(), key);

    println!("{:?} == {:?}", encrypted, expected);
    let expected_str = String::from_utf8(expected).unwrap();
    assert!(expected_str == plain_text);
}

#[test]
fn test_ctr() {
    let plain_text = "Polkadot Blockchain Academy";
    let plain_text_bytes = plain_text.as_bytes().to_vec();

    let mut rng = rand::thread_rng();
    let key: [u8; BLOCK_SIZE] = array_init::array_init(|_| rng.gen::<u8>());
    let encrypted = ctr_encrypt(plain_text_bytes.clone(), key);
    let expected = ctr_decrypt(encrypted.clone(), key);

    let expected_str = String::from_utf8(expected).unwrap();
    assert!(expected_str == plain_text);
}

#[test]
fn test_unpad() {
    let init_data = vec![1, 2, 4, 5];

    let data = pad(init_data.clone());
    let result = un_pad(data.clone());

    assert_eq!(init_data, result);
}

#[test]
fn test_group() {
    let data: Vec<u8> = vec![
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
        26, 27, 28, 29, 30, 31, 32,
    ];
    assert!(data.len() % BLOCK_SIZE == 0);
    let grouped_data: Vec<[u8; BLOCK_SIZE]> = group(data);
    let expected = vec![
        [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
        [
            17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32,
        ],
    ];
    assert_eq!(grouped_data, expected);
    assert_eq!(grouped_data.len(), 2);
    for block in expected {
        assert_eq!(block.len(), BLOCK_SIZE);
    }
}

#[test]
fn test_ungroup() {
    let data: Vec<[u8; BLOCK_SIZE]> = vec![
        [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
        [
            17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32,
        ],
    ];
    let grouped_data: Vec<u8> = un_group(data);
    let expected = vec![
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
        26, 27, 28, 29, 30, 31, 32,
    ];
    assert_eq!(grouped_data, expected);
}
/// Simple AES encryption
/// Helper function to make the core AES block cipher easier to understand.
fn aes_encrypt(data: [u8; BLOCK_SIZE], key: &[u8; BLOCK_SIZE]) -> [u8; BLOCK_SIZE] {
    // Convert the inputs to the necessary data type
    let mut block = GenericArray::from(data);
    let key = GenericArray::from(*key);

    let cipher = Aes128::new(&key);

    cipher.encrypt_block(&mut block);

    block.into()
}

/// Simple AES encryption
/// Helper function to make the core AES block cipher easier to understand.
fn aes_decrypt(data: [u8; BLOCK_SIZE], key: &[u8; BLOCK_SIZE]) -> [u8; BLOCK_SIZE] {
    // Convert the inputs to the necessary data type
    let mut block = GenericArray::from(data);
    let key = GenericArray::from(*key);

    let cipher = Aes128::new(&key);

    cipher.decrypt_block(&mut block);

    block.into()
}

/// Before we can begin encrypting our raw data, we need it to be a multiple of the
/// block length which is 16 bytes (128 bits) in AES128.
///
/// The padding algorithm here is actually not trivial. The trouble is that if we just
/// naively throw a bunch of zeros on the end, there is no way to know, later, whether
/// those zeros are padding, or part of the message, or some of each.
///
/// The scheme works like this. If the data is not a multiple of the block length,  we
/// compute how many pad bytes we need, and then write that number into the last several bytes.
/// Later we look at the last byte, and remove that number of bytes.
///
/// But if the data _is_ a multiple of the block length, then we have a problem. We don't want
/// to later look at the last byte and remove part of the data. Instead, in this case, we add
/// another entire block containing the block length in each byte. In our case,
/// [16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16, 16]
fn pad(mut data: Vec<u8>) -> Vec<u8> {
    // When twe have a multiple the second term is 0
    let number_pad_bytes = BLOCK_SIZE - data.len() % BLOCK_SIZE;

    for _ in 0..number_pad_bytes {
        data.push(number_pad_bytes as u8);
    }

    data
}

/// Groups the data into BLOCK_SIZE blocks. Assumes the data is already
/// a multiple of the block size. If this is not the case, call `pad` first.
fn group(mut data: Vec<u8>) -> Vec<[u8; BLOCK_SIZE]> {
    if data.len() % BLOCK_SIZE != 0 {
        data = pad(data)
    };

    let mut blocks = Vec::new();
    let mut i = 0;
    while i < data.len() {
        let mut block: [u8; BLOCK_SIZE] = Default::default();
        block.copy_from_slice(&data[i..i + BLOCK_SIZE]);
        blocks.push(block);

        i += BLOCK_SIZE;
    }

    blocks
}

/// Does the opposite of the group function
fn un_group(blocks: Vec<[u8; BLOCK_SIZE]>) -> Vec<u8> {
    let mut ungrouped: Vec<u8> = vec![];
    for block in blocks {
        for data in block.to_vec() {
            ungrouped.push(data);
        }
    }
    return ungrouped;
}

/// Does the opposite of the pad function.
fn un_pad(data: Vec<u8>) -> Vec<u8> {
    let mut data = data;
    let last_byte = *data.last().unwrap() as usize;

    match last_byte == BLOCK_SIZE {
        true => data.truncate(data.len() - BLOCK_SIZE),
        false => data.truncate(data.len() - last_byte),
    }

    data
}

/// The first mode we will implement is the Electronic Code Book, or ECB mode.
/// Warning: THIS MODE IS NOT SECURE!!!!
///
/// This is probably the first thing you think of when considering how to encrypt
/// large data. In this mode we simply encrypt each block of data under the same key.
/// One good thing about this mode is that it is parallelizable. But to see why it is
/// insecure look at: https://www.ubiqsecurity.com/wp-content/uploads/2022/02/ECB2.png
fn ecb_encrypt(plain_text: Vec<u8>, key: [u8; 16]) -> Vec<u8> {
    // Pad the data to the correct length
    let padded_text = pad(plain_text);

    // Group the data into blocks
    let blocks = group(padded_text);

    // Encrypt each block
    let mut cipher_text = Vec::new();

    for block in blocks {
        let encrypted_block = aes_encrypt(block, &key);
        cipher_text.extend_from_slice(&encrypted_block);
    }

    cipher_text
}

/// Opposite of ecb_encrypt.
fn ecb_decrypt(cipher_text: Vec<u8>, key: [u8; BLOCK_SIZE]) -> Vec<u8> {
    // Group the data into blocks
    let blocks = group(cipher_text);

    // Decrypt each block
    let mut plain_text = Vec::new();

    for block in blocks {
        let decrypted_block = aes_decrypt(block, &key);
        plain_text.extend_from_slice(&decrypted_block);
    }

    // Unpad the data
    un_pad(plain_text)
}

/// The next mode, which you can implement on your own is cipherblock chaining.
/// This mode actually is secure, and it often used in real world applications.
///
/// In this mode, the ciphertext from the first block is XORed with the
/// plaintext of the next block before it is encrypted.
///
/// For more information, and a very clear diagram,
/// see https://de.wikipedia.org/wiki/Cipher_Block_Chaining_Mode
///
/// You will need to generate a random initialization vector (IV) to encrypt the
/// very first block because it doesn't have a previous block. Typically this IV
/// is inserted as the first block of ciphertext.
fn cbc_encrypt(plain_text: Vec<u8>, key: [u8; BLOCK_SIZE]) -> Vec<u8> {
    // Remember to generate a random initialization vector for the first block.
    let mut rng = rand::thread_rng();
    let mut iv = [0u8; BLOCK_SIZE];
    rng.fill(&mut iv[..]);

    // Pad the data
    let padded_data = pad(plain_text);

    // Group the data into blocks
    let blocks = group(padded_data);

    // Encrypt each block
    let mut encrypted_blocks: Vec<[u8; BLOCK_SIZE]> = Vec::new();

    // XOR the first block with the IV
    let mut previous_block = iv;

    for block in blocks {
        // XOR the block with the previous block
        let xored_block: [u8; BLOCK_SIZE] = block
            .iter()
            .zip(previous_block.iter())
            .map(|(a, b)| a ^ b)
            .collect::<Vec<u8>>()
            .try_into()
            .unwrap();

        // Encrypt the xored block
        let encrypted_block = aes_encrypt(xored_block, &key);

        // Update the previous block
        previous_block = encrypted_block;

        encrypted_blocks.push(encrypted_block);
    }

    // Ungroup the blocks
    let encrypted_data = un_group(encrypted_blocks);

    // Insert the IV as the first block of the ciphertext
    let mut encrypted_data_with_iv = iv.to_vec();

    encrypted_data_with_iv.extend(encrypted_data);

    encrypted_data_with_iv
}

fn cbc_decrypt(cipher_text: Vec<u8>, key: [u8; BLOCK_SIZE]) -> Vec<u8> {
    // Group the data into blocks
    let blocks = group(cipher_text);

    // Decrypt each block
    let mut decrypted_blocks: Vec<[u8; BLOCK_SIZE]> = Vec::new();

    // XOR the first block with the IV
    let iv = blocks[0];

    let mut previous_block = iv;

    for block in blocks.iter().skip(1) {
        // Decrypt the block
        let decrypted_block = aes_decrypt(*block, &key);

        // XOR the decrypted block with the previous block
        let xored_block: [u8; BLOCK_SIZE] = decrypted_block
            .iter()
            .zip(previous_block.iter())
            .map(|(a, b)| a ^ b)
            .collect::<Vec<u8>>()
            .try_into()
            .unwrap();

        // Update the previous block
        previous_block = *block;

        decrypted_blocks.push(xored_block);
    }

    // Ungroup the blocks
    let decrypted_data = un_group(decrypted_blocks);

    // Unpad the data
    un_pad(decrypted_data)
}

/// Another mode which you can implement on your own is counter mode.
/// This mode is secure as well, and is used in real world applications.
/// It allows parallelized encryption and decryption, as well as random read access when decrypting.
///
/// In this mode, there is an index for each block being encrypted (the "counter"), as well as a random nonce.
/// For a 128-bit cipher, the nonce is 64 bits long.
///
/// For the ith block, the 128-bit value V of `nonce | counter` is constructed, where | denotes
/// concatenation. Then, V is encrypted with the key using ECB mode. Finally, the encrypted V is
/// XOR'd with the plaintext to produce the ciphertext.
///
/// A very clear diagram is present here:
/// https://en.wikipedia.org/wiki/Block_cipher_mode_of_operation#Counter_(CTR)
///
/// Once again, you will need to generate a random nonce which is 64 bits long. This should be
/// inserted as the first block of the ciphertext.
fn ctr_encrypt(plain_text: Vec<u8>, key: [u8; BLOCK_SIZE]) -> Vec<u8> {
    // Pad the data to the correct length
    let padded_text = pad(plain_text);

    let mut rng = rand::thread_rng();
    // random nonce which is 64 bits long
    let nonce: [u8; BLOCK_SIZE] = array_init::array_init(|_| rng.gen::<u8>());
    // Group the data into blocks
    let blocks = group(padded_text);

    blocks
        .into_par_iter()
        .enumerate()
        .map(|(i, block)| {
            let mut nonce_counter: [u8; BLOCK_SIZE] = [0; BLOCK_SIZE];
            nonce_counter.copy_from_slice(&nonce);
            nonce_counter[BLOCK_SIZE / 2..].copy_from_slice(&i.to_le_bytes());

            // encrypt the v and then XOR with the plain text block
            let encrypted_v = aes_encrypt(nonce_counter, &key);
            encrypted_v
                .iter()
                .zip(block.iter())
                .map(|(&x1, &x2)| x1 ^ x2)
                .collect::<Vec<u8>>()
        })
        .flatten()
        .collect()
}

fn ctr_decrypt(cipher_text: Vec<u8>, key: [u8; BLOCK_SIZE]) -> Vec<u8> {
    let group = group(cipher_text);
    let nonce = group[0];

    group[1..]
        .into_par_iter()
        .enumerate()
        .map(|(i, block)| {
            let mut nonce_counter: [u8; BLOCK_SIZE] = [0; BLOCK_SIZE];
            nonce_counter.copy_from_slice(&nonce);
            nonce_counter[BLOCK_SIZE / 2..].copy_from_slice(&i.to_be_bytes());

            let mask = aes_encrypt(nonce_counter, &key);

            block
                .into_iter()
                .zip(mask)
                .map(|(x1, x2)| x1 ^ x2)
                .collect::<Vec<u8>>()
        })
        .flatten()
        .collect()
}
