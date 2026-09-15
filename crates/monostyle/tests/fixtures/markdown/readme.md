# Example package

A short introduction that explains what the package is for and who should use it.

## Usage

Call the entry point with a configuration object:

```rust
let result = run(&config);

if result.is_ok() {
    println!("done");
}
```

An example that needs work:

```rust
fn bad(x:i32)->i32{if x>0{return x;}x}
```

Some prose that is deliberately long enough to be measured but is not code and therefore should
never produce a line-length finding, no matter how wide the line happens to be in the file that
contains this document.

## API

| Function | Purpose |
| --- | --- |
| `run` | Executes a configuration |
