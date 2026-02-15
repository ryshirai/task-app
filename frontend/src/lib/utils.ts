// src/lib/utils.ts
import { type Task } from './types';

export const START_HOUR = 9;
export const END_HOUR = 18;
export const TOTAL_MINUTES = (END_HOUR - START_HOUR) * 60; // 540 minutes

export function formatTime(date: Date): string {
    return date.toLocaleTimeString('ja-JP', { hour: '2-digit', minute: '2-digit', hour12: false });
}

export function toLocalISOString(date: Date): string {
    const year = date.getFullYear();
    const month = String(date.getMonth() + 1).padStart(2, '0');
    const day = String(date.getDate()).padStart(2, '0');
    const hours = String(date.getHours()).padStart(2, '0');
    const minutes = String(date.getMinutes()).padStart(2, '0');
    const seconds = String(date.getSeconds()).padStart(2, '0');
    const milliseconds = String(date.getMilliseconds()).padStart(3, '0');

    const offsetMinutes = -date.getTimezoneOffset();
    const sign = offsetMinutes >= 0 ? '+' : '-';
    const absOffsetMinutes = Math.abs(offsetMinutes);
    const offsetHours = String(Math.floor(absOffsetMinutes / 60)).padStart(2, '0');
    const offsetMins = String(absOffsetMinutes % 60).padStart(2, '0');

    return `${year}-${month}-${day}T${hours}:${minutes}:${seconds}.${milliseconds}${sign}${offsetHours}:${offsetMins}`;
}

export function getPercentage(date: Date): number {
    const startOfDay = new Date(date);
    startOfDay.setHours(START_HOUR, 0, 0, 0);

    const diffMs = date.getTime() - startOfDay.getTime();
    const diffMinutes = diffMs / (1000 * 60);

    return (diffMinutes / TOTAL_MINUTES) * 100;
}

export function getTaskPosition(task: Task): { left: number; width: number } {
    const start = new Date(task.start_at);
    const end = new Date(task.end_at);

    const startPercent = getPercentage(start);
    const endPercent = getPercentage(end);
    
    // Clamp values to be within the visible range (0-100%)
    const left = Math.max(0, Math.min(100, startPercent));
    const width = Math.max(0, Math.min(100 - left, endPercent - startPercent));

    return { left, width };
}

export function isTaskActive(task: Task, now: Date): boolean {
    const start = new Date(task.start_at);
    const end = new Date(task.end_at);
    return now >= start && now <= end;
}

export function xToTime(x: number, width: number, baseDate: Date): Date {
    const minutes = (x / width) * TOTAL_MINUTES;
    const date = new Date(baseDate);
    date.setHours(START_HOUR, 0, 0, 0);
    date.setMinutes(date.getMinutes() + minutes);
    return date;
}

export function percentageToDate(percent: number, baseDate: Date): Date {
    const minutes = (percent / 100) * TOTAL_MINUTES;
    const date = new Date(baseDate);
    date.setHours(START_HOUR, 0, 0, 0);
    date.setMinutes(date.getMinutes() + minutes);
    return date;
}

export function snapTo15Min(date: Date): Date {
    const minutes = date.getMinutes();
    const snappedMinutes = Math.round(minutes / 15) * 15;
    const snappedDate = new Date(date);
    snappedDate.setMinutes(snappedMinutes, 0, 0);
    return snappedDate;
}
