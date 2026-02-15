#!/bin/bash

API_URL="http://localhost:3000/api"
DATE="2026-02-15"

add_task() {
  local member_id="$1"
  local title="$2"
  local start_time="$3"
  local end_time="$4"
  local status="$5"
  local progress="$6"

  local start_at="${DATE}T${start_time}:00+09:00"
  local end_at="${DATE}T${end_time}:00+09:00"
  
  local json_data="{\"member_id\": $member_id, \"title\": \"$title\", \"start_at\": \"$start_at\", \"end_at\": \"$end_at\"}"
  
  echo "Creating task '$title' for member $member_id..."
  
  local response
  response=$(curl -s -X POST -H "Content-Type: application/json" -d "$json_data" "$API_URL/tasks")
  
  # Basic extraction of ID
  local task_id
  task_id=$(echo "$response" | grep -o '"id":[0-9]*' | head -1 | cut -d':' -f2)
  
  if [ -n "$task_id" ]; then
    echo "  Task created with ID: $task_id"
    
    if [ "$status" != "todo" ] || [ "$progress" -ne 0 ]; then
       local update_json="{\"status\": \"$status\", \"progress_rate\": $progress}"
       curl -s -X PATCH -H "Content-Type: application/json" -d "$update_json" "$API_URL/tasks/$task_id" > /dev/null
    fi
  else
    echo "  Failed to create task. Response: $response"
  fi
  echo ""
}

# Calls
add_task 1 "朝会" "09:00" "09:30" "done" 100
add_task 1 "API設計" "09:30" "11:30" "doing" 60
add_task 1 "DBマイグレーション作成" "13:00" "14:30" "todo" 0

add_task 2 "フロントエンド基盤構築" "10:00" "12:00" "done" 100
add_task 2 "コンポーネント実装" "13:00" "16:00" "doing" 40

add_task 3 "要件定義書レビュー" "11:00" "12:00" "done" 100
add_task 3 "顧客定例MTG" "14:00" "15:00" "todo" 0
add_task 3 "議事録作成" "15:00" "15:30" "todo" 0
