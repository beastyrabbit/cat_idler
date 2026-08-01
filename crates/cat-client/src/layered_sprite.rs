//! Deterministic, owner-scoped composition for layered pixel-art visuals.
//!
//! Adapted from the untracked `the-shrine-upgrade` source branch foundation.
//! This target's [`bevy::prelude::AssetPlugin`] serves the repository root, so
//! callers use the existing `public/images/...` asset-relative paths. Renderer
//! owners remain responsible for binding authoritative visual-state art keys;
//! this module deliberately registers no systems and renders no domain itself.
//!
//! The caller keeps one root entity per [`VisualOwner`] and stores the current
//! [`VariantSignature`] on that root. Layer entities are tagged with the same
//! owner and a [`LayerSlot`]. Calling [`VariantSpec::reconcile`] then decides
//! whether nothing changed or whether only that owner's layer entities need to
//! be despawned and rebuilt. The root transform is deliberately outside the
//! signature, so moving a visual never forces its image layers to be recreated.

use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

use bevy::prelude::{AssetServer, Component, IVec2, Image, Sprite, Transform, UVec2, Vec2, Vec3};

/// Stable identity shared by a layered visual's root and all of its layers.
///
/// `kind` keeps independently keyed domains from colliding (for example,
/// `"building"` and `"world-site"`), while `key` should be derived from stable
/// snapshot identity rather than an ephemeral Bevy [`bevy::prelude::Entity`].
#[derive(Component, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VisualOwner {
    pub kind: String,
    pub key: String,
}

impl VisualOwner {
    pub fn new(kind: impl Into<String>, key: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            key: key.into(),
        }
    }
}

/// Deterministic draw position and semantic name for one image layer.
///
/// The integer order is converted to a local z offset by
/// [`SpritePart::geometry`]. `name` is the first stable tie-breaker when
/// multiple parts share an order.
#[derive(Component, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LayerSlot {
    pub order: i16,
    pub name: String,
}

impl LayerSlot {
    pub fn new(order: i16, name: impl Into<String>) -> Self {
        Self {
            order,
            name: name.into(),
        }
    }
}

/// Conditions under which a sprite part participates in a resolved variant.
///
/// Named levels support independent progress axes such as `width`, `depth`,
/// and `darkness`; flags cover discrete workshop states such as `blocked` or
/// `has-output`. Composite predicates keep those rules in the data definition
/// instead of scattering them through rendering systems.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VisibilityPredicate {
    Always,
    LevelAtLeast { axis: String, level: u8 },
    LevelRange { axis: String, min: u8, max: u8 },
    Flag(String),
    All(Vec<Self>),
    Any(Vec<Self>),
    Not(Box<Self>),
}

impl VisibilityPredicate {
    pub fn level_at_least(axis: impl Into<String>, level: u8) -> Self {
        Self::LevelAtLeast {
            axis: axis.into(),
            level,
        }
    }

    pub fn level_range(axis: impl Into<String>, min: u8, max: u8) -> Self {
        Self::LevelRange {
            axis: axis.into(),
            min,
            max,
        }
    }

    pub fn flag(flag: impl Into<String>) -> Self {
        Self::Flag(flag.into())
    }

    pub fn all(predicates: impl IntoIterator<Item = Self>) -> Self {
        Self::All(predicates.into_iter().collect())
    }

    pub fn any(predicates: impl IntoIterator<Item = Self>) -> Self {
        Self::Any(predicates.into_iter().collect())
    }

    pub fn negate(predicate: Self) -> Self {
        Self::Not(Box::new(predicate))
    }

    pub fn is_visible(&self, state: &VariantState) -> bool {
        match self {
            Self::Always => true,
            Self::LevelAtLeast { axis, level } => state.level(axis) >= *level,
            Self::LevelRange { axis, min, max } => {
                let value = state.level(axis);
                (*min..=*max).contains(&value)
            }
            Self::Flag(flag) => state.has_flag(flag),
            Self::All(predicates) => predicates
                .iter()
                .all(|predicate| predicate.is_visible(state)),
            Self::Any(predicates) => predicates
                .iter()
                .any(|predicate| predicate.is_visible(state)),
            Self::Not(predicate) => !predicate.is_visible(state),
        }
    }
}

/// Snapshot-derived values used to select visible parts.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VariantState {
    levels: BTreeMap<String, u8>,
    flags: BTreeSet<String>,
}

impl VariantState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_level(mut self, axis: impl Into<String>, level: u8) -> Self {
        self.levels.insert(axis.into(), level);
        self
    }

    pub fn with_flag(mut self, flag: impl Into<String>) -> Self {
        self.flags.insert(flag.into());
        self
    }

    pub fn with_flag_value(mut self, flag: impl Into<String>, visible: bool) -> Self {
        let flag = flag.into();
        if visible {
            self.flags.insert(flag);
        } else {
            self.flags.remove(&flag);
        }
        self
    }

    pub fn level(&self, axis: &str) -> u8 {
        self.levels.get(axis).copied().unwrap_or(0)
    }

    pub fn has_flag(&self, flag: &str) -> bool {
        self.flags.contains(flag)
    }
}

