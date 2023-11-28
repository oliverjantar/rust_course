use crate::{client_error::ClientError, encryption};
use base64::{engine::general_purpose, Engine};
use image::io::Reader as ImageReader;
use shared::message::MessagePayload;
use std::{
    ffi::OsStr,
    fs,
    io::{Cursor, Write},
    path::{self, Path},
};

/// Utility functions that are relevant only to client.

pub fn save_file<T>(path: &T, data: &[u8]) -> Result<(), ClientError>
where
    T: AsRef<OsStr> + ?Sized,
{
    let path = path::Path::new(path);

    if let Some(dir_path) = path.parent() {
        if !dir_path.exists() {
            fs::create_dir_all(dir_path).map_err(ClientError::CreateDir)?;
        }
    }

    let mut file = fs::File::create(path).map_err(ClientError::CreateFile)?;
    file.write_all(data).map_err(ClientError::WriteToFile)?;
    Ok(())
}

pub fn get_file<T>(path: &T) -> Result<(String, Vec<u8>), ClientError>
where
    T: AsRef<OsStr> + ?Sized,
{
    let path = path::Path::new(path);

    let file_name_os = path.file_name();

    let file_name = match file_name_os {
        Some(file_name) => file_name.to_string_lossy(),
        None => return Err(ClientError::FileNotExists),
    };

    let bytes = fs::read(path).map_err(ClientError::ReadFromFile)?;

    Ok((file_name.to_string(), bytes))
}

pub fn get_image<T>(path: &T) -> Result<Vec<u8>, ClientError>
where
    T: AsRef<OsStr> + ?Sized,
{
    let path = Path::new(path);

    let bytes = match path.ends_with(".png") {
        true => fs::read(path).map_err(ClientError::ReadFromFile)?,
        false => convert_to_png(path)?,
    };
    Ok(bytes)
}

fn convert_to_png<T>(path: &T) -> Result<Vec<u8>, ClientError>
where
    T: AsRef<Path> + ?Sized,
{
    let mut bytes = vec![];

    let img = ImageReader::open(path)
        .map_err(ClientError::OpenImage)?
        .decode()
        .map_err(|_| ClientError::ConvertImagePng)?;

    img.write_to(&mut Cursor::new(&mut bytes), image::ImageOutputFormat::Png)
        .map_err(|_| ClientError::ConvertImagePng)?;
    Ok(bytes)
}

pub fn write_to_output<T>(writer: &mut T, buf: &[u8]) -> Result<(), ClientError>
where
    T: Write,
{
    writer.write_all(buf).map_err(ClientError::Write)?;
    writer.flush().map_err(ClientError::Write)?;
    Ok(())
}

pub fn encrypt_payload(data: MessagePayload, key: &[u8]) -> Result<MessagePayload, ClientError> {
    if let MessagePayload::Text(text) = data {
        let (encrypted_msg, nonce) = encryption::encrypt(key, text.as_bytes())?;

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
        let nonce = &decoded[..encryption::NONCE_SIZE];
        let encrypted_msg = &decoded[encryption::NONCE_SIZE..];
        let decrypted = encryption::decrypt(encryption_key, nonce, encrypted_msg)?;
        let text = String::from_utf8(decrypted).map_err(|_| ClientError::DecryptMessage(None))?;
        Ok(MessagePayload::Text(text))
    } else {
        Ok(data)
    }
}
