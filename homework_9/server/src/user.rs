use std::num::NonZeroU32;

use base64::{engine::general_purpose, Engine};
use ring::{
    digest, pbkdf2,
    rand::{SecureRandom, SystemRandom},
};
use secrecy::{ExposeSecret, Secret};
use shared::message::AuthUser;
use uuid::Uuid;

use crate::server_error::ServerError;

#[derive(Debug)]
pub struct User {
    pub id: Uuid,
    pub password: Secret<String>,
    pub username: String,
    pub salt: String,
}

const CREDENTIAL_LEN: usize = digest::SHA512_OUTPUT_LEN;
const N_ITER: Option<NonZeroU32> = NonZeroU32::new(100_000);

impl User {
    pub fn verify_user_password(&self, password_to_verify: &[u8]) -> Result<bool, ServerError> {
        let decoded_salt = general_purpose::STANDARD
            .decode(self.salt.as_bytes())
            .map_err(|_| ServerError::PasswordDecode)?;

        let decoded_pwd = general_purpose::STANDARD
            .decode(self.password.expose_secret().as_bytes())
            .map_err(|_| ServerError::PasswordDecode)?;

        Ok(Self::verify(
            &decoded_pwd,
            &decoded_salt,
            password_to_verify,
        ))
    }

    fn verify(secret: &[u8], salt: &[u8], password_to_verify: &[u8]) -> bool {
        pbkdf2::verify(
            pbkdf2::PBKDF2_HMAC_SHA512,
            N_ITER.unwrap(),
            salt,
            password_to_verify,
            secret,
        )
        .is_ok()
    }
}

impl TryFrom<AuthUser> for User {
    type Error = ServerError;

    fn try_from(value: AuthUser) -> Result<Self, Self::Error> {
        let mut salt = [0u8; CREDENTIAL_LEN];
        let rng = SystemRandom::new();

        rng.fill(&mut salt).map_err(|_| ServerError::CreateUser)?;

        let mut pwd_hash = [0u8; CREDENTIAL_LEN];
        pbkdf2::derive(
            pbkdf2::PBKDF2_HMAC_SHA512,
            N_ITER.unwrap(),
            &salt,
            value.password.as_bytes(),
            &mut pwd_hash,
        );

        let encoded_pwd = general_purpose::STANDARD.encode(pwd_hash);
        let encoded_salt = general_purpose::STANDARD.encode(salt);

        Ok(Self {
            id: Uuid::new_v4(),
            password: Secret::from(encoded_pwd),
            username: value.name,
            salt: encoded_salt,
        })
    }
}
