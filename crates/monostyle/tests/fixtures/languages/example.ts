/**
 * Resolves a user's display name.
 *
 * Falls back through the profile, the account, and finally the id, because older
 * accounts predate the profile field and must still render.
 */
export function displayName(user: User): string {
  const name = user?.profile?.name ?? user?.account?.name ?? String(user.id);

  if (name.length > 0) {
    return name;
  }

  return "unknown";
}
