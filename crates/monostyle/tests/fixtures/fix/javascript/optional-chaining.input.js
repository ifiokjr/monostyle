function resolve(config) {
  const count = config?.retries?.count ?? 3;
  const key = config?.keys?.[0] ?? "none";



  return `${count}:${key}`;
}
