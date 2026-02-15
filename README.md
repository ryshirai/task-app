# Team Status Dashboard "Glance"

## 1. 概要

チームメンバーの稼働状況を一目で把握するための視認性特化型ダッシュボード。
「誰が」「何に」「いつまで」取り組んでいるかを、タイムライン形式で可視化します。

## 2. 技術スタック

- **Backend:** Rust (Axum, sqlx, PostgreSQL)
- **Frontend:** Svelte (SvelteKit, Tailwind CSS)
- **Runtime:** Docker / Docker Compose

## 3. 開発プロセス

- **PM (Gemini-cli):** 要件定義、ロジック設計、進捗管理を担当。
- **Coder (Codex/Agent):** コード実装、テスト、バグ修正を担当。

## 4. ドキュメント構成

- `docs/requirements.md`: 機能・非機能要件
- `docs/technical_spec.md`: DB設計・API定義
- `docs/ux_interaction.md`: タイムライン操作ロジックの詳細
- `TODO.md`: 実装タスクリスト（Coderが参照するメインファイル）
