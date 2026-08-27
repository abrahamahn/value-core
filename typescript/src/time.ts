const RFC3339_PATTERN =
  /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})(?:\.\d+)?(?:Z|([+-])(\d{2}):(\d{2}))$/u;

/** Parse a strict RFC 3339 instant into Unix epoch milliseconds. */
export function parseRfc3339Millis(value: string, context = 'Value timestamp'): number {
  const match = RFC3339_PATTERN.exec(value);
  if (match === null) throw new Error(`${context} must be an RFC 3339 instant`);
  const year = Number(match[1]);
  const month = Number(match[2]);
  const day = Number(match[3]);
  const hour = Number(match[4]);
  const minute = Number(match[5]);
  const second = Number(match[6]);
  const offsetHour = match[8] === undefined ? 0 : Number(match[8]);
  const offsetMinute = match[9] === undefined ? 0 : Number(match[9]);
  const leapYear = year % 4 === 0 && (year % 100 !== 0 || year % 400 === 0);
  const daysInMonth = [0, 31, leapYear ? 29 : 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
  if (
    month < 1 ||
    month > 12 ||
    day < 1 ||
    day > (daysInMonth[month] ?? 0) ||
    hour > 23 ||
    minute > 59 ||
    second > 59 ||
    offsetHour > 23 ||
    offsetMinute > 59
  ) {
    throw new Error(`${context} must be an RFC 3339 instant`);
  }
  const parsed = Date.parse(value);
  if (!Number.isSafeInteger(parsed)) {
    throw new Error(`${context} is outside the supported millisecond range`);
  }
  return parsed;
}

/** Format Unix epoch milliseconds as a canonical UTC RFC 3339 instant. */
export function formatRfc3339Millis(millis: number, context = 'Value timestamp'): string {
  if (!Number.isSafeInteger(millis)) {
    throw new Error(`${context} is outside the supported millisecond range`);
  }
  const formatted = new Date(millis).toISOString();
  if (!/^\d{4}-/u.test(formatted)) {
    throw new Error(`${context} is outside the supported RFC 3339 calendar range`);
  }
  return formatted;
}
