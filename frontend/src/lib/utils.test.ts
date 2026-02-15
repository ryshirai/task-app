import { describe, expect, it } from 'vitest';
import { toLocalISOString } from './utils';

function timezoneOffsetSuffix(date: Date): string {
  const offsetMinutes = -date.getTimezoneOffset();
  const sign = offsetMinutes >= 0 ? '+' : '-';
  const absOffsetMinutes = Math.abs(offsetMinutes);
  const hours = String(Math.floor(absOffsetMinutes / 60)).padStart(2, '0');
  const minutes = String(absOffsetMinutes % 60).padStart(2, '0');
  return `${sign}${hours}:${minutes}`;
}

describe('toLocalISOString', () => {
  it('formats local date-time with millisecond precision', () => {
    const date = new Date(2026, 0, 2, 3, 4, 5, 6);
    const result = toLocalISOString(date);
    expect(result).toMatch(/^2026-01-02T03:04:05\.006[+-]\d{2}:\d{2}$/);
  });

  it('keeps date and time values for a different day', () => {
    const date = new Date(2024, 10, 30, 23, 59, 0, 120);
    const result = toLocalISOString(date);
    expect(result).toContain('2024-11-30T23:59:00.120');
  });

  it('includes the correct timezone offset', () => {
    const date = new Date(2025, 5, 15, 9, 0, 0, 0);
    const result = toLocalISOString(date);
    expect(result.endsWith(timezoneOffsetSuffix(date))).toBe(true);
  });
});
