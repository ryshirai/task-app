# Interactive UX Specification: Drag-to-Create

## 1. 座標計算ロジック

タイムラインの横幅を `W`、表示時間範囲を `T_range`（例: 10時間 = 600分）とする。

- **時刻への変換:** `time = (click_x / W) * T_range + start_time`
- **スナッピング:** 算出された `time` を 15分単位（15, 30, 45, 00）に丸める。

## 2. 操作フロー

1. **MouseDown:** クリック位置から `start_at` を取得。プレビュー用の `div` を表示開始。
2. **MouseMove:** 現在のカーソル位置から `end_at` を動的に計算し、プレビューバーの幅を更新。
3. **MouseUp:** `end_at` を確定。その位置にインライン入力フォームを表示。
4. **Submit:** `Enter` キーで `POST /api/tasks` を実行。

## 3. 視覚的フィードバック

- ドラッグ中はバーを半透明の青色（`bg-blue-400/50`）にする。
- 現在時刻（Now Line）を1分ごとに更新し、赤い縦線で表示。
