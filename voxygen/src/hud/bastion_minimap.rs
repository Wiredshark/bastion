//! bastion (B-MAP1): the Overseer minimap — a god's map, not a player's.
//!
//! Architecture (the WoW-addon tile technique, adapted):
//! - **Tile pyramid, near tier:** the world is rendered top-down into cached
//!   per-chunk tiles (32×32 texels, 1 texel/block) from the *actual loaded
//!   voxels* — buildings, trees and dig sites appear as themselves. Tiles are
//!   built off-thread ([`KeyedJobs`] on the `IMAGE_PROCESSING` slowjob pool, so
//!   re-renders trickle and never hitch the frame), hillshaded from a per-texel
//!   height field so relief reads like a rendered capture, and composited into
//!   a chunk-grid-anchored window texture.
//! - **Invalidation:** a chunk tile is re-rendered ONLY when a terrain edit
//!   lands under it (`TerrainChanges::modified_blocks` — the same client-side
//!   edit stream B5 work execution produces) or when the overseer Z-slice moves
//!   (tiles mirror the B1.6 slice: below-ground slices show that level, which
//!   is what the mining framework will want).
//! - **Far tier:** the worldgen map (1 texel/chunk) is always drawn beneath the
//!   tile layer; the tile layer alpha-fades out as the view widens past the
//!   tile window, leaving a seamless-enough handoff to worldgen scale.
//! - **Overlays:** pin/layer providers draw on top (colonists, zones, piles,
//!   camera frustum, alerts) — see [`MinimapLayer`]/[`MinimapPin`]. This is the
//!   §3s map-overlay foundation (territory, trade routes, dominion later ride
//!   the same API).
//! - **Navigation:** click jumps the god camera, drag pans it, scroll steps the
//!   zoom pyramid (colony → district → region → world).
//!
//! Vanilla is untouched: this widget replaces the vanilla `MiniMap` only
//! while the overseer HUD is active (`--bastion-overseer` + F9); a flagless
//! boot never constructs any of the state below beyond an empty struct.

use super::{TEXT_COLOR, UI_HIGHLIGHT_0, UI_MAIN, img_ids::Imgs};
use crate::{
    GlobalState,
    hud::{Graphic, Ui},
    scene::camera::Camera,
    session::settings_change::Interface as InterfaceChange,
    ui::{KeyedJobs, fonts::Fonts, img_ids},
};
use client::Client;
use common::{
    comp,
    slowjob::SlowJobPool,
    terrain::{Block, TerrainChunk, TerrainChunkSize},
    vol::{ReadVol, RectVolSize},
};
use conrod_core::{
    Color, Colorable, Labelable, Positionable, Sizeable, Widget, WidgetCommon, color, position,
    widget::{self, Button, Image, Line, Rectangle, Text},
    widget_ids,
};
use hashbrown::{HashMap, HashSet};
use image::{DynamicImage, RgbaImage};
use specs::{Join, LendJoin, WorldExt};
use std::sync::Arc;
use vek::{Rgba, Vec2, Vec3};

/// Tile window side length, in chunks. 16 chunks = 512 blocks of full-detail
/// tile coverage centered on the camera focus; beyond that the worldgen map
/// carries the picture. (Grow this if colonies outgrow it — memory is
/// ~6 KB/chunk for tiles + two 512² window buffers.)
const WINDOW_CHUNKS: i32 = 16;
/// Window side length in texels (1 texel = 1 block). Pub: the big map (B-MAP1
/// part 3) draws the same tile texture and needs the texel space.
pub const WINDOW_PX: u32 = WINDOW_CHUNKS as u32 * 32;
/// Tiles are kept cached this many chunks beyond the window (pan-back is
/// instant); beyond that they are evicted.
const EVICT_MARGIN: i32 = 4;
/// Zoom = display px per block. Max is "4 px/block" (the brief's near-zoom
/// resolution target — the texel is 1/block; display upscales it).
const ZOOM_MAX: f64 = 4.0;

/// One cached chunk tile: per-block top color + the z it was sampled at
/// (the height field that drives hillshading).
struct Tile {
    colors: Vec<Rgba<u8>>,
    heights: Vec<i16>,
}

/// Sentinel height for "nothing visible in this column".
const H_NONE: i16 = i16::MIN;

fn block_color(block: &Block) -> Option<Rgba<u8>> {
    block
        .get_color()
        .map(|rgb| Rgba::new(rgb.r, rgb.g, rgb.b, 255))
        .or_else(|| {
            matches!(block.kind(), common::terrain::BlockKind::Water)
                .then(|| Rgba::new(119, 149, 197, 255))
        })
}

/// Render one chunk's tile: per column, scan straight down from the sky (or
/// from the active Z-slice — the cutaway view the main camera shows) and take
/// the first visible block's color + height. This IS the "orthographic
/// straight-down capture" of the brief, sourced from the same voxel data the
/// renderer draws (trees/buildings are terrain voxels), CPU-side — see the
/// findings doc for the recorded RTT-vs-CPU decision.
fn build_tile(chunk: &TerrainChunk, slice_z: Option<i32>) -> Tile {
    let size = TerrainChunkSize::RECT_SIZE.map(|e| e as i32);
    let n = (size.x * size.y) as usize;
    let mut colors = vec![Rgba::zero(); n];
    let mut heights = vec![H_NONE; n];
    // get_max_z is exclusive-ish of content above; scanning from it inclusive
    // is safe (OOB gets return Err -> treated as air).
    let top = slice_z
        .map(|s| s.min(chunk.get_max_z()))
        .unwrap_or_else(|| chunk.get_max_z());
    let bottom = chunk.get_min_z();
    for y in 0..size.y {
        for x in 0..size.x {
            let idx = (y * size.x + x) as usize;
            let mut z = top;
            while z >= bottom {
                if let Some(c) = chunk.get(Vec3::new(x, y, z)).ok().and_then(block_color) {
                    colors[idx] = c;
                    heights[idx] = z.clamp(i16::MIN as i32 + 1, i16::MAX as i32) as i16;
                    break;
                }
                z -= 1;
            }
        }
    }
    Tile { colors, heights }
}

