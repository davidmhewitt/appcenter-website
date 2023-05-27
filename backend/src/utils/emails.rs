use lettre::AsyncTransport;
use secrecy::ExposeSecret;

#[cfg_attr(not(coverage), tracing::instrument(
    name = "Generic e-mail sending function.",
    skip(
        recipient_email,
        subject,
        html_content,
        text_content
    ),
    fields(
        recipient_email = %recipient_email,
    )
))]
pub async fn send_email(
    sender_email: Option<String>,
    recipient_email: String,
    subject: impl Into<String>,
    html_content: impl Into<String>,
    text_content: impl Into<String>,
) -> Result<(), String> {
    let settings = common::settings::get_settings().expect("Failed to read settings.");

    let email = lettre::Message::builder()
        .from(
            format!(
                "{} <{}>",
                "elementary Account",
                if sender_email.is_some() {
                    sender_email.unwrap()
                } else {
                    settings.email.host_user.clone()
                }
            )
            .parse()
            .unwrap(),
        )
        .to(recipient_email.parse().unwrap())
        .subject(subject)
        .multipart(
            lettre::message::MultiPart::alternative()
                .singlepart(
                    lettre::message::SinglePart::builder()
                        .header(lettre::message::header::ContentType::TEXT_PLAIN)
                        .body(text_content.into()),
                )
                .singlepart(
                    lettre::message::SinglePart::builder()
                        .header(lettre::message::header::ContentType::TEXT_HTML)
                        .body(html_content.into()),
                ),
        )
        .unwrap();

    let creds = lettre::transport::smtp::authentication::Credentials::new(
        settings.email.host_user,
        settings.email.host_user_password.expose_secret().to_owned(),
    );

    // Open a remote connection to the smtp server
    let mut mailer_builder =
        lettre::AsyncSmtpTransport::<lettre::Tokio1Executor>::relay(&settings.email.host)
            .unwrap()
            .port(settings.email.port);

    if settings.email.authentication {
        mailer_builder = mailer_builder.credentials(creds);
    } else {
        mailer_builder = mailer_builder.tls(lettre::transport::smtp::client::Tls::None);
    }

    let mailer: lettre::AsyncSmtpTransport<lettre::Tokio1Executor> = mailer_builder.build();

    // Send the email
    match mailer.send(email).await {
        Ok(_) => {
            tracing::event!(target: "backend", tracing::Level::INFO, "Email successfully sent!");
            Ok(())
        }
        Err(e) => {
            tracing::event!(target: "backend", tracing::Level::ERROR, "Could not send email: {:#?}", e);
            Err(format!("Could not send email: {:#?}", e))
        }
    }
}

#[cfg_attr(not(coverage), tracing::instrument(
    name = "Generic multipart e-mail sending function.",
    skip(redis_connection),
    fields(
        recipient_user_id = %user_id,
        recipient_email = %recipient_email,
    )
))]
pub async fn send_multipart_email(
    subject: String,
    user_id: uuid::Uuid,
    sender_email: Option<String>,
    recipient_email: String,
    template_name: &str,
    redis_connection: &mut deadpool_redis::redis::aio::Connection,
) -> Result<(), String> {
    let settings = common::settings::get_settings().expect("Unable to load settings.");
    let title: String = subject.clone();

    let is_for_password_change = template_name == "password_reset_email.html";

    let issued_token = match crate::utils::auth::tokens::issue_confirmation_token_pasetors(
        user_id,
        redis_connection,
        is_for_password_change,
        settings.secret.token_expiration,
    )
    .await
    {
        Ok(t) => t,
        Err(e) => {
            tracing::event!(target: "backend", tracing::Level::ERROR, "{}", e);
            return Err(format!("{}", e));
        }
    };
    let web_address = {
        if settings.debug {
            format!(
                "{}:{}",
                settings.application.base_url, settings.application.port,
            )
        } else {
            settings.application.base_url
        }
    };
    let confirmation_link = {
        if is_for_password_change {
            format!(
                "{}/api/users/password/confirm/change_password?token={}",
                web_address, issued_token,
            )
        } else {
            format!(
                "{}/api/users/register/confirm?token={}",
                web_address, issued_token,
            )
        }
    };
    let current_date_time = chrono::Local::now();
    let dt = current_date_time + chrono::Duration::minutes(settings.secret.token_expiration);

    let template = crate::ENV.get_template(template_name).unwrap();
    let ctx = minijinja::context! {
        title => &title,
        confirmation_link => &confirmation_link,
        domain => &settings.frontend_url,
        expiration_time => &settings.secret.token_expiration,
        exact_time => &dt.format("%A %B %d, %Y at %r").to_string()
    };
    let html_text = template.render(ctx).unwrap();

    let text = format!(
        r#"
        Tap the link below to confirm your email address.
        {}
        "#,
        confirmation_link
    );
    tokio::spawn(send_email(
        sender_email,
        recipient_email,
        subject,
        html_text,
        text,
    ));
    Ok(())
}
