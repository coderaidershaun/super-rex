# Rust ergonomics examples

Rules for each example are in `.claude/skills/rex-code-ergonomics/SKILL.md`, under the same heading.

## Names

```rust
// bad
fn proc(d: &Vec<u8>, f: bool) -> Result<Vec<u8>, Box<dyn Error>>

// good
fn decompress_payload(data: &[u8], strict: bool) -> Result<Vec<u8>, DecompressError>
```

## Types — newtype the domain

```rust
// bad — mix UserId + OrderId silently
fn ban(user_id: u64, reason: String)
fn refund(order_id: u64)
ban(order_id, refund_reason); // compiles. wrong.

// good — type system catch swap
struct UserId(u64);
struct OrderId(u64);
fn ban(user: UserId, reason: BanReason)
ban(OrderId(42), reason); // compile error. saved.
```

No `bool` flags in public fn:

```rust
// bad
fn copy(src: &Path, dst: &Path, overwrite: bool, follow_symlinks: bool)
copy(&a, &b, true, false); // what mean true false?

// good
fn copy(src: &Path, dst: &Path, opts: CopyOpts)
copy(&a, &b, CopyOpts { overwrite: Yes, symlinks: Follow });
```

## Function signatures

```rust
// bad
fn render(w: u32, h: u32, fmt: u8, q: u8, alpha: bool, gamma: f32) -> Vec<u8>

// good
struct RenderOpts { width: u32, height: u32, format: Format, quality: u8, alpha: bool, gamma: f32 }
fn render(opts: RenderOpts) -> Vec<u8>
// or builder if half optional
let img = Render::new(800, 600).quality(90).alpha().build();
```

## Iterators

```rust
// bad — alloc twice, panic on miss
let names: Vec<String> = users.iter().map(|u| u.name.clone()).collect();
let active: Vec<String> = names.into_iter().filter(|n| !n.is_empty()).collect();

// good — single pass, owned strings only when kept
let active: Vec<String> = users.iter()
    .filter(|u| !u.name.is_empty())
    .map(|u| u.name.clone())
    .collect();
```

## Option / Result combinators

```rust
// bad
let name = match user.name {
    Some(n) => n,
    None => String::from("anon"),
};

// good
let name = user.name.unwrap_or_else(|| "anon".into());
```

## Lifetimes — minimize

```rust
// bad — every caller fight lifetime
struct Parser<'a> { input: &'a str, cursor: usize }

// good if perf allow — owned, no lifetime virus
struct Parser { input: String, cursor: usize }
```

## Traits

```rust
// good — From for conversion that cannot fail
impl From<u64> for UserId {
    fn from(n: u64) -> Self { UserId(n) }
}
let id: UserId = 42.into();

// good — TryFrom for conversion that can fail. Caller gets error, `?` works.
impl TryFrom<&str> for UserId {
    type Error = std::num::ParseIntError;
    fn try_from(s: &str) -> Result<Self, Self::Error> { s.parse().map(UserId) }
}
let id = UserId::try_from("42")?;
```

## Comments

```rust
// bad
/// Gets the user by id
fn get_user(id: UserId) -> Option<User>

// good
/// Look up user by id. Returns `None` if user soft-deleted or never existed.
/// Hot path — uses cached lookup. See [`User::reload`] to force fresh.
fn get_user(id: UserId) -> Option<User>
```
