# Title
feat: adminによるユーザーロール管理機能の追加と即時反映の実現

# Summary
- 管理者が他ユーザーのロール（admin/member）を変更できる機能を追加。
- ミドルウェアの修正により、ログアウトなしで権限変更を即時反映。

# Changes
- Backend: `PUT /api/users/:id/role` エンドポイントの追加。
- Backend: 認証ミドルウェアでDBから最新ロールを取得するよう修正。
- Frontend: ユーザー管理モーダルにロール切り替えUIを追加。
- Tests: ロール管理と即時反映の結合テストを追加。

# Verification
- `admin_can_update_member_role`: adminが他ユーザーのロールを変更できること（`200 OK`）を確認。
- `member_cannot_update_role`: memberが他ユーザーのロールを変更できないこと（`403 FORBIDDEN`）を確認。
- `user_cannot_update_own_role`: ユーザー自身のロール変更が拒否されること（`403 FORBIDDEN`）を確認。
- `middleware_reflects_instant_role`: DB上のロール変更後、同一トークンでも再ログインなしで権限が即時反映されること（`403 FORBIDDEN -> 201 CREATED`）を確認。
