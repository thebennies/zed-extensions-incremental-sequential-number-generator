//! The shared [`Generator`] trait implemented by every sequence kind.

/// Produces the string that should be inserted at cursor index `index`
/// (0-based, in cursor/selection order).
///
/// Implementations must be pure functions of `index` (no shared mutable
/// state) so that generation for large cursor counts can be parallelized or
/// pre-sized trivially by the caller.
pub trait Generator {
    /// Returns the text for the `index`-th cursor (0-based).
    fn value_at(&self, index: usize) -> String;
}

/// Generates `count` values by calling [`Generator::value_at`] for
/// `0..count`, pre-allocating the output `Vec` and each `String`'s capacity
/// hint to avoid reallocation churn on large cursor counts (10,000+ per the
/// PRD's performance requirement).
pub fn generate_n(generator: &dyn Generator, count: usize) -> Vec<String> {
    let mut out = Vec::with_capacity(count);
    for index in 0..count {
        out.push(generator.value_at(index));
    }
    out
}
