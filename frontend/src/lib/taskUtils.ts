import { type Task, type User } from './types';

export function sortTasksByStartAt(tasks: Task[]): Task[] {
  return [...tasks].sort((a, b) => new Date(a.start_at).getTime() - new Date(b.start_at).getTime());
}

export function upsertTask(users: User[], task: Task): User[] {
  const previousOwner = users.find((user) => user.tasks.some((candidate) => candidate.id === task.id));

  return users.map((user) => {
    let nextTasks = user.tasks;
    let changed = false;

    if (previousOwner && previousOwner.id !== task.member_id && user.id === previousOwner.id) {
      nextTasks = nextTasks.filter((candidate) => candidate.id !== task.id);
      changed = true;
    }

    if (user.id === task.member_id) {
      const taskIndex = nextTasks.findIndex((candidate) => candidate.id === task.id);
      if (taskIndex !== -1) {
        nextTasks = [...nextTasks];
        nextTasks[taskIndex] = task;
      } else {
        nextTasks = [...nextTasks, task];
      }
      changed = true;
    }

    if (!changed) {
      return user;
    }

    return { ...user, tasks: sortTasksByStartAt(nextTasks) };
  });
}
