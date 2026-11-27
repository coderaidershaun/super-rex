# Timing code

The Rust form of [timing](timing.md):

```rust
let started = std::time::Instant::now();
println!("{:?}", started.elapsed());
```
