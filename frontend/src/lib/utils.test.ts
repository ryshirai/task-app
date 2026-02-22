import { describe, expect, it } from 'vitest';
import { toLocalISOString } from './utils';

describe('toLocalISOString', () => {
  it('formats with millisecond precision when milliseconds exist', () => {
    const date = new Date('2026-01-02T00:00:00.006Z');
    const result = toLocalISOString(date);
    expect(result).toBe('2026-01-02T09:00:00.006+09:00');
  });

  it('keeps .000 when source date has no milliseconds', () => {
    const date = new Date('2026-01-02T00:00:00Z');
    const result = toLocalISOString(date);
    expect(result).toBe('2026-01-02T09:00:00.000+09:00');
  });

  it('crosses date boundary correctly when converted to JST (+09:00)', () => {
    const date = new Date('2026-01-31T18:30:00Z');
    const result = toLocalISOString(date);
    expect(result).toBe('2026-02-01T03:30:00.000+09:00');
  });

  it('uses fixed +09:00 suffix regardless of runtime timezone', () => {
    const date = new Date('2025-06-15T09:00:00Z');
    const result = toLocalISOString(date);
    expect(result.endsWith('+09:00')).toBe(true);
  });
});
