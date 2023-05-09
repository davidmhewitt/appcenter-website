#[derive(serde::Serialize)]
pub enum ErrorTranslationKey {
    #[serde(rename = "confirmation.generic-problem")]
    GenericConfirmationProblem,
    #[serde(rename = "confirmation.token-used")]
    ConfirmationTokenUsed,
    #[serde(rename = "registration.generic-problem")]
    GenericRegistrationProblem,
    #[serde(rename = "registration.user-already-exists")]
    UserAlreadyExists,
    #[serde(rename = "registration.no-email-permission")]
    RegistrationNoEmailPermission,
    #[serde(rename = "login.username-password-mismatch")]
    UsernamePasswordMismatch,
    #[serde(rename = "login.user-nonexistent")]
    UserDoesntExist,
    #[serde(rename = "logout.generic-problem")]
    GenericLogoutProblem,
    #[serde(rename = "app-register.generic-problem")]
    GenericAppRegisterProblem,
}

#[derive(serde::Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub translation_key: ErrorTranslationKey,
}

#[derive(serde::Serialize)]
pub struct SuccessResponse {
    pub message: String,
}

pub const USER_ID_KEY: &str = "user_id";
pub const USER_EMAIL_KEY: &str = "user_email";
