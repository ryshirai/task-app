# Technical Specification

## 1. Database Schema (PostgreSQL)

```sql
CREATE TABLE members (
    id SERIAL PRIMARY KEY,
    name VARCHAR(50) NOT NULL,
    avatar_url TEXT
);

CREATE TABLE tasks (
    id SERIAL PRIMARY KEY,
    member_id INTEGER REFERENCES members(id),
    title VARCHAR(100) NOT NULL,
    status VARCHAR(20) DEFAULT 'todo', -- todo, doing, done
    progress_rate INTEGER DEFAULT 0,
    start_at TIMESTAMP WITH TIME ZONE NOT NULL,
    end_at TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
)
```

## 2. API Endpoints (Rust/Axum)

- `GET /api/members`: メンバー一覧と今日のタスクを取得
- `POST /api/tasks`: 新規タスク登録
- `PATCH /api/tasks/:id`: ステータス・進捗・時間の更新
- `DELETE /api/tasks/:id`: タスク削除

## 3. UI Components (Svelte)

- TimelineContainer: 時間軸（09:00 - 19:00）と現在時刻線の描画。
- MemberRow: メンバーごとの水平レーン。
- TaskBar: タイムライン上に配置されるタスク。進捗に応じて色が変わる。
- InteractionLayer: ドラッグによる新規作成を検知する透明なレイヤー。
