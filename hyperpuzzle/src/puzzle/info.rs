use hypermath::collections::GenericVec;
use hypermath::prelude::*;
use hypershape::{AtomicPolytopeRef, IsometryId, ManifoldRef, ManifoldSet};
use smallvec::SmallVec;
use tinyset::Set64;

hypermath::idx_struct! {
    /// ID of a **piece**, which is rigid component of the puzzle that moves
    /// together.
    pub struct Piece(pub u16);
    /// ID of a **sticker**, which is a facet of a **piece** having a single
    /// color and belonging to a single **facet**.
    pub struct Sticker(pub u16);
    /// ID of a **facet**, which is a manifold shared by one or more
    /// **stickers**.
    pub struct Facet(pub u16);
    /// ID of a **color** that appears on stickers.
    pub struct Color(pub u16);
    /// ID of a **twist axis**, an organizational unit containing several
    /// **twists**.
    pub struct Axis(pub u16);
    /// ID of a **twist**, which is a single move that can be applied to the
    /// puzzle.
    pub struct Twist(pub u32);
    /// ID of a **layer**, which is a region of the puzzle for each axis that may be twisted by a move on that axis.
    pub struct Layer(pub u8);
    /// ID of a **piece type**, a subset of the **pieces** of the puzzle.
    pub struct PieceType(pub u8);
}

impl Facet {
    /// Facet ID for pieces that are not on a facet, such as internals.
    pub const NONE: Facet = Facet::MAX;
}
impl Color {
    /// Color ID for internals.
    pub const INTERNAL: Color = Color::MAX;
}

/// List containing a value per piece.
pub type PerPiece<T> = GenericVec<Piece, T>;
/// List containing a value per sticker.
pub type PerSticker<T> = GenericVec<Sticker, T>;
/// List containing a value per facet.
pub type PerFacet<T> = GenericVec<Facet, T>;
/// List containing a value per color.
pub type PerColor<T> = GenericVec<Color, T>;
/// List containing a value per twist axis.
pub type PerAxis<T> = GenericVec<Axis, T>;
/// List containing a value per twist.
pub type PerTwist<T> = GenericVec<Twist, T>;
/// List containing a value per layer.
pub type PerLayer<T> = GenericVec<Layer, T>;
/// List containing a value per piece type.
pub type PerPieceType<T> = GenericVec<PieceType, T>;

/// Set of pieces in a puzzle.
pub type PieceSet = Set64<Piece>;
/// Set of stickers in a puzzle.
pub type StickerSet = Set64<Sticker>;
/// Set of facets in a puzzle.
pub type FacetSet = Set64<Facet>;
/// Set of colors in a puzzle.
pub type ColorSet = Set64<Color>;

/// Piece info.
#[derive(Debug, Clone, PartialEq)]
pub struct PieceInfo {
    /// Unordered list of stickers on the piece.
    pub stickers: SmallVec<[Sticker; 8]>,
    /// Piece type.
    pub piece_type: PieceType,
    /// Centroid of the piece.
    pub centroid: Vector,
    /// Polytope of the piece.
    pub(crate) polytope: AtomicPolytopeRef,
}

/// Sticker info.
#[derive(Debug, Clone, PartialEq)]
pub struct StickerInfo {
    /// Piece that the sticker is part of.
    pub piece: Piece,
    /// Color on the sticker.
    pub color: Color,
}

/// Facet info.
#[derive(Debug, Clone, PartialEq)]
pub struct FacetInfo {
    /// Human-friendly name for the facet. (e.g., "Up", "Right", etc.)
    pub name: String,
    /// Point on the facet that is closest to the origin. This is a scalar
    /// multiple of the facet's normal vector.
    pub pole: Vector,
    /// Name of default color.
    pub default_color: Option<String>,
}

/// Twist axis info.
#[derive(Debug, Clone, PartialEq)]
pub struct AxisInfo {
    /// Human-friendly name for the twist axis. (e.g, "U", "R", etc.)
    pub name: String,
    /// Layer.
    pub layers: PerLayer<LayerInfo>,
}
impl AsRef<str> for AxisInfo {
    fn as_ref(&self) -> &str {
        &self.name
    }
}

/// Layer info.
#[derive(Debug, Clone, PartialEq)]
pub struct LayerInfo {
    /// Manifold that bounds the bottom of the layer.
    pub(crate) bottom: ManifoldRef,
    /// Manifold that bounds the top of the layer, if any.
    pub(crate) top: Option<ManifoldRef>,
}

/// Twist info.
#[derive(Debug, Clone, PartialEq)]
pub struct TwistInfo {
    /// Human-friendly name for the twist. (e.g., "U2", "R'", etc.)
    pub name: String,

    /// Value of this twist in quarter turn metric.
    pub qtm: usize,

    /// Twist axis to use to determine which pieces are moved by the twist.
    pub axis: Axis,
    /// Transforation to apply to pieces.
    pub(crate) transform: IsometryId,

    /// Opposite twist. With a reversed layer mask, this applies the
    /// same transformation to the same pieces. For example, R and L' are
    /// opposite twists on a 3x3x3.
    pub opposite: Option<Twist>,

    /// Reverse twist, which undoes this one.
    pub reverse: Twist,
}
impl AsRef<str> for TwistInfo {
    fn as_ref(&self) -> &str {
        &self.name
    }
}

/// Piece type info.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PieceTypeInfo {
    /// TODO: remove and replace with piece type hierarchy
    pub name: String,
}
impl AsRef<str> for PieceTypeInfo {
    fn as_ref(&self) -> &str {
        &self.name
    }
}

/// Color info.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ColorInfo {
    /// User-facing color name.
    pub name: String,
    /// Optional string selecting a default color from the global color palette.
    pub default_color: Option<String>,
}
