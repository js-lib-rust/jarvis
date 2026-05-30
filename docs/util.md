# Util Module

The `util` module is a collection of small, stateless helper functions. Currently it contains one sub-module: `util::string`.

## util::string

| Function | Signature | Description |
|---|---|---|
| `snake_case` | `(name: &str) -> String` | Converts spaces and hyphens to underscores |
| `eq_no_case` | `(s1: &str, s2: &str) -> bool` | Case-insensitive string equality |
| `ellipsis` | `(s: &str, n: usize) -> String` | Truncates a string to at most `n` characters |

### Examples

```
snake_case("blood pressure")  →  "blood_pressure"
snake_case("body-weight")     →  "body_weight"

eq_no_case("Kitchen", "kitchen")  →  true

ellipsis("hello world", 5)    →  "hello"
```

### Usage

`snake_case` is used when normalising user-supplied property names before MongoDB queries (e.g., `"social security number"` → `"social_security_number"`).

`eq_no_case` is used for zone name matching in the home automation service (e.g., `"Kitchen"` vs `"kitchen"`).

`ellipsis` is used in log statements to prevent excessively long prompts from flooding the log.
