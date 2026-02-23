import { describe, expect, it } from 'vitest';
import { upsertTimeLog } from './taskUtils';
import type { TaskTimeLog, User } from './types';

function makeTimeLog(overrides: Partial<TaskTimeLog> = {}): TaskTimeLog {
  return {
    id: 1,
    organization_id: 10,
    user_id: 100,
    task_id: 900,
    start_at: '2026-02-15T09:00:00.000+00:00',
    end_at: '2026-02-15T10:00:00.000+00:00',
    duration_minutes: 60,
    task_title: 'Task',
    task_status: 'todo',
    task_progress_rate: 0,
    task_tags: [],
    ...overrides
  };
}

function makeUser(id: number, timeLogs: TaskTimeLog[] = []): User {
  return {
    id,
    organization_id: 10,
    name: `User ${id}`,
    username: `user${id}`,
    email_verified: 1,
    role: 'user',
    time_logs: timeLogs
  };
}

describe('upsertTimeLog', () => {
  it('adds a new time log to the target member', () => {
    const users = [makeUser(100), makeUser(200)];
    const newTimeLog = makeTimeLog({ id: 101, user_id: 100 });

    const nextUsers = upsertTimeLog(users, newTimeLog);

    expect(nextUsers[0].time_logs).toHaveLength(1);
    expect(nextUsers[0].time_logs![0].id).toBe(101);
    expect(nextUsers[1].time_logs).toHaveLength(0);
  });

  it('keeps untouched members as the same object reference when adding', () => {
    const users = [makeUser(100), makeUser(200)];
    const newTimeLog = makeTimeLog({ id: 111, user_id: 100 });

    const nextUsers = upsertTimeLog(users, newTimeLog);

    expect(nextUsers[1]).toBe(users[1]);
  });

  it('updates an existing time log in-place for the same member', () => {
    const existingTimeLog = makeTimeLog({ id: 42, task_title: 'Before', user_id: 100 });
    const users = [makeUser(100, [existingTimeLog]), makeUser(200)];
    const updatedTimeLog = makeTimeLog({
      id: 42,
      task_title: 'After',
      user_id: 100,
      task_progress_rate: 80
    });

    const nextUsers = upsertTimeLog(users, updatedTimeLog);

    expect(nextUsers[0].time_logs).toHaveLength(1);
    expect(nextUsers[0].time_logs![0].task_title).toBe('After');
    expect(nextUsers[0].time_logs![0].task_progress_rate).toBe(80);
  });

  it('moves a time log from one member to another', () => {
    const movedTimeLog = makeTimeLog({ id: 77, user_id: 200 });
    const users = [makeUser(100, [makeTimeLog({ id: 77, user_id: 100 })]), makeUser(200)];

    const nextUsers = upsertTimeLog(users, movedTimeLog);

    expect(nextUsers[0].time_logs).toHaveLength(0);
    expect(nextUsers[1].time_logs).toHaveLength(1);
    expect(nextUsers[1].time_logs![0].user_id).toBe(200);
    expect(nextUsers[1].time_logs![0].id).toBe(77);
  });

  it('always sorts member time logs by start_at ascending', () => {
    const lateTimeLog = makeTimeLog({
      id: 2,
      user_id: 100,
      start_at: '2026-02-15T11:00:00.000+00:00'
    });
    const earlyTimeLog = makeTimeLog({
      id: 3,
      user_id: 100,
      start_at: '2026-02-15T09:30:00.000+00:00'
    });
    const users = [makeUser(100, [lateTimeLog]), makeUser(200)];

    const nextUsers = upsertTimeLog(users, earlyTimeLog);

    expect(nextUsers[0].time_logs!.map((timeLog) => timeLog.id)).toEqual([3, 2]);
  });

  it('preserves sorting when updating an existing log', () => {
    const firstLog = makeTimeLog({
      id: 10,
      user_id: 100,
      start_at: '2026-02-15T09:00:00.000+00:00'
    });
    const secondLog = makeTimeLog({
      id: 20,
      user_id: 100,
      start_at: '2026-02-15T10:00:00.000+00:00'
    });
    const users = [makeUser(100, [firstLog, secondLog])];

    const updatedSecondLog = makeTimeLog({
      id: 20,
      user_id: 100,
      start_at: '2026-02-15T10:00:00.000+00:00',
      task_title: 'updated'
    });

    const nextUsers = upsertTimeLog(users, updatedSecondLog);

    expect(nextUsers[0].time_logs!.map((timeLog) => timeLog.id)).toEqual([10, 20]);
    expect(nextUsers[0].time_logs![1].task_title).toBe('updated');
  });
});
