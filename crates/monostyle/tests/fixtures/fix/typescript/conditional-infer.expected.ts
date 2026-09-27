type Unwrap<T> = T extends Promise<infer U> ? U : T;
type Flatten<T> = T extends Array<infer Item> ? Item : T;

type A = Unwrap<Promise<number>>;
type B = Flatten<string[]>;

const sample: A = 4;
