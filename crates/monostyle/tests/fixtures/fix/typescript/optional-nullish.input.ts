interface Config {
  retries?: { count?: number; backoff?: string };
}

function resolve(config: Config): string {
  const count = config.retries?.count ?? 3;
  const backoff = config.retries?.backoff?.toString() ?? "linear";



  return `${count}:${backoff}`;
}
