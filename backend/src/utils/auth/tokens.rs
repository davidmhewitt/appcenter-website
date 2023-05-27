use anyhow::{anyhow, Result};
use argon2::password_hash::rand_core::{OsRng, RngCore};
use base64::{engine::general_purpose, Engine as _};
use common::settings::Secret;
use deadpool_redis::redis::AsyncCommands;
use hex;
use pasetors::claims::{Claims, ClaimsValidationRules};
use pasetors::keys::SymmetricKey;
use pasetors::token::UntrustedToken;
use pasetors::{local, version4::V4, Local};
use secrecy::ExposeSecret;
use time::format_description::well_known;
use uuid::Uuid;

/// Store the session key prefix as a const so it can't be typo'd anywhere it's used.
const SESSION_KEY_PREFIX: &str = "valid_session_key_for_";

pub struct SessionIdAndToken {
    session_id: String,
    token: String,
}

pub fn generate_paseto_token(
    user_id: uuid::Uuid,
    is_for_password_change: bool,
    settings: &Secret,
) -> Result<SessionIdAndToken> {
    let session_key: String = {
        let mut buff = [0_u8; 128];
        OsRng.fill_bytes(&mut buff);
        hex::encode(buff)
    };

    let current_date_time = time::OffsetDateTime::now_utc();

    let time_to_live = time::Duration::minutes(settings.token_expiration);
    let dt = current_date_time + time_to_live;

    let mut claims = Claims::new().unwrap();
    claims
        .expiration(&dt.format(&well_known::Iso8601::DEFAULT).unwrap())
        .unwrap();
    claims
        .add_additional("user_id", serde_json::json!(user_id))
        .unwrap();
    claims
        .add_additional("session_key", serde_json::json!(session_key))
        .unwrap();

    let sk = SymmetricKey::<V4>::from(
        &general_purpose::STANDARD
            .decode(settings.secret_key.expose_secret())
            .expect("Unable to decode secret key for PASETO encryption"),
    )
    .unwrap();

    let token = local::encrypt(
        &sk,
        &claims,
        None,
        Some(
            &general_purpose::STANDARD
                .decode(settings.hmac_secret.expose_secret())
                .expect("Unable to decode HMAC secret for PASETO encryption"),
        ),
    )?;

    Ok(SessionIdAndToken {
        session_id: session_key,
        token,
    })
}

#[tracing::instrument(name = "Issue PASETO token", skip(redis_connection, settings))]
pub async fn issue_confirmation_token_pasetors(
    user_id: uuid::Uuid,
    redis_connection: &mut deadpool_redis::redis::aio::Connection,
    is_for_password_change: bool,
    settings: &Secret,
) -> Result<String> {
    let expiration_in_minutes = settings.token_expiration;

    let token = generate_paseto_token(user_id, is_for_password_change, settings)?;

    let redis_key = {
        if is_for_password_change {
            format!(
                "{}{}_is_for_password_change",
                SESSION_KEY_PREFIX, token.session_id
            )
        } else {
            format!("{}{}", SESSION_KEY_PREFIX, token.session_id)
        }
    };

    redis_connection
        .set(
            redis_key.clone(),
            // I just want to validate that the key exists to indicate the session is "live".
            String::new(),
        )
        .await
        .map_err(|e| {
            tracing::event!(target: "backend", tracing::Level::ERROR, "RedisError (set): {}", e);
            e
        })?;

    let time_to_live = time::Duration::minutes(expiration_in_minutes);

    redis_connection
        .expire(
            redis_key.clone(),
            time_to_live.whole_seconds().try_into().unwrap(),
        )
        .await
        .map_err(|e| {
            tracing::event!(target: "backend", tracing::Level::ERROR, "RedisError (expiry): {}", e);
            e
        })?;

    Ok(token.token)
}

pub struct SessionIdAndUuid {
    session_id: String,
    uuid: uuid::Uuid,
}