/// Which overlay layers exist. This enum is the §3s layer registry — future
/// consumers (territory §3w, trade routes, dominion) add variants here and a
/// pin provider in the widget (or push [`MinimapPin`]s via
/// [`BastionMinimapTiles::extra_pins`]).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MinimapLayer {
    Colonists,
    Zones,
    Piles,
    Frustum,
    Alerts,
    Weather,
}

impl MinimapLayer {
    pub const ALL: [MinimapLayer; 6] = [
        MinimapLayer::Colonists,
        MinimapLayer::Zones,
        MinimapLayer::Piles,
        MinimapLayer::Frustum,
        MinimapLayer::Alerts,
        MinimapLayer::Weather,
    ];

    /// One-letter chip label (icon art is a backlog item for the asset lab).
    pub fn chip(&self) -> &'static str {
        match self {
            MinimapLayer::Colonists => "C",
            MinimapLayer::Zones => "Z",
            MinimapLayer::Piles => "P",
            MinimapLayer::Frustum => "F",
            MinimapLayer::Alerts => "!",
            MinimapLayer::Weather => "W",
        }
    }
}

/// Per-layer visibility (the ON/OFF of the B5.6 visuals philosophy; SUBTLE
/// for the map means the layer's alpha is already tuned to sit under the
/// terrain reading — pins never shout louder than the tiles).
#[derive(Copy, Clone, Debug)]
pub struct LayerFlags {
    pub colonists: bool,
    pub zones: bool,
    pub piles: bool,
    pub frustum: bool,
    pub alerts: bool,
    pub weather: bool,
}

impl Default for LayerFlags {
    fn default() -> Self {
        Self {
            colonists: true,
            zones: true,
            piles: true,
            frustum: true,
            alerts: true,
            weather: true,
        }
    }
}

impl LayerFlags {
    pub fn get(&self, l: MinimapLayer) -> bool {
        match l {
            MinimapLayer::Colonists => self.colonists,
            MinimapLayer::Zones => self.zones,
            MinimapLayer::Piles => self.piles,
            MinimapLayer::Frustum => self.frustum,
            MinimapLayer::Alerts => self.alerts,
            MinimapLayer::Weather => self.weather,
        }
    }

    pub fn toggle(&mut self, l: MinimapLayer) {
        match l {
            MinimapLayer::Colonists => self.colonists = !self.colonists,
            MinimapLayer::Zones => self.zones = !self.zones,
            MinimapLayer::Piles => self.piles = !self.piles,
            MinimapLayer::Frustum => self.frustum = !self.frustum,
            MinimapLayer::Alerts => self.alerts = !self.alerts,
            MinimapLayer::Weather => self.weather = !self.weather,
        }
    }
}

/// An externally-supplied map pin (the open half of the pin API): anything
/// that wants to appear on the overseer map — alerts now; territory markers,
/// route endpoints, dominion badges later — pushes one of these into
/// [`BastionMinimapTiles::extra_pins`] each frame (cleared by the pusher).
/// Drawn on the Alerts layer.
#[derive(Copy, Clone, Debug)]
pub struct MinimapPin {
    /// World position (blocks).
    pub wpos: Vec2<f32>,
    /// RGBA, 0..1.
    pub color: [f32; 4],
    /// Pin square side, in display px.
    pub size: f32,
    /// Draw a white halo behind the pin (emphasis / selection).
    pub halo: bool,
}

/// The tile engine: cache, invalidation, compositing, and the minimap's
/// session-local view state (zoom + layer flags). Owned by the Hud; only
/// maintained while the overseer HUD is active.
pub struct BastionMinimapTiles {
    tiles: HashMap<Vec2<i32>, Tile>,
    /// Chunks whose tile must be re-rendered (terrain edit / slice change).
    /// The old tile keeps displaying until the replacement lands — no flicker.
    stale: HashSet<Vec2<i32>>,
    keyed_jobs: KeyedJobs<(Vec2<i32>, u32), Tile>,
    /// Quantized active slice; changing it re-renders everything (trickled).
    slice_key: Option<i32>,
    /// Bumped per slice change; part of the job key so an in-flight render
    /// for a previous slice can never be mistaken for a current one.
    slice_rev: u32,
    /// Window origin (min corner), in chunk coords. Chunk-grid-anchored so
    /// panning inside the window only moves the widget's source rectangle;
    /// the texture is recomposited only on re-anchor.
    anchor: Vec2<i32>,
    anchored: bool,
    /// Unshaded per-block colors for the window (base for re-shading).
    base: Vec<Rgba<u8>>,
    /// Per-block sampled height for the window (hillshade input).
    heights: Vec<i16>,
    /// Shaded output, uploaded to the UI on change.
    composited: RgbaImage,
    image_id: conrod_core::image::Id,
    image_dirty: bool,
    /// Display px per block (the zoom pyramid position).
    pub zoom: f64,
    pub layers: LayerFlags,
    /// Open pin input for other systems (§3s) — drawn on the Alerts layer.
    pub extra_pins: Vec<MinimapPin>,
}

