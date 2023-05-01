use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use secrecy::{ExposeSecret, SecretString};

#[tracing::instrument(name = "Hashing user password", skip(password))]
pub async fn hash(password: &[u8]) -> SecretString {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password, &salt)
        .expect("Unable to hash password.")
        .to_string()
        .into()
}

#[tracing::instrument(name = "Verifying user password", skip(password, hash))]
pub fn verify_password(
    hash: &SecretString,
    password: &SecretString,
) -> Result<(), argon2::password_hash::Error> {
    let parsed_hash = PasswordHash::new(hash.expose_secret())?;
    Argon2::default().verify_password(password.expose_secret().as_bytes(), &parsed_hash)
}
