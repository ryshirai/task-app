## Summary
本PRでは、認証機能を改善し、ログイン時に「ユーザーID（username）またはメールアドレス」のどちらでも認証できるようにしました。加えて、パスワード要件を強化し、登録・招待参加・パスワード再設定・パスワード変更・管理者によるユーザー作成の各経路で、同一のセキュアなルールを適用しています。

## Changes
### Backend
- `backend/src/handlers/auth.rs`
  - ログイン時のユーザー検索を `username = $1 OR email = $1` に変更し、ユーザーID/メールアドレスの両対応を実装。
  - `register` / `join` / `reset_password` で `is_secure_password` によるパスワード強度チェックを追加。
- `backend/src/handlers/users.rs`
  - `update_password` / `create_user` に同じパスワード強度チェックを追加。
- `backend/src/utils.rs`
  - `is_secure_password(password: &str) -> bool` を追加。
  - ルール: 8文字以上、英大文字・英小文字・数字・記号（ASCII punctuation）を各1文字以上含むこと。
  - `is_secure_password` のユニットテストを追加（正常系・異常系）。

### Frontend
- `frontend/src/lib/components/Login.svelte`
  - ログイン入力欄のラベルとプレースホルダを「ユーザー名 または メールアドレス」に変更。
- `frontend/src/routes/register/+page.svelte`
- `frontend/src/routes/join/+page.svelte`
- `frontend/src/lib/components/ProfileModal.svelte`
  - フロント側でも同等のパスワード強度チェック（正規表現）を追加。
  - パスワード要件の補助テキストを追加し、入力段階で要件を明示。

## Verification
実施した検証:
- `cargo test -p backend utils::tests -- --nocapture`
  - 結果: PASS（7件）
  - 追加テストで確認した内容:
    - `accepts_secure_password`: 要件を満たすパスワードを受理
    - `rejects_password_shorter_than_8`: 8文字未満を拒否
    - `rejects_password_missing_required_character_types`: 大文字/小文字/数字/記号の欠落を拒否
- `cargo test -p backend -- --nocapture`
  - `utils` のユニットテストはPASS
  - 一部 `api_tests` はDB接続（`DATABASE_URL`）が必要なため、この環境では PermissionDenied で失敗（本PR追加のユニットテスト範囲外）

## Security Considerations
- サーバー側でパスワード要件を強制しているため、クライアント回避による弱いパスワード登録を防止。
- クライアント側でも同ルールを先行チェックし、無駄なリクエストを減らしつつUXを改善。
- パスワードハッシュは既存通り Argon2 を利用し、認証処理の安全性を維持。
- ログイン識別子をメールアドレスまで広げても、認証成否メッセージは従来どおり汎用エラーで情報漏えいリスクを抑制。

## Screenshots/UI Changes
- ログイン画面: 入力欄表記を「ユーザー名 または メールアドレス」に変更。
- 登録画面/招待参加画面/プロフィール（パスワード変更）: パスワード要件の案内文を追加。
- 要件未達時は送信前にエラーメッセージを表示する挙動へ変更。
- スクリーンショットは未添付（必要に応じてPR上で追加可能）。
