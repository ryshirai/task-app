#!/usr/bin/env bash
set -euo pipefail

API_URL="http://localhost:3000/api"
ADMIN_USERNAME="admin"
ADMIN_PASSWORD="admin123"
DEV_PASSWORD="dev123"

TASK_TITLES=(
  "API implementation"
  "Frontend integration"
  "Fix bug ticket"
  "Review pull request"
  "Refactor module"
  "Write tests"
  "Update documentation"
  "Investigate incident"
  "Optimize query"
  "Sprint planning"
)

TASK_TAGS=(
  "backend"
  "frontend"
  "infra"
  "analytics"
  "testing"
  "bugfix"
  "feature"
  "ops"
)

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Required command not found: $1"
    exit 1
  fi
}

login_and_get_token() {
  local username="$1"
  local password="$2"
  local payload response token

  payload=$(jq -nc --arg username "$username" --arg password "$password" \
    '{username: $username, password: $password}')

  response=$(curl -sS -X POST \
    -H "Content-Type: application/json" \
    -d "$payload" \
    "$API_URL/auth/login")

  echo "Login Response for $username: $response"

  token=$(echo "$response" | jq -r '.token // empty')
  if [[ -z "$token" ]]; then
    echo "Login failed for '$username'. Response: $response"
    exit 1
  fi

  echo "$token"
}

create_user_attempt() {
  local token="$1"
  local name="$2"
  local username="$3"
  local password="$4"
  local role="${5:-user}"
  local payload http_code response

  payload=$(jq -nc \
    --arg name "$name" \
    --arg username "$username" \
    --arg password "$password" \
    --arg role "$role" \
    '{name: $name, username: $username, password: $password, role: $role}')

  response=$(curl -sS -w $'\n%{http_code}' -X POST \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $token" \
    -d "$payload" \
    "$API_URL/users")

  http_code=$(echo "$response" | tail -n1)
  response=$(echo "$response" | sed '$d')

  if [[ "$http_code" =~ ^2 ]]; then
    echo "User '$username' created."
  else
    echo "User '$username' create attempt returned HTTP $http_code (may already exist)."
    echo "Response: $response"
  fi
}

get_user_id_by_username() {
  local token="$1"
  local username="$2"
  local today id

  today=$(date +%F)
  id=$(curl -sS \
    -H "Authorization: Bearer $token" \
    "$API_URL/users?date=$today" \
    | jq -r --arg username "$username" '.[] | select(.username == $username) | .id' \
    | head -n1)

  if [[ -z "$id" ]]; then
    echo "Unable to resolve user ID for '$username'."
    exit 1
  fi

  echo "$id"
}

create_task_for_day() {
  local token="$1"
  local member_id="$2"
  local username="$3"
  local day="$4"
  local ordinal="$5"

  local statuses=("todo" "doing" "done")
  local status title tag1 tag2 progress_rate
  local start_hour start_min duration start_at end_at candidate_end
  local create_payload create_response task_id update_payload

  status="${statuses[$((RANDOM % ${#statuses[@]}))]}"
  title="${TASK_TITLES[$((RANDOM % ${#TASK_TITLES[@]}))]} #${ordinal} (${username})"
  tag1="${TASK_TAGS[$((RANDOM % ${#TASK_TAGS[@]}))]}"
  tag2="${TASK_TAGS[$((RANDOM % ${#TASK_TAGS[@]}))]}"

  start_hour=$((8 + RANDOM % 10))
  start_min=$(( (RANDOM % 2) * 30 ))
  duration=$((1 + RANDOM % 3))

  start_at=$(date -u -d "${day} ${start_hour}:${start_min}:00" +"%Y-%m-%dT%H:%M:%SZ")
  candidate_end=$(date -u -d "${day} ${start_hour}:${start_min}:00 +${duration} hour" +"%Y-%m-%dT%H:%M:%SZ")
  if [[ "${candidate_end:0:10}" == "$day" ]]; then
    end_at="$candidate_end"
  else
    end_at="${day}T23:59:00Z"
  fi

  case "$status" in
    done) progress_rate=100 ;;
    doing) progress_rate=$((20 + RANDOM % 71)) ;;
    todo) progress_rate=$((RANDOM % 16)) ;;
  esac

  create_payload=$(jq -nc \
    --argjson member_id "$member_id" \
    --arg title "$title" \
    --arg tag1 "$tag1" \
    --arg tag2 "$tag2" \
    '{member_id: $member_id, title: $title, tags: [$tag1, $tag2]}')

  create_response=$(curl -sS -X POST \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $token" \
    -d "$create_payload" \
    "$API_URL/tasks")

  task_id=$(echo "$create_response" | jq -r '.id // empty')
  if [[ -z "$task_id" ]]; then
    echo "Task create failed for user '$username' on $day. Response: $create_response"
    return
  fi

  # Add Time Log
  log_payload=$(jq -nc \
    --argjson user_id "$member_id" \
    --argjson task_id "$task_id" \
    --arg start_at "$start_at" \
    --arg end_at "$end_at" \
    '{user_id: $user_id, task_id: $task_id, start_at: $start_at, end_at: $end_at}')

  curl -sS -X POST \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $token" \
    -d "$log_payload" \
    "$API_URL/tasks/time-logs" >/dev/null

  if [[ "$status" == "done" ]]; then
    update_payload=$(jq -nc \
      --arg status "$status" \
      --argjson progress_rate "$progress_rate" \
      --arg end_at "$end_at" \
      '{status: $status, progress_rate: $progress_rate, end_at: $end_at}')
  else
    update_payload=$(jq -nc \
      --arg status "$status" \
      --argjson progress_rate "$progress_rate" \
      '{status: $status, progress_rate: $progress_rate}')
  fi

  curl -sS -X PATCH \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $token" \
    -d "$update_payload" \
    "$API_URL/tasks/$task_id" >/dev/null
}

