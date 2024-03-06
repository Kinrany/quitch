export type Result<T, E = Error> =
  | { ok: true; value: T }
  | { ok: false; error: E };
export const ok = <T>(value: T): Result<T> => ({ ok: true, value });
export const err = <E>(error: E): Result<never, E> => ({ ok: false, error });
/** Unwrap an OK value or throw an exception */
export const unwrap = <T>(result: Result<T>): T => {
  if (!result.ok) throw result.error;
  return result.value;
};
