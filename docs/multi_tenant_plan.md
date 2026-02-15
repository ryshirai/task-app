# マルチテナント化・招待・パスワードリセット実装計画

現在のシングルテナント（1グループ専用）システムを拡張し、複数の組織（グループ）が独立して利用できるSaaS型のシステムへ移行するための計画です。

## 1. データベース設計変更 (Migration)

既存のデータ構造に「組織 (Organization)」の概念を導入し、すべてのデータが特定の組織に紐付くようにします。

### 新規テーブル
1.  **organizations**
    *   `id`: SERIAL PRIMARY KEY
    *   `name`: VARCHAR(100) NOT NULL (組織名)
    *   `created_at`: TIMESTAMP

2.  **invitations** (招待管理)
    *   `id`: SERIAL PRIMARY KEY
    *   `organization_id`: INTEGER REFERENCES organizations(id)
    *   `token`: VARCHAR(64) UNIQUE NOT NULL (招待URLに含まれるトークン)
    *   `role`: VARCHAR(20) DEFAULT 'user'
    *   `expires_at`: TIMESTAMP NOT NULL
    *   `created_at`: TIMESTAMP

3.  **password_resets** (パスワードリセット)
    *   `id`: SERIAL PRIMARY KEY
    *   `user_id`: INTEGER REFERENCES users(id)
    *   `token`: VARCHAR(64) UNIQUE NOT NULL
    *   `expires_at`: TIMESTAMP NOT NULL
    *   `created_at`: TIMESTAMP

### 既存テーブルの変更
*   **users**
    *   `organization_id`: INTEGER REFERENCES organizations(id) を追加。
    *   `email`: VARCHAR(255) を追加（招待・リセット用、ログインIDとしても使用検討可能だが現状はusername維持）。
    *   **移行措置**: 既存のユーザーは、マイグレーション時に自動作成する「デフォルト組織」に所属させます。

## 2. バックエンドロジック (Rust/Axum)

### 2.1. マルチテナント分離 (Data Isolation)
*   すべてのAPIアクセスにおいて、ログインユーザーの `organization_id` を確認します。
*   `tasks`, `users`, `daily_reports`, `activity_logs` の取得・更新時に、必ず `organization_id` によるフィルタリング (`WHERE organization_id = $1`) を強制します。

### 2.2. 新規APIエンドポイント
*   `POST /api/auth/register`: 組織の新規作成と管理者ユーザーの登録。
    *   入力: 組織名, 管理者ユーザー名, パスワード
*   `POST /api/invitations`: 招待リンクの発行（管理者のみ）。
    *   出力: 招待トークン (フロントエンドで `http://.../join?token=...` を生成)
*   `POST /api/auth/join`: 招待トークンを使ったユーザー登録。
    *   入力: トークン, ユーザー名, パスワード
*   `POST /api/auth/forgot-password`: リセットトークンの発行。
    *   入力: ユーザー名 (本来はEmail推奨だが、簡易的にユーザー名で特定)
    *   動作: トークンを生成し、(メール送信の代わりに) レスポンスまたはログに出力。
*   `POST /api/auth/reset-password`: トークンを使ったパスワード再設定。

## 3. フロントエンド実装 (SvelteKit)

### 3.1. 新規ページ
*   `/register`: 組織の新規登録画面（LPから遷移するイメージ）。
*   `/join`: 招待受け入れ画面。トークンが有効ならユーザー名・パスワード入力フォームを表示。
*   `/forgot-password`: パスワード再設定リクエスト画面。
*   `/reset-password`: 新しいパスワードの設定画面。

### 3.2. 既存画面の改修
*   **ログイン画面**: 「組織を新規作成」や「パスワードを忘れた場合」へのリンクを追加。
*   **ユーザー管理モーダル**: 「招待URLを発行」ボタンを追加し、生成されたURLをクリップボードにコピーできるようにする。

## 4. 開発ステップ

1.  **DBマイグレーション**: テーブル作成と既存データの移行。
2.  **組織登録API**: `register` エンドポイントの実装。
3.  **データ分離の実装**: 既存の `get_users`, `get_tasks` 等のクエリを `organization_id` 対応に書き換え。
4.  **招待機能**: 招待トークン発行と、それを使ったユーザー登録の実装。
5.  **パスワードリセット**: トークン発行とリセットロジックの実装。
6.  **UI実装**: 各種画面の作成とルーティング設定。

---

この計画で進めます。まずはデータベースの変更から着手します。