/// Pixel dimensions common to every part of one layered visual.
///
/// Dimensions must be non-zero and the canvas must contain a whole number of
/// source tiles on each axis. This catches accidental high-resolution or
/// incorrectly cropped assets at spec construction time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CanvasSpec {
    pub tile_pixels: UVec2,
    pub canvas_pixels: UVec2,
}

impl CanvasSpec {
    pub fn new(tile_pixels: UVec2, canvas_pixels: UVec2) -> Result<Self, VariantSpecError> {
        if tile_pixels.x == 0 || tile_pixels.y == 0 || canvas_pixels.x == 0 || canvas_pixels.y == 0
        {
            return Err(VariantSpecError::ZeroDimension);
        }
        if !canvas_pixels.x.is_multiple_of(tile_pixels.x)
            || !canvas_pixels.y.is_multiple_of(tile_pixels.y)
        {
            return Err(VariantSpecError::PartialTileCanvas {
                tile_pixels,
                canvas_pixels,
            });
        }
        Ok(Self {
            tile_pixels,
            canvas_pixels,
        })
    }

    pub fn tile_dimensions(self) -> UVec2 {
        self.canvas_pixels / self.tile_pixels
    }

    pub fn world_size(self, tile_world_size: Vec2) -> Vec2 {
        Vec2::new(
            self.canvas_pixels.x as f32 / self.tile_pixels.x as f32 * tile_world_size.x,
            self.canvas_pixels.y as f32 / self.tile_pixels.y as f32 * tile_world_size.y,
        )
    }
}

/// Declarative description of one image in a variant.
///
/// Paths are asset-root relative and can be passed directly to Bevy 0.19's
/// [`AssetServer::load`]. Pixel dimensions and offsets are converted to world
/// units from the surrounding [`CanvasSpec`], keeping 16-pixel tiles crisp even
/// when larger props (for example 34×16 benches) are composed with them.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct SpritePart {
    pub slot: LayerSlot,
    pub asset_path: String,
    pub draw_pixels: Option<UVec2>,
    pub offset_pixels: IVec2,
    pub visible_when: VisibilityPredicate,
    tile_pixels: UVec2,
}

impl SpritePart {
    pub fn new(slot: LayerSlot, asset_path: impl Into<String>) -> Self {
        Self {
            slot,
            asset_path: asset_path.into(),
            draw_pixels: None,
            offset_pixels: IVec2::ZERO,
            visible_when: VisibilityPredicate::Always,
            tile_pixels: UVec2::ONE,
        }
    }

    pub fn with_draw_pixels(mut self, draw_pixels: UVec2) -> Self {
        self.draw_pixels = Some(draw_pixels);
        self
    }

    pub fn with_offset_pixels(mut self, offset_pixels: IVec2) -> Self {
        self.offset_pixels = offset_pixels;
        self
    }

    pub fn visible_when(mut self, predicate: VisibilityPredicate) -> Self {
        self.visible_when = predicate;
        self
    }

    pub fn geometry(&self, tile_world_size: Vec2, z_step: f32) -> LayerGeometry {
        let draw_pixels = self
            .draw_pixels
            .expect("resolved sprite parts always have draw dimensions");
        let pixel_world_size = Vec2::new(
            tile_world_size.x / self.tile_pixels.x as f32,
            tile_world_size.y / self.tile_pixels.y as f32,
        );
        LayerGeometry {
            custom_size: Vec2::new(
                draw_pixels.x as f32 * pixel_world_size.x,
                draw_pixels.y as f32 * pixel_world_size.y,
            ),
            translation: Vec2::new(
                self.offset_pixels.x as f32 * pixel_world_size.x,
                self.offset_pixels.y as f32 * pixel_world_size.y,
            ),
            z: self.slot.order as f32 * z_step,
        }
    }

    /// Build the Bevy 0.19 sprite component for this already-resolved part.
    pub fn sprite(&self, assets: &AssetServer, tile_world_size: Vec2) -> Sprite {
        let geometry = self.geometry(tile_world_size, 0.0);
        Sprite {
            image: assets.load::<Image>(self.asset_path.clone()),
            custom_size: Some(geometry.custom_size),
            ..Default::default()
        }
    }

