# Development Roadmap

## Phase 1: Infrastructure & API

- [x] Docker Composeの設定 (Postgres)
- [x] Rust (Axum) プロジェクトの初期化
- [x] sqlx によるテーブル作成マイグレーション
- [x] 基本的な CRUD API の実装

## Phase 2: Timeline View

- [x] SvelteKit プロジェクトの初期化
- [x] Tailwind CSS によるタイムラインレイアウト実装
- [x] APIからデータを取得し、タイムライン上にタスクを描画
- [x] 現在時刻を示す赤い縦線の実装

## Phase 3: Interactive Registration

- [x] 座標 -> 時刻変換ユーティリティの作成 (15分スナップ対応)
- [x] ドラッグ&ドロップによる範囲選択機能の実装
- [x] インライン入力フォームとAPI連携

## Phase 4: Polishing

- [x] 進捗率に応じたバーの色の動的変化
- [x] 予定時間超過時のアラート表示 (Red Lineを越えたら赤くする)
- [x] UIデザインのブラッシュアップ (モダンな配色とアニメーション)
