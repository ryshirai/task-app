// src/lib/utils.ts
import { type Task, type TaskTimeLog } from './types';

export const START_HOUR = 9;
export const END_HOUR = 18;
export const TOTAL_MINUTES = (END_HOUR - START_HOUR) * 60; // 540 minutes
const JST_OFFSET_MS = 9 * 60 * 60 * 1000;

export const jstTimeFormatter = new Intl.DateTimeFormat('ja-JP', {
    timeZone: 'Asia/Tokyo',
    hour: '2-digit',
    minute: '2-digit',
    hour12: false
});

export const jstDateFormatter = new Intl.DateTimeFormat('sv-SE', {
    timeZone: 'Asia/Tokyo'
});

export function formatTime(date: Date): string {
    return jstTimeFormatter.format(date);
}

export function formatDateTime(date: Date): string {
    return new Intl.DateTimeFormat('ja-JP', {
        timeZone: 'Asia/Tokyo',
        year: 'numeric',
        month: '2-digit',
        day: '2-digit',
        hour: '2-digit',
        minute: '2-digit',
        hour12: false
    }).format(date);
}

export function getJSTDateString(date: Date): string {
    return jstDateFormatter.format(date);
}

export function getTodayJSTString(): string {
    return getJSTDateString(new Date());
}

export function isSameJSTDate(d1: Date, d2: Date): boolean {
    return getJSTDateString(d1) === getJSTDateString(d2);
}

export function getPercentage(date: Date): number {
    // Total minutes since JST midnight
    const minutesInDay = Math.floor(((date.getTime() + JST_OFFSET_MS) % (24 * 60 * 60 * 1000)) / (60 * 1000));
    const minutesSince9AM = minutesInDay - (START_HOUR * 60);

    return (minutesSince9AM / TOTAL_MINUTES) * 100;
}

type TimeRange = Pick<Task, 'start_at' | 'end_at'> | Pick<TaskTimeLog, 'start_at' | 'end_at'>;

export function getTaskPosition(item: TimeRange): { left: number; width: number } {
    const start = new Date(item.start_at);
    const end = new Date(item.end_at);

    const startPercent = getPercentage(start);
    const endPercent = getPercentage(end);
    
    // Clamp values to be within the visible range (0-100%)
    const left = Math.max(0, Math.min(100, startPercent));
    const width = Math.max(0, Math.min(100 - left, endPercent - startPercent));

    return { left, width };
}

export function isTaskActive(item: TimeRange, now: Date): boolean {
    const start = new Date(item.start_at);
    const end = new Date(item.end_at);
    return now >= start && now <= end;
}

export function xToTime(x: number, width: number, baseDate: Date): Date {
    // baseDate should be 00:00 JST of the selected day
    const minutesSince9AM = (x / width) * TOTAL_MINUTES;
    const totalMinutesSinceMidnightJST = (START_HOUR * 60) + minutesSince9AM;
    
    const date = new Date(baseDate.getTime());
    date.setUTCMinutes(date.getUTCMinutes() + totalMinutesSinceMidnightJST);
    return date;
}

export function percentageToDate(percent: number, baseDate: Date): Date {
    const minutesSince9AM = (percent / 100) * TOTAL_MINUTES;
    const totalMinutesSinceMidnightJST = (START_HOUR * 60) + minutesSince9AM;
    
    const date = new Date(baseDate.getTime());
    date.setUTCMinutes(date.getUTCMinutes() + totalMinutesSinceMidnightJST);
    return date;
}

export function snapTo15Min(date: Date): Date {
    const minutesInDay = Math.floor(((date.getTime() + JST_OFFSET_MS) % (24 * 60 * 60 * 1000)) / (60 * 1000));
    const snappedMinutesInDay = Math.round(minutesInDay / 15) * 15;
    
    const baseOfDayJST = date.getTime() + JST_OFFSET_MS - ((date.getTime() + JST_OFFSET_MS) % (24 * 60 * 60 * 1000));
    return new Date(baseOfDayJST - JST_OFFSET_MS + (snappedMinutesInDay * 60 * 1000));
}

export function toLocalISOString(date: Date): string {
    // Returns ISO string with JST offset (+09:00)
    const jstDate = new Date(date.getTime() + JST_OFFSET_MS);
    return jstDate.toISOString().replace('Z', '+09:00');
}
