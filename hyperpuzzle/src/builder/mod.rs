//! Puzzle construction API usable by Rust code.
//!
//! These are all wrapped in `Arc<Mutex<T>>` so that the Lua API can access each
//! independently. These builders are a rare place where we accept mutable
//! aliasing in the Lua API, so the Rust API must also have mutable aliasing.

use std::collections::HashSet;

use eyre::eyre;

mod axis_system;
mod color_system;
mod naming_scheme;
mod ordering;
mod puzzle;
mod shape;
mod twist_system;

pub use axis_system::{AxisBuilder, AxisLayerBuilder, AxisSystemBuilder};
pub use color_system::{ColorBuilder, ColorSystemBuilder};
pub use naming_scheme::{BadName, NamingScheme};
pub use ordering::CustomOrdering;
pub use puzzle::{PieceBuilder, PuzzleBuilder};
pub use shape::ShapeBuilder;
pub use twist_system::{TwistBuilder, TwistKey, TwistSystemBuilder};

/// Iterates over elements names in canonical order, assigning unused
/// autogenerated names to unnamed elements.
///
/// The first string in each pair is the name; the second string in each pair is
/// the display name. If no display name is specified, then the name is used
/// instead.
///
/// A warning is emitted if any short or long name is duplicated.
pub fn iter_autonamed<'a, I: hypermath::IndexNewtype>(
    names: &'a NamingScheme<I>,
    order: impl 'a + IntoIterator<Item = I>,
    autonames: impl 'a + IntoIterator<Item = String>,
    warn_fn: impl Copy + Fn(eyre::Report),
) -> impl 'a + Iterator<Item = (I, (String, String))> {
    let ids_to_names = names.ids_to_names();
    let ids_to_display_names = names.ids_to_display_names();

    // Ensure names are unqiue.
    warn_on_duplicates(ids_to_names.values(), warn_fn);
    warn_on_duplicates(ids_to_display_names.values(), warn_fn);

    let used_names: HashSet<&String> = ids_to_names.values().collect();

    let mut unused_names = autonames
        .into_iter()
        .filter(move |s| !used_names.contains(&s));
    let mut next_unused_name = move || unused_names.next().expect("ran out of names");

    order.into_iter().map(move |id| {
        let name = match ids_to_names.get(&id) {
            Some(s) => s.to_owned(),
            None => next_unused_name(),
        };
        let display = match ids_to_display_names.get(&id) {
            Some(s) => s.to_owned(),
            None => name.clone(),
        };
        (id, (name, display))
    })
}

fn warn_on_duplicates<'a>(
    iter: impl IntoIterator<Item = &'a String>,
    warn_fn: impl Fn(eyre::Report),
) {
    let mut seen = HashSet::new();
    for it in iter {
        if !seen.insert(it) {
            warn_fn(eyre!("duplicate name {it:?}"));
        }
    }
}
