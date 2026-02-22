use async_trait::async_trait;
use aws_sdk_sesv2::Client as SesClient;
use aws_sdk_sesv2::types::{Body, Content, Destination, EmailContent, Message};

#[async_trait]
pub trait EmailService: Send + Sync {
    async fn send_password_reset_email(&self, to: &str, token: &str) -> Result<(), String>;
    async fn send_invitation_email(
        &self,
        to: &str,
        token: &str,
        group_name: &str,
    ) -> Result<(), String>;
    async fn send_verification_email(&self, to: &str, token: &str) -> Result<(), String>;
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

    fn verification_link(&self, token: &str) -> String {
        format!("{}/verify-email?token={token}", self.frontend_url)
    }
}

#[async_trait]
impl EmailService for StdoutEmailProvider {
    async fn send_password_reset_email(&self, to: &str, token: &str) -> Result<(), String> {
        println!("【パスワードリセットメール送信】宛先: {to}, リンク: {}", self.reset_link(token));
        Ok(())
    }

    async fn send_invitation_email(
        &self,
        to: &str,
        token: &str,
        group_name: &str,
    ) -> Result<(), String> {
        println!(
            "【招待メール送信】宛先: {to}, グループ: {group_name}, リンク: {}",
            self.invitation_link(token)
        );
        Ok(())
    }

    async fn send_verification_email(&self, to: &str, token: &str) -> Result<(), String> {
        println!(
            "【メールアドレス認証メール送信】宛先: {to}, リンク: {}",
            self.verification_link(token)
        );
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct SesEmailProvider {
    client: SesClient,
    frontend_url: String,
    from_email: String,
}

impl SesEmailProvider {
    pub fn new(client: SesClient, frontend_url: String, from_email: String) -> Self {
        Self {
            client,
            frontend_url,
            from_email,
        }
    }

    async fn send_email(&self, to: &str, subject: &str, content: &str) -> Result<(), String> {
        let dest = Destination::builder().to_addresses(to).build();
        let subject_content = Content::builder().data(subject).charset("UTF-8").build().map_err(|e| e.to_string())?;
        let body_content = Content::builder().data(content).charset("UTF-8").build().map_err(|e| e.to_string())?;
        let body = Body::builder().text(body_content).build();

        let message = Message::builder()
            .subject(subject_content)
            .body(body)
            .build();

        let email_content = EmailContent::builder().simple(message).build();

        self.client
            .send_email()
            .from_email_address(&self.from_email)
            .destination(dest)
            .content(email_content)
            .send()
            .await
            .map_err(|e| format!("SES request failed: {e}"))?;

        Ok(())
    }
}

#[async_trait]
impl EmailService for SesEmailProvider {
    async fn send_password_reset_email(&self, to: &str, token: &str) -> Result<(), String> {
        let reset_link = format!("{}/reset-password?token={token}", self.frontend_url);
        let content = format!("以下のリンクからパスワードをリセットしてください:\n\n{}", reset_link);

        self.send_email(to, "パスワードリセットのご案内", &content).await
    }

    async fn send_invitation_email(
        &self,
        to: &str,
        token: &str,
        group_name: &str,
    ) -> Result<(), String> {
        let join_link = format!("{}/join?token={token}", self.frontend_url);
        let content = format!(
            "{} への招待が届いています。\n\n以下のリンクから参加してください:\n\n{}",
            group_name, join_link
        );

        self.send_email(to, "チームへの招待が届いています", &content).await
    }

    async fn send_verification_email(&self, to: &str, token: &str) -> Result<(), String> {
        let verify_link = format!("{}/verify-email?token={token}", self.frontend_url);
        let content = format!(
            "メールアドレスの認証を完了するには、以下のリンクをクリックしてください:\n\n{}",
            verify_link
        );

        self.send_email(to, "メールアドレスの認証をお願いします", &content).await
    }
}
