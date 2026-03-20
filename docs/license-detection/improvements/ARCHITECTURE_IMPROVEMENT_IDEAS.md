# Architecture Improvement Ideas

## Make `PositionSpan` earn its abstraction

`PositionSpan` is currently only partly useful: `contains()` now helps readability,
but the removed `difference()` helper did not match real call sites.

Most subtraction sites work on `HashSet<usize> - span positions`, not
`PositionSpan - PositionSpan`. If we revisit this area, we should first inventory
how token-position differences are actually used and then design helpers around
those operations instead of keeping an unused span-to-span API.

Possible directions:

- add helpers that operate on sets with a span input,
- expose a lighter-weight iterator over span positions,
- or reshape query/run code so span-based operations are first-class throughout.

## Explore making `QueryRun` span-first

`QueryRun` conceptually represents a bounded token span, but today it stores
`start` and `end` fields separately and rebuilds `PositionSpan` values on demand.

If we revisit this area, we should evaluate whether `QueryRun` should own a real
`PositionSpan`-like value for non-empty runs, with a separate representation for
the empty-run case. That could make the composition more natural and reduce
ad-hoc conversions, but it needs to be weighed against the current `end: Option<usize>`
shape that also encodes emptiness.

## Use Rule references in LicenseMatch

In many places such as `LicenseMatch`, we store ids of rules as `usize` meaning we
need to look them up in the `LicenseIndex` everytime we want to access some `Rule` data.
Instead, it would likely be more appropriate to store a reference `&Rule` to the `Rule`.
This will complicate the lifetimes but is conceptually the correct choice.
