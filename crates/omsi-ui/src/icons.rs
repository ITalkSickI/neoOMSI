//! Material Symbols (Rounded, filled) - the SVG files of `assets/icons/material`, built in.

include!(concat!(env!("OUT_DIR"), "/icons.rs"));

/// The SVG source of an icon by its Material Symbols name (`directions_bus`).
pub fn svg(name: &str) -> Option<&'static str> {
    ICONS.iter().find(|(n, _)| *n == name).map(|(_, s)| *s)
}

/// Every icon name there is.
pub fn names() -> impl Iterator<Item = &'static str> {
    ICONS.iter().map(|(n, _)| *n)
}

/// An icon as an alpha mask of `size` x `size` pixels (the symbol fills its 48-unit box
/// the way Google draws it, with its own margin).
pub fn rasterize(name: &str, size: u32) -> Option<Vec<u8>> {
    let src = svg(name)?;
    let opt = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_str(src, &opt).ok()?;
    let mut pix = resvg::tiny_skia::Pixmap::new(size, size)?;
    let s = tree.size();
    let k = size as f32 / s.width().max(s.height());
    resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(k, k), &mut pix.as_mut());
    Some(pix.pixels().iter().map(|p| p.alpha()).collect())
}

#[cfg(test)]
mod tests {
    #[test]
    fn icons_are_built_in_and_draw() {
        assert!(super::names().count() > 50);
        let a = super::rasterize("directions_bus", 32).unwrap();
        assert_eq!(a.len(), 32 * 32);
        let covered = a.iter().filter(|&&v| v > 128).count();
        assert!(covered > 150 && covered < 900, "{covered}");
        assert!(super::rasterize("no_such_icon", 32).is_none());
    }
}