    /// Build the layer-local transform. The owning root carries world position.
    pub fn local_transform(&self, tile_world_size: Vec2, z_step: f32) -> Transform {
        let geometry = self.geometry(tile_world_size, z_step);
        Transform::from_translation(Vec3::new(
            geometry.translation.x,
            geometry.translation.y,
            geometry.z,
        ))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayerGeometry {
    pub custom_size: Vec2,
    pub translation: Vec2,
    pub z: f32,
}

/// Validated collection of all possible parts for one visual family.
#[derive(Clone, Debug)]
pub struct VariantSpec {
    canvas: CanvasSpec,
    parts: Vec<SpritePart>,
}

impl VariantSpec {
    pub fn new(
        canvas: CanvasSpec,
        parts: impl IntoIterator<Item = SpritePart>,
    ) -> Result<Self, VariantSpecError> {
        let mut parts: Vec<_> = parts.into_iter().collect();
        for part in &mut parts {
            if part.asset_path.trim().is_empty() {
                return Err(VariantSpecError::EmptyAssetPath);
            }
            if let Some(draw_pixels) = part.draw_pixels
                && (draw_pixels.x == 0 || draw_pixels.y == 0)
            {
                return Err(VariantSpecError::ZeroDimension);
            }
            part.draw_pixels.get_or_insert(canvas.canvas_pixels);
            part.tile_pixels = canvas.tile_pixels;
        }
        parts.sort_by(part_order);
        Ok(Self { canvas, parts })
    }

    pub fn canvas(&self) -> CanvasSpec {
        self.canvas
    }

    pub fn resolve(&self, state: &VariantState) -> ResolvedVariant {
        let parts: Vec<_> = self
            .parts
            .iter()
            .filter(|part| part.visible_when.is_visible(state))
            .cloned()
            .collect();
        let signature = VariantSignature::from_parts(&parts);
        ResolvedVariant {
            canvas: self.canvas,
            parts,
            signature,
        }
    }

    /// Compare a desired variant with the signature stored on its owner root.
    ///
    /// A [`ReconcilePlan::Rebuild`] applies only to the returned owner. The
    /// integration system should despawn entities matching that owner and
    /// carrying [`LayerSlot`], spawn the returned parts as root children, and
    /// replace the root's signature. No other visual entities are touched.
    pub fn reconcile(
        &self,
        owner: &VisualOwner,
        current: Option<&VariantSignature>,
        state: &VariantState,
    ) -> ReconcilePlan {
        let resolved = self.resolve(state);
        if current == Some(resolved.signature()) {
            ReconcilePlan::Unchanged
        } else {
            ReconcilePlan::Rebuild {
                owner: owner.clone(),
                signature: resolved.signature,
                parts: resolved.parts,
            }
        }
    }
}

fn part_order(left: &SpritePart, right: &SpritePart) -> std::cmp::Ordering {
    left.slot
        .cmp(&right.slot)
        .then_with(|| left.asset_path.cmp(&right.asset_path))
        .then_with(|| left.offset_pixels.x.cmp(&right.offset_pixels.x))
        .then_with(|| left.offset_pixels.y.cmp(&right.offset_pixels.y))
        .then_with(|| {
            left.draw_pixels
                .map(|pixels| (pixels.x, pixels.y))
                .cmp(&right.draw_pixels.map(|pixels| (pixels.x, pixels.y)))
        })
}

/// Exact description of the active image layers, stored on the owner root.
///
/// This intentionally uses exact fields instead of a hash, avoiding collision
/// risk and making signature changes inspectable in tests and Bevy diagnostics.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct VariantSignature(Vec<PartSignature>);

impl VariantSignature {
    fn from_parts(parts: &[SpritePart]) -> Self {
        Self(
            parts
                .iter()
                .map(|part| PartSignature {
                    slot: part.slot.clone(),
                    asset_path: part.asset_path.clone(),
                    draw_pixels: part
                        .draw_pixels
                        .expect("resolved sprite parts always have draw dimensions"),
                    offset_pixels: part.offset_pixels,
                })
                .collect(),
        )
    }

    pub fn layer_count(&self) -> usize {
        self.0.len()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PartSignature {
    slot: LayerSlot,
    asset_path: String,
    draw_pixels: UVec2,
    offset_pixels: IVec2,
}

#[derive(Clone, Debug)]
pub struct ResolvedVariant {
    canvas: CanvasSpec,
    parts: Vec<SpritePart>,
    signature: VariantSignature,
}

impl ResolvedVariant {
    pub fn parts(&self) -> &[SpritePart] {
        &self.parts
    }

    pub fn signature(&self) -> &VariantSignature {
        &self.signature
    }

    pub fn canvas_world_size(&self, tile_world_size: Vec2) -> Vec2 {
        self.canvas.world_size(tile_world_size)
    }
}

/// Minimal instruction returned to the client reconciliation system.
#[derive(Clone, Debug)]
pub enum ReconcilePlan {
    Unchanged,
    Rebuild {
        owner: VisualOwner,
        signature: VariantSignature,
        parts: Vec<SpritePart>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VariantSpecError {
    ZeroDimension,
    PartialTileCanvas {
        tile_pixels: UVec2,
        canvas_pixels: UVec2,
    },
    EmptyAssetPath,
}

impl fmt::Display for VariantSpecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroDimension => formatter.write_str("sprite dimensions must be non-zero"),
            Self::PartialTileCanvas {
                tile_pixels,
                canvas_pixels,
            } => write!(
                formatter,
                "canvas {canvas_pixels:?} is not a whole number of {tile_pixels:?} tiles"
            ),
            Self::EmptyAssetPath => formatter.write_str("sprite asset path must not be empty"),
        }
    }
}

impl Error for VariantSpecError {}
