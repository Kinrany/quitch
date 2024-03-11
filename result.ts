export type Result<T, E = Error> =
  | { ok: true; value: T }
  | { ok: false; error: E };

export default class Result2<T, E = Error> {
  /** True if this is an Ok variant, False if this is an Err variant */
  #ok: boolean;
  /** Always present for Ok values, otherwise undefined */
  #value?: T;
  /** Always present for Err values, otherwise undefined */
  #error?: E;

  private constructor(ok: boolean, value: T, error: E) {
    this.#ok = ok;
    this.#value = value;
    this.#error = error;
  }

  static Ok<T, E = Error>(value: T): Result2<T, E> {
    return new Result2(true, value, undefined as E);
  }

  static Err<T, E = Error>(error: E): Result2<T, E> {
    return new Result2(false, undefined as T, error);
  }

  ok(): T | undefined {
    return this.#ok ? this.#value : undefined;
  }

  err(): E | undefined {
    return this.#ok ? undefined : this.#error;
  }

  unwrap(): T {
    if (this.#ok) {
      return this.#value!;
    } else {
      throw this.#error;
    }
  }

  or(value: T): Result2<T, E> {
    if (this.#ok) return this;
    else return Result2.Ok(value);
  }
}

export const ok = <T>(value: T): Result<T> => ({ ok: true, value });
export const err = <E>(error: E): Result<never, E> => ({ ok: false, error });
/** Unwrap an OK value or throw an exception */
export const unwrap = <T>(result: Result<T>): T => {
  if (!result.ok) throw result.error;
  return result.value;
};
