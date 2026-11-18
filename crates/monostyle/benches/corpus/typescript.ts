/**
 * Route matching for a tiny HTTP framework.
 *
 * Regex literals sit next to division, template literals nest, and optional chaining mixes with
 * nullish coalescing — the places a scanner mistakes one token for another.
 */

export interface Route {
	/** The method, upper-cased. */
	method: "GET" | "POST" | "PUT" | "DELETE";
	/** The pattern, with `:name` parameters. */
	pattern: string;
	/** The handler to invoke. */
	handler: (request: Request, params: Record<string, string>) => Promise<Response>;
}

/** A parameter segment: a colon followed by an identifier. */
const PARAMETER = /:([A-Za-z_][A-Za-z0-9_]*)/g;

/** Characters that must be escaped before the pattern becomes a regex. */
const SPECIAL = /[.*+?^${}()|[\]\\]/g;

/**
 * Compiles a route pattern into a regular expression and the parameter names it captures.
 */
export function compile(pattern: string): { matcher: RegExp; names: string[] } {
	const names: string[] = [];
	const escaped = pattern.replace(SPECIAL, (match) => `\\${match}`);
	const source = escaped.replace(PARAMETER, (_match, name: string) => {
		names.push(name);

		return "([^/]+)";
	});

	return { matcher: new RegExp(`^${source}/?$`), names };
}

export class Router {
	private readonly routes: Route[] = [];
	private hits = 0;
	private misses = 0;

	add(route: Route): this {
		this.routes.push(route);

		return this;
	}

	/** The share of requests that matched a route, as a percentage with one decimal. */
	get hitRate(): string {
		const total = this.hits + this.misses;
		const rate = total === 0 ? 0 : (this.hits / total) * 100;

		return `${rate.toFixed(1)}%`;
	}

	async dispatch(request: Request): Promise<Response> {
		const url = new URL(request.url);
		const path = url.pathname.replace(/\/+$/, "") || "/";

		for (const route of this.routes) {
			if (route.method !== request.method) {
				continue;
			}

			const { matcher, names } = compile(route.pattern);
			const match = matcher.exec(path);

			if (!match) {
				continue;
			}

			this.hits += 1;

			const params = Object.fromEntries(
				names.map((name, index) => [name, decodeURIComponent(match[index + 1] ?? "")]),
			);

			return route.handler(request, params);
		}

		this.misses += 1;

		return new Response(`No route for ${request.method} ${path} (hit rate ${this.hitRate})`, {
			status: 404,
		});
	}
}

/** Renders a summary table; the inner template literal is nested inside the outer one. */
export function summary(router: Router, label?: { name: string } | null): string {
	const title = label?.name ?? "router";
	const rows = ["method", "pattern"].map((column) => `| ${column.padEnd(12)} |`);

	return `# ${title}

${rows.join("\n")}
${`hit rate: ${router.hitRate}`}
`;
}

export function average(values: number[]): number {
	if (values.length === 0) {
		return 0;
	}

	const total = values.reduce((sum, value) => sum + value, 0);

	// Division, not a regex: the slash follows an operand.
	return total / values.length;
}
