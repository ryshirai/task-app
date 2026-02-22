# 本番デプロイ チェックリスト & タスクロードマップ

Rust (Axum) バックエンド、SvelteKit フロントエンド、PostgreSQL を AWS 本番環境へデプロイするための実行計画。

## 1. 想定インフラ構成（AWS）

### 1.1 構成概要
- VPC: `public`/`private` サブネットを 2AZ 以上で構成
- Compute:
  - Backend: ECS Fargate または App Runner
  - Frontend: S3 + CloudFront（静的配信。必要に応じて SSR は ECS/App Runner へ）
- Database: Amazon RDS for PostgreSQL（Multi-AZ 推奨）
- DNS/証明書: Route 53 + ACM
- Edge Security: AWS WAF（CloudFront または ALB にアタッチ）
- Secrets: AWS Secrets Manager / SSM Parameter Store
- Logging/Monitoring: CloudWatch + エラートラッキング（Sentry 等）

### 1.2 ネットワーク設計チェック
- [ ] VPC CIDR とサブネット CIDR を定義（将来拡張を考慮）
- [ ] `public subnet` に ALB/NAT、`private subnet` に ECS/RDS を配置
- [ ] AZ 障害に備えて 2AZ 以上で冗長化
- [ ] VPC エンドポイント（S3, ECR, CloudWatch Logs, SSM）利用可否を決定
- [ ] NAT Gateway のコスト最適化方針（常時/時間帯/環境別）を定義

## 2. セキュリティ実装チェックリスト

### 2.1 SSL/TLS
- [ ] ACM で証明書発行（本番ドメイン・サブドメイン）
- [ ] CloudFront/ALB で HTTPS 強制（HTTP -> HTTPS リダイレクト）
- [ ] TLS ポリシーを最新推奨に設定（TLS 1.2+）
- [ ] HSTS ヘッダーを有効化

### 2.2 IAM
- [ ] `least privilege` でロール設計（ECS タスクロール、GitHub Actions 用ロール）
- [ ] 長期アクセスキーを廃止し OIDC（GitHub Actions -> AWS）を採用
- [ ] 本番 IAM ロールに MFA・権限境界・監査ログを適用
- [ ] 管理者権限の常用を禁止し、運用ロールを分離

### 2.3 Security Groups / NACL
- [ ] RDS はアプリケーション SG からの 5432 のみ許可
- [ ] ECS/App Runner への Ingress を ALB/CloudFront 経由に限定
- [ ] 不要ポート・全開放ルール（0.0.0.0/0）を除去
- [ ] Egress 制限方針（必要な宛先のみ）を定義

### 2.4 WAF
- [ ] AWS Managed Rules を有効化（SQLi, XSS, Known bad inputs）
- [ ] レート制限ルールを追加（API/ログイン系）
- [ ] Bot 制御の要否を判定
- [ ] WAF ログを S3/CloudWatch に保存し定期レビュー

## 3. CI/CD パイプライン（GitHub Actions）

### 3.1 ブランチ戦略
- [ ] `main` を本番デプロイ専用に定義
- [ ] `develop`/feature ブランチで検証後に `main` へマージ
- [ ] 保護ルール（必須レビュー、必須ステータスチェック）を設定

### 3.2 Pipeline ステージ
- [ ] Backend CI: `cargo fmt --check`, `cargo clippy`, `cargo test`
- [ ] Frontend CI: `npm ci`, `npm run lint`, `npm run test`, `npm run build`
- [ ] SAST/Dependency scan: `cargo audit`, `npm audit` または専用ツール
- [ ] Docker build（backend/frontend 必要に応じて）と脆弱性スキャン
- [ ] ECR へ push（タグ: `sha`, `semver`, `latest` の運用ルール策定）
- [ ] CD: 本番デプロイ（ECS service update / App Runner deploy / S3 sync + CloudFront invalidation）
- [ ] デプロイ後の smoke test（ヘルスチェック API / 主要画面ロード）

### 3.3 デプロイ安全策
- [ ] Blue/Green もしくは Rolling update を定義
- [ ] 失敗時の自動ロールバック条件を明文化
- [ ] DB マイグレーションの実行順序を CI/CD に固定
- [ ] 手動承認ステップ（本番のみ）を導入

## 4. 環境変数・シークレット管理

### 4.1 分離ルール
- [ ] `dev/stg/prod` で値を完全分離
- [ ] `.env` を本番で使用せず、Secrets Manager/SSM に集約
- [ ] 命名規約を統一（例: `GLANCEFLOW_<SERVICE>_<KEY>`）

### 4.2 最低限必要な変数（例）
- [ ] Backend: `DATABASE_URL`, `JWT_SECRET`, `RUST_LOG`, `CORS_ORIGIN`
- [ ] Frontend: `PUBLIC_API_BASE_URL`, `PUBLIC_APP_ENV`
- [ ] 共通: `SENTRY_DSN`, `AWS_REGION`

### 4.3 運用
- [ ] シークレットローテーション手順を定義（四半期など）
- [ ] 変更監査（誰がいつ変更したか）を有効化
- [ ] 起動時必須変数チェックを実装（欠落時 fail-fast）

## 5. モニタリング・ロギング

