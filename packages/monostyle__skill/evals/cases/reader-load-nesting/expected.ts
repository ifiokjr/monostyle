export type Permission = "none" | "read" | "write" | "admin";

export function resolvePermission(
	role: string,
	owner: string,
	actor: string,
	flags: string[],
): Permission {
	const suspended = flags.includes("suspended");
	if (role === "guest") {
		return !suspended && flags.includes("invited") ? "read" : "none";
	}
	if (role === "admin" && actor === owner) {
		return suspended ? "write" : "admin";
	}
	if (role === "admin") {
		return suspended ? "read" : "write";
	}
	if (role === "member" && actor === owner) {
		return suspended ? "none" : "write";
	}
	if (role === "member") {
		return suspended ? "none" : "read";
	}
	return "none";
}
