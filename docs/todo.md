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

## Phase 5: Multi-tenancy & Advanced Security

- [x] Organization isolation の実装（`organization_id` によるデータ分離）
- [x] JWT認証の実装（組織コンテキスト付きクレーム）
- [x] 招待機能の実装（招待トークン発行・参加登録）
- [x] パスワードリセット機能の基盤
- [x] フロントエンド導線（登録・招待参加・リセット画面）の整備

## Phase 6: Task & Time Management Optimization

- [x] タスク（Entity）と作業時間（Logs）の分離
- [x] アクティブタスク・セレクター（既存タスクへのクイック紐付け）
- [x] タスク単位での合計稼働時間の自動集計
- [x] JST（日本時間）への完全対応（サーバー/クライアント間の時差解消）

## Phase 7: UI/UX & Visibility

- [x] 表示グループ設定（カスタムメンバーフィルタリング）
- [x] ヘッダーメニューの整理（ドロップダウン集約）
- [x] ログインユーザーの最上位表示
- [x] 選択した日付・グループの永続化 (localStorage)

## Future Roadmap

- [ ] タスク間の依存関係（Connection）の可視化
- [ ] AIによる日報要約・進捗分析 (Gemini連携)
- [ ] WebSocketによる完全リアルタイム同期（現在はポーリング）
- [ ] モバイルアプリ版またはPWA対応
