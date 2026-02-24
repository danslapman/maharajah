/** Result type representing either success or failure. */
type Result<T, E> =
  | { ok: true; value: T }
  | { ok: false; error: E };

/**
 * Validates that a string is a well-formed email address.
 * Returns the trimmed email on success, or an error message.
 */
function validateEmail(input: string): Result<string, string> {
  const trimmed = input.trim();
  const re = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
  if (!re.test(trimmed)) {
    return { ok: false, error: `"${trimmed}" is not a valid email` };
  }
  return { ok: true, value: trimmed };
}

/**
 * Formats a Date as an ISO-8601 date string (YYYY-MM-DD).
 */
function formatDate(date: Date): string {
  return date.toISOString().slice(0, 10);
}

/** Represents a user with an id and display name. */
interface User {
  id: number;
  name: string;
  email: string;
}

/**
 * Finds the first user with the given email, case-insensitively.
 */
function findByEmail(users: User[], email: string): User | undefined {
  const lower = email.toLowerCase();
  return users.find(u => u.email.toLowerCase() === lower);
}
