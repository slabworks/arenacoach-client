import { describe, expect, test } from "bun:test";
import { accountActionError } from "../src/account";

describe("accountActionError", () => {
  test("surfaces Laravel validation messages from a Rust HTTP error", () => {
    expect(
      accountActionError(
        'HTTP 422: {"message":"The email has already been taken.","errors":{"email":["The email has already been taken."]}}',
        "Couldn’t create your account.",
      ),
    ).toBe("The email has already been taken.");
  });

  test("falls back when the error has no useful message", () => {
    expect(accountActionError("HTTP 500: boom", "Couldn’t save your account.")).toBe(
      "Couldn’t save your account.",
    );
  });
});