create_report_for_day() {
  local token="$1"
  local day="$2"
  local content payload response http_code

  content="Daily summary for ${day}: completed implementation and review tasks, tracked blockers, and prepared next steps."
  payload=$(jq -nc --arg report_date "$day" --arg content "$content" \
    '{report_date: $report_date, content: $content}')

  response=$(curl -sS -w $'\n%{http_code}' -X POST \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $token" \
    -d "$payload" \
    "$API_URL/reports")

  http_code=$(echo "$response" | tail -n1)
  if [[ ! "$http_code" =~ ^2 ]]; then
    echo "Report create failed on $day (HTTP $http_code)."
  fi
}

main() {
  require_cmd curl
  require_cmd jq
  require_cmd date

  echo "1) Logging in as admin to obtain JWT..."
  local admin_token
  admin_token=$(login_and_get_token "$ADMIN_USERNAME" "$ADMIN_PASSWORD")

  echo "2) Creating additional users (attempts)..."
  create_user_attempt "$admin_token" "Developer One" "dev1" "$DEV_PASSWORD" "user"
  create_user_attempt "$admin_token" "Developer Two" "dev2" "$DEV_PASSWORD" "user"

  echo "3) Resolving user IDs..."
  local admin_id dev1_id dev2_id
  admin_id=$(get_user_id_by_username "$admin_token" "admin")
  dev1_id=$(get_user_id_by_username "$admin_token" "dev1")
  dev2_id=$(get_user_id_by_username "$admin_token" "dev2")

  declare -A USER_IDS=(
    ["admin"]="$admin_id"
    ["dev1"]="$dev1_id"
    ["dev2"]="$dev2_id"
  )

  echo "4) Generating historical tasks/reports for last 30 days through today..."
  for offset in $(seq 30 -1 0); do
    local day
    day=$(date -d "-${offset} days" +%F)
    echo "  - ${day}"

    for username in admin dev1 dev2; do
      local member_id task_count idx
      member_id="${USER_IDS[$username]}"
      task_count=$((1 + RANDOM % 3))

      for ((idx = 1; idx <= task_count; idx++)); do
        create_task_for_day "$admin_token" "$member_id" "$username" "$day" "$idx"
      done
    done

    if (( RANDOM % 10 < 8 )); then
      create_report_for_day "$admin_token" "$day"
    fi
  done

  echo "Done. Dummy analytics data generated successfully."
}

main "$@"