pub fn verify_paseto_token(
    token: &str,
    settings: &Secret,
) -> Result<SessionIdAndUuid> {
    let sk = SymmetricKey::<V4>::from(
        &general_purpose::STANDARD
            .decode(settings.secret_key.expose_secret())
            .expect("Unable to decode secret key for PASETO encryption"),
    )?;

    let validation_rules = ClaimsValidationRules::new();
    let untrusted_token = UntrustedToken::<Local, V4>::try_from(token)?;
    let trusted_token = local::decrypt(
        &sk,
        &untrusted_token,
        &validation_rules,
        None,
        Some(
            &general_purpose::STANDARD
                .decode(settings.hmac_secret.expose_secret())
                .expect("Unable to decode HMAC secret for PASETO decryption"),
        ),
    )?;

    let claims = trusted_token
        .payload_claims()
        .ok_or(anyhow!("Couldn't get claims from token"))?;

    let uid = serde_json::from_value::<Uuid>(
        claims
            .get_claim("user_id")
            .ok_or(anyhow!("No user_id claim in token"))?
            .clone(),
    )?;

    let session_key = serde_json::from_value::<String>(
        claims
            .get_claim("session_key")
            .ok_or(anyhow!("No session_key claim in token"))?
            .clone(),
    )?;

    Ok(SessionIdAndUuid {
        session_id: session_key,
        uuid: uid,
    })
}

/// Verifies and destroys a token. A token is destroyed immediately
/// it has successfully been verified and all encoded data extracted.
/// Redis is used for such destruction.
#[cfg_attr(not(coverage), tracing::instrument(name = "Verify pasetors token", skip(token, redis_connection)))]
pub async fn verify_confirmation_token_pasetor(
    token: String,
    redis_connection: &mut deadpool_redis::redis::aio::Connection,
    is_for_password_change: bool,
) -> Result<crate::types::ConfirmationToken> {
    let settings = common::settings::get_settings().expect("Cannot load settings.");

    let result = verify_paseto_token(&token, &settings.secret)?;

    let redis_key = {
        if is_for_password_change {
            format!(
                "{}{}_is_for_password_change",
                SESSION_KEY_PREFIX, result.session_id
            )
        } else {
            format!("{}{}", SESSION_KEY_PREFIX, result.session_id)
        }
    };

    if redis_connection
        .get::<_, Option<String>>(redis_key.clone())
        .await?
        .is_none()
    {
        return Err(anyhow!("Token has been used or expired."));
    }
    redis_connection
        .del(redis_key.clone())
        .await?;

    Ok(crate::types::ConfirmationToken { user_id: result.uuid })
}

#[cfg(test)]
mod tests {
    use secrecy::SecretString;

    use super::*;

    #[test]
    fn test_valid_token() -> Result<()> {
        let secret_key: String = {
            let mut buff = [0_u8; 32];
            OsRng.fill_bytes(&mut buff);
            general_purpose::STANDARD.encode(buff)
        };

        let hmac_secret: String = {
            let mut buff = [0_u8; 64];
            OsRng.fill_bytes(&mut buff);
            general_purpose::STANDARD.encode(buff)
        };

        let settings = Secret {
            secret_key: SecretString::new(secret_key),
            token_expiration: 30,
            hmac_secret: SecretString::new(hmac_secret),
        };

        let uid = uuid::Uuid::new_v4();

        let token = generate_paseto_token(uid, false, &settings)?;

        let result = verify_paseto_token(&token.token, &settings)?;

        assert_eq!(uid, result.uuid);

        Ok(())
    }

    #[test]
    fn test_invalid_token() -> Result<()> {
        let secret_key: String = {
            let mut buff = [0_u8; 32];
            OsRng.fill_bytes(&mut buff);
            general_purpose::STANDARD.encode(buff)
        };

        let hmac_secret: String = {
            let mut buff = [0_u8; 64];
            OsRng.fill_bytes(&mut buff);
            general_purpose::STANDARD.encode(buff)
        };

        let settings = Secret {
            secret_key: SecretString::new(secret_key),
            token_expiration: 30,
            hmac_secret: SecretString::new(hmac_secret),
        };

        let uid = uuid::Uuid::new_v4();

        let token = generate_paseto_token(uid, false, &settings)?;

        // Regenerate secrets to invalidate token
        let secret_key: String = {
            let mut buff = [0_u8; 32];
            OsRng.fill_bytes(&mut buff);
            general_purpose::STANDARD.encode(buff)
        };

        let hmac_secret: String = {
            let mut buff = [0_u8; 64];
            OsRng.fill_bytes(&mut buff);
            general_purpose::STANDARD.encode(buff)
        };

        let settings = Secret {
            secret_key: SecretString::new(secret_key),
            token_expiration: 30,
            hmac_secret: SecretString::new(hmac_secret),
        };

        let result = verify_paseto_token(&token.token, &settings);

        assert!(result.is_err());

        Ok(())
    }
}