impl BastionMinimapTiles {
    pub fn new(ui: &mut Ui) -> Self {
        let composited = RgbaImage::from_pixel(WINDOW_PX, WINDOW_PX, image::Rgba([0, 0, 0, 0]));
        Self {
            tiles: HashMap::new(),
            stale: HashSet::new(),
            keyed_jobs: KeyedJobs::new("IMAGE_PROCESSING"),
            slice_key: None,
            slice_rev: 0,
            anchor: Vec2::zero(),
            anchored: false,
            base: vec![Rgba::zero(); (WINDOW_PX * WINDOW_PX) as usize],
            heights: vec![H_NONE; (WINDOW_PX * WINDOW_PX) as usize],
            image_id: ui.add_graphic(Graphic::Image(
                Arc::new(DynamicImage::ImageRgba8(composited.clone())),
                Some(Rgba::from([0.0, 0.0, 0.0, 0.0])),
            )),
            composited,
            image_dirty: false,
            zoom: 1.5,
            layers: LayerFlags::default(),
            extra_pins: Vec::new(),
        }
    }

    pub fn image_id(&self) -> conrod_core::image::Id { self.image_id }

    /// Window origin in world block coords.
    pub fn anchor_wpos(&self) -> Vec2<f32> {
        (self.anchor * TerrainChunkSize::RECT_SIZE.map(|e| e as i32)).map(|e| e as f32)
    }

    pub fn is_anchored(&self) -> bool { self.anchored }

    /// Per-frame upkeep (overseer HUD active only): invalidate, (re)anchor,
    /// trickle tile renders, upload on change. Never blocks on a render.
    pub fn maintain(
        &mut self,
        client: &Client,
        ui: &mut Ui,
        focus: Vec3<f32>,
        slice_z: Option<f32>,
    ) {
        common_base::prof_span!("BastionMinimapTiles::maintain");
        let terrain = client.state().terrain();

        // Slice change: every cached tile is now the wrong view. Keep showing
        // the old tiles while replacements trickle in.
        let skey = slice_z.map(|s| s.floor() as i32);
        if skey != self.slice_key {
            self.slice_key = skey;
            self.slice_rev = self.slice_rev.wrapping_add(1);
            self.stale.extend(self.tiles.keys().copied());
        }

        // Terrain edits: re-render the owning chunk's tile. Any modified
        // block counts (dig, chop, build, replace) — the tile shows colors,
        // not just occupancy, so vanilla's is_terrain-flip filter would miss
        // in-place material swaps.
        for (wpos, _) in client.state().terrain_changes().modified_blocks.iter() {
            let key = terrain.pos_key(*wpos);
            if self.tiles.contains_key(&key) {
                self.stale.insert(key);
            }
        }

        // Anchor the window to the chunk grid around the camera focus;
        // re-anchor (full recomposite from cache — rare) only when the focus
        // chunk drifts outside the central deadzone.
        let fchunk: Vec2<i32> = focus.xy().map2(TerrainChunkSize::RECT_SIZE, |e, sz| {
            (e.floor() as i32).div_euclid(sz as i32)
        });
        let rel = fchunk - self.anchor;
        let dead_lo = WINDOW_CHUNKS / 4;
        let dead_hi = WINDOW_CHUNKS - WINDOW_CHUNKS / 4;
        if !self.anchored
            || rel.x < dead_lo
            || rel.y < dead_lo
            || rel.x > dead_hi
            || rel.y > dead_hi
        {
            self.anchor = fchunk - WINDOW_CHUNKS / 2;
            self.anchored = true;
            self.reblit_window();
        }

        // Trickle tile renders for missing/stale chunks inside the window.
        let pool = client.state().ecs().read_resource::<SlowJobPool>();
        let srev = self.slice_rev;
        let skey = self.slice_key;
        for cy in 0..WINDOW_CHUNKS {
            for cx in 0..WINDOW_CHUNKS {
                let key = self.anchor + Vec2::new(cx, cy);
                let needs = !self.tiles.contains_key(&key) || self.stale.contains(&key);
                if !needs {
                    continue;
                }
                let Some(chunk) = terrain.get_key_arc(key) else {
                    continue;
                };
                if let Some((_, tile)) = self.keyed_jobs.spawn(Some(&pool), (key, srev), || {
                    let chunk = Arc::clone(chunk);
                    move |_| build_tile(&chunk, skey)
                }) {
                    self.tiles.insert(key, tile);
                    self.stale.remove(&key);
                    self.blit_chunk(key);
                }
            }
        }
        drop(pool);

        // Evict tiles far outside the window.
        let anchor = self.anchor;
        let keep = |key: &Vec2<i32>| {
            let r = *key - anchor;
            r.x >= -EVICT_MARGIN
                && r.y >= -EVICT_MARGIN
                && r.x < WINDOW_CHUNKS + EVICT_MARGIN
                && r.y < WINDOW_CHUNKS + EVICT_MARGIN
        };
        self.tiles.retain(|k, _| keep(k));
        self.stale.retain(keep);

        if self.image_dirty {
            ui.replace_graphic(
                self.image_id,
                Graphic::Image(
                    Arc::new(DynamicImage::ImageRgba8(self.composited.clone())),
                    Some(Rgba::from([0.0, 0.0, 0.0, 0.0])),
                ),
            );
            self.image_dirty = false;
        }
    }

    /// Full window recomposite from the tile cache (re-anchor only).
    fn reblit_window(&mut self) {
        self.base.iter_mut().for_each(|c| *c = Rgba::zero());
        self.heights.iter_mut().for_each(|h| *h = H_NONE);
        self.composited
            .pixels_mut()
            .for_each(|p| *p = image::Rgba([0, 0, 0, 0]));
        let keys: Vec<Vec2<i32>> = self.tiles.keys().copied().collect();
        for key in keys {
            self.blit_chunk(key);
        }
        self.image_dirty = true;
    }

