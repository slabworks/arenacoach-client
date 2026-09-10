/// <reference lib="dom" />

import { describe, expect, test } from "bun:test";
import { fireEvent, render, screen } from "@testing-library/react";
import {
  AccountPanel,
  type CreateAccountPayload,
  type DeleteAccountPayload,
  type SignInPayload,
  type UpdateAccountPayload,
} from "../src/AccountPanel";

function renderAccount(
  overrides: Partial<Parameters<typeof AccountPanel>[0]> = {},
) {
  return render(
    <AccountPanel
      signedIn={false}
      name=""
      email=""
      synced={false}
      busy={false}
      ready
      error={null}
      needsTwoFactor={false}
      onSignIn={() => undefined}
      onCreate={() => undefined}
      onUpdate={() => undefined}
      onDelete={() => undefined}
      onSignOut={() => undefined}
      {...overrides}
    />,
  );
}

describe("AccountPanel", () => {
  test("lets a visitor sign in", () => {
    const signedIn: SignInPayload[] = [];
    renderAccount({
      onSignIn: (payload) => signedIn.push(payload),
    });

    fireEvent.change(screen.getByLabelText("Email address"), {
      target: { value: "pilot@example.com" },
    });
    fireEvent.change(screen.getByLabelText("Password"), {
      target: { value: "password" },
    });
    fireEvent.click(
      screen.getByRole("button", { name: "Sign in to Arena Coach" }),
    );

    expect(signedIn).toEqual([
      {
        email: "pilot@example.com",
        password: "password",
        code: null,
      },
    ]);
  });

  test("lets a visitor create an account", () => {
    const created: CreateAccountPayload[] = [];
    renderAccount({
      onCreate: (payload) => created.push(payload),
    });

    fireEvent.click(screen.getByRole("button", { name: "Create an account" }));
    fireEvent.change(screen.getByLabelText("Name"), {
      target: { value: "Arena Pilot" },
    });
    fireEvent.change(screen.getByLabelText("Email address"), {
      target: { value: "pilot@example.com" },
    });
    fireEvent.change(screen.getByLabelText("Password"), {
      target: { value: "password" },
    });
    fireEvent.change(screen.getByLabelText("Confirm password"), {
      target: { value: "password" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Create account" }));

    expect(created).toEqual([
      {
        name: "Arena Pilot",
        email: "pilot@example.com",
        password: "password",
        password_confirmation: "password",
      },
    ]);
  });

  test("lets a signed-in player update their account", () => {
    const updated: UpdateAccountPayload[] = [];
    renderAccount({
      signedIn: true,
      name: "You",
      email: "you@example.com",
      synced: true,
      onUpdate: (payload) => updated.push(payload),
    });

    fireEvent.click(screen.getByRole("button", { name: "Manage account" }));
    fireEvent.change(screen.getByLabelText("Name"), {
      target: { value: "Arena Pilot" },
    });
    fireEvent.change(screen.getByLabelText("Email address"), {
      target: { value: "pilot@example.com" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save account" }));

    expect(updated).toEqual([
      {
        name: "Arena Pilot",
        email: "pilot@example.com",
      },
    ]);
  });

  test("includes a password change when those fields are filled", () => {
    const updated: UpdateAccountPayload[] = [];
    renderAccount({
      signedIn: true,
      name: "You",
      email: "you@example.com",
      synced: true,
      onUpdate: (payload) => updated.push(payload),
    });

    fireEvent.click(screen.getByRole("button", { name: "Manage account" }));
    fireEvent.change(screen.getByLabelText("Current password"), {
      target: { value: "password" },
    });
    fireEvent.change(screen.getByLabelText("New password"), {
      target: { value: "new-password" },
    });
    fireEvent.change(screen.getByLabelText("Confirm new password"), {
      target: { value: "new-password" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save account" }));

    expect(updated).toEqual([
      {
        name: "You",
        email: "you@example.com",
        current_password: "password",
        password: "new-password",
        password_confirmation: "new-password",
      },
    ]);
  });

  test("lets a signed-in player delete their account", () => {
    const deleted: DeleteAccountPayload[] = [];
    renderAccount({
      signedIn: true,
      name: "You",
      email: "you@example.com",
      synced: true,
      onDelete: (payload) => deleted.push(payload),
    });

    fireEvent.click(screen.getByRole("button", { name: "Manage account" }));
    fireEvent.change(screen.getByLabelText("Confirm deletion with password"), {
      target: { value: "password" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Delete account" }));

    expect(deleted).toEqual([{ password: "password" }]);
  });
});