### 5.1 CloudWatch
- [ ] Backend 構造化ログ（JSON）を CloudWatch Logs に集約
- [ ] 主要メトリクスを可視化（CPU, Memory, 5xx, p95 latency）
- [ ] アラーム設定（高エラー率、DB 接続枯渇、レイテンシ悪化）
- [ ] アラート通知先（Slack/SNS/PagerDuty）を設定

### 5.2 Error Tracking
- [ ] Backend/Frontend に Sentry 等を導入
- [ ] release バージョンを CI で自動付与
- [ ] PII マスキング・送信制御を適用
- [ ] 重大度分類とオンコール手順を定義

### 5.3 監視対象の最小セット
- [ ] 稼働率（SLA/SLO）
- [ ] API エラー率（4xx/5xx）
- [ ] 応答時間（p50/p95/p99）
- [ ] DB 指標（接続数、CPU、ストレージ、遅いクエリ）

## 6. データベース移行（Migration）戦略

### 6.1 方針
- [ ] マイグレーションツールを固定（`sqlx migrate` など）
- [ ] `expand/contract`（後方互換）を原則化
- [ ] 破壊的変更（カラム削除・型変更）は 2 段階以上で実施
- [ ] すべてのマイグレーションにロールバック方針を記載

### 6.2 実行フロー
1. [ ] Staging で migration + 負荷/回帰テスト
2. [ ] 本番デプロイ前に DB バックアップ（スナップショット）取得
3. [ ] アプリ先行/後行の順序を変更内容ごとに定義
4. [ ] デプロイ直後に DB ヘルス確認（接続・クエリ・ロック）
5. [ ] 問題時の即時ロールバック手順を実行可能な形で保存

### 6.3 禁止事項
- [ ] ピーク時間帯の長時間ロック DDL
- [ ] 検証なしの手動 SQL 直接実行
- [ ] バックアップ未取得での本番 migration

## 7. ドメイン・DNS（Route 53）

### 7.1 レコード設計
- [ ] `api.example.com` -> ALB/App Runner
- [ ] `app.example.com` -> CloudFront
- [ ] `A/AAAA Alias` を優先し CNAME を最小化
- [ ] TTL を運用方針に合わせて設定（切替前は短め）

### 7.2 証明書・検証
- [ ] ACM DNS 検証を Route 53 で自動化
- [ ] 更新失敗監視を設定
- [ ] 複数環境の証明書とドメインを混在させない

### 7.3 切替計画
- [ ] 本番切替前にステージング相当ドメインで総合検証
- [ ] カットオーバー手順書を作成（担当者・時刻・判定基準）
- [ ] ロールバック DNS 手順（旧系への戻し）を準備

## 8. フェーズ別タスクロードマップ

### Phase 0: 事前設計（1 週間）
- [ ] AWS アカウント/組織構成と責任分界を確定
- [ ] IaC 方針（Terraform/CDK/CloudFormation）を決定
- [ ] 命名規約、タグ規約、コスト配賦ルールを策定
- [ ] SLO/SLA と運用体制（障害対応時間）を確定

### Phase 1: 基盤構築（1-2 週間）
- [ ] VPC, Subnet, SG, IAM, RDS, S3, CloudFront, Route 53 を IaC で構築
- [ ] ECS/App Runner の最小起動構成を作成
- [ ] Secrets Manager/SSM の導線を構築
- [ ] ACM + WAF を有効化

### Phase 2: CI/CD 構築（1 週間）
- [ ] GitHub Actions で CI ワークフロー作成
- [ ] OIDC 連携で AWS デプロイ権限を付与
- [ ] CD（staging -> production）と承認フローを構築
- [ ] 自動 smoke test とロールバック導線を追加

### Phase 3: 可観測性/運用整備（3-5 日）
- [ ] CloudWatch Dashboard/Alarm を作成
- [ ] Sentry 等のエラートラッキング導入
- [ ] Runbook（障害時対応・復旧手順）を整備
- [ ] バックアップ/復元演習を 1 回実施

### Phase 4: 本番リリース（2-3 日）
- [ ] リリース判定会（Go/No-Go）を実施
- [ ] DNS 切替、モニタリング強化、初期監視を実施
- [ ] 24-48 時間の重点監視期間を設定
- [ ] ポストモーテム/改善タスクを backlog 化

## 9. Go-Live 判定チェック（最終ゲート）
- [ ] 重大脆弱性（Critical/High）が未解決でない
- [ ] バックアップ復元テストが成功している
- [ ] 監視アラートが有効で通知テスト済み
- [ ] デプロイ/ロールバックが手順書通り再現可能
- [ ] ドメイン証明書が有効で HTTPS 強制済み
- [ ] 責任者・連絡経路・当番体制が明確

## 10. 実装メモ（推奨）
- ECS vs App Runner は以下で判断:
  - ECS: ネットワーク制御・拡張性・運用自由度重視
  - App Runner: 構築速度・運用負荷軽減重視
- SvelteKit が完全静的化できるなら S3 + CloudFront を優先。SSR 必須の場合は Node 実行基盤（ECS/App Runner）を採用。
- まず staging を本番同等構成で作り、検証完了後に prod へ昇格する運用を固定する。
