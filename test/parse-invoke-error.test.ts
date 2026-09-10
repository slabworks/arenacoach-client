import { describe, expect, test } from "bun:test";
import { parseInvokeError } from "../src/matches";

describe("parseInvokeError", () => {
  test("reads a JSON payload after a Rust prefix", () => {
    expect(
      parseInvokeError(
        new Error('invoke failed: {"message":"Match not found","errors":{"id":["missing"]}}'),
      ),
    ).toEqual({
      message: "Match not found",
      errors: { id: ["missing"] },
    });
  });

  test("accepts a bare JSON string", () => {
    expect(
      parseInvokeError('{"message":"Could not sign in","errors":{"email":["required"]}}'),
    ).toEqual({
      message: "Could not sign in",
      errors: { email: ["required"] },
    });
  });

  test("keeps the raw text when the JSON has no message", () => {
    const raw = '{"errors":{"password":["too short"]}}';
    expect(parseInvokeError(raw)).toEqual({
      message: raw,
      errors: { password: ["too short"] },
    });
  });

  test("treats a non-JSON brace as plain text", () => {
    expect(parseInvokeError("broken {not json")).toEqual({
      message: "broken {not json",
      errors: {},
    });
  });

  test("stringifies values that are not Error instances", () => {
    expect(parseInvokeError("two_factor")).toEqual({
      message: "two_factor",
      errors: {},
    });
  });
});
