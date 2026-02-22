BEGIN;

SET TIME ZONE 'Asia/Tokyo';

-- ========================================
-- 0) Idempotent cleanup (seed-specific only)
-- ========================================
DELETE FROM display_group_members
WHERE group_id IN (
    SELECT id FROM display_groups
    WHERE organization_id = 1
      AND name IN ('エンジニアチーム', 'プロダクトチーム')
);

DELETE FROM display_groups
WHERE organization_id = 1
  AND name IN ('エンジニアチーム', 'プロダクトチーム');

DELETE FROM tasks
WHERE organization_id = 1
  AND title LIKE '[Seed] %';

DELETE FROM users
WHERE organization_id = 1
  AND username IN ('admin', 'dev_a', 'dev_b', 'design_c', 'pm_d', 'qa_e');

-- ========================================
-- 1) Organization
-- ========================================
INSERT INTO organizations (id, name)
VALUES (1, 'Demo Org')
ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name;

-- ========================================
-- 2) Users (1 admin + 5 members)
--    NOTE: password_hash is temporary Argon2-formatted placeholder.
-- ========================================
INSERT INTO users (organization_id, name, username, email, password_hash, role)
VALUES
    (1, '管理者', 'admin', 'admin@example.com', '$argon2id$v=19$m=65536,t=3,p=4$Y2hhbmdlX21lX3NhbHQ$0mSx9iG6m7Bf66X6v9fN6O9xN4fG1k4JYf2x7nq9n6U', 'admin'),
    (1, '開発 A', 'dev_a', 'dev.a@example.com', '$argon2id$v=19$m=65536,t=3,p=4$Y2hhbmdlX21lX3NhbHQ$0mSx9iG6m7Bf66X6v9fN6O9xN4fG1k4JYf2x7nq9n6U', 'user'),
    (1, '開発 B', 'dev_b', 'dev.b@example.com', '$argon2id$v=19$m=65536,t=3,p=4$Y2hhbmdlX21lX3NhbHQ$0mSx9iG6m7Bf66X6v9fN6O9xN4fG1k4JYf2x7nq9n6U', 'user'),
    (1, 'デザイン C', 'design_c', 'design.c@example.com', '$argon2id$v=19$m=65536,t=3,p=4$Y2hhbmdlX21lX3NhbHQ$0mSx9iG6m7Bf66X6v9fN6O9xN4fG1k4JYf2x7nq9n6U', 'user'),
    (1, 'PM D', 'pm_d', 'pm.d@example.com', '$argon2id$v=19$m=65536,t=3,p=4$Y2hhbmdlX21lX3NhbHQ$0mSx9iG6m7Bf66X6v9fN6O9xN4fG1k4JYf2x7nq9n6U', 'user'),
    (1, 'QA E', 'qa_e', 'qa.e@example.com', '$argon2id$v=19$m=65536,t=3,p=4$Y2hhbmdlX21lX3NhbHQ$0mSx9iG6m7Bf66X6v9fN6O9xN4fG1k4JYf2x7nq9n6U', 'user');

-- ========================================
-- 3) Display Groups
--    owner: 管理者 (admin)
-- ========================================
WITH owner_user AS (
    SELECT id FROM users WHERE organization_id = 1 AND username = 'admin'
), created_groups AS (
    INSERT INTO display_groups (organization_id, user_id, name)
    SELECT 1, owner_user.id, g.name
    FROM owner_user
    CROSS JOIN (VALUES
        ('エンジニアチーム'),
        ('プロダクトチーム')
    ) AS g(name)
    RETURNING id, name
)
INSERT INTO display_group_members (group_id, member_id)
SELECT cg.id, u.id
FROM created_groups cg
JOIN users u ON u.organization_id = 1
WHERE (cg.name = 'エンジニアチーム' AND u.username IN ('dev_a', 'dev_b'))
   OR (cg.name = 'プロダクトチーム' AND u.username IN ('pm_d', 'design_c'));

-- ========================================
-- 4) Tags
-- ========================================
INSERT INTO tags (organization_id, name)
VALUES
    (1, 'Feature'),
    (1, 'Bug'),
    (1, 'Refactor'),
    (1, 'Design'),
    (1, 'Meeting')
ON CONFLICT (organization_id, name) DO NOTHING;

