use anyhow::Result;
use std::fmt;
use std::hash::Hash;
use std::ops::*;
use std::sync::{Arc, Weak};

use ahash::AHashMap;

use super::*;
use crate::geometry::{IsometryGroup, ShapeArena, ShapeRef};

/// Puzzle type info.
pub struct PuzzleType {
    /// Reference-counted pointer to this struct.
    pub this: Weak<PuzzleType>,
    /// Human-friendly name of the puzzle.
    pub name: String,
    /// Base shape, without any internal cuts.
    pub shape: Arc<PuzzleShape>,
    /// Twist set.
    pub twists: Arc<PuzzleTwists>,

    /// Puzzle mesh for rendering.
    pub mesh: Mesh,

    /// List of pieces, indexed by ID.
    pub pieces: Vec<PieceInfo>,
    /// List of stickers, indexed by ID.
    pub stickers: Vec<StickerInfo>,
    /// List of piece types, indexed by ID.
    pub piece_types: Vec<PieceTypeInfo>,

    /// Number of moves for a full scramble.
    pub scramble_moves_count: usize,

    /// Move notation.
    pub notation: NotationScheme,

    /// Function to create a new solved puzzle state.
    pub new: Box<dyn Send + Sync + Fn(Arc<PuzzleType>) -> Box<dyn PuzzleState>>,
}
impl fmt::Debug for PuzzleType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PuzzleType")
            .field("this", &self.this)
            .field("name", &self.name)
            .field("shape", &self.shape)
            .field("twists", &self.twists)
            .field("pieces", &self.pieces)
            .field("stickers", &self.stickers)
            .field("piece_types", &self.piece_types)
            .field("scramble_moves_count", &self.scramble_moves_count)
            .field("notation", &self.notation)
            .finish()
    }
}
impl fmt::Display for PuzzleType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}
impl Hash for PuzzleType {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}
impl AsRef<str> for PuzzleType {
    fn as_ref(&self) -> &str {
        &self.name
    }
}
impl_puzzle_info_trait!(for PuzzleType { fn info(Facet) -> &FacetInfo { .shape.facets } });
impl_puzzle_info_trait!(for PuzzleType { fn info(TwistAxis) -> &TwistAxisInfo { .twists.axes } });
impl_puzzle_info_trait!(for PuzzleType { fn info(TwistTransform) -> &TwistTransformInfo { .twists.transforms } });
impl_puzzle_info_trait!(for PuzzleType { fn info(Piece) -> &PieceInfo { .pieces } });
impl_puzzle_info_trait!(for PuzzleType { fn info(Sticker) -> &StickerInfo { .stickers } });
impl_puzzle_info_trait!(for PuzzleType { fn info(PieceType) -> &PieceTypeInfo { .piece_types } });
impl PuzzleType {
    /// Returns a new solved puzzle.
    #[allow(clippy::new_ret_no_self)]
    pub fn new(&self) -> Box<dyn PuzzleState> {
        (self.new)(self.arc())
    }
    /// Returns a new reference to the `PuzzleType`.
    pub fn arc(&self) -> Arc<Self> {
        self.this
            .upgrade()
            .expect("unable to promote Weak<PuzzleType> to Arc<PuzzleType>")
    }

    /// Returns the number of dimensions.
    pub fn ndim(&self) -> u8 {
        self.shape.ndim
    }

    /// Returns the reverse of a twist.
    pub fn reverse_twist(&self, twist: Twist) -> Twist {
        Twist {
            layers: twist.layers,
            transform: self.info(twist.transform).reverse,
        }
    }
    /// Canonicalizes a twist.
    pub fn canonicalize_twist(&self, twist: Twist) -> Twist {
        let transform_info = self.info(twist.transform);
        if let Some(opposite_transform) = transform_info.opposite {
            let axis_info = &self.info(transform_info.axis);
            let layer_count = axis_info.layer_count();

            // Reverse the layer mask.
            let reversed_layers = LayerMask(
                twist.layers.0.reverse_bits()
                    >> (LayerMaskUint::BITS - axis_info.layer_count() as u32),
            );

            let opposite_twist = Twist {
                layers: reversed_layers,
                transform: opposite_transform,
            };

            // Return whichever twist has the smaller layer mask. If the layer
            // masks are equivalent, then return whichever one was generated
            // first.
            std::cmp::min(twist, opposite_twist)
        } else {
            twist
        }
    }