    /// Copy one chunk's tile into the window buffers and hillshade its
    /// region. Image row 0 is the window's NORTH edge (max y), matching the
    /// widget's source-rectangle math.
    fn blit_chunk(&mut self, key: Vec2<i32>) {
        let rel = key - self.anchor;
        if rel.x < 0 || rel.y < 0 || rel.x >= WINDOW_CHUNKS || rel.y >= WINDOW_CHUNKS {
            return;
        }
        let Some(tile) = self.tiles.get(&key) else {
            return;
        };
        let cw = TerrainChunkSize::RECT_SIZE.x as i32;
        for y in 0..cw {
            for x in 0..cw {
                let src = (y * cw + x) as usize;
                let px = (rel.x * cw + x) as u32;
                let py = WINDOW_PX - 1 - (rel.y * cw + y) as u32;
                let dst = (py * WINDOW_PX + px) as usize;
                self.base[dst] = tile.colors[src];
                self.heights[dst] = tile.heights[src];
            }
        }
        self.shade_region(
            (rel.x * cw) as u32,
            (rel.y * cw) as u32,
            cw as u32,
            cw as u32,
        );
        self.image_dirty = true;
    }

    /// Hillshade a window region (region given in WORLD-oriented window px:
    /// x east, y north, origin at the window's SW corner). Cartographic
    /// NW-high light; flat terrain shades ~neutral. 1-px shading staleness on
    /// the far side of chunk borders is accepted (corrected when the
    /// neighbor re-blits).
    fn shade_region(&mut self, x0: u32, y0: u32, w: u32, h: u32) {
        // Light toward the surface, from the north-west, well above.
        let l = Vec3::new(-0.5f32, 0.5, 0.7).normalized();
        let wp = WINDOW_PX as i32;
        for wy in y0 as i32..(y0 + h) as i32 {
            for wx in x0 as i32..(x0 + w) as i32 {
                // World-oriented (wx, wy) -> image px (iy flipped).
                let iy = WINDOW_PX as i32 - 1 - wy;
                let dst = (iy * wp + wx) as usize;
                let hc = self.heights[dst];
                if hc == H_NONE {
                    continue;
                }
                let sample = |dx: i32, dy_world: i32| -> f32 {
                    let sx = (wx + dx).clamp(0, wp - 1);
                    let sy_img = (iy - dy_world).clamp(0, wp - 1);
                    let v = self.heights[(sy_img * wp + sx) as usize];
                    if v == H_NONE { hc as f32 } else { v as f32 }
                };
                let dzdx = (sample(1, 0) - sample(-1, 0)) * 0.5;
                let dzdy = (sample(0, 1) - sample(0, -1)) * 0.5;
                let n = Vec3::new(-dzdx, -dzdy, 1.0).normalized();
                // Flat ground -> ~0.95 (near-neutral); NW slopes brighten,
                // SE slopes fall into shade.
                let shade = (0.6 + 0.5 * n.dot(l)).clamp(0.35, 1.25);
                let c = self.base[dst];
                self.composited.put_pixel(
                    wx as u32,
                    iy as u32,
                    image::Rgba([
                        (c.r as f32 * shade).min(255.0) as u8,
                        (c.g as f32 * shade).min(255.0) as u8,
                        (c.b as f32 * shade).min(255.0) as u8,
                        c.a,
                    ]),
                );
            }
        }
    }
}

widget_ids! {
    struct Ids {
        frame,
        frame_2,
        map_bg,
        worldmap_img,
        tiles_img,
        zone_rects[],
        pile_dots[],
        colonist_halos[],
        colonist_dots[],
        extra_halos[],
        extra_dots[],
        frustum_lines[],
        layer_chips[],
        zoom_plus,
        zoom_minus,
        size_btn,
        level_text,
        north_text,
        lens_bg,
        lens_text,
        toggle_btn,
    }
}

pub enum Event {
    SettingsChange(InterfaceChange),
    /// Jump the god camera focus to this world XY.
    Jump(Vec2<f32>),
    /// Pan the god camera focus by this world-space delta.
    Pan(Vec2<f32>),
    /// New zoom (display px per block) — applied to the engine by the Hud.
    Zoom(f64),
    ToggleLayer(MinimapLayer),
}

#[derive(WidgetCommon)]
pub struct BastionMiniMap<'a> {
    client: &'a Client,
    imgs: &'a Imgs,
    world_map: &'a (Vec<img_ids::Rotations>, Vec2<u32>),
    fonts: &'a Fonts,
    camera: &'a Camera,
    tiles: &'a BastionMinimapTiles,
    global_state: &'a GlobalState,
    #[conrod(common_builder)]
    common: widget::CommonBuilder,
}

impl<'a> BastionMiniMap<'a> {
    pub fn new(
        client: &'a Client,
        imgs: &'a Imgs,
        world_map: &'a (Vec<img_ids::Rotations>, Vec2<u32>),
        fonts: &'a Fonts,
        camera: &'a Camera,
        tiles: &'a BastionMinimapTiles,
        global_state: &'a GlobalState,
    ) -> Self {
        Self {
            client,
            imgs,
            world_map,
            fonts,
            camera,
            tiles,
            global_state,
            common: widget::CommonBuilder::default(),
        }
    }
}

pub struct State {
    ids: Ids,
}

/// Shared pin providers — the minimap and the big world map (B-MAP1) draw the
/// same data with their own coordinate transforms, so the queries live here.
/// Returns (world XY, selected).
pub fn collect_colonist_pins(client: &Client) -> Vec<(Vec2<f32>, bool)> {
    let ecs = client.state().ecs();
    let positions = ecs.read_storage::<comp::Pos>();
    let colonists = ecs.read_storage::<comp::Colonist>();
    let selected = ecs.read_storage::<comp::BastionSelected>();
    let entities = ecs.entities();
    (&entities, &colonists, &positions)
        .join()
        .map(|(e, _, pos)| (pos.0.xy(), selected.contains(e)))
        .collect()
}

/// Returns (world XY, size multiplier from the pile's tier `Scale`).
pub fn collect_pile_pins(client: &Client) -> Vec<(Vec2<f32>, f32)> {
    let ecs = client.state().ecs();
    let positions = ecs.read_storage::<comp::Pos>();
    let items = ecs.read_storage::<comp::PickupItem>();
    let scales = ecs.read_storage::<comp::Scale>();
    (&items, &positions, scales.maybe())
        .join()
        .map(|(_, pos, s)| (pos.0.xy(), s.map_or(1.0, |s| s.0.clamp(1.0, 2.0))))
        .collect()
}

