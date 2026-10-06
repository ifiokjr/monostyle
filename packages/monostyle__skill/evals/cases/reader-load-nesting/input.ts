export type Permission = "none" | "read" | "write" | "admin";

export function resolvePermission(
	role: string,
	owner: string,
	actor: string,
	flags: string[],
): Permission {
	let permission: Permission = "none";
	if (role === "admin") {
		if (actor === owner) {
			if (flags.includes("suspended")) {
				permission = "write";
			} else {
				permission = "admin";
			}
		} else {
			if (flags.includes("suspended")) {
				permission = "read";
			} else {
				permission = "write";
			}
		}
	} else {
		if (role === "member") {
			if (actor === owner) {
				if (flags.includes("suspended")) {
					permission = "none";
				} else {
					permission = "write";
				}
			} else {
				if (flags.includes("suspended")) {
					permission = "none";
				} else {
					permission = "read";
				}
			}
		} else {
			if (flags.includes("invited")) {
				permission = "read";
			}
		}
	}
	return permission;
}
