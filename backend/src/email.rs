use async_trait::async_trait;
#[cfg(target_arch = "wasm32")]
use worker::{Fetch, Headers, Method, Request, RequestInit};

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
const RESEND_SEND_ENDPOINT: &str = "https://api.resend.com/emails";

#[async_trait(?Send)]
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

#[async_trait(?Send)]
impl EmailService for StdoutEmailProvider {
    async fn send_password_reset_email(&self, to: &str, token: &str) -> Result<(), String> {
        println!(
            "【パスワードリセットメール送信】宛先: {to}, リンク: {}",
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
pub struct ResendEmailProvider {
    frontend_url: String,
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    from_email: String,
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    resend_api_key: String,
}

impl ResendEmailProvider {
    pub fn new(frontend_url: String, from_email: String, resend_api_key: String) -> Self {
        Self {
            frontend_url,
            from_email,
            resend_api_key,
        }
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

    #[cfg(target_arch = "wasm32")]
    async fn send_email(&self, to: &str, subject: &str, text: &str) -> Result<(), String> {
        let body = serde_json::json!({
            "from": self.from_email,
            "to": to,
            "subject": subject,
            "text": text,
        })
        .to_string();

        let headers = Headers::new();
        headers
            .set("Authorization", &format!("Bearer {}", self.resend_api_key))
            .map_err(|e| e.to_string())?;
        headers
            .set("Content-Type", "application/json")
            .map_err(|e| e.to_string())?;

        let mut init = RequestInit::new();
        init.with_method(Method::Post);
        init.with_headers(headers);
        init.with_body(Some(body.into()));

        let req = Request::new_with_init(RESEND_SEND_ENDPOINT, &init).map_err(|e| e.to_string())?;
        let mut res = Fetch::Request(req)
            .send()
            .await
            .map_err(|e| format!("Resend fetch failed: {e}"))?;

        if !(200..300).contains(&res.status_code()) {
            let status = res.status_code();
            let body = res
                .text()
                .await
                .unwrap_or_else(|_| "<empty body>".to_string());
            return Err(format!("Resend API error: status={} body={}", status, body));
        }

        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    async fn send_email(&self, _to: &str, _subject: &str, _text: &str) -> Result<(), String> {
        Err("ResendEmailProvider is currently intended for Cloudflare Workers (wasm32) only".into())
    }
}

#[async_trait(?Send)]
impl EmailService for ResendEmailProvider {
    async fn send_password_reset_email(&self, to: &str, token: &str) -> Result<(), String> {
        let text = format!(
            "以下のリンクからパスワードをリセットしてください:\n\n{}",
            self.reset_link(token)
        );

        self.send_email(to, "パスワードリセットのご案内", &text)
            .await
    }

    async fn send_invitation_email(
        &self,
        to: &str,
        token: &str,
        group_name: &str,
    ) -> Result<(), String> {
        let text = format!(
            "{} への招待が届いています。\n\n以下のリンクから参加してください:\n\n{}",
            group_name,
            self.invitation_link(token)
        );

        self.send_email(to, "チームへの招待が届いています", &text)
            .await
    }

    async fn send_verification_email(&self, to: &str, token: &str) -> Result<(), String> {
        let text = format!(
            "メールアドレスの認証を完了するには、以下のリンクをクリックしてください:\n\n{}",
            self.verification_link(token)
        );

        self.send_email(to, "メールアドレスの認証をお願いします", &text)
            .await
    }
}