/// The main camera's ground footprint: the 4 screen corners unprojected onto
/// the plane `z = plane_z`, world XY. Corner order walks the screen border,
/// so consecutive (wrapping) pairs are the frustum-rect edges. Resolution-
/// agnostic: corners at (0,0)..(1,1) with res (1,1) hit NDC exactly.
pub fn frustum_ground_quad(camera: &Camera, plane_z: f32) -> [Option<Vec2<f32>>; 4] {
    let res = Vec2::new(1.0, 1.0);
    [
        Vec2::new(0.0, 0.0),
        Vec2::new(1.0, 0.0),
        Vec2::new(1.0, 1.0),
        Vec2::new(0.0, 1.0),
    ]
    .map(|c| crate::bastion::unproject_to_world_plane(camera, c, res, plane_z).map(|w| w.xy()))
}

/// Liang–Barsky segment clip against the centered rect [-hx,hx]×[-hy,hy].
/// Pub: the big map clips frustum edges the same way.
pub fn clip_seg(a: Vec2<f64>, b: Vec2<f64>, half: Vec2<f64>) -> Option<(Vec2<f64>, Vec2<f64>)> {
    let d = b - a;
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    for (p, q) in [
        (-d.x, a.x + half.x),
        (d.x, half.x - a.x),
        (-d.y, a.y + half.y),
        (d.y, half.y - a.y),
    ] {
        if p.abs() < 1e-12 {
            if q < 0.0 {
                return None;
            }
        } else {
            let r = q / p;
            if p < 0.0 {
                t0 = t0.max(r);
            } else {
                t1 = t1.min(r);
            }
        }
    }
    (t0 <= t1).then(|| (a + d * t0, a + d * t1))
}

