use async_trait::async_trait;
use reqwest::Client;

#[async_trait]
pub trait EmailService: Send + Sync {
    async fn send_password_reset_email(&self, to: &str, token: &str) -> Result<(), String>;
    async fn send_invitation_email(
        &self,
        to: &str,
        token: &str,
        group_name: &str,
    ) -> Result<(), String>;
}

#[derive(Debug, Clone)]
pub struct StdoutEmailProvider {
    frontend_url: String,
}

impl StdoutEmailProvider {
    pub fn new(frontend_url: String) -> Self {
        Self { frontend_url }
    }

    fn reset_link(&self, token: &str) -> String {
        format!("{}/reset-password?token={token}", self.frontend_url)
    }

    fn invitation_link(&self, token: &str) -> String {
        format!("{}/join?token={token}", self.frontend_url)
    }
}

#[async_trait]
impl EmailService for StdoutEmailProvider {
    async fn send_password_reset_email(&self, to: &str, token: &str) -> Result<(), String> {
        println!(
            "PASSWORD RESET EMAIL to {to}: {}",
            self.reset_link(token)
        );
        Ok(())
    }

    async fn send_invitation_email(
        &self,
        to: &str,
        token: &str,
        group_name: &str,
    ) -> Result<(), String> {
        println!(
            "INVITATION EMAIL to {to} for group '{group_name}': {}",
            self.invitation_link(token)
        );
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct SendgridEmailProvider {
    client: Client,
    api_key: String,
    frontend_url: String,
    from_email: String,
}

impl SendgridEmailProvider {
    pub fn new(api_key: String, frontend_url: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            frontend_url,
            from_email: "no-reply@example.com".to_string(),
        }
    }

    async fn send_email(&self, to: &str, subject: &str, content: &str) -> Result<(), String> {
        let body = serde_json::json!({
            "personalizations": [{
                "to": [{ "email": to }]
            }],
            "from": { "email": self.from_email },
            "subject": subject,
            "content": [{
                "type": "text/plain",
                "value": content
            }]
        });

        self.client
            .post("https://api.sendgrid.com/v3/mail/send")
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("sendgrid request failed: {e}"))?
            .error_for_status()
            .map_err(|e| format!("sendgrid returned error status: {e}"))?;

        Ok(())
    }
}

#[async_trait]
impl EmailService for SendgridEmailProvider {
    async fn send_password_reset_email(&self, to: &str, token: &str) -> Result<(), String> {
        let reset_link = format!("{}/reset-password?token={token}", self.frontend_url);
        let content = format!("Reset your password using this link: {reset_link}");

        self.send_email(to, "Password Reset", &content).await
    }

    async fn send_invitation_email(
        &self,
        to: &str,
        token: &str,
        group_name: &str,
    ) -> Result<(), String> {
        let join_link = format!("{}/join?token={token}", self.frontend_url);
        let content = format!(
            "You were invited to join {group_name}. Use this link to join: {join_link}"
        );

        self.send_email(to, "You're Invited", &content).await
    }
}
