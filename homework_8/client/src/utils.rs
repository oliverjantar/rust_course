use crate::client_error::ClientError;
use image::io::Reader as ImageReader;
use std::{
    ffi::OsStr,
    io::Cursor,
    path::{self, Path},
};
use tokio::fs::{self};
use tokio::io::{AsyncWrite, AsyncWriteExt};

/// Utility functions that are relevant only to client.

pub async fn save_file<T>(path: &T, data: &[u8]) -> Result<(), ClientError>
where
    T: AsRef<OsStr> + ?Sized,
{
    let path = path::Path::new(path);

    if let Some(dir_path) = path.parent() {
        if !dir_path.exists() {
            fs::create_dir_all(dir_path)
                .await
                .map_err(ClientError::CreateDir)?;
        }
    }

    let mut file = fs::File::create(path)
        .await
        .map_err(ClientError::CreateFile)?;
    file.write_all(data)
        .await
        .map_err(ClientError::WriteToFile)?;
    Ok(())
}

pub async fn get_file<T>(path: &T) -> Result<(String, Vec<u8>), ClientError>
where
    T: AsRef<OsStr> + ?Sized,
{
    let path = path::Path::new(path);

    let file_name_os = path.file_name();

    let file_name = match file_name_os {
        Some(file_name) => file_name.to_string_lossy(),
        None => return Err(ClientError::FileNotExists),
    };

    let bytes = fs::read(path).await.map_err(ClientError::ReadFromFile)?;

    Ok((file_name.to_string(), bytes))
}

pub async fn get_image<T>(path: &T) -> Result<Vec<u8>, ClientError>
where
    T: AsRef<OsStr> + ?Sized,
{
    let path = Path::new(path);

    let bytes = match path.ends_with(".png") {
        true => fs::read(path).await.map_err(ClientError::ReadFromFile)?,
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

pub async fn write_to_output<T>(writer: &mut T, buf: &[u8]) -> Result<(), ClientError>
where
    T: AsyncWrite + Unpin,
{
    writer.write_all(buf).await.map_err(ClientError::Write)?;
    writer.flush().await.map_err(ClientError::Write)?;
    Ok(())
}

#[cfg(test)]
mod tests {

    #[tokio::test]
    async fn get_file() {
        let (file_name, bytes) = super::get_file("Cargo.toml").await.unwrap();
        assert_eq!(file_name, "Cargo.toml");
        assert!(!bytes.is_empty());
    }

    #[tokio::test]
    async fn get_image_fails_on_regular_file() {
        let result = super::get_image("Cargo.toml").await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            super::ClientError::ConvertImagePng
        ));
    }

    #[tokio::test]
    async fn get_image_fails_when_image_not_present() {
        let result = super::get_image("sth").await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            super::ClientError::OpenImage(_)
        ));
    }

    #[tokio::test]
    async fn save_file() {
        let file_name = "test_file.txt";
        let data = b"test data";

        let result = super::save_file(file_name, data).await;

        assert!(result.is_ok());

        let (file_name, bytes) = super::get_file(file_name).await.unwrap();

        assert_eq!(file_name, "test_file.txt");
        assert_eq!(bytes, data);

        tokio::fs::remove_file(file_name).await.unwrap();
    }
}