impl Widget for BastionMiniMap<'_> {
    type Event = Vec<Event>;
    type State = State;
    type Style = ();

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        State {
            ids: Ids::new(id_gen),
        }
    }

    fn style(&self) -> Self::Style {}

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        common_base::prof_span!("BastionMiniMap::update");
        let widget::UpdateArgs { state, ui, .. } = args;
        let mut events = Vec::new();

        let interface = &self.global_state.settings.interface;
        let show_minimap = interface.minimap_show;
        let mut scale = interface.minimap_scale;
        if scale <= 0.0 {
            scale = 1.5;
        }
        let minimap_pos = self.global_state.settings.hud_position.minimap;
        let scaled_window = Vec2::new(174.0 * scale, 190.0 * scale);
        let map_size = Vec2::new(170.0 * scale, 170.0 * scale);

        if !show_minimap {
            // Collapsed bar, mirroring vanilla so the setting behaves the same.
            Image::new(self.imgs.mmap_frame_closed)
                .w_h(scaled_window.x, 18.0 * scale)
                .color(Some(UI_MAIN))
                .top_right_with_margins_on(ui.window, minimap_pos.y, minimap_pos.x)
                .set(state.ids.frame, ui);
            if Button::image(self.imgs.mmap_closed)
                .w_h(18.0 * scale, 18.0 * scale)
                .hover_image(self.imgs.mmap_closed_hover)
                .press_image(self.imgs.mmap_closed_press)
                .top_right_with_margins_on(state.ids.frame, 0.0, 0.0)
                .image_color(UI_HIGHLIGHT_0)
                .set(state.ids.toggle_btn, ui)
                .was_clicked()
            {
                events.push(Event::SettingsChange(InterfaceChange::MinimapShow(true)));
            }
            return events;
        }

        // ---- Frame ----------------------------------------------------
        Image::new(self.imgs.mmap_frame)
            .w_h(scaled_window.x, scaled_window.y)
            .top_right_with_margins_on(ui.window, minimap_pos.y, minimap_pos.x)
            .color(Some(UI_MAIN))
            .set(state.ids.frame, ui);
        Image::new(self.imgs.mmap_frame_2)
            .w_h(scaled_window.x, scaled_window.y)
            .middle_of(state.ids.frame)
            .color(Some(UI_HIGHLIGHT_0))
            .set(state.ids.frame_2, ui);
        // The map area: input sink for click/drag/scroll and the positioning
        // parent for every layer.
        Rectangle::fill_with([map_size.x, map_size.y], color::TRANSPARENT)
            .mid_top_with_margin_on(state.ids.frame_2, 18.0 * scale)
            .set(state.ids.map_bg, ui);

        if Button::image(self.imgs.mmap_open)
            .w_h(18.0 * scale, 18.0 * scale)
            .hover_image(self.imgs.mmap_open_hover)
            .press_image(self.imgs.mmap_open_press)
            .top_right_with_margins_on(state.ids.frame, 0.0, 0.0)
            .image_color(UI_HIGHLIGHT_0)
            .set(state.ids.toggle_btn, ui)
            .was_clicked()
        {
            events.push(Event::SettingsChange(InterfaceChange::MinimapShow(false)));
        }

        // ---- View parameters ------------------------------------------
        let focus = self.camera.get_focus_pos();
        let zoom = self.tiles.zoom.max(1e-6);
        let worldsize = self.world_map.1;
        let chunk_px = TerrainChunkSize::RECT_SIZE.x as f64;
        // Zoom limits: whole world fits .. ZOOM_MAX px/block.
        let zoom_min =
            (map_size.x / (worldsize.reduce_partial_max() as f64 * chunk_px)).min(ZOOM_MAX);
        // Visible width in blocks decides the pyramid level + tile fade.
        let view_blocks = map_size.x / zoom;

        // ---- Worldgen underlay (far tier, always beneath) --------------
        let focus_c = focus.xy().map(|e| e as f64) / chunk_px;
        let src_chunks = Vec2::new(map_size.x, map_size.y) / (zoom * chunk_px);
        let world_src = position::Rect::from_xy_dim([focus_c.x, worldsize.y as f64 - focus_c.y], [
            src_chunks.x,
            src_chunks.y,
        ]);
        Image::new(self.world_map.0[0].none)
            .middle_of(state.ids.map_bg)
            .w_h(map_size.x, map_size.y)
            .parent(state.ids.map_bg)
            .source_rectangle(world_src)
            .graphics_for(state.ids.map_bg)
            .set(state.ids.worldmap_img, ui);

        // ---- Rendered tile layer (near tier, fades to worldgen) --------
        // Fully opaque while the view fits inside the tile window; gone once
        // the view is twice the window. (OOB source sampling is safe: the
        // graphic's border color is transparent.)
        let win = WINDOW_PX as f64;
        let tile_alpha = (1.0 - (view_blocks - win) / win).clamp(0.0, 1.0) as f32;
        if self.tiles.is_anchored() && tile_alpha > 0.0 {
            let origin = self.tiles.anchor_wpos().map(|e| e as f64);
            let tex = Vec2::new(focus.x as f64 - origin.x, win - (focus.y as f64 - origin.y));
            let tiles_src =
                position::Rect::from_xy_dim([tex.x, tex.y], [map_size.x / zoom, map_size.y / zoom]);
            Image::new(self.tiles.image_id())
                .middle_of(state.ids.map_bg)
                .w_h(map_size.x, map_size.y)
                .parent(state.ids.map_bg)
                .source_rectangle(tiles_src)
                .color(Some(Color::Rgba(1.0, 1.0, 1.0, tile_alpha)))
                .graphics_for(state.ids.map_bg)
                .set(state.ids.tiles_img, ui);
        }

        // ---- Shared transforms ------------------------------------------
        let half = Vec2::new(map_size.x / 2.0, map_size.y / 2.0);
        // World XY -> map px relative to map center (north-up: no rotation,
        // conrod +y is up, world +y is north).
        let wpos_to_px = |w: Vec2<f32>| -> Vec2<f64> {
            Vec2::new((w.x - focus.x) as f64 * zoom, (w.y - focus.y) as f64 * zoom)
        };
        let inside = |p: Vec2<f64>, m: f64| p.x.abs() <= half.x - m && p.y.abs() <= half.y - m;

        // ---- Zones layer (draped overlay's 2D projection) ---------------
        if self.tiles.layers.zones {
            let designations = self.client.bastion_designations();
            if state.ids.zone_rects.len() < designations.len() {
                state.update(|s| {
                    s.ids
                        .zone_rects
                        .resize(designations.len(), &mut ui.widget_id_generator())
                });
            }
            for (i, (region, kind, _)) in designations.iter().enumerate() {
                // b-1's one zone-color legend; the map applies its own alpha.
                let [r, g, b] = crate::bastion::tools::zone_rgb(*kind);
                let lo = wpos_to_px(Vec2::new(region.min.x as f32, region.min.y as f32));
                let hi = wpos_to_px(Vec2::new(
                    region.max.x as f32 + 1.0,
                    region.max.y as f32 + 1.0,
                ));
                // Clip the rect to the map area.
                let c_lo = Vec2::new(lo.x.max(-half.x), lo.y.max(-half.y));
                let c_hi = Vec2::new(hi.x.min(half.x), hi.y.min(half.y));
                if c_lo.x >= c_hi.x || c_lo.y >= c_hi.y {
                    continue;
                }
                let dim = c_hi - c_lo;
                let center = (c_lo + c_hi) / 2.0;
                Rectangle::fill_with([dim.x, dim.y], Color::Rgba(r, g, b, 0.32))
                    .x_y_position_relative_to(
                        state.ids.map_bg,
                        position::Relative::Scalar(center.x),
                        position::Relative::Scalar(center.y),
                    )
                    .parent(state.ids.map_bg)
                    .graphics_for(state.ids.map_bg)
                    .set(state.ids.zone_rects[i], ui);
            }
        }

        // ---- Piles layer -------------------------------------------------
        if self.tiles.layers.piles {
            let pile_pts: Vec<(Vec2<f64>, f64)> = collect_pile_pins(self.client)
                .into_iter()
                .filter_map(|(wpos, size_mul)| {
                    let p = wpos_to_px(wpos);
                    inside(p, 2.0).then(|| (p, 3.0 * size_mul as f64))
                })
                .collect();
            if state.ids.pile_dots.len() < pile_pts.len() {
                state.update(|s| {
                    s.ids
                        .pile_dots
                        .resize(pile_pts.len(), &mut ui.widget_id_generator())
                });
            }
            for (i, (p, sz)) in pile_pts.iter().enumerate() {
                Rectangle::fill_with([*sz, *sz], Color::Rgba(0.95, 0.8, 0.3, 0.9))
                    .x_y_position_relative_to(
                        state.ids.map_bg,
                        position::Relative::Scalar(p.x),
                        position::Relative::Scalar(p.y),
                    )
                    .parent(state.ids.map_bg)
                    .graphics_for(state.ids.map_bg)
                    .set(state.ids.pile_dots[i], ui);
            }
        }

        // ---- Colonists layer ----------------------------------------------
        if self.tiles.layers.colonists {
            let col_pts: Vec<(Vec2<f64>, bool)> = collect_colonist_pins(self.client)
                .into_iter()
                .filter_map(|(wpos, sel)| {
                    let p = wpos_to_px(wpos);
                    inside(p, 2.0).then(|| (p, sel))
                })
                .collect();
            if state.ids.colonist_dots.len() < col_pts.len() {
                state.update(|s| {
                    s.ids
                        .colonist_dots
                        .resize(col_pts.len(), &mut ui.widget_id_generator());
                    s.ids
                        .colonist_halos
                        .resize(col_pts.len(), &mut ui.widget_id_generator());
                });
            }
            for (i, (p, is_sel)) in col_pts.iter().enumerate() {
                if *is_sel {
                    Rectangle::fill_with([9.0, 9.0], Color::Rgba(1.0, 0.9, 0.2, 0.85))
                        .x_y_position_relative_to(
                            state.ids.map_bg,
                            position::Relative::Scalar(p.x),
                            position::Relative::Scalar(p.y),
                        )
                        .parent(state.ids.map_bg)
                        .graphics_for(state.ids.map_bg)
                        .set(state.ids.colonist_halos[i], ui);
                }
                Rectangle::fill_with([5.0, 5.0], Color::Rgba(0.95, 0.98, 1.0, 1.0))
                    .x_y_position_relative_to(
                        state.ids.map_bg,
                        position::Relative::Scalar(p.x),
                        position::Relative::Scalar(p.y),
                    )
                    .parent(state.ids.map_bg)
                    .graphics_for(state.ids.map_bg)
                    .set(state.ids.colonist_dots[i], ui);
            }
        }

        // ---- Alerts / extra pins (the open §3s hook) ----------------------
        if self.tiles.layers.alerts && !self.tiles.extra_pins.is_empty() {
            let pins = &self.tiles.extra_pins;
            if state.ids.extra_dots.len() < pins.len() {
                state.update(|s| {
                    s.ids
                        .extra_dots
                        .resize(pins.len(), &mut ui.widget_id_generator());
                    s.ids
                        .extra_halos
                        .resize(pins.len(), &mut ui.widget_id_generator());
                });
            }
            for (i, pin) in pins.iter().enumerate() {
                let p = wpos_to_px(pin.wpos);
                if !inside(p, 2.0) {
                    continue;
                }
                let [r, g, b, a] = pin.color;
                if pin.halo {
                    Rectangle::fill_with(
                        [pin.size as f64 + 4.0, pin.size as f64 + 4.0],
                        Color::Rgba(1.0, 1.0, 1.0, 0.8),
                    )
                    .x_y_position_relative_to(
                        state.ids.map_bg,
                        position::Relative::Scalar(p.x),
                        position::Relative::Scalar(p.y),
                    )
                    .parent(state.ids.map_bg)
                    .graphics_for(state.ids.map_bg)
                    .set(state.ids.extra_halos[i], ui);
                }
                Rectangle::fill_with([pin.size as f64, pin.size as f64], Color::Rgba(r, g, b, a))
                    .x_y_position_relative_to(
                        state.ids.map_bg,
                        position::Relative::Scalar(p.x),
                        position::Relative::Scalar(p.y),
                    )
                    .parent(state.ids.map_bg)
                    .graphics_for(state.ids.map_bg)
                    .set(state.ids.extra_dots[i], ui);
            }
        }

        // ---- Camera frustum (what the main view sees) ---------------------
        if self.tiles.layers.frustum {
            let ground: Vec<Option<Vec2<f64>>> = frustum_ground_quad(self.camera, focus.z)
                .into_iter()
                .map(|c| c.map(wpos_to_px))
                .collect();
            if state.ids.frustum_lines.len() < 4 {
                state.update(|s| s.ids.frustum_lines.resize(4, &mut ui.widget_id_generator()));
            }
            let map_abs = ui.rect_of(state.ids.map_bg).map(|r| r.xy());
            if let Some(center) = map_abs {
                for i in 0..4 {
                    if let (Some(a), Some(b)) = (ground[i], ground[(i + 1) % 4])
                        && let Some((ca, cb)) = clip_seg(a, b, half)
                    {
                        Line::abs([center[0] + ca.x, center[1] + ca.y], [
                            center[0] + cb.x,
                            center[1] + cb.y,
                        ])
                        .color(Color::Rgba(1.0, 1.0, 1.0, 0.7))
                        .thickness(1.5)
                        .parent(state.ids.map_bg)
                        .graphics_for(state.ids.map_bg)
                        .set(state.ids.frustum_lines[i], ui);
                    }
                }
            }
        }

        // ---- Zoom buttons + pyramid-level label ---------------------------
        const ZOOM_FACTOR: f64 = 2.0;
        if Button::image(self.imgs.mmap_minus)
            .w_h(16.0 * scale, 18.0 * scale)
            .hover_image(self.imgs.mmap_minus_hover)
            .press_image(self.imgs.mmap_minus_press)
            .top_left_with_margins_on(state.ids.frame, 0.0, 0.0)
            .image_color(UI_HIGHLIGHT_0)
            .enabled(zoom > zoom_min)
            .set(state.ids.zoom_minus, ui)
            .was_clicked()
            && zoom > zoom_min
        {
            events.push(Event::Zoom((zoom / ZOOM_FACTOR).clamp(zoom_min, ZOOM_MAX)));
        }
        if Button::image(self.imgs.mmap_plus)
            .w_h(18.0 * scale, 18.0 * scale)
            .hover_image(self.imgs.mmap_plus_hover)
            .press_image(self.imgs.mmap_plus_press)
            .right_from(state.ids.zoom_minus, 0.0)
            .image_color(UI_HIGHLIGHT_0)
            .enabled(zoom < ZOOM_MAX)
            .set(state.ids.zoom_plus, ui)
            .was_clicked()
            && zoom < ZOOM_MAX
        {
            events.push(Event::Zoom((zoom * ZOOM_FACTOR).clamp(zoom_min, ZOOM_MAX)));
        }

        // Window-size cycle (S/M/L/XL) — steps the persisted vanilla
        // minimap-scale setting, so both minimaps share one size preference.
        const SIZE_STEPS: [f64; 4] = [1.0, 1.5, 2.0, 2.5];
        let size_idx = SIZE_STEPS
            .iter()
            .position(|s| (s - scale).abs() < 0.26)
            .unwrap_or(1);
        if Button::new()
            .w_h(20.0 * scale, 18.0 * scale)
            .label(["S", "M", "L", "XL"][size_idx])
            .label_font_size(self.fonts.cyri.scale(10))
            .label_font_id(self.fonts.cyri.conrod_id)
            .label_color(TEXT_COLOR)
            .color(Color::Rgba(0.0, 0.0, 0.0, 0.55))
            .right_from(state.ids.zoom_plus, 2.0)
            .set(state.ids.size_btn, ui)
            .was_clicked()
        {
            let next = SIZE_STEPS[(size_idx + 1) % SIZE_STEPS.len()];
            events.push(Event::SettingsChange(InterfaceChange::MinimapScale(next)));
        }

        let level = if view_blocks <= 384.0 {
            "Colony"
        } else if view_blocks <= 1536.0 {
            "District"
        } else if view_blocks <= 6144.0 {
            "Region"
        } else {
            "World"
        };
        Text::new(level)
            .mid_top_with_margin_on(state.ids.frame, 3.0 * scale)
            .font_size(self.fonts.cyri.scale(10 + (2.0 * scale) as u32))
            .font_id(self.fonts.cyri.conrod_id)
            .color(TEXT_COLOR)
            .graphics_for(state.ids.map_bg)
            .set(state.ids.level_text, ui);
        Text::new("N")
            .x_y_position_relative_to(
                state.ids.map_bg,
                position::Relative::Scalar(0.0),
                position::Relative::Scalar(half.y - 9.0),
            )
            .font_size(self.fonts.cyri.scale(11))
            .font_id(self.fonts.cyri.conrod_id)
            .color(Color::Rgba(0.75, 0.0, 0.0, 0.9))
            .graphics_for(state.ids.map_bg)
            .set(state.ids.north_text, ui);

        // R1G: one real, bounded lens on the existing map surface. The frame
        // is already camera/selection/generation-bound by the production
        // adapter; this widget only visualizes its canonical label.
        if self.tiles.layers.weather
            && let Some(lens) = crate::r1g_lens::latest_frame()
            && lens.mode() == bastion_renderer_r0d::lens::LensModeV1::Weather
            && let Some(datum) = lens.datums().first()
        {
            Rectangle::fill_with(
                [map_size.x - 14.0 * scale, 22.0 * scale],
                Color::Rgba(0.02, 0.05, 0.08, 0.82),
            )
            .mid_bottom_with_margin_on(state.ids.map_bg, 7.0 * scale)
            .graphics_for(state.ids.map_bg)
            .set(state.ids.lens_bg, ui);
            Text::new(&datum.label)
                .middle_of(state.ids.lens_bg)
                .font_size(self.fonts.cyri.scale(9 + scale as u32))
                .font_id(self.fonts.cyri.conrod_id)
                .color(Color::Rgba(0.72, 0.90, 1.0, 1.0))
                .graphics_for(state.ids.map_bg)
                .set(state.ids.lens_text, ui);
        }

        // ---- Layer chips ---------------------------------------------------
        if state.ids.layer_chips.len() < MinimapLayer::ALL.len() {
            state.update(|s| {
                s.ids
                    .layer_chips
                    .resize(MinimapLayer::ALL.len(), &mut ui.widget_id_generator())
            });
        }
        for (i, layer) in MinimapLayer::ALL.iter().enumerate() {
            let on = self.tiles.layers.get(*layer);
            let btn = Button::new()
                .w_h(13.0 * scale, 13.0 * scale)
                .label(layer.chip())
                .label_font_size(self.fonts.cyri.scale(9))
                .label_font_id(self.fonts.cyri.conrod_id)
                .label_color(if on {
                    TEXT_COLOR
                } else {
                    Color::Rgba(0.45, 0.45, 0.45, 0.8)
                })
                .color(if on {
                    Color::Rgba(0.0, 0.0, 0.0, 0.6)
                } else {
                    Color::Rgba(0.0, 0.0, 0.0, 0.25)
                });
            let btn = if i == 0 {
                btn.bottom_left_with_margins_on(state.ids.frame, 2.0 * scale, 4.0 * scale)
            } else {
                btn.right_from(state.ids.layer_chips[i - 1], 2.0 * scale)
            };
            if btn.set(state.ids.layer_chips[i], ui).was_clicked() {
                events.push(Event::ToggleLayer(*layer));
            }
        }

        // ---- Navigation input (the map IS the interface) -------------------
        // Click-to-jump.
        for click in ui.widget_input(state.ids.map_bg).clicks().left() {
            let rel = Vec2::new(click.xy[0], click.xy[1]);
            let wpos = focus.xy() + rel.map(|e| e as f32) / zoom as f32;
            events.push(Event::Jump(wpos));
        }
        // Drag-to-pan (world moves with the cursor: focus shifts opposite).
        let dragged: Vec2<f64> = ui
            .widget_input(state.ids.map_bg)
            .drags()
            .left()
            .map(|d| Vec2::<f64>::from(d.delta_xy))
            .sum();
        if dragged.map(|e| e.abs()).reduce_partial_max() > 0.0 {
            let delta = -(dragged / zoom).map(|e| e as f32);
            events.push(Event::Pan(delta));
        }
        // Scroll-to-zoom, stepped through the pyramid.
        let scrolled: f64 = ui
            .widget_input(state.ids.map_bg)
            .scrolls()
            .map(|s| s.y)
            .sum();
        if scrolled != 0.0 {
            let new_zoom = (zoom.log2() - scrolled * 0.05)
                .exp2()
                .clamp(zoom_min, ZOOM_MAX);
            if (new_zoom - zoom).abs() > f64::EPSILON {
                events.push(Event::Zoom(new_zoom));
            }
        }

        events
    }
}
