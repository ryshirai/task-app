import type { TaskTimeLog, User } from './types';

/**
 * Returns a new list of time logs sorted by ascending `start_at` timestamp.
 */
export function sortTimeLogsByStartAt(timeLogs: TaskTimeLog[]): TaskTimeLog[] {
  return [...timeLogs].sort(
    (a, b) => new Date(a.start_at).getTime() - new Date(b.start_at).getTime()
  );
}

/**
 * Inserts or updates a time log in the correct user and removes it from a previous owner if reassigned.
 *
 * Logic overview:
 * 1. Find the current owner of the incoming `timeLog` (if it already exists in any user's list).
 * 2. For each user:
 *    - Remove the log if this user is the previous owner and ownership changed.
 *    - Upsert the log if this user is the new owner.
 * 3. Only return a new user object when that user's `time_logs` changed.
 * 4. Keep changed `time_logs` sorted by `start_at`.
 */
export function upsertTimeLog(users: User[], timeLog: TaskTimeLog): User[] {
  const previousOwner = users.find((user) =>
    (user.time_logs || []).some((log) => log.id === timeLog.id)
  );

  return users.map((user) => {
    let nextTimeLogs = user.time_logs || [];
    let changed = false;

    // If the log was reassigned, remove it from the previous owner's list.
    if (previousOwner && previousOwner.id !== timeLog.user_id && user.id === previousOwner.id) {
      nextTimeLogs = nextTimeLogs.filter((log) => log.id !== timeLog.id);
      changed = true;
    }

    // Upsert the log for its current owner.
    if (user.id === timeLog.user_id) {
      const timeLogIndex = nextTimeLogs.findIndex((log) => log.id === timeLog.id);
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
