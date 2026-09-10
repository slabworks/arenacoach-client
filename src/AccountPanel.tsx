import { useEffect, useState } from "react";

export type CreateAccountPayload = {
  name: string;
  email: string;
  password: string;
  password_confirmation: string;
};

export type SignInPayload = {
  email: string;
  password: string;
  code: string | null;
};

export type UpdateAccountPayload = {
  name: string;
  email: string;
  current_password?: string;
  password?: string;
  password_confirmation?: string;
};

export type DeleteAccountPayload = {
  password: string;
};

type AccountMode = "sign-in" | "create";

export function AccountPanel({
  signedIn,
  name,
  email,
  synced,
  busy,
  ready,
  error,
  needsTwoFactor,
  onSignIn,
  onCreate,
  onUpdate,
  onDelete,
  onSignOut,
}: {
  signedIn: boolean;
  name: string;
  email: string;
  synced: boolean;
  busy: boolean;
  ready: boolean;
  error: string | null;
  needsTwoFactor: boolean;
  onSignIn: (payload: SignInPayload) => void;
  onCreate: (payload: CreateAccountPayload) => void;
  onUpdate: (payload: UpdateAccountPayload) => void;
  onDelete: (payload: DeleteAccountPayload) => void;
  onSignOut: () => void;
}) {
  const [mode, setMode] = useState<AccountMode>("sign-in");
  const [managing, setManaging] = useState(false);
  const [nameValue, setNameValue] = useState(name);
  const [emailValue, setEmailValue] = useState(email);
  const [password, setPassword] = useState("");
  const [passwordConfirmation, setPasswordConfirmation] = useState("");
  const [currentPassword, setCurrentPassword] = useState("");
  const [deletePassword, setDeletePassword] = useState("");
  const [code, setCode] = useState("");

  useEffect(() => {
    setNameValue(name);
    setEmailValue(email);
  }, [name, email, signedIn]);

  useEffect(() => {
    if (signedIn) {
      setMode("sign-in");
      setPassword("");
      setPasswordConfirmation("");
      setCurrentPassword("");
      setDeletePassword("");
      setCode("");
    } else {
      setManaging(false);
    }
  }, [signedIn]);

  return (
    <section className="account-card" aria-labelledby="account-title">
      <div className="account-heading">
        <div className="small-icon">
          <AccountIcon />
        </div>
        <div>
          <h2 id="account-title">
            {signedIn
              ? "You’re connected"
              : mode === "create"
                ? "Create your account"
                : "Connect your account"}
          </h2>
          <p>
            {signedIn
              ? email || "Signed in to Arena Coach"
              : mode === "create"
                ? "Save your games and coaching to this companion."
                : "Bring your games and your coaching together."}
          </p>
        </div>
        {signedIn ? (
          <span className="connected-dot" aria-label="Signed in" />
        ) : null}
      </div>
      {signedIn ? (
        <>
          <div className="signed-in-details">
            <span>
              {synced
                ? "Your latest match is synced."
                : "Your next completed match will sync automatically."}
            </span>
            <div className="match-actions">
              <button
                className="text-button"
                type="button"
                disabled={busy}
                onClick={() => setManaging((open) => !open)}
              >
                {managing ? "Done" : "Manage account"}
              </button>
              <button
                className="text-button"
                type="button"
                disabled={busy}
                onClick={onSignOut}
              >
                Sign out
              </button>
            </div>
          </div>
          {managing ? (
            <form
              onSubmit={(event) => {
                event.preventDefault();
                const payload: UpdateAccountPayload = {
                  name: nameValue.trim(),
                  email: emailValue.trim(),
                };
                if (password || currentPassword) {
                  payload.current_password = currentPassword;
                  payload.password = password;
                  payload.password_confirmation = passwordConfirmation;
                }
                onUpdate(payload);
              }}
            >
              <label htmlFor="account-name">Name</label>
              <input
                id="account-name"
                type="text"
                placeholder="Your name"
                value={nameValue}
                onChange={(e) => setNameValue(e.currentTarget.value)}
                autoComplete="name"
                required
                disabled={busy}
              />
              <label htmlFor="account-email">Email address</label>
              <input
                id="account-email"
                type="email"
                placeholder="you@example.com"
                value={emailValue}
                onChange={(e) => setEmailValue(e.currentTarget.value)}
                autoComplete="email"
                required
                disabled={busy}
              />
              <label htmlFor="account-current-password">Current password</label>
              <input
                id="account-current-password"
                type="password"
                placeholder="Needed to change your password"
                value={currentPassword}
                onChange={(e) => setCurrentPassword(e.currentTarget.value)}
                autoComplete="current-password"
                disabled={busy}
              />
              <label htmlFor="account-password">New password</label>
              <input
                id="account-password"
                type="password"
                placeholder="Leave blank to keep your password"
                value={password}
                onChange={(e) => setPassword(e.currentTarget.value)}
                autoComplete="new-password"
                disabled={busy}
              />
              <label htmlFor="account-password-confirmation">
                Confirm new password
              </label>
              <input
                id="account-password-confirmation"
                type="password"
                placeholder="Confirm your new password"
                value={passwordConfirmation}
                onChange={(e) => setPasswordConfirmation(e.currentTarget.value)}
                autoComplete="new-password"
                disabled={busy}
              />
              <button
                className="primary-button"
                type="submit"
                disabled={busy || !ready}
              >
                {busy ? "Saving…" : "Save account"}
                <span aria-hidden="true">↗</span>
              </button>
            </form>
          ) : null}
          {managing ? (
            <form
              className="account-danger"
              onSubmit={(event) => {
                event.preventDefault();
                onDelete({ password: deletePassword });
              }}
            >
              <h3>Delete account</h3>
              <p>This permanently removes your Arena Coach account.</p>
              <label htmlFor="account-delete-password">
                Confirm deletion with password
              </label>
              <input
                id="account-delete-password"
                type="password"
                placeholder="Your current password"
                value={deletePassword}
                onChange={(e) => setDeletePassword(e.currentTarget.value)}
                autoComplete="current-password"
                required
                disabled={busy}
              />
              <button
                className="secondary-button danger-button"
                type="submit"
                disabled={busy || !ready}
              >
                Delete account
              </button>
            </form>
          ) : null}
        </>
      ) : mode === "create" ? (
        <form
          onSubmit={(event) => {
            event.preventDefault();
            onCreate({
              name: nameValue.trim(),
              email: emailValue.trim(),
              password,
              password_confirmation: passwordConfirmation,
            });
          }}
        >
          <label htmlFor="account-name">Name</label>
          <input
            id="account-name"
            type="text"
            placeholder="Your name"
            value={nameValue}
            onChange={(e) => setNameValue(e.currentTarget.value)}
            autoComplete="name"
            required
            disabled={busy}
          />
          <label htmlFor="account-email">Email address</label>
          <input
            id="account-email"
            type="email"
            placeholder="you@example.com"
            value={emailValue}
            onChange={(e) => setEmailValue(e.currentTarget.value)}
            autoComplete="email"
            required
            disabled={busy}
          />
          <label htmlFor="account-password">Password</label>
          <input
            id="account-password"
            type="password"
            placeholder="Create a password"
            value={password}
            onChange={(e) => setPassword(e.currentTarget.value)}
            autoComplete="new-password"
            required
            disabled={busy}
          />
          <label htmlFor="account-password-confirmation">
            Confirm password
          </label>
          <input
            id="account-password-confirmation"
            type="password"
            placeholder="Confirm your password"
            value={passwordConfirmation}
            onChange={(e) => setPasswordConfirmation(e.currentTarget.value)}
            autoComplete="new-password"
            required
            disabled={busy}
          />
          <button className="primary-button" type="submit" disabled={busy || !ready}>
            {busy ? "Creating…" : "Create account"}
            <span aria-hidden="true">↗</span>
          </button>
          <p className="account-switch">
            Already have an account?{" "}
            <button
              className="text-button"
              type="button"
              disabled={busy}
              onClick={() => setMode("sign-in")}
            >
              Sign in
            </button>
          </p>
        </form>
      ) : (
        <form
          onSubmit={(event) => {
            event.preventDefault();
            onSignIn({
              email: emailValue.trim(),
              password,
              code: needsTwoFactor ? code : null,
            });
          }}
        >
          <label htmlFor="account-email">Email address</label>
          <input
            id="account-email"
            type="email"
            placeholder="you@example.com"
            value={emailValue}
            onChange={(e) => setEmailValue(e.currentTarget.value)}
            autoComplete="username"
            required
            disabled={busy}
          />
          <label htmlFor="account-password">Password</label>
          <input
            id="account-password"
            type="password"
            placeholder="Your password"
            value={password}
            onChange={(e) => setPassword(e.currentTarget.value)}
            autoComplete="current-password"
            required
            disabled={busy}
          />
          {needsTwoFactor ? (
            <>
              <label htmlFor="account-code">Authenticator code</label>
              <input
                id="account-code"
                value={code}
                onChange={(e) => setCode(e.currentTarget.value)}
                autoComplete="one-time-code"
                inputMode="numeric"
                required
                autoFocus
              />
            </>
          ) : null}
          <button className="primary-button" type="submit" disabled={busy || !ready}>
            {busy
              ? "Connecting…"
              : needsTwoFactor
                ? "Verify & connect"
                : "Sign in to Arena Coach"}
            <span aria-hidden="true">↗</span>
          </button>
          <p className="account-switch">
            New here?{" "}
            <button
              className="text-button"
              type="button"
              disabled={busy}
              onClick={() => setMode("create")}
            >
              Create an account
            </button>
          </p>
        </form>
      )}
      {error ? (
        <p className="error" role="alert">
          {error}
        </p>
      ) : null}
    </section>
  );
}

function AccountIcon() {
  return (
    <svg
      width="20"
      height="20"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <circle cx="12" cy="8" r="3" />
      <path d="M5 21v-3a7 7 0 0 1 14 0v3" />
    </svg>
  );
}