-- ========================================
-- 5) Tasks
--    2-3 tasks per user, mixed statuses
-- ========================================
INSERT INTO tasks (organization_id, member_id, title, status, progress_rate)
VALUES
    (1, (SELECT id FROM users WHERE organization_id = 1 AND username = 'admin'), '[Seed] 全体進捗レビュー', 'doing', 60),
    (1, (SELECT id FROM users WHERE organization_id = 1 AND username = 'admin'), '[Seed] 週次KPI整理', 'todo', 0),
    (1, (SELECT id FROM users WHERE organization_id = 1 AND username = 'admin'), '[Seed] 権限棚卸し', 'done', 100),

    (1, (SELECT id FROM users WHERE organization_id = 1 AND username = 'dev_a'), '[Seed] API レスポンス最適化', 'doing', 70),
    (1, (SELECT id FROM users WHERE organization_id = 1 AND username = 'dev_a'), '[Seed] バックエンドテスト拡充', 'todo', 20),
    (1, (SELECT id FROM users WHERE organization_id = 1 AND username = 'dev_a'), '[Seed] 認証バグ修正', 'done', 100),

    (1, (SELECT id FROM users WHERE organization_id = 1 AND username = 'dev_b'), '[Seed] WebSocket再接続改善', 'doing', 50),
    (1, (SELECT id FROM users WHERE organization_id = 1 AND username = 'dev_b'), '[Seed] バッチ処理リファクタ', 'done', 100),

    (1, (SELECT id FROM users WHERE organization_id = 1 AND username = 'design_c'), '[Seed] ダッシュボード配色調整', 'doing', 65),
    (1, (SELECT id FROM users WHERE organization_id = 1 AND username = 'design_c'), '[Seed] アイコン整備', 'todo', 10),
    (1, (SELECT id FROM users WHERE organization_id = 1 AND username = 'design_c'), '[Seed] オンボーディング画面改修', 'done', 100),

    (1, (SELECT id FROM users WHERE organization_id = 1 AND username = 'pm_d'), '[Seed] 仕様優先度見直し', 'doing', 40),
    (1, (SELECT id FROM users WHERE organization_id = 1 AND username = 'pm_d'), '[Seed] スプリント計画作成', 'done', 100),

    (1, (SELECT id FROM users WHERE organization_id = 1 AND username = 'qa_e'), '[Seed] 回帰テスト実施', 'doing', 55),
    (1, (SELECT id FROM users WHERE organization_id = 1 AND username = 'qa_e'), '[Seed] 不具合再現手順整理', 'todo', 15),
    (1, (SELECT id FROM users WHERE organization_id = 1 AND username = 'qa_e'), '[Seed] E2Eシナリオ更新', 'done', 100);

-- ========================================
-- 5.1) Task-Tag relations
-- ========================================
INSERT INTO task_tags (task_id, tag_id)
SELECT t.id, tg.id
FROM (
    VALUES
        ('[Seed] 全体進捗レビュー', 'Meeting'),
        ('[Seed] 週次KPI整理', 'Feature'),
        ('[Seed] 権限棚卸し', 'Refactor'),

        ('[Seed] API レスポンス最適化', 'Refactor'),
        ('[Seed] バックエンドテスト拡充', 'Feature'),
        ('[Seed] 認証バグ修正', 'Bug'),

        ('[Seed] WebSocket再接続改善', 'Feature'),
        ('[Seed] WebSocket再接続改善', 'Refactor'),
        ('[Seed] バッチ処理リファクタ', 'Refactor'),

        ('[Seed] ダッシュボード配色調整', 'Design'),
        ('[Seed] アイコン整備', 'Design'),
        ('[Seed] オンボーディング画面改修', 'Design'),

        ('[Seed] 仕様優先度見直し', 'Meeting'),
        ('[Seed] スプリント計画作成', 'Meeting'),

        ('[Seed] 回帰テスト実施', 'Bug'),
        ('[Seed] 不具合再現手順整理', 'Bug'),
        ('[Seed] E2Eシナリオ更新', 'Feature')
) AS m(task_title, tag_name)
JOIN tasks t
  ON t.organization_id = 1
 AND t.title = m.task_title
JOIN tags tg
  ON tg.organization_id = 1
 AND tg.name = m.tag_name
ON CONFLICT DO NOTHING;

