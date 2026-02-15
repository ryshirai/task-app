import { describe, expect, it } from 'vitest';
import { upsertTask } from './taskUtils';
import type { Task, User } from './types';

function makeTask(overrides: Partial<Task> = {}): Task {
  return {
    id: 1,
    organization_id: 10,
    member_id: 100,
    title: 'Task',
    status: 'todo',
    progress_rate: 0,
    tags: [],
    start_at: '2026-02-15T09:00:00.000+00:00',
    end_at: '2026-02-15T10:00:00.000+00:00',
    created_at: '2026-02-15T08:00:00.000+00:00',
    ...overrides
  };
}

function makeUser(id: number, tasks: Task[] = []): User {
  return {
    id,
    organization_id: 10,
    name: `User ${id}`,
    username: `user${id}`,
    role: 'user',
    tasks
  };
}

describe('upsertTask', () => {
  it('adds a new task to the target member', () => {
    const users = [makeUser(100), makeUser(200)];
    const newTask = makeTask({ id: 101, member_id: 100 });

    const nextUsers = upsertTask(users, newTask);

    expect(nextUsers[0].tasks).toHaveLength(1);
    expect(nextUsers[0].tasks[0].id).toBe(101);
    expect(nextUsers[1].tasks).toHaveLength(0);
  });

  it('updates an existing task in-place for the same member', () => {
    const existingTask = makeTask({ id: 42, title: 'Before', member_id: 100 });
    const users = [makeUser(100, [existingTask]), makeUser(200)];
    const updatedTask = makeTask({ id: 42, title: 'After', member_id: 100, progress_rate: 80 });

    const nextUsers = upsertTask(users, updatedTask);

    expect(nextUsers[0].tasks).toHaveLength(1);
    expect(nextUsers[0].tasks[0].title).toBe('After');
    expect(nextUsers[0].tasks[0].progress_rate).toBe(80);
  });

  it('moves a task from one member to another', () => {
    const movedTask = makeTask({ id: 77, member_id: 200 });
    const users = [makeUser(100, [makeTask({ id: 77, member_id: 100 })]), makeUser(200)];

    const nextUsers = upsertTask(users, movedTask);

    expect(nextUsers[0].tasks).toHaveLength(0);
    expect(nextUsers[1].tasks).toHaveLength(1);
    expect(nextUsers[1].tasks[0].member_id).toBe(200);
  });

  it('always sorts member tasks by start_at ascending', () => {
    const lateTask = makeTask({ id: 2, member_id: 100, start_at: '2026-02-15T11:00:00.000+00:00' });
    const earlyTask = makeTask({ id: 3, member_id: 100, start_at: '2026-02-15T09:30:00.000+00:00' });
    const users = [makeUser(100, [lateTask]), makeUser(200)];

    const nextUsers = upsertTask(users, earlyTask);

    expect(nextUsers[0].tasks.map((task) => task.id)).toEqual([3, 2]);
  });
});