    /// TODO: remove or refactor
    pub fn twist_command_short_description(
        &self,
        axis_name: Option<TwistAxis>,
        direction: (),
        layers: LayerMask,
    ) -> String {
        todo!()
        // match axis_name {
        //     Some(axis) => self
        //         .notation
        //         .twist_to_string(self.canonicalize_twist(Twist {
        //             axis,
        //             direction,
        //             layers,
        //         })),
        //     None => {
        //         let dir = &self.info(direction).symbol;
        //         format!("{layers}Ø{dir}")
        //     }
        // }
    }

    pub(crate) fn create_puzzle_type_from_shapes(
        name: String,
        arena: &ShapeArena,
        pieces: Vec<ShapeRef>,
    ) -> Result<Arc<PuzzleType>> {
        let mut ret = PuzzleType {
            this: Weak::new(),
            name,
            shape: Arc::new(PuzzleShape {
                name: None,
                ndim: arena.space().ndim()?,
                facets: vec![],
                facet_order: vec![],
                radius: 1.0,
                facets_by_name: AHashMap::new(),
            }),
            twists: Arc::new(PuzzleTwists {
                name: "unknown".to_string(),
                axes: vec![],
                axis_order: vec![],
                non_empty_axes: vec![],
                axes_by_name: AHashMap::new(),
                transforms: vec![],
                symmetry: IsometryGroup::from_generators(&[])?,
                notation: NotationScheme {},
            }),
            mesh: Mesh::from_arena(arena, false)?,
            pieces: vec![],
            stickers: vec![],
            piece_types: vec![],
            scramble_moves_count: 10,
            notation: NotationScheme {},
            new: Box::new(|ty| {
                #[derive(Debug, Clone)]
                struct PuzzleStateStruct {
                    ty: Arc<PuzzleType>,
                }
                impl PuzzleState for PuzzleStateStruct {
                    fn ty(&self) -> &Arc<PuzzleType> {
                        &self.ty
                    }

                    fn clone_boxed(&self) -> Box<dyn PuzzleState> {
                        Box::new(self.clone())
                    }

                    fn twist(&mut self, twist: Twist) -> Result<(), &'static str> {
                        todo!("twist puzzle")
                    }

                    fn piece_transform(&self, p: Piece) -> Matrix {
                        Matrix::ident(self.ty.ndim())
                    }

                    fn is_solved(&self) -> bool {
                        false
                    }
                }

                Box::new(PuzzleStateStruct { ty })
            }),
        };

        Ok(Arc::new_cyclic(|this| {
            ret.this = this.clone();
            ret
        }))
    }
}

// Ok(Arc::new_cyclic(|this| PuzzleType {
//     this: this.clone(),
//     name: puzzle_name,
//     shape: Arc::new(PuzzleShape {
//         name: None,
//         ndim,
//         facets: (0..mesh.facet_count())
//             .map(|id| FacetInfo {
//                 name: (('A' as u8 + id as u8) as char).to_string(),
//                 pole: vector![],
//                 default_color: None,
//             })
//             .collect(),
//         facet_order: (0..mesh.facet_count() as u16).map(Facet).collect(),
//         radius: mesh.vertex_positions.iter().copied().fold(1.0, f32::max),
//         facets_by_name: (0..mesh.facet_count() as u16)
//             .map(|id| ((('A' as u8 + id as u8) as char).to_string(), Facet(id)))
//             .collect(),
//     }),
//     twists: Arc::new(PuzzleTwists::default()),
//     mesh,
//     pieces: (0..mesh.piece_count()).map(|id| PieceInfo {
//         stickers: mesh.pie,
//         piece_type: todo!(),
//         points: todo!(),
//     }),
//     stickers: todo!(),
//     piece_types: todo!(),
//     scramble_moves_count: todo!(),
//     notation: todo!(),
//     new: todo!(),
// }))
