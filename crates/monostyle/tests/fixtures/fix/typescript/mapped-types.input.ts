type Optional<T> = {
  [K in keyof T]?: T[K];
};

type ReadonlyDeep<T> = {
  readonly [K in keyof T]: T[K] extends object ? ReadonlyDeep<T[K]> : T[K];
};

interface Config {
  retries: number;
  backoff: string;
}
