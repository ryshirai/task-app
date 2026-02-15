# Development Roadmap

## Phase 1: Infrastructure & API

- [ ] Docker Composeの設定 (Postgres)
- [ ] Rust (Axum) プロジェクトの初期化
- [ ] sqlx によるテーブル作成マイグレーション
- [ ] 基本的な CRUD API の実装

## Phase 2: Timeline View

- [ ] SvelteKit プロジェクトの初期化
- [ ] Tailwind CSS によるタイムラインレイアウト実装
- [ ] APIからデータを取得し、タイムライン上にタスクを描画
- [ ] 現在時刻を示す赤い縦線の実装

## Phase 3: Interactive Registration

- [ ] 座標 -> 時刻変換ユーティリティの作成 (15分スナップ対応)
- [ ] ドラッグ&ドロップによる範囲選択機能の実装
- [ ] インライン入力フォームとAPI連携

## Phase 4: Polishing

- [ ] 進捗率に応じたバーの色の動的変化
- [ ] 予定時間超過時のアラート表示 (Red Lineを越えたら赤くする)
