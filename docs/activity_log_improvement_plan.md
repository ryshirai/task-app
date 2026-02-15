# Activity Log System Enhancement Proposal

## Purpose
The Activity Log is critical for traceability, compliance, and operational debugging. This proposal defines a phased enhancement plan to improve audit depth, usability, governance controls, and scalability so teams can investigate events faster and administrators can manage logs more effectively over time.

## Proposed Features

| Area | Enhancement | Description | Priority |
|---|---|---|---|
| Advanced Filtering & Search | Multi-dimensional filtering | Add filters for User, Action Type, and Date Range with combinational query support. | High |
| Advanced Filtering & Search | Full-text search support | Enable indexed text search across key log attributes for fast discovery. | High |
| Detail Enhancements | Diff view for changes | Show before/after field-level differences for update events to reduce investigation time. | High |
| Detail Enhancements | Deep links to related entities | Add direct links from log entries to tasks, reports, and related objects for one-click navigation. | Medium |
| Admin & Governance | Export to CSV | Allow authorized admins to export filtered log data for audits and offline analysis. | Medium |
| Admin & Governance | Log retention policy controls | Configure retention windows by environment/compliance policy and support automated archival/purge. | High |
| Real-time UX | Live updates via WebSocket | Stream new log entries in near real-time without page refresh for operations visibility. | Medium |
| Performance Optimization | Database indexing strategy | Add/optimize indexes on high-cardinality and frequently queried columns (e.g., `user_id`, `action_type`, `created_at`). | High |

## Technical Strategy

### 1. Advanced Filtering & Search
- Introduce a query API contract with optional parameters: `user_id`, `action_type`, `start_date`, `end_date`, `q`.
- Use validated server-side query builders to prevent inefficient scans and enforce authorization boundaries.
- Add pagination defaults and cursor-based pagination for large result sets.

### 2. Detail Enhancements
- Persist structured change snapshots for mutable entities to support deterministic diff rendering.
- Implement a reusable diff component highlighting added, removed, and modified fields.
- Add entity metadata (`entity_type`, `entity_id`, `entity_url`) in log payloads for deep-link routing.

### 3. Admin & Governance
- Build role-gated export endpoint with scoped filters and asynchronous job processing for large exports.
- Produce RFC 4180-compliant CSV output and include export audit entries for accountability.
- Implement retention scheduler (daily job) with policy-based archival and deletion lifecycle.

### 4. Real-time UX
- Publish new log events through an event bus and broadcast via WebSocket channels.
- Support reconnect/backfill behavior using last-seen event timestamp or cursor.
- Include feature flags to roll out real-time updates gradually.

### 5. Performance Optimization
- Create composite indexes aligned to common filters, such as (`created_at`, `action_type`) and (`user_id`, `created_at`).
- Evaluate partitioning strategy for very large tables (time-based partitions) if growth warrants.
- Define SLOs for query latency and monitor with dashboards/alerts.

### Delivery Approach
1. Phase 1 (High Priority): Filtering/Search, Diff View, Retention Controls, Core Indexing.
2. Phase 2 (Medium Priority): Deep Links, CSV Export, WebSocket Live Feed.
3. Phase 3 (Scale Hardening): Partitioning, advanced monitoring, and tuning based on production usage.

### Success Metrics
- 50% reduction in median investigation time for support/admin teams.
- 95th percentile log query latency under defined SLO.
- 100% policy-compliant retention execution across environments.
- Increased admin adoption of export and deep-link workflows.