-- ========================================
-- 5.2) Task Time Logs (today: 2026-02-22 JST)
--    1-2 logs per user; some doing tasks are overdue around 10:00 JST
-- ========================================
INSERT INTO task_time_logs (organization_id, user_id, task_id, start_at, end_at)
VALUES
    -- 管理者
    (1,
      (SELECT id FROM users WHERE organization_id = 1 AND username = 'admin'),
      (SELECT id FROM tasks WHERE organization_id = 1 AND title = '[Seed] 全体進捗レビュー'),
      '2026-02-22 08:30:00+09', '2026-02-22 09:20:00+09'),
    (1,
      (SELECT id FROM users WHERE organization_id = 1 AND username = 'admin'),
      (SELECT id FROM tasks WHERE organization_id = 1 AND title = '[Seed] 権限棚卸し'),
      '2026-02-22 09:30:00+09', '2026-02-22 10:00:00+09'),

    -- 開発 A
    (1,
      (SELECT id FROM users WHERE organization_id = 1 AND username = 'dev_a'),
      (SELECT id FROM tasks WHERE organization_id = 1 AND title = '[Seed] API レスポンス最適化'),
      '2026-02-22 08:45:00+09', '2026-02-22 09:30:00+09'),
    (1,
      (SELECT id FROM users WHERE organization_id = 1 AND username = 'dev_a'),
      (SELECT id FROM tasks WHERE organization_id = 1 AND title = '[Seed] 認証バグ修正'),
      '2026-02-22 09:40:00+09', '2026-02-22 10:10:00+09'),

    -- 開発 B
    (1,
      (SELECT id FROM users WHERE organization_id = 1 AND username = 'dev_b'),
      (SELECT id FROM tasks WHERE organization_id = 1 AND title = '[Seed] WebSocket再接続改善'),
      '2026-02-22 09:00:00+09', '2026-02-22 09:50:00+09'),
    (1,
      (SELECT id FROM users WHERE organization_id = 1 AND username = 'dev_b'),
      (SELECT id FROM tasks WHERE organization_id = 1 AND title = '[Seed] バッチ処理リファクタ'),
      '2026-02-22 10:10:00+09', '2026-02-22 11:10:00+09'),

    -- デザイン C
    (1,
      (SELECT id FROM users WHERE organization_id = 1 AND username = 'design_c'),
      (SELECT id FROM tasks WHERE organization_id = 1 AND title = '[Seed] ダッシュボード配色調整'),
      '2026-02-22 09:20:00+09', '2026-02-22 11:00:00+09'),
    (1,
      (SELECT id FROM users WHERE organization_id = 1 AND username = 'design_c'),
      (SELECT id FROM tasks WHERE organization_id = 1 AND title = '[Seed] オンボーディング画面改修'),
      '2026-02-22 08:30:00+09', '2026-02-22 09:10:00+09'),

    -- PM D
    (1,
      (SELECT id FROM users WHERE organization_id = 1 AND username = 'pm_d'),
      (SELECT id FROM tasks WHERE organization_id = 1 AND title = '[Seed] 仕様優先度見直し'),
      '2026-02-22 08:50:00+09', '2026-02-22 09:40:00+09'),
    (1,
      (SELECT id FROM users WHERE organization_id = 1 AND username = 'pm_d'),
      (SELECT id FROM tasks WHERE organization_id = 1 AND title = '[Seed] スプリント計画作成'),
      '2026-02-22 09:45:00+09', '2026-02-22 10:30:00+09'),

    -- QA E
    (1,
      (SELECT id FROM users WHERE organization_id = 1 AND username = 'qa_e'),
      (SELECT id FROM tasks WHERE organization_id = 1 AND title = '[Seed] 回帰テスト実施'),
      '2026-02-22 09:10:00+09', '2026-02-22 09:55:00+09'),
    (1,
      (SELECT id FROM users WHERE organization_id = 1 AND username = 'qa_e'),
      (SELECT id FROM tasks WHERE organization_id = 1 AND title = '[Seed] E2Eシナリオ更新'),
      '2026-02-22 10:00:00+09', '2026-02-22 10:40:00+09');

-- Keep sequence in sync in case id=1 was inserted explicitly.
SELECT setval('organizations_id_seq', (SELECT GREATEST(COALESCE(MAX(id), 1), 1) FROM organizations));

COMMIT;
