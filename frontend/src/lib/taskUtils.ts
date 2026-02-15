import { type TaskTimeLog, type User } from './types';

export function sortTimeLogsByStartAt(timeLogs: TaskTimeLog[]): TaskTimeLog[] {
  return [...timeLogs].sort((a, b) => new Date(a.start_at).getTime() - new Date(b.start_at).getTime());
}

export function upsertTimeLog(users: User[], timeLog: TaskTimeLog): User[] {
  const previousOwner = users.find((user) => (user.time_logs || []).some((candidate) => candidate.id === timeLog.id));

  return users.map((user) => {
    let nextTimeLogs = user.time_logs || [];
    let changed = false;

    if (previousOwner && previousOwner.id !== timeLog.user_id && user.id === previousOwner.id) {
      nextTimeLogs = nextTimeLogs.filter((candidate) => candidate.id !== timeLog.id);
      changed = true;
    }

    if (user.id === timeLog.user_id) {
      const timeLogIndex = nextTimeLogs.findIndex((candidate) => candidate.id === timeLog.id);
      if (timeLogIndex !== -1) {
        nextTimeLogs = [...nextTimeLogs];
        nextTimeLogs[timeLogIndex] = timeLog;
      } else {
        nextTimeLogs = [...nextTimeLogs, timeLog];
      }
      changed = true;
    }

    if (!changed) {
      return user;
    }

    return { ...user, time_logs: sortTimeLogsByStartAt(nextTimeLogs) };
  });
}
