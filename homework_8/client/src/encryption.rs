#![allow(dead_code)]
#![deny(missing_docs)]
use crate::client_error::ClientError;
use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use base64::{engine::general_purpose, Engine};
use shared::message::MessagePayload;

/// Provides encryption and decryption functions

pub const NONCE_SIZE: usize = 12;

pub fn encrypt(key: &[u8], message: &[u8]) -> Result<(Vec<u8>, Vec<u8>), ClientError> {
    // key needs to be 32 bytes, otherwise from_slice panics
    let key = Key::<Aes256Gcm>::from_slice(key);

    let cipher = Aes256Gcm::new(key);
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng); // 96-bits; unique per message
    let ciphertext = cipher
        .encrypt(&nonce, message)
        .map_err(|_| ClientError::EncryptMessage)?;
    Ok((ciphertext, nonce.to_vec()))
}

pub fn decrypt(key: &[u8], nonce: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>, ClientError> {
    let key = Key::<Aes256Gcm>::from_slice(key);

    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(nonce);

    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|_| ClientError::DecryptMessage(None))?;

    Ok(plaintext)
}

pub fn pad_to_32_bytes(input: &[u8]) -> [u8; 32] {
    let mut padded = [0u8; 32];
    let bytes_to_copy = input.len().min(32);

    padded[..bytes_to_copy].copy_from_slice(&input[..bytes_to_copy]);

    padded
}

pub fn encrypt_payload(data: MessagePayload, key: &[u8]) -> Result<MessagePayload, ClientError> {
    if let MessagePayload::Text(text) = data {
        let (encrypted_msg, nonce) = encrypt(key, text.as_bytes())?;

        let mut message_to_send = nonce;
        message_to_send.extend_from_slice(&encrypted_msg);
        let encoded = general_purpose::STANDARD.encode(&message_to_send);
        Ok(MessagePayload::Text(encoded))
    } else {
        Ok(data)
    }
}

pub fn decrypt_payload(
    data: MessagePayload,
    encryption_key: &[u8],
) -> Result<MessagePayload, ClientError> {
    if let MessagePayload::Text(text) = data {
        let decoded = general_purpose::STANDARD
            .decode(text.as_bytes())
            .map_err(|_| {
                ClientError::DecryptMessage(Some(
                    "Error while decoding message from base64".to_string(),
                ))
            })?;
        let nonce = &decoded[..NONCE_SIZE];
        let encrypted_msg = &decoded[NONCE_SIZE..];
        let decrypted = decrypt(encryption_key, nonce, encrypted_msg)?;
        let text = String::from_utf8(decrypted).map_err(|_| ClientError::DecryptMessage(None))?;
        Ok(MessagePayload::Text(text))
    } else {
        Ok(data)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let text = b"Hello, world!";

        let key = pad_to_32_bytes(b"my_scrt_key");

        // let key = Aes256Gcm::generate_key(&mut OsRng);

        let encrypted = encrypt(&key, text);

        assert!(encrypted.is_ok());

        let (encrypted_message, nonce) = encrypted.unwrap();

        let decrypted = decrypt(&key, &nonce, &encrypted_message);

        assert!(decrypted.is_ok());

        let decrypted_message = decrypted.unwrap();

        assert_eq!(text, decrypted_message.as_slice());
    }
}
