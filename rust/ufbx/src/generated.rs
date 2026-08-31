// GENERATED FILE — do not edit by hand. Produced by rust/regen.sh from
// ufbx.h via bindgen/ufbx_ir.py + rust/ufbx/bindgen/generate_rust.py.
// Fixes belong in the GENERATOR (see PORTING.md); hand edits are
// silently overwritten on the next regeneration and CI diffs this file.

use crate::prelude::{
    call_close_memory_cb, call_open_file_cb, call_progress_cb, Allocator, Stream, ThreadPool,
};
use crate::prelude::{
    format_flags, Arena, Blob, BlobOpt, ExternalRef, InlineBuf, List, ListOpt, OpenFileContext,
    RawBlob, RawEnum, RawList, RawString, Real, Ref, RefList, String, StringOpt, ThreadPoolContext,
    ToRaw, Unsafe, VertexStream,
};
use std::ffi::c_void;
use std::fmt::{self, Debug};
use std::ops::{
    BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Deref, FnMut, Index,
};
use std::{marker, mem, ptr, result, str};

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct Vec2 {
    pub x: Real,
    pub y: Real,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct Vec3 {
    pub x: Real,
    pub y: Real,
    pub z: Real,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct Vec4 {
    pub x: Real,
    pub y: Real,
    pub z: Real,
    pub w: Real,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct Quat {
    pub x: Real,
    pub y: Real,
    pub z: Real,
    pub w: Real,
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RotationOrder {
    Xyz = 0,
    Xzy = 1,
    Yzx = 2,
    Yxz = 3,
    Zxy = 4,
    Zyx = 5,
    Spheric = 6,
}

#[allow(clippy::derivable_impls)]
impl Default for RotationOrder {
    fn default() -> Self {
        Self::Xyz
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct Matrix {
    pub m00: Real,
    pub m10: Real,
    pub m20: Real,
    pub m01: Real,
    pub m11: Real,
    pub m21: Real,
    pub m02: Real,
    pub m12: Real,
    pub m22: Real,
    pub m03: Real,
    pub m13: Real,
    pub m23: Real,
}

#[repr(C)]
pub struct VoidList {
    pub data: *mut c_void,
    pub count: usize,
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DomValueType {
    Number = 0,
    String = 1,
    Blob = 2,
    ArrayI32 = 3,
    ArrayI64 = 4,
    ArrayF32 = 5,
    ArrayF64 = 6,
    ArrayBlob = 7,
    ArrayIgnored = 8,
}

#[allow(clippy::derivable_impls)]
impl Default for DomValueType {
    fn default() -> Self {
        Self::Number
    }
}

#[repr(C)]
pub struct DomValue {
    pub type_: DomValueType,
    pub value_str: String,
    pub value_blob: Blob,
    pub value_int: i64,
    pub value_float: f64,
}

#[repr(C)]
pub struct DomNode {
    pub name: String,
    pub children: RefList<DomNode>,
    pub values: List<DomValue>,
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PropType {
    Unknown = 0,
    Boolean = 1,
    Integer = 2,
    Number = 3,
    Vector = 4,
    Color = 5,
    ColorWithAlpha = 6,
    String = 7,
    DateTime = 8,
    Translation = 9,
    Rotation = 10,
    Scaling = 11,
    Distance = 12,
    Compound = 13,
    Blob = 14,
    Reference = 15,
}

#[allow(clippy::derivable_impls)]
impl Default for PropType {
    fn default() -> Self {
        Self::Unknown
    }
}

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct PropFlags(u32);
impl PropFlags {
    pub const NONE: PropFlags = PropFlags(0);
    pub const ANIMATABLE: PropFlags = PropFlags(0x1);
    pub const USER_DEFINED: PropFlags = PropFlags(0x2);
    pub const HIDDEN: PropFlags = PropFlags(0x4);
    pub const LOCK_X: PropFlags = PropFlags(0x10);
    pub const LOCK_Y: PropFlags = PropFlags(0x20);
    pub const LOCK_Z: PropFlags = PropFlags(0x40);
    pub const LOCK_W: PropFlags = PropFlags(0x80);
    pub const MUTE_X: PropFlags = PropFlags(0x100);
    pub const MUTE_Y: PropFlags = PropFlags(0x200);
    pub const MUTE_Z: PropFlags = PropFlags(0x400);
    pub const MUTE_W: PropFlags = PropFlags(0x800);
    pub const SYNTHETIC: PropFlags = PropFlags(0x1000);
    pub const ANIMATED: PropFlags = PropFlags(0x2000);
    pub const NOT_FOUND: PropFlags = PropFlags(0x4000);
    pub const CONNECTED: PropFlags = PropFlags(0x8000);
    pub const NO_VALUE: PropFlags = PropFlags(0x10000);
    pub const OVERRIDDEN: PropFlags = PropFlags(0x20000);
    pub const VALUE_REAL: PropFlags = PropFlags(0x100000);
    pub const VALUE_VEC2: PropFlags = PropFlags(0x200000);
    pub const VALUE_VEC3: PropFlags = PropFlags(0x400000);
    pub const VALUE_VEC4: PropFlags = PropFlags(0x800000);
    pub const VALUE_INT: PropFlags = PropFlags(0x1000000);
    pub const VALUE_STR: PropFlags = PropFlags(0x2000000);
    pub const VALUE_BLOB: PropFlags = PropFlags(0x4000000);
}

const PROPFLAGS_NAMES: [(&str, u32); 24] = [
    ("ANIMATABLE", 0x1),
    ("USER_DEFINED", 0x2),
    ("HIDDEN", 0x4),
    ("LOCK_X", 0x10),
    ("LOCK_Y", 0x20),
    ("LOCK_Z", 0x40),
    ("LOCK_W", 0x80),
    ("MUTE_X", 0x100),
    ("MUTE_Y", 0x200),
    ("MUTE_Z", 0x400),
    ("MUTE_W", 0x800),
    ("SYNTHETIC", 0x1000),
    ("ANIMATED", 0x2000),
    ("NOT_FOUND", 0x4000),
    ("CONNECTED", 0x8000),
    ("NO_VALUE", 0x10000),
    ("OVERRIDDEN", 0x20000),
    ("VALUE_REAL", 0x100000),
    ("VALUE_VEC2", 0x200000),
    ("VALUE_VEC3", 0x400000),
    ("VALUE_VEC4", 0x800000),
    ("VALUE_INT", 0x1000000),
    ("VALUE_STR", 0x2000000),
    ("VALUE_BLOB", 0x4000000),
];

impl PropFlags {
    pub fn any(self) -> bool {
        self.0 != 0
    }
    pub fn has_any(self, bits: Self) -> bool {
        (self.0 & bits.0) != 0
    }
    pub fn has_all(self, bits: Self) -> bool {
        (self.0 & bits.0) == bits.0
    }
    #[allow(dead_code)]
    pub(crate) const fn from_raw(bits: u32) -> Self {
        Self(bits)
    }
    #[allow(dead_code)]
    pub(crate) const fn raw(self) -> u32 {
        self.0
    }
}
#[allow(clippy::derivable_impls)]
impl Default for PropFlags {
    fn default() -> Self {
        Self(0)
    }
}
impl Debug for PropFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        format_flags(f, &PROPFLAGS_NAMES, self.0)
    }
}
impl BitAnd for PropFlags {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}
impl BitAndAssign for PropFlags {
    fn bitand_assign(&mut self, rhs: Self) {
        *self = Self(self.0 & rhs.0)
    }
}
impl BitOr for PropFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}
impl BitOrAssign for PropFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = Self(self.0 | rhs.0)
    }
}
impl BitXor for PropFlags {
    type Output = Self;
    fn bitxor(self, rhs: Self) -> Self::Output {
        Self(self.0 ^ rhs.0)
    }
}
impl BitXorAssign for PropFlags {
    fn bitxor_assign(&mut self, rhs: Self) {
        *self = Self(self.0 ^ rhs.0)
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Prop {
    pub name: String,
    pub(crate) _internal_key: u32,
    pub type_: PropType,
    pub flags: PropFlags,
    pub value_str: String,
    pub value_blob: Blob,
    pub value_int: i64,
    pub value_vec4: Vec4,
}

#[repr(C)]
pub struct Props {
    pub props: List<Prop>,
    pub num_animated: usize,
    pub defaults: Option<Ref<Props>>,
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ElementType {
    Unknown = 0,
    Node = 1,
    Mesh = 2,
    Light = 3,
    Camera = 4,
    Bone = 5,
    Empty = 6,
    LineCurve = 7,
    NurbsCurve = 8,
    NurbsSurface = 9,
    NurbsTrimSurface = 10,
    NurbsTrimBoundary = 11,
    ProceduralGeometry = 12,
    StereoCamera = 13,
    CameraSwitcher = 14,
    Marker = 15,
    LodGroup = 16,
    SkinDeformer = 17,
    SkinCluster = 18,
    BlendDeformer = 19,
    BlendChannel = 20,
    BlendShape = 21,
    CacheDeformer = 22,
    CacheFile = 23,
    Material = 24,
    Texture = 25,
    Video = 26,
    Shader = 27,
    ShaderBinding = 28,
    AnimStack = 29,
    AnimLayer = 30,
    AnimValue = 31,
    AnimCurve = 32,
    DisplayLayer = 33,
    SelectionSet = 34,
    SelectionNode = 35,
    Character = 36,
    Constraint = 37,
    AudioLayer = 38,
    AudioClip = 39,
    Pose = 40,
    MetadataObject = 41,
}

#[allow(clippy::derivable_impls)]
impl Default for ElementType {
    fn default() -> Self {
        Self::Unknown
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Connection {
    pub src: Ref<Element>,
    pub dst: Ref<Element>,
    pub src_prop: String,
    pub dst_prop: String,
}

#[repr(C)]
pub struct Element {
    pub name: String,
    pub props: Props,
    pub element_id: u32,
    pub typed_id: u32,
    pub instances: RefList<Node>,
    pub type_: ElementType,
    pub connections_src: List<Connection>,
    pub connections_dst: List<Connection>,
    pub dom_node: Option<Ref<DomNode>>,
    pub scene: Ref<Scene>,
}

#[repr(C)]
pub struct Unknown {
    pub element: Element,
    pub type_: String,
    pub super_type: String,
    pub sub_type: String,
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InheritMode {
    Normal = 0,
    IgnoreParentScale = 1,
    ComponentwiseScale = 2,
}

#[allow(clippy::derivable_impls)]
impl Default for InheritMode {
    fn default() -> Self {
        Self::Normal
    }
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MirrorAxis {
    None = 0,
    X = 1,
    Y = 2,
    Z = 3,
}

#[allow(clippy::derivable_impls)]
impl Default for MirrorAxis {
    fn default() -> Self {
        Self::None
    }
}

#[repr(C)]
pub struct Node {
    pub element: Element,
    pub parent: Option<Ref<Node>>,
    pub children: RefList<Node>,
    pub mesh: Option<Ref<Mesh>>,
    pub light: Option<Ref<Light>>,
    pub camera: Option<Ref<Camera>>,
    pub bone: Option<Ref<Bone>>,
    pub attrib: Option<Ref<Element>>,
    pub geometry_transform_helper: Option<Ref<Node>>,
    pub scale_helper: Option<Ref<Node>>,
    pub attrib_type: ElementType,
    pub all_attribs: RefList<Element>,
    pub inherit_mode: InheritMode,
    pub original_inherit_mode: InheritMode,
    pub local_transform: Transform,
    pub geometry_transform: Transform,
    pub inherit_scale: Vec3,
    pub inherit_scale_node: Option<Ref<Node>>,
    pub rotation_order: RotationOrder,
    pub euler_rotation: Vec3,
    pub node_to_parent: Matrix,
    pub node_to_world: Matrix,
    pub geometry_to_node: Matrix,
    pub geometry_to_world: Matrix,
    pub unscaled_node_to_world: Matrix,
    pub adjust_pre_translation: Vec3,
    pub adjust_pre_rotation: Quat,
    pub adjust_pre_scale: Real,
    pub adjust_post_rotation: Quat,
    pub adjust_post_scale: Real,
    pub adjust_translation_scale: Real,
    pub adjust_mirror_axis: MirrorAxis,
    pub materials: RefList<Material>,
    pub bind_pose: Option<Ref<Pose>>,
    pub visible: bool,
    pub is_root: bool,
    pub has_geometry_transform: bool,
    pub use_rotation_space: bool,
    pub has_adjust_transform: bool,
    pub has_root_adjust_transform: bool,
    pub is_geometry_transform_helper: bool,
    pub is_scale_helper: bool,
    pub is_scale_compensate_parent: bool,
    pub node_depth: u32,
}

#[repr(C)]
pub struct VertexAttrib {
    pub exists: bool,
    pub values: VoidList,
    pub indices: List<u32>,
    pub value_reals: usize,
    pub unique_per_vertex: bool,
    pub values_w: List<Real>,
}

#[repr(C)]
pub struct VertexReal {
    pub exists: bool,
    pub values: List<Real>,
    pub indices: List<u32>,
    pub value_reals: usize,
    pub unique_per_vertex: bool,
    pub values_w: List<Real>,
}

impl Index<usize> for VertexReal {
    type Output = Real;
    fn index(&self, index: usize) -> &Real {
        &self.values[self.indices[index] as usize]
    }
}

#[repr(C)]
pub struct VertexVec2 {
    pub exists: bool,
    pub values: List<Vec2>,
    pub indices: List<u32>,
    pub value_reals: usize,
    pub unique_per_vertex: bool,
    pub values_w: List<Real>,
}

impl Index<usize> for VertexVec2 {
    type Output = Vec2;
    fn index(&self, index: usize) -> &Vec2 {
        &self.values[self.indices[index] as usize]
    }
}

#[repr(C)]
pub struct VertexVec3 {
    pub exists: bool,
    pub values: List<Vec3>,
    pub indices: List<u32>,
    pub value_reals: usize,
    pub unique_per_vertex: bool,
    pub values_w: List<Real>,
}

impl Index<usize> for VertexVec3 {
    type Output = Vec3;
    fn index(&self, index: usize) -> &Vec3 {
        &self.values[self.indices[index] as usize]
    }
}

#[repr(C)]
pub struct VertexVec4 {
    pub exists: bool,
    pub values: List<Vec4>,
    pub indices: List<u32>,
    pub value_reals: usize,
    pub unique_per_vertex: bool,
    pub values_w: List<Real>,
}

impl Index<usize> for VertexVec4 {
    type Output = Vec4;
    fn index(&self, index: usize) -> &Vec4 {
        &self.values[self.indices[index] as usize]
    }
}

#[repr(C)]
pub struct UvSet {
    pub name: String,
    pub index: u32,
    pub vertex_uv: VertexVec2,
    pub vertex_tangent: VertexVec3,
    pub vertex_bitangent: VertexVec3,
}

#[repr(C)]
pub struct ColorSet {
    pub name: String,
    pub index: u32,
    pub vertex_color: VertexVec4,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct Edge {
    pub a: u32,
    pub b: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct Face {
    pub index_begin: u32,
    pub num_indices: u32,
}

#[repr(C)]
pub struct MeshPart {
    pub index: u32,
    pub num_faces: usize,
    pub num_triangles: usize,
    pub num_empty_faces: usize,
    pub num_point_faces: usize,
    pub num_line_faces: usize,
    pub face_indices: List<u32>,
}

#[repr(C)]
pub struct FaceGroup {
    pub id: i32,
    pub name: String,
}

#[repr(C)]
pub struct SubdivisionWeightRange {
    pub weight_begin: u32,
    pub num_weights: u32,
}

#[repr(C)]
pub struct SubdivisionWeight {
    pub weight: Real,
    pub index: u32,
}

#[repr(C)]
pub struct SubdivisionResult {
    pub result_memory_used: usize,
    pub temp_memory_used: usize,
    pub result_allocs: usize,
    pub temp_allocs: usize,
    pub source_vertex_ranges: List<SubdivisionWeightRange>,
    pub source_vertex_weights: List<SubdivisionWeight>,
    pub skin_cluster_ranges: List<SubdivisionWeightRange>,
    pub skin_cluster_weights: List<SubdivisionWeight>,
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SubdivisionDisplayMode {
    Disabled = 0,
    Hull = 1,
    HullAndSmooth = 2,
    Smooth = 3,
}

#[allow(clippy::derivable_impls)]
impl Default for SubdivisionDisplayMode {
    fn default() -> Self {
        Self::Disabled
    }
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SubdivisionBoundary {
    Default = 0,
    Legacy = 1,
    SharpCorners = 2,
    SharpNone = 3,
    SharpBoundary = 4,
    SharpInterior = 5,
}

#[allow(clippy::derivable_impls)]
impl Default for SubdivisionBoundary {
    fn default() -> Self {
        Self::Default
    }
}

#[repr(C)]
pub struct Mesh {
    pub element: Element,
    pub num_vertices: usize,
    pub num_indices: usize,
    pub num_faces: usize,
    pub num_triangles: usize,
    pub num_edges: usize,
    pub max_face_triangles: usize,
    pub num_empty_faces: usize,
    pub num_point_faces: usize,
    pub num_line_faces: usize,
    pub faces: List<Face>,
    pub face_smoothing: List<bool>,
    pub face_material: List<u32>,
    pub face_group: List<u32>,
    pub face_hole: List<bool>,
    pub edges: List<Edge>,
    pub edge_smoothing: List<bool>,
    pub edge_crease: List<Real>,
    pub edge_visibility: List<bool>,
    pub vertex_indices: List<u32>,
    pub vertices: List<Vec3>,
    pub vertex_first_index: List<u32>,
    pub vertex_position: VertexVec3,
    pub vertex_normal: VertexVec3,
    pub vertex_uv: VertexVec2,
    pub vertex_tangent: VertexVec3,
    pub vertex_bitangent: VertexVec3,
    pub vertex_color: VertexVec4,
    pub vertex_crease: VertexReal,
    pub uv_sets: List<UvSet>,
    pub color_sets: List<ColorSet>,
    pub materials: RefList<Material>,
    pub face_groups: List<FaceGroup>,
    pub material_parts: List<MeshPart>,
    pub face_group_parts: List<MeshPart>,
    pub material_part_usage_order: List<u32>,
    pub skinned_is_local: bool,
    pub skinned_position: VertexVec3,
    pub skinned_normal: VertexVec3,
    pub skin_deformers: RefList<SkinDeformer>,
    pub blend_deformers: RefList<BlendDeformer>,
    pub cache_deformers: RefList<CacheDeformer>,
    pub all_deformers: RefList<Element>,
    pub subdivision_preview_levels: u32,
    pub subdivision_render_levels: u32,
    pub subdivision_display_mode: SubdivisionDisplayMode,
    pub subdivision_boundary: SubdivisionBoundary,
    pub subdivision_uv_boundary: SubdivisionBoundary,
    pub reversed_winding: bool,
    pub generated_normals: bool,
    pub subdivision_evaluated: bool,
    pub subdivision_result: Option<Ref<SubdivisionResult>>,
    pub from_tessellated_nurbs: bool,
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LightType {
    Point = 0,
    Directional = 1,
    Spot = 2,
    Area = 3,
    Volume = 4,
}

#[allow(clippy::derivable_impls)]
impl Default for LightType {
    fn default() -> Self {
        Self::Point
    }
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LightDecay {
    None = 0,
    Linear = 1,
    Quadratic = 2,
    Cubic = 3,
}

#[allow(clippy::derivable_impls)]
impl Default for LightDecay {
    fn default() -> Self {
        Self::None
    }
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LightAreaShape {
    Rectangle = 0,
    Sphere = 1,
}

#[allow(clippy::derivable_impls)]
impl Default for LightAreaShape {
    fn default() -> Self {
        Self::Rectangle
    }
}

#[repr(C)]
pub struct Light {
    pub element: Element,
    pub color: Vec3,
    pub intensity: Real,
    pub local_direction: Vec3,
    pub type_: LightType,
    pub decay: LightDecay,
    pub area_shape: LightAreaShape,
    pub inner_angle: Real,
    pub outer_angle: Real,
    pub cast_light: bool,
    pub cast_shadows: bool,
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ProjectionMode {
    Perspective = 0,
    Orthographic = 1,
}

#[allow(clippy::derivable_impls)]
impl Default for ProjectionMode {
    fn default() -> Self {
        Self::Perspective
    }
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AspectMode {
    WindowSize = 0,
    FixedRatio = 1,
    FixedResolution = 2,
    FixedWidth = 3,
    FixedHeight = 4,
}

#[allow(clippy::derivable_impls)]
impl Default for AspectMode {
    fn default() -> Self {
        Self::WindowSize
    }
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ApertureMode {
    HorizontalAndVertical = 0,
    Horizontal = 1,
    Vertical = 2,
    FocalLength = 3,
}

#[allow(clippy::derivable_impls)]
impl Default for ApertureMode {
    fn default() -> Self {
        Self::HorizontalAndVertical
    }
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GateFit {
    None = 0,
    Vertical = 1,
    Horizontal = 2,
    Fill = 3,
    Overscan = 4,
    Stretch = 5,
}

#[allow(clippy::derivable_impls)]
impl Default for GateFit {
    fn default() -> Self {
        Self::None
    }
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ApertureFormat {
    Custom = 0,
    E16MmTheatrical = 1,
    Super16Mm = 2,
    E35MmAcademy = 3,
    E35MmTvProjection = 4,
    E35MmFullAperture = 5,
    E35Mm185Projection = 6,
    E35MmAnamorphic = 7,
    E70MmProjection = 8,
    Vistavision = 9,
    Dynavision = 10,
    Imax = 11,
}

#[allow(clippy::derivable_impls)]
impl Default for ApertureFormat {
    fn default() -> Self {
        Self::Custom
    }
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CoordinateAxis {
    PositiveX = 0,
    NegativeX = 1,
    PositiveY = 2,
    NegativeY = 3,
    PositiveZ = 4,
    NegativeZ = 5,
    Unknown = 6,
}

#[allow(clippy::derivable_impls)]
impl Default for CoordinateAxis {
    fn default() -> Self {
        Self::PositiveX
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct CoordinateAxes {
    pub right: CoordinateAxis,
    pub up: CoordinateAxis,
    pub front: CoordinateAxis,
}

#[repr(C)]
pub struct Camera {
    pub element: Element,
    pub projection_mode: ProjectionMode,
    pub resolution_is_pixels: bool,
    pub resolution: Vec2,
    pub field_of_view_deg: Vec2,
    pub field_of_view_tan: Vec2,
    pub orthographic_extent: Real,
    pub orthographic_size: Vec2,
    pub projection_plane: Vec2,
    pub aspect_ratio: Real,
    pub near_plane: Real,
    pub far_plane: Real,
    pub projection_axes: CoordinateAxes,
    pub aspect_mode: AspectMode,
    pub aperture_mode: ApertureMode,
    pub gate_fit: GateFit,
    pub aperture_format: ApertureFormat,
    pub focal_length_mm: Real,
    pub film_size_inch: Vec2,
    pub aperture_size_inch: Vec2,
    pub squeeze_ratio: Real,
}

#[repr(C)]
pub struct Bone {
    pub element: Element,
    pub radius: Real,
    pub relative_length: Real,
    pub is_root: bool,
}

#[repr(C)]
pub struct Empty {
    pub element: Element,
}

#[repr(C)]
pub struct LineSegment {
    pub index_begin: u32,
    pub num_indices: u32,
}

#[repr(C)]
pub struct LineCurve {
    pub element: Element,
    pub color: Vec3,
    pub control_points: List<Vec3>,
    pub point_indices: List<u32>,
    pub segments: List<LineSegment>,
    pub from_tessellated_nurbs: bool,
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NurbsTopology {
    Open = 0,
    Periodic = 1,
    Closed = 2,
}

#[allow(clippy::derivable_impls)]
impl Default for NurbsTopology {
    fn default() -> Self {
        Self::Open
    }
}

#[repr(C)]
pub struct NurbsBasis {
    pub order: u32,
    pub topology: NurbsTopology,
    pub knot_vector: List<Real>,
    pub t_min: Real,
    pub t_max: Real,
    pub spans: List<Real>,
    pub is_2d: bool,
    pub num_wrap_control_points: usize,
    pub valid: bool,
}

#[repr(C)]
pub struct NurbsCurve {
    pub element: Element,
    pub basis: NurbsBasis,
    pub control_points: List<Vec4>,
}

#[repr(C)]
pub struct NurbsSurface {
    pub element: Element,
    pub basis_u: NurbsBasis,
    pub basis_v: NurbsBasis,
    pub num_control_points_u: usize,
    pub num_control_points_v: usize,
    pub control_points: List<Vec4>,
    pub span_subdivision_u: u32,
    pub span_subdivision_v: u32,
    pub flip_normals: bool,
    pub material: Option<Ref<Material>>,
}

#[repr(C)]
pub struct NurbsTrimSurface {
    pub element: Element,
}

#[repr(C)]
pub struct NurbsTrimBoundary {
    pub element: Element,
}

#[repr(C)]
pub struct ProceduralGeometry {
    pub element: Element,
}

#[repr(C)]
pub struct StereoCamera {
    pub element: Element,
    pub left: Option<Ref<Camera>>,
    pub right: Option<Ref<Camera>>,
}

#[repr(C)]
pub struct CameraSwitcher {
    pub element: Element,
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MarkerType {
    Unknown = 0,
    FkEffector = 1,
    IkEffector = 2,
}

#[allow(clippy::derivable_impls)]
impl Default for MarkerType {
    fn default() -> Self {
        Self::Unknown
    }
}

#[repr(C)]
pub struct Marker {
    pub element: Element,
    pub type_: MarkerType,
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LodDisplay {
    UseLod = 0,
    Show = 1,
    Hide = 2,
}

#[allow(clippy::derivable_impls)]
impl Default for LodDisplay {
    fn default() -> Self {
        Self::UseLod
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct LodLevel {
    pub distance: Real,
    pub display: LodDisplay,
}

#[repr(C)]
pub struct LodGroup {
    pub element: Element,
    pub relative_distances: bool,
    pub lod_levels: List<LodLevel>,
    pub ignore_parent_transform: bool,
    pub use_distance_limit: bool,
    pub distance_limit_min: Real,
    pub distance_limit_max: Real,
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SkinningMethod {
    Linear = 0,
    Rigid = 1,
    DualQuaternion = 2,
    BlendedDqLinear = 3,
}

#[allow(clippy::derivable_impls)]
impl Default for SkinningMethod {
    fn default() -> Self {
        Self::Linear
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct SkinVertex {
    pub weight_begin: u32,
    pub num_weights: u32,
    pub dq_weight: Real,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct SkinWeight {
    pub cluster_index: u32,
    pub weight: Real,
}

#[repr(C)]
pub struct SkinDeformer {
    pub element: Element,
    pub skinning_method: SkinningMethod,
    pub clusters: RefList<SkinCluster>,
    pub vertices: List<SkinVertex>,
    pub weights: List<SkinWeight>,
    pub max_weights_per_vertex: usize,
    pub num_dq_weights: usize,
    pub dq_vertices: List<u32>,
    pub dq_weights: List<Real>,
}

#[repr(C)]
pub struct SkinCluster {
    pub element: Element,
    pub bone_node: Option<Ref<Node>>,
    pub geometry_to_bone: Matrix,
    pub mesh_node_to_bone: Matrix,
    pub bind_to_world: Matrix,
    pub geometry_to_world: Matrix,
    pub geometry_to_world_transform: Transform,
    pub num_weights: usize,
    pub vertices: List<u32>,
    pub weights: List<Real>,
}

#[repr(C)]
pub struct BlendDeformer {
    pub element: Element,
    pub channels: RefList<BlendChannel>,
}

#[repr(C)]
pub struct BlendKeyframe {
    pub shape: Ref<BlendShape>,
    pub target_weight: Real,
    pub effective_weight: Real,
}

#[repr(C)]
pub struct BlendChannel {
    pub element: Element,
    pub weight: Real,
    pub keyframes: List<BlendKeyframe>,
    pub target_shape: Option<Ref<BlendShape>>,
}

#[repr(C)]
pub struct BlendShape {
    pub element: Element,
    pub num_offsets: usize,
    pub offset_vertices: List<u32>,
    pub position_offsets: List<Vec3>,
    pub normal_offsets: List<Vec3>,
    pub offset_weights: List<Real>,
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CacheFileFormat {
    Unknown = 0,
    Pc2 = 1,
    Mc = 2,
}

#[allow(clippy::derivable_impls)]
impl Default for CacheFileFormat {
    fn default() -> Self {
        Self::Unknown
    }
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CacheDataFormat {
    Unknown = 0,
    RealFloat = 1,
    Vec3Float = 2,
    RealDouble = 3,
    Vec3Double = 4,
}

#[allow(clippy::derivable_impls)]
impl Default for CacheDataFormat {
    fn default() -> Self {
        Self::Unknown
    }
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CacheDataEncoding {
    Unknown = 0,
    LittleEndian = 1,
    BigEndian = 2,
}

#[allow(clippy::derivable_impls)]
impl Default for CacheDataEncoding {
    fn default() -> Self {
        Self::Unknown
    }
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CacheInterpretation {
    Unknown = 0,
    Points = 1,
    VertexPosition = 2,
    VertexNormal = 3,
}

#[allow(clippy::derivable_impls)]
impl Default for CacheInterpretation {
    fn default() -> Self {
        Self::Unknown
    }
}

#[repr(C)]
pub struct CacheFrame {
    pub channel: String,
    pub time: f64,
    pub filename: String,
    pub file_format: CacheFileFormat,
    pub mirror_axis: MirrorAxis,
    pub scale_factor: Real,
    pub data_format: CacheDataFormat,
    pub data_encoding: CacheDataEncoding,
    pub data_offset: u64,
    pub data_count: u32,
    pub data_element_bytes: u32,
    pub data_total_bytes: u64,
}

#[repr(C)]
pub struct CacheChannel {
    pub name: String,
    pub interpretation: CacheInterpretation,
    pub interpretation_name: String,
    pub frames: List<CacheFrame>,
    pub mirror_axis: MirrorAxis,
    pub scale_factor: Real,
}

#[repr(C)]
pub struct GeometryCache {
    pub root_filename: String,
    pub channels: List<CacheChannel>,
    pub frames: List<CacheFrame>,
    pub extra_info: List<String>,
}

#[repr(C)]
pub struct CacheDeformer {
    pub element: Element,
    pub channel: String,
    pub file: Option<Ref<CacheFile>>,
    pub external_cache: Option<Ref<GeometryCache>>,
    pub external_channel: Option<Ref<CacheChannel>>,
}

#[repr(C)]
pub struct CacheFile {
    pub element: Element,
    pub filename: String,
    pub absolute_filename: String,
    pub relative_filename: String,
    pub raw_filename: Blob,
    pub raw_absolute_filename: Blob,
    pub raw_relative_filename: Blob,
    pub format: CacheFileFormat,
    pub external_cache: Option<Ref<GeometryCache>>,
}

#[repr(C)]
pub struct MaterialMap {
    pub value_vec4: Vec4,
    pub value_int: i64,
    pub texture: Option<Ref<Texture>>,
    pub has_value: bool,
    pub texture_enabled: bool,
    pub feature_disabled: bool,
    pub value_components: u8,
}

#[repr(C)]
pub struct MaterialFeatureInfo {
    pub enabled: bool,
    pub is_explicit: bool,
}

#[repr(C)]
pub struct MaterialTexture {
    pub material_prop: String,
    pub shader_prop: String,
    pub texture: Ref<Texture>,
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ShaderType {
    Unknown = 0,
    FbxLambert = 1,
    FbxPhong = 2,
    OslStandardSurface = 3,
    ArnoldStandardSurface = 4,
    E3DsMaxPhysicalMaterial = 5,
    E3DsMaxPbrMetalRough = 6,
    E3DsMaxPbrSpecGloss = 7,
    GltfMaterial = 8,
    OpenpbrMaterial = 9,
    ShaderfxGraph = 10,
    BlenderPhong = 11,
    WavefrontMtl = 12,
}

#[allow(clippy::derivable_impls)]
impl Default for ShaderType {
    fn default() -> Self {
        Self::Unknown
    }
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MaterialFbxMap {
    DiffuseFactor = 0,
    DiffuseColor = 1,
    SpecularFactor = 2,
    SpecularColor = 3,
    SpecularExponent = 4,
    ReflectionFactor = 5,
    ReflectionColor = 6,
    TransparencyFactor = 7,
    TransparencyColor = 8,
    EmissionFactor = 9,
    EmissionColor = 10,
    AmbientFactor = 11,
    AmbientColor = 12,
    NormalMap = 13,
    Bump = 14,
    BumpFactor = 15,
    DisplacementFactor = 16,
    Displacement = 17,
    VectorDisplacementFactor = 18,
    VectorDisplacement = 19,
}

#[allow(clippy::derivable_impls)]
impl Default for MaterialFbxMap {
    fn default() -> Self {
        Self::DiffuseFactor
    }
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MaterialPbrMap {
    BaseFactor = 0,
    BaseColor = 1,
    Roughness = 2,
    Metalness = 3,
    DiffuseRoughness = 4,
    SpecularFactor = 5,
    SpecularColor = 6,
    SpecularIor = 7,
    SpecularAnisotropy = 8,
    SpecularRotation = 9,
    TransmissionFactor = 10,
    TransmissionColor = 11,
    TransmissionDepth = 12,
    TransmissionScatter = 13,
    TransmissionScatterAnisotropy = 14,
    TransmissionDispersion = 15,
    TransmissionRoughness = 16,
    TransmissionExtraRoughness = 17,
    TransmissionPriority = 18,
    TransmissionEnableInAov = 19,
    SubsurfaceFactor = 20,
    SubsurfaceColor = 21,
    SubsurfaceRadius = 22,
    SubsurfaceScale = 23,
    SubsurfaceAnisotropy = 24,
    SubsurfaceTintColor = 25,
    SubsurfaceType = 26,
    SheenFactor = 27,
    SheenColor = 28,
    SheenRoughness = 29,
    CoatFactor = 30,
    CoatColor = 31,
    CoatRoughness = 32,
    CoatIor = 33,
    CoatAnisotropy = 34,
    CoatRotation = 35,
    CoatNormal = 36,
    CoatAffectBaseColor = 37,
    CoatAffectBaseRoughness = 38,
    ThinFilmFactor = 39,
    ThinFilmThickness = 40,
    ThinFilmIor = 41,
    EmissionFactor = 42,
    EmissionColor = 43,
    Opacity = 44,
    IndirectDiffuse = 45,
    IndirectSpecular = 46,
    NormalMap = 47,
    TangentMap = 48,
    DisplacementMap = 49,
    MatteFactor = 50,
    MatteColor = 51,
    AmbientOcclusion = 52,
    Glossiness = 53,
    CoatGlossiness = 54,
    TransmissionGlossiness = 55,
}

#[allow(clippy::derivable_impls)]
impl Default for MaterialPbrMap {
    fn default() -> Self {
        Self::BaseFactor
    }
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MaterialFeature {
    Pbr = 0,
    Metalness = 1,
    Diffuse = 2,
    Specular = 3,
    Emission = 4,
    Transmission = 5,
    Coat = 6,
    Sheen = 7,
    Opacity = 8,
    AmbientOcclusion = 9,
    Matte = 10,
    Unlit = 11,
    Ior = 12,
    DiffuseRoughness = 13,
    TransmissionRoughness = 14,
    ThinWalled = 15,
    Caustics = 16,
    ExitToBackground = 17,
    InternalReflections = 18,
    DoubleSided = 19,
    RoughnessAsGlossiness = 20,
    CoatRoughnessAsGlossiness = 21,
    TransmissionRoughnessAsGlossiness = 22,
}

#[allow(clippy::derivable_impls)]
impl Default for MaterialFeature {
    fn default() -> Self {
        Self::Pbr
    }
}

#[repr(C)]
pub struct MaterialFbxMaps {
    pub diffuse_factor: MaterialMap,
    pub diffuse_color: MaterialMap,
    pub specular_factor: MaterialMap,
    pub specular_color: MaterialMap,
    pub specular_exponent: MaterialMap,
    pub reflection_factor: MaterialMap,
    pub reflection_color: MaterialMap,
    pub transparency_factor: MaterialMap,
    pub transparency_color: MaterialMap,
    pub emission_factor: MaterialMap,
    pub emission_color: MaterialMap,
    pub ambient_factor: MaterialMap,
    pub ambient_color: MaterialMap,
    pub normal_map: MaterialMap,
    pub bump: MaterialMap,
    pub bump_factor: MaterialMap,
    pub displacement_factor: MaterialMap,
    pub displacement: MaterialMap,
    pub vector_displacement_factor: MaterialMap,
    pub vector_displacement: MaterialMap,
}

#[repr(C)]
pub struct MaterialPbrMaps {
    pub base_factor: MaterialMap,
    pub base_color: MaterialMap,
    pub roughness: MaterialMap,
    pub metalness: MaterialMap,
    pub diffuse_roughness: MaterialMap,
    pub specular_factor: MaterialMap,
    pub specular_color: MaterialMap,
    pub specular_ior: MaterialMap,
    pub specular_anisotropy: MaterialMap,
    pub specular_rotation: MaterialMap,
    pub transmission_factor: MaterialMap,
    pub transmission_color: MaterialMap,
    pub transmission_depth: MaterialMap,
    pub transmission_scatter: MaterialMap,
    pub transmission_scatter_anisotropy: MaterialMap,
    pub transmission_dispersion: MaterialMap,
    pub transmission_roughness: MaterialMap,
    pub transmission_extra_roughness: MaterialMap,
    pub transmission_priority: MaterialMap,
    pub transmission_enable_in_aov: MaterialMap,
    pub subsurface_factor: MaterialMap,
    pub subsurface_color: MaterialMap,
    pub subsurface_radius: MaterialMap,
    pub subsurface_scale: MaterialMap,
    pub subsurface_anisotropy: MaterialMap,
    pub subsurface_tint_color: MaterialMap,
    pub subsurface_type: MaterialMap,
    pub sheen_factor: MaterialMap,
    pub sheen_color: MaterialMap,
    pub sheen_roughness: MaterialMap,
    pub coat_factor: MaterialMap,
    pub coat_color: MaterialMap,
    pub coat_roughness: MaterialMap,
    pub coat_ior: MaterialMap,
    pub coat_anisotropy: MaterialMap,
    pub coat_rotation: MaterialMap,
    pub coat_normal: MaterialMap,
    pub coat_affect_base_color: MaterialMap,
    pub coat_affect_base_roughness: MaterialMap,
    pub thin_film_factor: MaterialMap,
    pub thin_film_thickness: MaterialMap,
    pub thin_film_ior: MaterialMap,
    pub emission_factor: MaterialMap,
    pub emission_color: MaterialMap,
    pub opacity: MaterialMap,
    pub indirect_diffuse: MaterialMap,
    pub indirect_specular: MaterialMap,
    pub normal_map: MaterialMap,
    pub tangent_map: MaterialMap,
    pub displacement_map: MaterialMap,
    pub matte_factor: MaterialMap,
    pub matte_color: MaterialMap,
    pub ambient_occlusion: MaterialMap,
    pub glossiness: MaterialMap,
    pub coat_glossiness: MaterialMap,
    pub transmission_glossiness: MaterialMap,
}

#[repr(C)]
pub struct MaterialFeatures {
    pub pbr: MaterialFeatureInfo,
    pub metalness: MaterialFeatureInfo,
    pub diffuse: MaterialFeatureInfo,
    pub specular: MaterialFeatureInfo,
    pub emission: MaterialFeatureInfo,
    pub transmission: MaterialFeatureInfo,
    pub coat: MaterialFeatureInfo,
    pub sheen: MaterialFeatureInfo,
    pub opacity: MaterialFeatureInfo,
    pub ambient_occlusion: MaterialFeatureInfo,
    pub matte: MaterialFeatureInfo,
    pub unlit: MaterialFeatureInfo,
    pub ior: MaterialFeatureInfo,
    pub diffuse_roughness: MaterialFeatureInfo,
    pub transmission_roughness: MaterialFeatureInfo,
    pub thin_walled: MaterialFeatureInfo,
    pub caustics: MaterialFeatureInfo,
    pub exit_to_background: MaterialFeatureInfo,
    pub internal_reflections: MaterialFeatureInfo,
    pub double_sided: MaterialFeatureInfo,
    pub roughness_as_glossiness: MaterialFeatureInfo,
    pub coat_roughness_as_glossiness: MaterialFeatureInfo,
    pub transmission_roughness_as_glossiness: MaterialFeatureInfo,
}

#[repr(C)]
pub struct Material {
    pub element: Element,
    pub fbx: MaterialFbxMaps,
    pub pbr: MaterialPbrMaps,
    pub features: MaterialFeatures,
    pub shader_type: ShaderType,
    pub shader: Option<Ref<Shader>>,
    pub shading_model_name: String,
    pub shader_prop_prefix: String,
    pub textures: List<MaterialTexture>,
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TextureType {
    File = 0,
    Layered = 1,
    Procedural = 2,
    Shader = 3,
}

#[allow(clippy::derivable_impls)]
impl Default for TextureType {
    fn default() -> Self {
        Self::File
    }
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BlendMode {
    Translucent = 0,
    Additive = 1,
    Multiply = 2,
    Multiply2X = 3,
    Over = 4,
    Replace = 5,
    Dissolve = 6,
    Darken = 7,
    ColorBurn = 8,
    LinearBurn = 9,
    DarkerColor = 10,
    Lighten = 11,
    Screen = 12,
    ColorDodge = 13,
    LinearDodge = 14,
    LighterColor = 15,
    SoftLight = 16,
    HardLight = 17,
    VividLight = 18,
    LinearLight = 19,
    PinLight = 20,
    HardMix = 21,
    Difference = 22,
    Exclusion = 23,
    Subtract = 24,
    Divide = 25,
    Hue = 26,
    Saturation = 27,
    Color = 28,
    Luminosity = 29,
    Overlay = 30,
}

#[allow(clippy::derivable_impls)]
impl Default for BlendMode {
    fn default() -> Self {
        Self::Translucent
    }
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WrapMode {
    Repeat = 0,
    Clamp = 1,
}

#[allow(clippy::derivable_impls)]
impl Default for WrapMode {
    fn default() -> Self {
        Self::Repeat
    }
}

#[repr(C)]
pub struct TextureLayer {
    pub texture: Ref<Texture>,
    pub blend_mode: BlendMode,
    pub alpha: Real,
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ShaderTextureType {
    Unknown = 0,
    SelectOutput = 1,
    Osl = 2,
}

#[allow(clippy::derivable_impls)]
impl Default for ShaderTextureType {
    fn default() -> Self {
        Self::Unknown
    }
}

#[repr(C)]
pub struct ShaderTextureInput {
    pub name: String,
    pub value_vec4: Vec4,
    pub value_int: i64,
    pub value_str: String,
    pub value_blob: Blob,
    pub texture: Option<Ref<Texture>>,
    pub texture_output_index: i64,
    pub texture_enabled: bool,
    pub prop: Option<Ref<Prop>>,
    pub texture_prop: Option<Ref<Prop>>,
    pub texture_enabled_prop: Option<Ref<Prop>>,
}

#[repr(C)]
pub struct ShaderTexture {
    pub type_: ShaderTextureType,
    pub shader_name: String,
    pub shader_type_id: u64,
    pub inputs: List<ShaderTextureInput>,
    pub shader_source: String,
    pub raw_shader_source: Blob,
    pub main_texture: Option<Ref<Texture>>,
    pub main_texture_output_index: i64,
    pub prop_prefix: String,
}

#[repr(C)]
pub struct TextureFile {
    pub index: u32,
    pub filename: String,
    pub absolute_filename: String,
    pub relative_filename: String,
    pub raw_filename: Blob,
    pub raw_absolute_filename: Blob,
    pub raw_relative_filename: Blob,
    pub content: Blob,
}

#[repr(C)]
pub struct Texture {
    pub element: Element,
    pub type_: TextureType,
    pub filename: String,
    pub absolute_filename: String,
    pub relative_filename: String,
    pub raw_filename: Blob,
    pub raw_absolute_filename: Blob,
    pub raw_relative_filename: Blob,
    pub content: Blob,
    pub video: Option<Ref<Video>>,
    pub file_index: u32,
    pub has_file: bool,
    pub layers: List<TextureLayer>,
    pub shader: Option<Ref<ShaderTexture>>,
    pub file_textures: RefList<Texture>,
    pub uv_set: String,
    pub wrap_u: WrapMode,
    pub wrap_v: WrapMode,
    pub has_uv_transform: bool,
    pub uv_transform: Transform,
    pub texture_to_uv: Matrix,
    pub uv_to_texture: Matrix,
}

#[repr(C)]
pub struct Video {
    pub element: Element,
    pub filename: String,
    pub absolute_filename: String,
    pub relative_filename: String,
    pub raw_filename: Blob,
    pub raw_absolute_filename: Blob,
    pub raw_relative_filename: Blob,
    pub content: Blob,
}

#[repr(C)]
pub struct Shader {
    pub element: Element,
    pub type_: ShaderType,
    pub bindings: RefList<ShaderBinding>,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ShaderPropBinding {
    pub shader_prop: String,
    pub material_prop: String,
}

#[repr(C)]
pub struct ShaderBinding {
    pub element: Element,
    pub prop_bindings: List<ShaderPropBinding>,
}

#[repr(C)]
pub struct PropOverride {
    pub element_id: u32,
    pub(crate) _internal_key: u32,
    pub prop_name: String,
    pub value: Vec4,
    pub value_str: String,
    pub value_int: i64,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct TransformOverride {
    pub node_id: u32,
    pub transform: Transform,
}

#[repr(C)]
pub struct Anim {
    pub time_begin: f64,
    pub time_end: f64,
    pub layers: RefList<AnimLayer>,
    pub override_layer_weights: List<Real>,
    pub prop_overrides: List<PropOverride>,
    pub transform_overrides: List<TransformOverride>,
    pub ignore_connections: bool,
    pub custom: bool,
}

#[repr(C)]
pub struct AnimStack {
    pub element: Element,
    pub time_begin: f64,
    pub time_end: f64,
    pub layers: RefList<AnimLayer>,
    pub anim: Ref<Anim>,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AnimProp {
    pub element: Ref<Element>,
    pub(crate) _internal_key: u32,
    pub prop_name: String,
    pub anim_value: Ref<AnimValue>,
}

#[repr(C)]
pub struct AnimLayer {
    pub element: Element,
    pub weight: Real,
    pub weight_is_animated: bool,
    pub blended: bool,
    pub additive: bool,
    pub compose_rotation: bool,
    pub compose_scale: bool,
    pub anim_values: RefList<AnimValue>,
    pub anim_props: List<AnimProp>,
    pub anim: Ref<Anim>,
    pub(crate) _min_element_id: u32,
    pub(crate) _max_element_id: u32,
    pub(crate) _element_id_bitmask: [u32; 4],
}

#[repr(C)]
pub struct AnimValue {
    pub element: Element,
    pub default_value: Vec3,
    pub curves: [Option<Ref<AnimCurve>>; 3],
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Interpolation {
    ConstantPrev = 0,
    ConstantNext = 1,
    Linear = 2,
    Cubic = 3,
}

#[allow(clippy::derivable_impls)]
impl Default for Interpolation {
    fn default() -> Self {
        Self::ConstantPrev
    }
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ExtrapolationMode {
    Constant = 0,
    Repeat = 1,
    Mirror = 2,
    Slope = 3,
    RepeatRelative = 4,
}

#[allow(clippy::derivable_impls)]
impl Default for ExtrapolationMode {
    fn default() -> Self {
        Self::Constant
    }
}

#[repr(C)]
pub struct Extrapolation {
    pub mode: ExtrapolationMode,
    pub repeat_count: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct Tangent {
    pub dx: f32,
    pub dy: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct Keyframe {
    pub time: f64,
    pub value: Real,
    pub interpolation: Interpolation,
    pub left: Tangent,
    pub right: Tangent,
}

#[repr(C)]
pub struct AnimCurve {
    pub element: Element,
    pub keyframes: List<Keyframe>,
    pub pre_extrapolation: Extrapolation,
    pub post_extrapolation: Extrapolation,
    pub min_value: Real,
    pub max_value: Real,
    pub min_time: f64,
    pub max_time: f64,
}

#[repr(C)]
pub struct DisplayLayer {
    pub element: Element,
    pub nodes: RefList<Node>,
    pub visible: bool,
    pub frozen: bool,
    pub ui_color: Vec3,
}

#[repr(C)]
pub struct SelectionSet {
    pub element: Element,
    pub nodes: RefList<SelectionNode>,
}

#[repr(C)]
pub struct SelectionNode {
    pub element: Element,
    pub target_node: Option<Ref<Node>>,
    pub target_mesh: Option<Ref<Mesh>>,
    pub include_node: bool,
    pub vertices: List<u32>,
    pub edges: List<u32>,
    pub faces: List<u32>,
}

#[repr(C)]
pub struct Character {
    pub element: Element,
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ConstraintType {
    Unknown = 0,
    Aim = 1,
    Parent = 2,
    Position = 3,
    Rotation = 4,
    Scale = 5,
    SingleChainIk = 6,
}

#[allow(clippy::derivable_impls)]
impl Default for ConstraintType {
    fn default() -> Self {
        Self::Unknown
    }
}

#[repr(C)]
pub struct ConstraintTarget {
    pub node: Ref<Node>,
    pub weight: Real,
    pub transform: Transform,
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ConstraintAimUpType {
    Scene = 0,
    ToNode = 1,
    AlignNode = 2,
    Vector = 3,
    None = 4,
}

#[allow(clippy::derivable_impls)]
impl Default for ConstraintAimUpType {
    fn default() -> Self {
        Self::Scene
    }
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ConstraintIkPoleType {
    Vector = 0,
    Node = 1,
}

#[allow(clippy::derivable_impls)]
impl Default for ConstraintIkPoleType {
    fn default() -> Self {
        Self::Vector
    }
}

#[repr(C)]
pub struct Constraint {
    pub element: Element,
    pub type_: ConstraintType,
    pub type_name: String,
    pub node: Option<Ref<Node>>,
    pub targets: List<ConstraintTarget>,
    pub weight: Real,
    pub active: bool,
    pub constrain_translation: [bool; 3],
    pub constrain_rotation: [bool; 3],
    pub constrain_scale: [bool; 3],
    pub transform_offset: Transform,
    pub aim_vector: Vec3,
    pub aim_up_type: ConstraintAimUpType,
    pub aim_up_node: Option<Ref<Node>>,
    pub aim_up_vector: Vec3,
    pub ik_effector: Option<Ref<Node>>,
    pub ik_end_node: Option<Ref<Node>>,
    pub ik_pole_vector: Vec3,
}

#[repr(C)]
pub struct AudioLayer {
    pub element: Element,
    pub clips: RefList<AudioClip>,
}

#[repr(C)]
pub struct AudioClip {
    pub element: Element,
    pub filename: String,
    pub absolute_filename: String,
    pub relative_filename: String,
    pub raw_filename: Blob,
    pub raw_absolute_filename: Blob,
    pub raw_relative_filename: Blob,
    pub content: Blob,
}

#[repr(C)]
pub struct BonePose {
    pub bone_node: Ref<Node>,
    pub bone_to_world: Matrix,
    pub bone_to_parent: Matrix,
}

#[repr(C)]
pub struct Pose {
    pub element: Element,
    pub is_bind_pose: bool,
    pub bone_poses: List<BonePose>,
}

#[repr(C)]
pub struct MetadataObject {
    pub element: Element,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct NameElement {
    pub name: String,
    pub type_: ElementType,
    pub(crate) _internal_key: u32,
    pub element: Ref<Element>,
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Exporter {
    Unknown = 0,
    FbxSdk = 1,
    BlenderBinary = 2,
    BlenderAscii = 3,
    MotionBuilder = 4,
    UfbxWrite = 5,
}

#[allow(clippy::derivable_impls)]
impl Default for Exporter {
    fn default() -> Self {
        Self::Unknown
    }
}

#[repr(C)]
pub struct Application {
    pub vendor: String,
    pub name: String,
    pub version: String,
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FileFormat {
    Unknown = 0,
    Fbx = 1,
    Obj = 2,
    Mtl = 3,
}

#[allow(clippy::derivable_impls)]
impl Default for FileFormat {
    fn default() -> Self {
        Self::Unknown
    }
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WarningType {
    MissingExternalFile = 0,
    ImplicitMtl = 1,
    TruncatedArray = 2,
    MissingGeometryData = 3,
    DuplicateConnection = 4,
    BadVertexWAttribute = 5,
    MissingPolygonMapping = 6,
    UnsupportedVersion = 7,
    IndexClamped = 8,
    BadUnicode = 9,
    BadBase64Content = 10,
    BadElementConnectedToRoot = 11,
    DuplicateObjectId = 12,
    EmptyFaceRemoved = 13,
    UnknownObjDirective = 14,
}

#[allow(clippy::derivable_impls)]
impl Default for WarningType {
    fn default() -> Self {
        Self::MissingExternalFile
    }
}

#[repr(C)]
pub struct Warning {
    pub type_: WarningType,
    pub description: String,
    pub element_id: u32,
    pub count: usize,
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ThumbnailFormat {
    Unknown = 0,
    Rgb24 = 1,
    Rgba32 = 2,
}

#[allow(clippy::derivable_impls)]
impl Default for ThumbnailFormat {
    fn default() -> Self {
        Self::Unknown
    }
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SpaceConversion {
    TransformRoot = 0,
    AdjustTransforms = 1,
    ModifyGeometry = 2,
}

#[allow(clippy::derivable_impls)]
impl Default for SpaceConversion {
    fn default() -> Self {
        Self::TransformRoot
    }
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GeometryTransformHandling {
    Preserve = 0,
    HelperNodes = 1,
    ModifyGeometry = 2,
    ModifyGeometryNoFallback = 3,
}

#[allow(clippy::derivable_impls)]
impl Default for GeometryTransformHandling {
    fn default() -> Self {
        Self::Preserve
    }
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InheritModeHandling {
    Preserve = 0,
    HelperNodes = 1,
    Compensate = 2,
    CompensateNoFallback = 3,
    Ignore = 4,
}

#[allow(clippy::derivable_impls)]
impl Default for InheritModeHandling {
    fn default() -> Self {
        Self::Preserve
    }
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PivotHandling {
    Retain = 0,
    AdjustToPivot = 1,
    AdjustToRotationPivot = 2,
}

#[allow(clippy::derivable_impls)]
impl Default for PivotHandling {
    fn default() -> Self {
        Self::Retain
    }
}

#[repr(C)]
pub struct Thumbnail {
    pub props: Props,
    pub width: u32,
    pub height: u32,
    pub format: ThumbnailFormat,
    pub data: Blob,
}

#[repr(C)]
pub struct Metadata {
    pub warnings: List<Warning>,
    pub ascii: bool,
    pub version: u32,
    pub file_format: FileFormat,
    pub may_contain_no_index: bool,
    pub may_contain_missing_vertex_position: bool,
    pub may_contain_broken_elements: bool,
    pub is_unsafe: bool,
    pub has_warning: [bool; 15],
    pub creator: String,
    pub big_endian: bool,
    pub filename: String,
    pub relative_root: String,
    pub raw_filename: Blob,
    pub raw_relative_root: Blob,
    pub exporter: Exporter,
    pub exporter_version: u32,
    pub scene_props: Props,
    pub original_application: Application,
    pub latest_application: Application,
    pub thumbnail: Thumbnail,
    pub geometry_ignored: bool,
    pub animation_ignored: bool,
    pub embedded_ignored: bool,
    pub max_face_triangles: usize,
    pub result_memory_used: usize,
    pub temp_memory_used: usize,
    pub result_allocs: usize,
    pub temp_allocs: usize,
    pub element_buffer_size: usize,
    pub num_shader_textures: usize,
    pub bone_prop_size_unit: Real,
    pub bone_prop_limb_length_relative: bool,
    pub ortho_size_unit: Real,
    pub ktime_second: i64,
    pub original_file_path: String,
    pub raw_original_file_path: Blob,
    pub space_conversion: SpaceConversion,
    pub geometry_transform_handling: GeometryTransformHandling,
    pub inherit_mode_handling: InheritModeHandling,
    pub pivot_handling: PivotHandling,
    pub handedness_conversion_axis: MirrorAxis,
    pub root_rotation: Quat,
    pub root_scale: Real,
    pub mirror_axis: MirrorAxis,
    pub geometry_scale: Real,
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TimeMode {
    Default = 0,
    E120Fps = 1,
    E100Fps = 2,
    E60Fps = 3,
    E50Fps = 4,
    E48Fps = 5,
    E30Fps = 6,
    E30FpsDrop = 7,
    NtscDropFrame = 8,
    NtscFullFrame = 9,
    Pal = 10,
    E24Fps = 11,
    E1000Fps = 12,
    FilmFullFrame = 13,
    Custom = 14,
    E96Fps = 15,
    E72Fps = 16,
    E5994Fps = 17,
}

#[allow(clippy::derivable_impls)]
impl Default for TimeMode {
    fn default() -> Self {
        Self::Default
    }
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TimeProtocol {
    Smpte = 0,
    FrameCount = 1,
    Default = 2,
}

#[allow(clippy::derivable_impls)]
impl Default for TimeProtocol {
    fn default() -> Self {
        Self::Smpte
    }
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SnapMode {
    None = 0,
    Snap = 1,
    Play = 2,
    SnapAndPlay = 3,
}

#[allow(clippy::derivable_impls)]
impl Default for SnapMode {
    fn default() -> Self {
        Self::None
    }
}

#[repr(C)]
pub struct SceneSettings {
    pub props: Props,
    pub axes: CoordinateAxes,
    pub unit_meters: Real,
    pub frames_per_second: f64,
    pub ambient_color: Vec3,
    pub default_camera: String,
    pub time_mode: TimeMode,
    pub time_protocol: TimeProtocol,
    pub snap_mode: SnapMode,
    pub original_axis_up: CoordinateAxis,
    pub original_unit_meters: Real,
}

#[repr(C)]
pub struct Scene {
    pub metadata: Metadata,
    pub settings: SceneSettings,
    pub root_node: Ref<Node>,
    pub anim: Ref<Anim>,
    pub unknowns: RefList<Unknown>,
    pub nodes: RefList<Node>,
    pub meshes: RefList<Mesh>,
    pub lights: RefList<Light>,
    pub cameras: RefList<Camera>,
    pub bones: RefList<Bone>,
    pub empties: RefList<Empty>,
    pub line_curves: RefList<LineCurve>,
    pub nurbs_curves: RefList<NurbsCurve>,
    pub nurbs_surfaces: RefList<NurbsSurface>,
    pub nurbs_trim_surfaces: RefList<NurbsTrimSurface>,
    pub nurbs_trim_boundaries: RefList<NurbsTrimBoundary>,
    pub procedural_geometries: RefList<ProceduralGeometry>,
    pub stereo_cameras: RefList<StereoCamera>,
    pub camera_switchers: RefList<CameraSwitcher>,
    pub markers: RefList<Marker>,
    pub lod_groups: RefList<LodGroup>,
    pub skin_deformers: RefList<SkinDeformer>,
    pub skin_clusters: RefList<SkinCluster>,
    pub blend_deformers: RefList<BlendDeformer>,
    pub blend_channels: RefList<BlendChannel>,
    pub blend_shapes: RefList<BlendShape>,
    pub cache_deformers: RefList<CacheDeformer>,
    pub cache_files: RefList<CacheFile>,
    pub materials: RefList<Material>,
    pub textures: RefList<Texture>,
    pub videos: RefList<Video>,
    pub shaders: RefList<Shader>,
    pub shader_bindings: RefList<ShaderBinding>,
    pub anim_stacks: RefList<AnimStack>,
    pub anim_layers: RefList<AnimLayer>,
    pub anim_values: RefList<AnimValue>,
    pub anim_curves: RefList<AnimCurve>,
    pub display_layers: RefList<DisplayLayer>,
    pub selection_sets: RefList<SelectionSet>,
    pub selection_nodes: RefList<SelectionNode>,
    pub characters: RefList<Character>,
    pub constraints: RefList<Constraint>,
    pub audio_layers: RefList<AudioLayer>,
    pub audio_clips: RefList<AudioClip>,
    pub poses: RefList<Pose>,
    pub metadata_objects: RefList<MetadataObject>,
    pub texture_files: List<TextureFile>,
    pub elements: RefList<Element>,
    pub connections_src: List<Connection>,
    pub connections_dst: List<Connection>,
    pub elements_by_name: List<NameElement>,
    pub dom_root: Option<Ref<DomNode>>,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct CurvePoint {
    pub valid: bool,
    pub position: Vec3,
    pub derivative: Vec3,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct SurfacePoint {
    pub valid: bool,
    pub position: Vec3,
    pub derivative_u: Vec3,
    pub derivative_v: Vec3,
}

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct TopoFlags(u32);
impl TopoFlags {
    pub const NONE: TopoFlags = TopoFlags(0);
    pub const NON_MANIFOLD: TopoFlags = TopoFlags(0x1);
}

const TOPOFLAGS_NAMES: [(&str, u32); 1] = [("NON_MANIFOLD", 0x1)];

impl TopoFlags {
    pub fn any(self) -> bool {
        self.0 != 0
    }
    pub fn has_any(self, bits: Self) -> bool {
        (self.0 & bits.0) != 0
    }
    pub fn has_all(self, bits: Self) -> bool {
        (self.0 & bits.0) == bits.0
    }
    #[allow(dead_code)]
    pub(crate) const fn from_raw(bits: u32) -> Self {
        Self(bits)
    }
    #[allow(dead_code)]
    pub(crate) const fn raw(self) -> u32 {
        self.0
    }
}
#[allow(clippy::derivable_impls)]
impl Default for TopoFlags {
    fn default() -> Self {
        Self(0)
    }
}
impl Debug for TopoFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        format_flags(f, &TOPOFLAGS_NAMES, self.0)
    }
}
impl BitAnd for TopoFlags {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}
impl BitAndAssign for TopoFlags {
    fn bitand_assign(&mut self, rhs: Self) {
        *self = Self(self.0 & rhs.0)
    }
}
impl BitOr for TopoFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}
impl BitOrAssign for TopoFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = Self(self.0 | rhs.0)
    }
}
impl BitXor for TopoFlags {
    type Output = Self;
    fn bitxor(self, rhs: Self) -> Self::Output {
        Self(self.0 ^ rhs.0)
    }
}
impl BitXorAssign for TopoFlags {
    fn bitxor_assign(&mut self, rhs: Self) {
        *self = Self(self.0 ^ rhs.0)
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct TopoEdge {
    pub index: u32,
    pub next: u32,
    pub prev: u32,
    pub twin: u32,
    pub face: u32,
    pub edge: u32,
    pub flags: TopoFlags,
}

#[repr(C)]
pub struct RawVertexStream {
    pub data: *mut c_void,
    pub vertex_count: usize,
    pub vertex_size: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RawAllocator {
    pub alloc_fn: Option<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>,
    pub realloc_fn:
        Option<unsafe extern "C" fn(*mut c_void, *mut c_void, usize, usize) -> *mut c_void>,
    pub free_fn: Option<unsafe extern "C" fn(*mut c_void, *mut c_void, usize)>,
    pub free_allocator_fn: Option<unsafe extern "C" fn(*mut c_void)>,
    pub user: *mut c_void,
}

#[allow(clippy::derivable_impls)]
impl Default for RawAllocator {
    fn default() -> Self {
        RawAllocator {
            alloc_fn: None,
            realloc_fn: None,
            free_fn: None,
            free_allocator_fn: None,
            user: ptr::null::<c_void>() as *mut c_void,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct RawAllocatorOpts {
    pub allocator: RawAllocator,
    pub memory_limit: usize,
    pub allocation_limit: usize,
    pub huge_threshold: usize,
    pub max_chunk_size: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RawStream {
    pub read_fn: Option<unsafe extern "C" fn(*mut c_void, *mut c_void, usize) -> usize>,
    pub skip_fn: Option<unsafe extern "C" fn(*mut c_void, usize) -> bool>,
    pub size_fn: Option<unsafe extern "C" fn(*mut c_void) -> u64>,
    pub close_fn: Option<unsafe extern "C" fn(*mut c_void)>,
    pub user: *mut c_void,
}

#[allow(clippy::derivable_impls)]
impl Default for RawStream {
    fn default() -> Self {
        RawStream {
            read_fn: None,
            skip_fn: None,
            size_fn: None,
            close_fn: None,
            user: ptr::null::<c_void>() as *mut c_void,
        }
    }
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OpenFileType {
    MainModel = 0,
    GeometryCache = 1,
    ObjMtl = 2,
}

#[allow(clippy::derivable_impls)]
impl Default for OpenFileType {
    fn default() -> Self {
        Self::MainModel
    }
}

#[repr(C)]
pub struct OpenFileInfo {
    pub context: OpenFileContext,
    pub type_: OpenFileType,
    pub original_filename: Blob,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RawOpenFileCb {
    pub fn_: Option<
        unsafe extern "C" fn(
            *mut c_void,
            *mut RawStream,
            *const u8,
            usize,
            *const OpenFileInfo,
        ) -> bool,
    >,
    pub user: *mut c_void,
}

#[allow(clippy::derivable_impls)]
impl Default for RawOpenFileCb {
    fn default() -> Self {
        RawOpenFileCb {
            fn_: None,
            user: ptr::null::<c_void>() as *mut c_void,
        }
    }
}

#[repr(C)]
#[derive(Default)]
pub struct RawOpenFileOpts {
    pub _begin_zero: u32,
    pub allocator: RawAllocatorOpts,
    pub filename_null_terminated: bool,
    pub _end_zero: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RawCloseMemoryCb {
    pub fn_: Option<unsafe extern "C" fn(*mut c_void, *mut c_void, usize)>,
    pub user: *mut c_void,
}

#[allow(clippy::derivable_impls)]
impl Default for RawCloseMemoryCb {
    fn default() -> Self {
        RawCloseMemoryCb {
            fn_: None,
            user: ptr::null::<c_void>() as *mut c_void,
        }
    }
}

#[repr(C)]
#[derive(Default)]
pub struct RawOpenMemoryOpts {
    pub _begin_zero: u32,
    pub allocator: RawAllocatorOpts,
    pub no_copy: bool,
    pub close_cb: RawCloseMemoryCb,
    pub _end_zero: u32,
}

#[repr(C)]
#[derive(Default)]
pub struct ErrorFrame {
    pub source_line: u32,
    pub function: String,
    pub description: String,
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ErrorType {
    None = 0,
    Unknown = 1,
    FileNotFound = 2,
    EmptyFile = 3,
    ExternalFileNotFound = 4,
    OutOfMemory = 5,
    MemoryLimit = 6,
    AllocationLimit = 7,
    TruncatedFile = 8,
    Io = 9,
    Cancelled = 10,
    UnrecognizedFileFormat = 11,
    UninitializedOptions = 12,
    ZeroVertexSize = 13,
    TruncatedVertexStream = 14,
    InvalidUtf8 = 15,
    FeatureDisabled = 16,
    BadNurbs = 17,
    BadIndex = 18,
    NodeDepthLimit = 19,
    ThreadedAsciiParse = 20,
    UnsafeOptions = 21,
    DuplicateOverride = 22,
    UnsupportedVersion = 23,
}

#[allow(clippy::derivable_impls)]
impl Default for ErrorType {
    fn default() -> Self {
        Self::None
    }
}

#[repr(C)]
#[derive(Default)]
pub struct Error {
    pub type_: ErrorType,
    pub description: String,
    pub stack_size: u32,
    pub stack: [ErrorFrame; 8],
    pub(crate) info_length: usize,
    pub(crate) info_buf: InlineBuf<[u8; 256]>,
}

impl Error {
    #[allow(clippy::missing_transmute_annotations)]
    pub fn info(&self) -> &str {
        unsafe {
            let buf: &[mem::MaybeUninit<u8>; 256] = mem::transmute(&self.info_buf);
            str::from_utf8(mem::transmute(&buf[..self.info_length])).unwrap()
        }
    }
}

#[repr(C)]
pub struct Progress {
    pub bytes_read: u64,
    pub bytes_total: u64,
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ProgressResult {
    Continue = 256,
    Cancel = 512,
}

#[allow(clippy::derivable_impls)]
impl Default for ProgressResult {
    fn default() -> Self {
        Self::Continue
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RawProgressCb {
    pub fn_: Option<unsafe extern "C" fn(*mut c_void, *const Progress) -> RawEnum<ProgressResult>>,
    pub user: *mut c_void,
}

#[allow(clippy::derivable_impls)]
impl Default for RawProgressCb {
    fn default() -> Self {
        RawProgressCb {
            fn_: None,
            user: ptr::null::<c_void>() as *mut c_void,
        }
    }
}

#[repr(C)]
pub struct InflateInput {
    pub total_size: usize,
    pub data: *const c_void,
    pub data_size: usize,
    pub buffer: *mut c_void,
    pub buffer_size: usize,
    pub read_fn: Option<unsafe extern "C" fn(*mut c_void, *mut c_void, usize) -> usize>,
    pub read_user: *mut c_void,
    pub progress_cb: RawProgressCb,
    pub progress_interval_hint: u64,
    pub progress_size_before: u64,
    pub progress_size_after: u64,
    pub no_header: bool,
    pub no_checksum: bool,
    pub internal_fast_bits: usize,
}

#[repr(C)]
pub struct InflateRetain {
    pub initialized: bool,
    pub data: [u64; 1024],
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IndexErrorHandling {
    Clamp = 0,
    NoIndex = 1,
    AbortLoading = 2,
    UnsafeIgnore = 3,
}

#[allow(clippy::derivable_impls)]
impl Default for IndexErrorHandling {
    fn default() -> Self {
        Self::Clamp
    }
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum UnicodeErrorHandling {
    ReplacementCharacter = 0,
    Underscore = 1,
    QuestionMark = 2,
    Remove = 3,
    AbortLoading = 4,
    UnsafeIgnore = 5,
}

#[allow(clippy::derivable_impls)]
impl Default for UnicodeErrorHandling {
    fn default() -> Self {
        Self::ReplacementCharacter
    }
}

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct BakedKeyFlags(u32);
impl BakedKeyFlags {
    pub const NONE: BakedKeyFlags = BakedKeyFlags(0);
    pub const STEP_LEFT: BakedKeyFlags = BakedKeyFlags(0x1);
    pub const STEP_RIGHT: BakedKeyFlags = BakedKeyFlags(0x2);
    pub const STEP_KEY: BakedKeyFlags = BakedKeyFlags(0x4);
    pub const KEYFRAME: BakedKeyFlags = BakedKeyFlags(0x8);
    pub const REDUCED: BakedKeyFlags = BakedKeyFlags(0x10);
}

const BAKEDKEYFLAGS_NAMES: [(&str, u32); 5] = [
    ("STEP_LEFT", 0x1),
    ("STEP_RIGHT", 0x2),
    ("STEP_KEY", 0x4),
    ("KEYFRAME", 0x8),
    ("REDUCED", 0x10),
];

impl BakedKeyFlags {
    pub fn any(self) -> bool {
        self.0 != 0
    }
    pub fn has_any(self, bits: Self) -> bool {
        (self.0 & bits.0) != 0
    }
    pub fn has_all(self, bits: Self) -> bool {
        (self.0 & bits.0) == bits.0
    }
    #[allow(dead_code)]
    pub(crate) const fn from_raw(bits: u32) -> Self {
        Self(bits)
    }
    #[allow(dead_code)]
    pub(crate) const fn raw(self) -> u32 {
        self.0
    }
}
#[allow(clippy::derivable_impls)]
impl Default for BakedKeyFlags {
    fn default() -> Self {
        Self(0)
    }
}
impl Debug for BakedKeyFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        format_flags(f, &BAKEDKEYFLAGS_NAMES, self.0)
    }
}
impl BitAnd for BakedKeyFlags {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}
impl BitAndAssign for BakedKeyFlags {
    fn bitand_assign(&mut self, rhs: Self) {
        *self = Self(self.0 & rhs.0)
    }
}
impl BitOr for BakedKeyFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}
impl BitOrAssign for BakedKeyFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = Self(self.0 | rhs.0)
    }
}
impl BitXor for BakedKeyFlags {
    type Output = Self;
    fn bitxor(self, rhs: Self) -> Self::Output {
        Self(self.0 ^ rhs.0)
    }
}
impl BitXorAssign for BakedKeyFlags {
    fn bitxor_assign(&mut self, rhs: Self) {
        *self = Self(self.0 ^ rhs.0)
    }
}

#[repr(C)]
pub struct BakedVec3 {
    pub time: f64,
    pub value: Vec3,
    pub flags: BakedKeyFlags,
}

#[repr(C)]
pub struct BakedQuat {
    pub time: f64,
    pub value: Quat,
    pub flags: BakedKeyFlags,
}

#[repr(C)]
pub struct BakedNode {
    pub typed_id: u32,
    pub element_id: u32,
    pub constant_translation: bool,
    pub constant_rotation: bool,
    pub constant_scale: bool,
    pub translation_keys: List<BakedVec3>,
    pub rotation_keys: List<BakedQuat>,
    pub scale_keys: List<BakedVec3>,
}

#[repr(C)]
pub struct BakedProp {
    pub name: String,
    pub constant_value: bool,
    pub keys: List<BakedVec3>,
}

#[repr(C)]
pub struct BakedElement {
    pub element_id: u32,
    pub props: List<BakedProp>,
}

#[repr(C)]
pub struct BakedAnimMetadata {
    pub result_memory_used: usize,
    pub temp_memory_used: usize,
    pub result_allocs: usize,
    pub temp_allocs: usize,
}

#[repr(C)]
pub struct BakedAnim {
    pub nodes: List<BakedNode>,
    pub elements: List<BakedElement>,
    pub playback_time_begin: f64,
    pub playback_time_end: f64,
    pub playback_duration: f64,
    pub key_time_min: f64,
    pub key_time_max: f64,
    pub metadata: BakedAnimMetadata,
}

#[repr(C)]
pub struct ThreadPoolInfo {
    pub max_concurrent_tasks: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RawThreadPool {
    pub init_fn:
        Option<unsafe extern "C" fn(*mut c_void, ThreadPoolContext, *const ThreadPoolInfo) -> bool>,
    pub run_fn: Option<unsafe extern "C" fn(*mut c_void, ThreadPoolContext, u32, u32, u32)>,
    pub wait_fn: Option<unsafe extern "C" fn(*mut c_void, ThreadPoolContext, u32, u32)>,
    pub free_fn: Option<unsafe extern "C" fn(*mut c_void, ThreadPoolContext)>,
    pub user: *mut c_void,
}

#[allow(clippy::derivable_impls)]
impl Default for RawThreadPool {
    fn default() -> Self {
        RawThreadPool {
            init_fn: None,
            run_fn: None,
            wait_fn: None,
            free_fn: None,
            user: ptr::null::<c_void>() as *mut c_void,
        }
    }
}

#[repr(C)]
#[derive(Default)]
pub struct RawThreadOpts {
    pub pool: RawThreadPool,
    pub num_tasks: usize,
    pub memory_limit: usize,
}

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct EvaluateFlags(u32);
impl EvaluateFlags {
    pub const NONE: EvaluateFlags = EvaluateFlags(0);
    pub const NO_EXTRAPOLATION: EvaluateFlags = EvaluateFlags(0x1);
}

const EVALUATEFLAGS_NAMES: [(&str, u32); 1] = [("NO_EXTRAPOLATION", 0x1)];

impl EvaluateFlags {
    pub fn any(self) -> bool {
        self.0 != 0
    }
    pub fn has_any(self, bits: Self) -> bool {
        (self.0 & bits.0) != 0
    }
    pub fn has_all(self, bits: Self) -> bool {
        (self.0 & bits.0) == bits.0
    }
    #[allow(dead_code)]
    pub(crate) const fn from_raw(bits: u32) -> Self {
        Self(bits)
    }
    #[allow(dead_code)]
    pub(crate) const fn raw(self) -> u32 {
        self.0
    }
}
#[allow(clippy::derivable_impls)]
impl Default for EvaluateFlags {
    fn default() -> Self {
        Self(0)
    }
}
impl Debug for EvaluateFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        format_flags(f, &EVALUATEFLAGS_NAMES, self.0)
    }
}
impl BitAnd for EvaluateFlags {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}
impl BitAndAssign for EvaluateFlags {
    fn bitand_assign(&mut self, rhs: Self) {
        *self = Self(self.0 & rhs.0)
    }
}
impl BitOr for EvaluateFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}
impl BitOrAssign for EvaluateFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = Self(self.0 | rhs.0)
    }
}
impl BitXor for EvaluateFlags {
    type Output = Self;
    fn bitxor(self, rhs: Self) -> Self::Output {
        Self(self.0 ^ rhs.0)
    }
}
impl BitXorAssign for EvaluateFlags {
    fn bitxor_assign(&mut self, rhs: Self) {
        *self = Self(self.0 ^ rhs.0)
    }
}

#[repr(C)]
#[derive(Default)]
pub struct RawLoadOpts {
    pub _begin_zero: u32,
    pub temp_allocator: RawAllocatorOpts,
    pub result_allocator: RawAllocatorOpts,
    pub thread_opts: RawThreadOpts,
    pub ignore_geometry: bool,
    pub ignore_animation: bool,
    pub ignore_embedded: bool,
    pub ignore_all_content: bool,
    pub evaluate_skinning: bool,
    pub evaluate_caches: bool,
    pub load_external_files: bool,
    pub ignore_missing_external_files: bool,
    pub skip_skin_vertices: bool,
    pub skip_mesh_parts: bool,
    pub clean_skin_weights: bool,
    pub use_blender_pbr_material: bool,
    pub disable_quirks: bool,
    pub strict: bool,
    pub force_single_thread_ascii_parsing: bool,
    pub allow_unsafe: bool,
    pub index_error_handling: IndexErrorHandling,
    pub connect_broken_elements: bool,
    pub allow_nodes_out_of_root: bool,
    pub allow_missing_vertex_position: bool,
    pub allow_empty_faces: bool,
    pub generate_missing_normals: bool,
    pub open_main_file_with_default: bool,
    pub path_separator: u8,
    pub node_depth_limit: u32,
    pub file_size_estimate: u64,
    pub read_buffer_size: usize,
    pub filename: RawString,
    pub raw_filename: RawBlob,
    pub progress_cb: RawProgressCb,
    pub progress_interval_hint: u64,
    pub open_file_cb: RawOpenFileCb,
    pub geometry_transform_handling: GeometryTransformHandling,
    pub inherit_mode_handling: InheritModeHandling,
    pub space_conversion: SpaceConversion,
    pub pivot_handling: PivotHandling,
    pub pivot_handling_retain_empties: bool,
    pub handedness_conversion_axis: MirrorAxis,
    pub handedness_conversion_retain_winding: bool,
    pub reverse_winding: bool,
    pub target_axes: CoordinateAxes,
    pub target_unit_meters: Real,
    pub target_camera_axes: CoordinateAxes,
    pub target_light_axes: CoordinateAxes,
    pub geometry_transform_helper_name: RawString,
    pub scale_helper_name: RawString,
    pub normalize_normals: bool,
    pub normalize_tangents: bool,
    pub use_root_transform: bool,
    pub root_transform: Transform,
    pub key_clamp_threshold: f64,
    pub unicode_error_handling: UnicodeErrorHandling,
    pub retain_vertex_attrib_w: bool,
    pub retain_dom: bool,
    pub file_format: FileFormat,
    pub file_format_lookahead: usize,
    pub no_format_from_content: bool,
    pub no_format_from_extension: bool,
    pub obj_search_mtl_by_filename: bool,
    pub obj_merge_objects: bool,
    pub obj_merge_groups: bool,
    pub obj_split_groups: bool,
    pub obj_mtl_path: RawString,
    pub obj_mtl_data: RawBlob,
    pub obj_unit_meters: Real,
    pub obj_axes: CoordinateAxes,
    pub _end_zero: u32,
}

#[repr(C)]
#[derive(Default)]
pub struct RawEvaluateOpts {
    pub _begin_zero: u32,
    pub temp_allocator: RawAllocatorOpts,
    pub result_allocator: RawAllocatorOpts,
    pub evaluate_skinning: bool,
    pub evaluate_caches: bool,
    pub evaluate_flags: u32,
    pub load_external_files: bool,
    pub open_file_cb: RawOpenFileCb,
    pub _end_zero: u32,
}

#[repr(C)]
#[derive(Default)]
pub struct RawPropOverrideDesc {
    pub element_id: u32,
    pub prop_name: RawString,
    pub value: Vec4,
    pub value_str: RawString,
    pub value_int: i64,
}

#[repr(C)]
#[derive(Default)]
pub struct RawAnimOpts {
    pub _begin_zero: u32,
    pub layer_ids: RawList<u32>,
    pub override_layer_weights: RawList<Real>,
    pub prop_overrides: RawList<RawPropOverrideDesc>,
    pub transform_overrides: RawList<TransformOverride>,
    pub ignore_connections: bool,
    pub result_allocator: RawAllocatorOpts,
    pub _end_zero: u32,
}

#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BakeStepHandling {
    Default = 0,
    CustomDuration = 1,
    IdenticalTime = 2,
    AdjacentDouble = 3,
    Ignore = 4,
}

#[allow(clippy::derivable_impls)]
impl Default for BakeStepHandling {
    fn default() -> Self {
        Self::Default
    }
}

#[repr(C)]
#[derive(Default)]
pub struct RawBakeOpts {
    pub _begin_zero: u32,
    pub temp_allocator: RawAllocatorOpts,
    pub result_allocator: RawAllocatorOpts,
    pub trim_start_time: bool,
    pub resample_rate: f64,
    pub minimum_sample_rate: f64,
    pub maximum_sample_rate: f64,
    pub bake_transform_props: bool,
    pub skip_node_transforms: bool,
    pub no_resample_rotation: bool,
    pub ignore_layer_weight_animation: bool,
    pub max_keyframe_segments: usize,
    pub step_handling: BakeStepHandling,
    pub step_custom_duration: f64,
    pub step_custom_epsilon: f64,
    pub evaluate_flags: u32,
    pub key_reduction_enabled: bool,
    pub key_reduction_rotation: bool,
    pub key_reduction_threshold: f64,
    pub key_reduction_passes: usize,
    pub _end_zero: u32,
}

#[repr(C)]
#[derive(Default)]
pub struct RawTessellateCurveOpts {
    pub _begin_zero: u32,
    pub temp_allocator: RawAllocatorOpts,
    pub result_allocator: RawAllocatorOpts,
    pub span_subdivision: usize,
    pub _end_zero: u32,
}

#[repr(C)]
#[derive(Default)]
pub struct RawTessellateSurfaceOpts {
    pub _begin_zero: u32,
    pub temp_allocator: RawAllocatorOpts,
    pub result_allocator: RawAllocatorOpts,
    pub span_subdivision_u: usize,
    pub span_subdivision_v: usize,
    pub skip_mesh_parts: bool,
    pub _end_zero: u32,
}

#[repr(C)]
#[derive(Default)]
pub struct RawSubdivideOpts {
    pub _begin_zero: u32,
    pub temp_allocator: RawAllocatorOpts,
    pub result_allocator: RawAllocatorOpts,
    pub boundary: SubdivisionBoundary,
    pub uv_boundary: SubdivisionBoundary,
    pub ignore_normals: bool,
    pub interpolate_normals: bool,
    pub interpolate_tangents: bool,
    pub evaluate_source_vertices: bool,
    pub max_source_vertices: usize,
    pub evaluate_skin_weights: bool,
    pub max_skin_weights: usize,
    pub skin_deformer_index: usize,
    pub _end_zero: u32,
}

#[repr(C)]
#[derive(Default)]
pub struct RawGeometryCacheOpts {
    pub _begin_zero: u32,
    pub temp_allocator: RawAllocatorOpts,
    pub result_allocator: RawAllocatorOpts,
    pub open_file_cb: RawOpenFileCb,
    pub frames_per_second: f64,
    pub mirror_axis: MirrorAxis,
    pub use_scale_factor: bool,
    pub scale_factor: Real,
    pub _end_zero: u32,
}

#[repr(C)]
#[derive(Default)]
pub struct RawGeometryCacheDataOpts {
    pub _begin_zero: u32,
    pub open_file_cb: RawOpenFileCb,
    pub additive: bool,
    pub use_weight: bool,
    pub weight: Real,
    pub ignore_transform: bool,
    pub _end_zero: u32,
}

#[repr(C)]
#[derive(Default)]
pub struct Panic {
    pub did_panic: bool,
    pub(crate) message_length: usize,
    pub(crate) message_buf: InlineBuf<[u8; 128]>,
}

impl Panic {
    #[allow(clippy::missing_transmute_annotations)]
    pub fn message(&self) -> &str {
        unsafe {
            let buf: &[mem::MaybeUninit<u8>; 128] = mem::transmute(&self.message_buf);
            str::from_utf8(mem::transmute(&buf[..self.message_length])).unwrap()
        }
    }
}

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct TransformFlags(u32);
impl TransformFlags {
    pub const NONE: TransformFlags = TransformFlags(0);
    pub const IGNORE_SCALE_HELPER: TransformFlags = TransformFlags(0x1);
    pub const IGNORE_COMPONENTWISE_SCALE: TransformFlags = TransformFlags(0x2);
    pub const EXPLICIT_INCLUDES: TransformFlags = TransformFlags(0x4);
    pub const INCLUDE_TRANSLATION: TransformFlags = TransformFlags(0x10);
    pub const INCLUDE_ROTATION: TransformFlags = TransformFlags(0x20);
    pub const INCLUDE_SCALE: TransformFlags = TransformFlags(0x40);
    pub const NO_EXTRAPOLATION: TransformFlags = TransformFlags(0x80);
}

const TRANSFORMFLAGS_NAMES: [(&str, u32); 7] = [
    ("IGNORE_SCALE_HELPER", 0x1),
    ("IGNORE_COMPONENTWISE_SCALE", 0x2),
    ("EXPLICIT_INCLUDES", 0x4),
    ("INCLUDE_TRANSLATION", 0x10),
    ("INCLUDE_ROTATION", 0x20),
    ("INCLUDE_SCALE", 0x40),
    ("NO_EXTRAPOLATION", 0x80),
];

impl TransformFlags {
    pub fn any(self) -> bool {
        self.0 != 0
    }
    pub fn has_any(self, bits: Self) -> bool {
        (self.0 & bits.0) != 0
    }
    pub fn has_all(self, bits: Self) -> bool {
        (self.0 & bits.0) == bits.0
    }
    #[allow(dead_code)]
    pub(crate) const fn from_raw(bits: u32) -> Self {
        Self(bits)
    }
    #[allow(dead_code)]
    pub(crate) const fn raw(self) -> u32 {
        self.0
    }
}
#[allow(clippy::derivable_impls)]
impl Default for TransformFlags {
    fn default() -> Self {
        Self(0)
    }
}
impl Debug for TransformFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        format_flags(f, &TRANSFORMFLAGS_NAMES, self.0)
    }
}
impl BitAnd for TransformFlags {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}
impl BitAndAssign for TransformFlags {
    fn bitand_assign(&mut self, rhs: Self) {
        *self = Self(self.0 & rhs.0)
    }
}
impl BitOr for TransformFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}
impl BitOrAssign for TransformFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = Self(self.0 | rhs.0)
    }
}
impl BitXor for TransformFlags {
    type Output = Self;
    fn bitxor(self, rhs: Self) -> Self::Output {
        Self(self.0 ^ rhs.0)
    }
}
impl BitXorAssign for TransformFlags {
    fn bitxor_assign(&mut self, rhs: Self) {
        *self = Self(self.0 ^ rhs.0)
    }
}

#[derive(Default)]
pub struct AllocatorOpts {
    pub allocator: Allocator,
    pub memory_limit: usize,
    pub allocation_limit: usize,
    pub huge_threshold: usize,
    pub max_chunk_size: usize,
}

impl ToRaw for AllocatorOpts {
    type Result = RawAllocatorOpts;
    #[allow(unused, unused_variables, dead_code)]
    fn to_raw(&self, arena: &mut Arena) -> Self::Result {
        RawAllocatorOpts {
            allocator: self.allocator.to_raw(),
            memory_limit: self.memory_limit,
            allocation_limit: self.allocation_limit,
            huge_threshold: self.huge_threshold,
            max_chunk_size: self.max_chunk_size,
        }
    }
    #[allow(unused, unused_variables, dead_code)]
    fn to_raw_mut(&mut self, arena: &mut Arena) -> Self::Result {
        RawAllocatorOpts {
            allocator: self.allocator.to_raw_mut(),
            memory_limit: self.memory_limit,
            allocation_limit: self.allocation_limit,
            huge_threshold: self.huge_threshold,
            max_chunk_size: self.max_chunk_size,
        }
    }
}

pub enum OpenFileCb<'a> {
    Unset,
    Mut(&'a mut dyn FnMut(&str, &OpenFileInfo) -> Option<Stream>),
    Ref(&'a dyn Fn(&str, &OpenFileInfo) -> Option<Stream>),
    Raw(Unsafe<RawOpenFileCb>),
}

#[allow(clippy::derivable_impls)]
impl<'a> Default for OpenFileCb<'a> {
    fn default() -> Self {
        Self::Unset
    }
}

impl RawOpenFileCb {
    fn from_func<F: FnMut(&str, &OpenFileInfo) -> Option<Stream>>(arg: &mut F) -> Self {
        RawOpenFileCb {
            fn_: Some(call_open_file_cb::<F>),
            user: arg as *mut F as *mut c_void,
        }
    }
}

impl OpenFileCb<'_> {
    fn to_raw(&self) -> RawOpenFileCb {
        match self {
            OpenFileCb::Unset => Default::default(),
            _ => panic!("required mutable"),
        }
    }

    fn to_raw_mut(&mut self) -> RawOpenFileCb {
        match self {
            OpenFileCb::Unset => Default::default(),
            OpenFileCb::Ref(f) => RawOpenFileCb::from_func(f),
            OpenFileCb::Mut(f) => RawOpenFileCb::from_func(f),
            OpenFileCb::Raw(raw) => raw.take(),
        }
    }
}

#[derive(Default)]
pub struct OpenFileOpts {
    pub allocator: AllocatorOpts,
    pub filename_null_terminated: Unsafe<bool>,
}

impl ToRaw for OpenFileOpts {
    type Result = RawOpenFileOpts;
    #[allow(unused, unused_variables, dead_code)]
    fn to_raw(&self, arena: &mut Arena) -> Self::Result {
        RawOpenFileOpts {
            _begin_zero: 0,
            allocator: self.allocator.to_raw(arena),
            filename_null_terminated: panic!("required mutable"),
            _end_zero: 0,
        }
    }
    #[allow(unused, unused_variables, dead_code)]
    fn to_raw_mut(&mut self, arena: &mut Arena) -> Self::Result {
        RawOpenFileOpts {
            _begin_zero: 0,
            allocator: self.allocator.to_raw_mut(arena),
            filename_null_terminated: self.filename_null_terminated.take(),
            _end_zero: 0,
        }
    }
}

pub enum CloseMemoryCb<'a> {
    Unset,
    Mut(&'a mut dyn FnMut(*mut c_void, usize)),
    Ref(&'a dyn Fn(*mut c_void, usize)),
    Raw(Unsafe<RawCloseMemoryCb>),
}

#[allow(clippy::derivable_impls)]
impl<'a> Default for CloseMemoryCb<'a> {
    fn default() -> Self {
        Self::Unset
    }
}

impl RawCloseMemoryCb {
    fn from_func<F: FnMut(*mut c_void, usize)>(arg: &mut F) -> Self {
        RawCloseMemoryCb {
            fn_: Some(call_close_memory_cb::<F>),
            user: arg as *mut F as *mut c_void,
        }
    }
}

impl CloseMemoryCb<'_> {
    fn to_raw(&self) -> RawCloseMemoryCb {
        match self {
            CloseMemoryCb::Unset => Default::default(),
            _ => panic!("required mutable"),
        }
    }

    fn to_raw_mut(&mut self) -> RawCloseMemoryCb {
        match self {
            CloseMemoryCb::Unset => Default::default(),
            CloseMemoryCb::Ref(f) => RawCloseMemoryCb::from_func(f),
            CloseMemoryCb::Mut(f) => RawCloseMemoryCb::from_func(f),
            CloseMemoryCb::Raw(raw) => raw.take(),
        }
    }
}

#[derive(Default)]
pub struct OpenMemoryOpts<'a> {
    pub allocator: AllocatorOpts,
    pub no_copy: Unsafe<bool>,
    pub close_cb: CloseMemoryCb<'a>,
}

impl<'a> ToRaw for OpenMemoryOpts<'a> {
    type Result = RawOpenMemoryOpts;
    #[allow(unused, unused_variables, dead_code)]
    fn to_raw(&self, arena: &mut Arena) -> Self::Result {
        RawOpenMemoryOpts {
            _begin_zero: 0,
            allocator: self.allocator.to_raw(arena),
            no_copy: panic!("required mutable"),
            close_cb: self.close_cb.to_raw(),
            _end_zero: 0,
        }
    }
    #[allow(unused, unused_variables, dead_code)]
    fn to_raw_mut(&mut self, arena: &mut Arena) -> Self::Result {
        RawOpenMemoryOpts {
            _begin_zero: 0,
            allocator: self.allocator.to_raw_mut(arena),
            no_copy: self.no_copy.take(),
            close_cb: self.close_cb.to_raw_mut(),
            _end_zero: 0,
        }
    }
}

pub enum ProgressCb<'a> {
    Unset,
    Mut(&'a mut dyn FnMut(&Progress) -> ProgressResult),
    Ref(&'a dyn Fn(&Progress) -> ProgressResult),
    Raw(Unsafe<RawProgressCb>),
}

#[allow(clippy::derivable_impls)]
impl<'a> Default for ProgressCb<'a> {
    fn default() -> Self {
        Self::Unset
    }
}

impl RawProgressCb {
    fn from_func<F: FnMut(&Progress) -> ProgressResult>(arg: &mut F) -> Self {
        RawProgressCb {
            fn_: Some(call_progress_cb::<F>),
            user: arg as *mut F as *mut c_void,
        }
    }
}

impl ProgressCb<'_> {
    fn to_raw(&self) -> RawProgressCb {
        match self {
            ProgressCb::Unset => Default::default(),
            _ => panic!("required mutable"),
        }
    }

    fn to_raw_mut(&mut self) -> RawProgressCb {
        match self {
            ProgressCb::Unset => Default::default(),
            ProgressCb::Ref(f) => RawProgressCb::from_func(f),
            ProgressCb::Mut(f) => RawProgressCb::from_func(f),
            ProgressCb::Raw(raw) => raw.take(),
        }
    }
}

#[derive(Default)]
pub struct ThreadOpts {
    pub pool: ThreadPool,
    pub num_tasks: usize,
    pub memory_limit: usize,
}

impl ToRaw for ThreadOpts {
    type Result = RawThreadOpts;
    #[allow(unused, unused_variables, dead_code)]
    fn to_raw(&self, arena: &mut Arena) -> Self::Result {
        RawThreadOpts {
            pool: self.pool.to_raw(),
            num_tasks: self.num_tasks,
            memory_limit: self.memory_limit,
        }
    }
    #[allow(unused, unused_variables, dead_code)]
    fn to_raw_mut(&mut self, arena: &mut Arena) -> Self::Result {
        RawThreadOpts {
            pool: self.pool.to_raw_mut(),
            num_tasks: self.num_tasks,
            memory_limit: self.memory_limit,
        }
    }
}

#[derive(Default)]
pub struct LoadOpts<'a> {
    pub temp_allocator: AllocatorOpts,
    pub result_allocator: AllocatorOpts,
    pub thread_opts: ThreadOpts,
    pub ignore_geometry: bool,
    pub ignore_animation: bool,
    pub ignore_embedded: bool,
    pub ignore_all_content: bool,
    pub evaluate_skinning: bool,
    pub evaluate_caches: bool,
    pub load_external_files: bool,
    pub ignore_missing_external_files: bool,
    pub skip_skin_vertices: bool,
    pub skip_mesh_parts: bool,
    pub clean_skin_weights: bool,
    pub use_blender_pbr_material: bool,
    pub disable_quirks: bool,
    pub strict: bool,
    pub force_single_thread_ascii_parsing: bool,
    pub allow_unsafe: Unsafe<bool>,
    pub index_error_handling: IndexErrorHandling,
    pub connect_broken_elements: bool,
    pub allow_nodes_out_of_root: bool,
    pub allow_missing_vertex_position: bool,
    pub allow_empty_faces: bool,
    pub generate_missing_normals: bool,
    pub open_main_file_with_default: bool,
    pub path_separator: u8,
    pub node_depth_limit: u32,
    pub file_size_estimate: u64,
    pub read_buffer_size: usize,
    pub filename: StringOpt<'a>,
    pub raw_filename: BlobOpt<'a>,
    pub progress_cb: ProgressCb<'a>,
    pub progress_interval_hint: u64,
    pub open_file_cb: OpenFileCb<'a>,
    pub geometry_transform_handling: GeometryTransformHandling,
    pub inherit_mode_handling: InheritModeHandling,
    pub space_conversion: SpaceConversion,
    pub pivot_handling: PivotHandling,
    pub pivot_handling_retain_empties: bool,
    pub handedness_conversion_axis: MirrorAxis,
    pub handedness_conversion_retain_winding: bool,
    pub reverse_winding: bool,
    pub target_axes: CoordinateAxes,
    pub target_unit_meters: Real,
    pub target_camera_axes: CoordinateAxes,
    pub target_light_axes: CoordinateAxes,
    pub geometry_transform_helper_name: StringOpt<'a>,
    pub scale_helper_name: StringOpt<'a>,
    pub normalize_normals: bool,
    pub normalize_tangents: bool,
    pub use_root_transform: bool,
    pub root_transform: Transform,
    pub key_clamp_threshold: f64,
    pub unicode_error_handling: UnicodeErrorHandling,
    pub retain_vertex_attrib_w: bool,
    pub retain_dom: bool,
    pub file_format: FileFormat,
    pub file_format_lookahead: usize,
    pub no_format_from_content: bool,
    pub no_format_from_extension: bool,
    pub obj_search_mtl_by_filename: bool,
    pub obj_merge_objects: bool,
    pub obj_merge_groups: bool,
    pub obj_split_groups: bool,
    pub obj_mtl_path: StringOpt<'a>,
    pub obj_mtl_data: BlobOpt<'a>,
    pub obj_unit_meters: Real,
    pub obj_axes: CoordinateAxes,
}

impl<'a> ToRaw for LoadOpts<'a> {
    type Result = RawLoadOpts;
    #[allow(unused, unused_variables, dead_code)]
    fn to_raw(&self, arena: &mut Arena) -> Self::Result {
        RawLoadOpts {
            _begin_zero: 0,
            temp_allocator: self.temp_allocator.to_raw(arena),
            result_allocator: self.result_allocator.to_raw(arena),
            thread_opts: self.thread_opts.to_raw(arena),
            ignore_geometry: self.ignore_geometry,
            ignore_animation: self.ignore_animation,
            ignore_embedded: self.ignore_embedded,
            ignore_all_content: self.ignore_all_content,
            evaluate_skinning: self.evaluate_skinning,
            evaluate_caches: self.evaluate_caches,
            load_external_files: self.load_external_files,
            ignore_missing_external_files: self.ignore_missing_external_files,
            skip_skin_vertices: self.skip_skin_vertices,
            skip_mesh_parts: self.skip_mesh_parts,
            clean_skin_weights: self.clean_skin_weights,
            use_blender_pbr_material: self.use_blender_pbr_material,
            disable_quirks: self.disable_quirks,
            strict: self.strict,
            force_single_thread_ascii_parsing: self.force_single_thread_ascii_parsing,
            allow_unsafe: panic!("required mutable"),
            index_error_handling: self.index_error_handling,
            connect_broken_elements: self.connect_broken_elements,
            allow_nodes_out_of_root: self.allow_nodes_out_of_root,
            allow_missing_vertex_position: self.allow_missing_vertex_position,
            allow_empty_faces: self.allow_empty_faces,
            generate_missing_normals: self.generate_missing_normals,
            open_main_file_with_default: self.open_main_file_with_default,
            path_separator: self.path_separator,
            node_depth_limit: self.node_depth_limit,
            file_size_estimate: self.file_size_estimate,
            read_buffer_size: self.read_buffer_size,
            filename: self.filename.to_raw(arena),
            raw_filename: self.raw_filename.to_raw(arena),
            progress_cb: self.progress_cb.to_raw(),
            progress_interval_hint: self.progress_interval_hint,
            open_file_cb: self.open_file_cb.to_raw(),
            geometry_transform_handling: self.geometry_transform_handling,
            inherit_mode_handling: self.inherit_mode_handling,
            space_conversion: self.space_conversion,
            pivot_handling: self.pivot_handling,
            pivot_handling_retain_empties: self.pivot_handling_retain_empties,
            handedness_conversion_axis: self.handedness_conversion_axis,
            handedness_conversion_retain_winding: self.handedness_conversion_retain_winding,
            reverse_winding: self.reverse_winding,
            target_axes: self.target_axes,
            target_unit_meters: self.target_unit_meters,
            target_camera_axes: self.target_camera_axes,
            target_light_axes: self.target_light_axes,
            geometry_transform_helper_name: self.geometry_transform_helper_name.to_raw(arena),
            scale_helper_name: self.scale_helper_name.to_raw(arena),
            normalize_normals: self.normalize_normals,
            normalize_tangents: self.normalize_tangents,
            use_root_transform: self.use_root_transform,
            root_transform: self.root_transform,
            key_clamp_threshold: self.key_clamp_threshold,
            unicode_error_handling: self.unicode_error_handling,
            retain_vertex_attrib_w: self.retain_vertex_attrib_w,
            retain_dom: self.retain_dom,
            file_format: self.file_format,
            file_format_lookahead: self.file_format_lookahead,
            no_format_from_content: self.no_format_from_content,
            no_format_from_extension: self.no_format_from_extension,
            obj_search_mtl_by_filename: self.obj_search_mtl_by_filename,
            obj_merge_objects: self.obj_merge_objects,
            obj_merge_groups: self.obj_merge_groups,
            obj_split_groups: self.obj_split_groups,
            obj_mtl_path: self.obj_mtl_path.to_raw(arena),
            obj_mtl_data: self.obj_mtl_data.to_raw(arena),
            obj_unit_meters: self.obj_unit_meters,
            obj_axes: self.obj_axes,
            _end_zero: 0,
        }
    }
    #[allow(unused, unused_variables, dead_code)]
    fn to_raw_mut(&mut self, arena: &mut Arena) -> Self::Result {
        RawLoadOpts {
            _begin_zero: 0,
            temp_allocator: self.temp_allocator.to_raw_mut(arena),
            result_allocator: self.result_allocator.to_raw_mut(arena),
            thread_opts: self.thread_opts.to_raw_mut(arena),
            ignore_geometry: self.ignore_geometry,
            ignore_animation: self.ignore_animation,
            ignore_embedded: self.ignore_embedded,
            ignore_all_content: self.ignore_all_content,
            evaluate_skinning: self.evaluate_skinning,
            evaluate_caches: self.evaluate_caches,
            load_external_files: self.load_external_files,
            ignore_missing_external_files: self.ignore_missing_external_files,
            skip_skin_vertices: self.skip_skin_vertices,
            skip_mesh_parts: self.skip_mesh_parts,
            clean_skin_weights: self.clean_skin_weights,
            use_blender_pbr_material: self.use_blender_pbr_material,
            disable_quirks: self.disable_quirks,
            strict: self.strict,
            force_single_thread_ascii_parsing: self.force_single_thread_ascii_parsing,
            allow_unsafe: self.allow_unsafe.take(),
            index_error_handling: self.index_error_handling,
            connect_broken_elements: self.connect_broken_elements,
            allow_nodes_out_of_root: self.allow_nodes_out_of_root,
            allow_missing_vertex_position: self.allow_missing_vertex_position,
            allow_empty_faces: self.allow_empty_faces,
            generate_missing_normals: self.generate_missing_normals,
            open_main_file_with_default: self.open_main_file_with_default,
            path_separator: self.path_separator,
            node_depth_limit: self.node_depth_limit,
            file_size_estimate: self.file_size_estimate,
            read_buffer_size: self.read_buffer_size,
            filename: self.filename.to_raw_mut(arena),
            raw_filename: self.raw_filename.to_raw_mut(arena),
            progress_cb: self.progress_cb.to_raw_mut(),
            progress_interval_hint: self.progress_interval_hint,
            open_file_cb: self.open_file_cb.to_raw_mut(),
            geometry_transform_handling: self.geometry_transform_handling,
            inherit_mode_handling: self.inherit_mode_handling,
            space_conversion: self.space_conversion,
            pivot_handling: self.pivot_handling,
            pivot_handling_retain_empties: self.pivot_handling_retain_empties,
            handedness_conversion_axis: self.handedness_conversion_axis,
            handedness_conversion_retain_winding: self.handedness_conversion_retain_winding,
            reverse_winding: self.reverse_winding,
            target_axes: self.target_axes,
            target_unit_meters: self.target_unit_meters,
            target_camera_axes: self.target_camera_axes,
            target_light_axes: self.target_light_axes,
            geometry_transform_helper_name: self.geometry_transform_helper_name.to_raw_mut(arena),
            scale_helper_name: self.scale_helper_name.to_raw_mut(arena),
            normalize_normals: self.normalize_normals,
            normalize_tangents: self.normalize_tangents,
            use_root_transform: self.use_root_transform,
            root_transform: self.root_transform,
            key_clamp_threshold: self.key_clamp_threshold,
            unicode_error_handling: self.unicode_error_handling,
            retain_vertex_attrib_w: self.retain_vertex_attrib_w,
            retain_dom: self.retain_dom,
            file_format: self.file_format,
            file_format_lookahead: self.file_format_lookahead,
            no_format_from_content: self.no_format_from_content,
            no_format_from_extension: self.no_format_from_extension,
            obj_search_mtl_by_filename: self.obj_search_mtl_by_filename,
            obj_merge_objects: self.obj_merge_objects,
            obj_merge_groups: self.obj_merge_groups,
            obj_split_groups: self.obj_split_groups,
            obj_mtl_path: self.obj_mtl_path.to_raw_mut(arena),
            obj_mtl_data: self.obj_mtl_data.to_raw_mut(arena),
            obj_unit_meters: self.obj_unit_meters,
            obj_axes: self.obj_axes,
            _end_zero: 0,
        }
    }
}

#[derive(Default)]
pub struct EvaluateOpts<'a> {
    pub temp_allocator: AllocatorOpts,
    pub result_allocator: AllocatorOpts,
    pub evaluate_skinning: bool,
    pub evaluate_caches: bool,
    pub evaluate_flags: u32,
    pub load_external_files: bool,
    pub open_file_cb: OpenFileCb<'a>,
}

impl<'a> ToRaw for EvaluateOpts<'a> {
    type Result = RawEvaluateOpts;
    #[allow(unused, unused_variables, dead_code)]
    fn to_raw(&self, arena: &mut Arena) -> Self::Result {
        RawEvaluateOpts {
            _begin_zero: 0,
            temp_allocator: self.temp_allocator.to_raw(arena),
            result_allocator: self.result_allocator.to_raw(arena),
            evaluate_skinning: self.evaluate_skinning,
            evaluate_caches: self.evaluate_caches,
            evaluate_flags: self.evaluate_flags,
            load_external_files: self.load_external_files,
            open_file_cb: self.open_file_cb.to_raw(),
            _end_zero: 0,
        }
    }
    #[allow(unused, unused_variables, dead_code)]
    fn to_raw_mut(&mut self, arena: &mut Arena) -> Self::Result {
        RawEvaluateOpts {
            _begin_zero: 0,
            temp_allocator: self.temp_allocator.to_raw_mut(arena),
            result_allocator: self.result_allocator.to_raw_mut(arena),
            evaluate_skinning: self.evaluate_skinning,
            evaluate_caches: self.evaluate_caches,
            evaluate_flags: self.evaluate_flags,
            load_external_files: self.load_external_files,
            open_file_cb: self.open_file_cb.to_raw_mut(),
            _end_zero: 0,
        }
    }
}

#[derive(Default)]
pub struct PropOverrideDesc<'a> {
    pub element_id: u32,
    pub prop_name: StringOpt<'a>,
    pub value: Vec4,
    pub value_str: StringOpt<'a>,
    pub value_int: i64,
}

impl<'a> ToRaw for PropOverrideDesc<'a> {
    type Result = RawPropOverrideDesc;
    #[allow(unused, unused_variables, dead_code)]
    fn to_raw(&self, arena: &mut Arena) -> Self::Result {
        RawPropOverrideDesc {
            element_id: self.element_id,
            prop_name: self.prop_name.to_raw(arena),
            value: self.value,
            value_str: self.value_str.to_raw(arena),
            value_int: self.value_int,
        }
    }
    #[allow(unused, unused_variables, dead_code)]
    fn to_raw_mut(&mut self, arena: &mut Arena) -> Self::Result {
        RawPropOverrideDesc {
            element_id: self.element_id,
            prop_name: self.prop_name.to_raw_mut(arena),
            value: self.value,
            value_str: self.value_str.to_raw_mut(arena),
            value_int: self.value_int,
        }
    }
}

#[derive(Default)]
pub struct AnimOpts<'a> {
    pub layer_ids: ListOpt<'a, u32>,
    pub override_layer_weights: ListOpt<'a, Real>,
    pub prop_overrides: ListOpt<'a, PropOverrideDesc<'a>>,
    pub transform_overrides: ListOpt<'a, TransformOverride>,
    pub ignore_connections: bool,
    pub result_allocator: AllocatorOpts,
}

impl<'a> ToRaw for AnimOpts<'a> {
    type Result = RawAnimOpts;
    #[allow(unused, unused_variables, dead_code)]
    fn to_raw(&self, arena: &mut Arena) -> Self::Result {
        RawAnimOpts {
            _begin_zero: 0,
            layer_ids: self.layer_ids.to_raw(arena),
            override_layer_weights: self.override_layer_weights.to_raw(arena),
            prop_overrides: self.prop_overrides.to_raw(arena),
            transform_overrides: self.transform_overrides.to_raw(arena),
            ignore_connections: self.ignore_connections,
            result_allocator: self.result_allocator.to_raw(arena),
            _end_zero: 0,
        }
    }
    #[allow(unused, unused_variables, dead_code)]
    fn to_raw_mut(&mut self, arena: &mut Arena) -> Self::Result {
        RawAnimOpts {
            _begin_zero: 0,
            layer_ids: self.layer_ids.to_raw_mut(arena),
            override_layer_weights: self.override_layer_weights.to_raw_mut(arena),
            prop_overrides: self.prop_overrides.to_raw_mut(arena),
            transform_overrides: self.transform_overrides.to_raw_mut(arena),
            ignore_connections: self.ignore_connections,
            result_allocator: self.result_allocator.to_raw_mut(arena),
            _end_zero: 0,
        }
    }
}

#[derive(Default)]
pub struct BakeOpts {
    pub temp_allocator: AllocatorOpts,
    pub result_allocator: AllocatorOpts,
    pub trim_start_time: bool,
    pub resample_rate: f64,
    pub minimum_sample_rate: f64,
    pub maximum_sample_rate: f64,
    pub bake_transform_props: bool,
    pub skip_node_transforms: bool,
    pub no_resample_rotation: bool,
    pub ignore_layer_weight_animation: bool,
    pub max_keyframe_segments: usize,
    pub step_handling: BakeStepHandling,
    pub step_custom_duration: f64,
    pub step_custom_epsilon: f64,
    pub evaluate_flags: u32,
    pub key_reduction_enabled: bool,
    pub key_reduction_rotation: bool,
    pub key_reduction_threshold: f64,
    pub key_reduction_passes: usize,
}

impl ToRaw for BakeOpts {
    type Result = RawBakeOpts;
    #[allow(unused, unused_variables, dead_code)]
    fn to_raw(&self, arena: &mut Arena) -> Self::Result {
        RawBakeOpts {
            _begin_zero: 0,
            temp_allocator: self.temp_allocator.to_raw(arena),
            result_allocator: self.result_allocator.to_raw(arena),
            trim_start_time: self.trim_start_time,
            resample_rate: self.resample_rate,
            minimum_sample_rate: self.minimum_sample_rate,
            maximum_sample_rate: self.maximum_sample_rate,
            bake_transform_props: self.bake_transform_props,
            skip_node_transforms: self.skip_node_transforms,
            no_resample_rotation: self.no_resample_rotation,
            ignore_layer_weight_animation: self.ignore_layer_weight_animation,
            max_keyframe_segments: self.max_keyframe_segments,
            step_handling: self.step_handling,
            step_custom_duration: self.step_custom_duration,
            step_custom_epsilon: self.step_custom_epsilon,
            evaluate_flags: self.evaluate_flags,
            key_reduction_enabled: self.key_reduction_enabled,
            key_reduction_rotation: self.key_reduction_rotation,
            key_reduction_threshold: self.key_reduction_threshold,
            key_reduction_passes: self.key_reduction_passes,
            _end_zero: 0,
        }
    }
    #[allow(unused, unused_variables, dead_code)]
    fn to_raw_mut(&mut self, arena: &mut Arena) -> Self::Result {
        RawBakeOpts {
            _begin_zero: 0,
            temp_allocator: self.temp_allocator.to_raw_mut(arena),
            result_allocator: self.result_allocator.to_raw_mut(arena),
            trim_start_time: self.trim_start_time,
            resample_rate: self.resample_rate,
            minimum_sample_rate: self.minimum_sample_rate,
            maximum_sample_rate: self.maximum_sample_rate,
            bake_transform_props: self.bake_transform_props,
            skip_node_transforms: self.skip_node_transforms,
            no_resample_rotation: self.no_resample_rotation,
            ignore_layer_weight_animation: self.ignore_layer_weight_animation,
            max_keyframe_segments: self.max_keyframe_segments,
            step_handling: self.step_handling,
            step_custom_duration: self.step_custom_duration,
            step_custom_epsilon: self.step_custom_epsilon,
            evaluate_flags: self.evaluate_flags,
            key_reduction_enabled: self.key_reduction_enabled,
            key_reduction_rotation: self.key_reduction_rotation,
            key_reduction_threshold: self.key_reduction_threshold,
            key_reduction_passes: self.key_reduction_passes,
            _end_zero: 0,
        }
    }
}

#[derive(Default)]
pub struct TessellateCurveOpts {
    pub temp_allocator: AllocatorOpts,
    pub result_allocator: AllocatorOpts,
    pub span_subdivision: usize,
}

impl ToRaw for TessellateCurveOpts {
    type Result = RawTessellateCurveOpts;
    #[allow(unused, unused_variables, dead_code)]
    fn to_raw(&self, arena: &mut Arena) -> Self::Result {
        RawTessellateCurveOpts {
            _begin_zero: 0,
            temp_allocator: self.temp_allocator.to_raw(arena),
            result_allocator: self.result_allocator.to_raw(arena),
            span_subdivision: self.span_subdivision,
            _end_zero: 0,
        }
    }
    #[allow(unused, unused_variables, dead_code)]
    fn to_raw_mut(&mut self, arena: &mut Arena) -> Self::Result {
        RawTessellateCurveOpts {
            _begin_zero: 0,
            temp_allocator: self.temp_allocator.to_raw_mut(arena),
            result_allocator: self.result_allocator.to_raw_mut(arena),
            span_subdivision: self.span_subdivision,
            _end_zero: 0,
        }
    }
}

#[derive(Default)]
pub struct TessellateSurfaceOpts {
    pub temp_allocator: AllocatorOpts,
    pub result_allocator: AllocatorOpts,
    pub span_subdivision_u: usize,
    pub span_subdivision_v: usize,
    pub skip_mesh_parts: bool,
}

impl ToRaw for TessellateSurfaceOpts {
    type Result = RawTessellateSurfaceOpts;
    #[allow(unused, unused_variables, dead_code)]
    fn to_raw(&self, arena: &mut Arena) -> Self::Result {
        RawTessellateSurfaceOpts {
            _begin_zero: 0,
            temp_allocator: self.temp_allocator.to_raw(arena),
            result_allocator: self.result_allocator.to_raw(arena),
            span_subdivision_u: self.span_subdivision_u,
            span_subdivision_v: self.span_subdivision_v,
            skip_mesh_parts: self.skip_mesh_parts,
            _end_zero: 0,
        }
    }
    #[allow(unused, unused_variables, dead_code)]
    fn to_raw_mut(&mut self, arena: &mut Arena) -> Self::Result {
        RawTessellateSurfaceOpts {
            _begin_zero: 0,
            temp_allocator: self.temp_allocator.to_raw_mut(arena),
            result_allocator: self.result_allocator.to_raw_mut(arena),
            span_subdivision_u: self.span_subdivision_u,
            span_subdivision_v: self.span_subdivision_v,
            skip_mesh_parts: self.skip_mesh_parts,
            _end_zero: 0,
        }
    }
}

#[derive(Default)]
pub struct SubdivideOpts {
    pub temp_allocator: AllocatorOpts,
    pub result_allocator: AllocatorOpts,
    pub boundary: SubdivisionBoundary,
    pub uv_boundary: SubdivisionBoundary,
    pub ignore_normals: bool,
    pub interpolate_normals: bool,
    pub interpolate_tangents: bool,
    pub evaluate_source_vertices: bool,
    pub max_source_vertices: usize,
    pub evaluate_skin_weights: bool,
    pub max_skin_weights: usize,
    pub skin_deformer_index: usize,
}

impl ToRaw for SubdivideOpts {
    type Result = RawSubdivideOpts;
    #[allow(unused, unused_variables, dead_code)]
    fn to_raw(&self, arena: &mut Arena) -> Self::Result {
        RawSubdivideOpts {
            _begin_zero: 0,
            temp_allocator: self.temp_allocator.to_raw(arena),
            result_allocator: self.result_allocator.to_raw(arena),
            boundary: self.boundary,
            uv_boundary: self.uv_boundary,
            ignore_normals: self.ignore_normals,
            interpolate_normals: self.interpolate_normals,
            interpolate_tangents: self.interpolate_tangents,
            evaluate_source_vertices: self.evaluate_source_vertices,
            max_source_vertices: self.max_source_vertices,
            evaluate_skin_weights: self.evaluate_skin_weights,
            max_skin_weights: self.max_skin_weights,
            skin_deformer_index: self.skin_deformer_index,
            _end_zero: 0,
        }
    }
    #[allow(unused, unused_variables, dead_code)]
    fn to_raw_mut(&mut self, arena: &mut Arena) -> Self::Result {
        RawSubdivideOpts {
            _begin_zero: 0,
            temp_allocator: self.temp_allocator.to_raw_mut(arena),
            result_allocator: self.result_allocator.to_raw_mut(arena),
            boundary: self.boundary,
            uv_boundary: self.uv_boundary,
            ignore_normals: self.ignore_normals,
            interpolate_normals: self.interpolate_normals,
            interpolate_tangents: self.interpolate_tangents,
            evaluate_source_vertices: self.evaluate_source_vertices,
            max_source_vertices: self.max_source_vertices,
            evaluate_skin_weights: self.evaluate_skin_weights,
            max_skin_weights: self.max_skin_weights,
            skin_deformer_index: self.skin_deformer_index,
            _end_zero: 0,
        }
    }
}

#[derive(Default)]
pub struct GeometryCacheOpts<'a> {
    pub temp_allocator: AllocatorOpts,
    pub result_allocator: AllocatorOpts,
    pub open_file_cb: OpenFileCb<'a>,
    pub frames_per_second: f64,
    pub mirror_axis: MirrorAxis,
    pub use_scale_factor: bool,
    pub scale_factor: Real,
}

impl<'a> ToRaw for GeometryCacheOpts<'a> {
    type Result = RawGeometryCacheOpts;
    #[allow(unused, unused_variables, dead_code)]
    fn to_raw(&self, arena: &mut Arena) -> Self::Result {
        RawGeometryCacheOpts {
            _begin_zero: 0,
            temp_allocator: self.temp_allocator.to_raw(arena),
            result_allocator: self.result_allocator.to_raw(arena),
            open_file_cb: self.open_file_cb.to_raw(),
            frames_per_second: self.frames_per_second,
            mirror_axis: self.mirror_axis,
            use_scale_factor: self.use_scale_factor,
            scale_factor: self.scale_factor,
            _end_zero: 0,
        }
    }
    #[allow(unused, unused_variables, dead_code)]
    fn to_raw_mut(&mut self, arena: &mut Arena) -> Self::Result {
        RawGeometryCacheOpts {
            _begin_zero: 0,
            temp_allocator: self.temp_allocator.to_raw_mut(arena),
            result_allocator: self.result_allocator.to_raw_mut(arena),
            open_file_cb: self.open_file_cb.to_raw_mut(),
            frames_per_second: self.frames_per_second,
            mirror_axis: self.mirror_axis,
            use_scale_factor: self.use_scale_factor,
            scale_factor: self.scale_factor,
            _end_zero: 0,
        }
    }
}

#[derive(Default)]
pub struct GeometryCacheDataOpts<'a> {
    pub open_file_cb: OpenFileCb<'a>,
    pub additive: bool,
    pub use_weight: bool,
    pub weight: Real,
    pub ignore_transform: bool,
}

impl<'a> ToRaw for GeometryCacheDataOpts<'a> {
    type Result = RawGeometryCacheDataOpts;
    #[allow(unused, unused_variables, dead_code)]
    fn to_raw(&self, arena: &mut Arena) -> Self::Result {
        RawGeometryCacheDataOpts {
            _begin_zero: 0,
            open_file_cb: self.open_file_cb.to_raw(),
            additive: self.additive,
            use_weight: self.use_weight,
            weight: self.weight,
            ignore_transform: self.ignore_transform,
            _end_zero: 0,
        }
    }
    #[allow(unused, unused_variables, dead_code)]
    fn to_raw_mut(&mut self, arena: &mut Arena) -> Self::Result {
        RawGeometryCacheDataOpts {
            _begin_zero: 0,
            open_file_cb: self.open_file_cb.to_raw_mut(),
            additive: self.additive,
            use_weight: self.use_weight,
            weight: self.weight,
            ignore_transform: self.ignore_transform,
            _end_zero: 0,
        }
    }
}

pub type Result<T> = result::Result<T, Error>;

#[allow(unused_imports)]
pub use crate::capi::{
    ufbx_add_blend_shape_vertex_offsets, ufbx_add_blend_vertex_offsets, ufbx_as_anim_curve,
    ufbx_as_anim_layer, ufbx_as_anim_stack, ufbx_as_anim_value, ufbx_as_audio_clip,
    ufbx_as_audio_layer, ufbx_as_blend_channel, ufbx_as_blend_deformer, ufbx_as_blend_shape,
    ufbx_as_bone, ufbx_as_cache_deformer, ufbx_as_cache_file, ufbx_as_camera,
    ufbx_as_camera_switcher, ufbx_as_character, ufbx_as_constraint, ufbx_as_display_layer,
    ufbx_as_empty, ufbx_as_light, ufbx_as_line_curve, ufbx_as_lod_group, ufbx_as_marker,
    ufbx_as_material, ufbx_as_mesh, ufbx_as_metadata_object, ufbx_as_node, ufbx_as_nurbs_curve,
    ufbx_as_nurbs_surface, ufbx_as_nurbs_trim_boundary, ufbx_as_nurbs_trim_surface, ufbx_as_pose,
    ufbx_as_procedural_geometry, ufbx_as_selection_node, ufbx_as_selection_set, ufbx_as_shader,
    ufbx_as_shader_binding, ufbx_as_skin_cluster, ufbx_as_skin_deformer, ufbx_as_stereo_camera,
    ufbx_as_texture, ufbx_as_unknown, ufbx_as_video, ufbx_axes_left_handed_y_up,
    ufbx_axes_left_handed_z_up, ufbx_axes_right_handed_y_up, ufbx_axes_right_handed_z_up,
    ufbx_bake_anim, ufbx_catch_compute_normals, ufbx_catch_compute_topology,
    ufbx_catch_generate_normal_mapping, ufbx_catch_get_skin_vertex_matrix,
    ufbx_catch_get_vertex_real, ufbx_catch_get_vertex_vec2, ufbx_catch_get_vertex_vec3,
    ufbx_catch_get_vertex_vec4, ufbx_catch_get_vertex_w_vec3, ufbx_catch_get_weighted_face_normal,
    ufbx_catch_topo_next_vertex_edge, ufbx_catch_topo_prev_vertex_edge,
    ufbx_catch_triangulate_face, ufbx_compute_normals, ufbx_compute_topology,
    ufbx_coordinate_axes_valid, ufbx_create_anim, ufbx_default_open_file, ufbx_dom_array_size,
    ufbx_dom_as_blob_list, ufbx_dom_as_double_list, ufbx_dom_as_float_list, ufbx_dom_as_int32_list,
    ufbx_dom_as_int64_list, ufbx_dom_as_real_list, ufbx_dom_find, ufbx_dom_find_len,
    ufbx_dom_is_array, ufbx_euler_to_quat, ufbx_evaluate_anim_value_real,
    ufbx_evaluate_anim_value_real_flags, ufbx_evaluate_anim_value_vec3,
    ufbx_evaluate_anim_value_vec3_flags, ufbx_evaluate_baked_quat, ufbx_evaluate_baked_vec3,
    ufbx_evaluate_blend_weight, ufbx_evaluate_blend_weight_flags, ufbx_evaluate_curve,
    ufbx_evaluate_curve_flags, ufbx_evaluate_nurbs_basis, ufbx_evaluate_nurbs_curve,
    ufbx_evaluate_nurbs_surface, ufbx_evaluate_prop, ufbx_evaluate_prop_flags,
    ufbx_evaluate_prop_flags_len, ufbx_evaluate_prop_len, ufbx_evaluate_props,
    ufbx_evaluate_props_flags, ufbx_evaluate_scene, ufbx_evaluate_transform,
    ufbx_evaluate_transform_flags, ufbx_find_anim_prop, ufbx_find_anim_prop_len,
    ufbx_find_anim_props, ufbx_find_anim_stack, ufbx_find_anim_stack_len, ufbx_find_baked_element,
    ufbx_find_baked_element_by_element_id, ufbx_find_baked_node, ufbx_find_baked_node_by_typed_id,
    ufbx_find_blob, ufbx_find_blob_len, ufbx_find_bool, ufbx_find_bool_len, ufbx_find_element,
    ufbx_find_element_len, ufbx_find_face_index, ufbx_find_int, ufbx_find_int_len,
    ufbx_find_material, ufbx_find_material_len, ufbx_find_node, ufbx_find_node_len, ufbx_find_prop,
    ufbx_find_prop_concat, ufbx_find_prop_element, ufbx_find_prop_element_len, ufbx_find_prop_len,
    ufbx_find_prop_texture, ufbx_find_prop_texture_len, ufbx_find_real, ufbx_find_real_len,
    ufbx_find_shader_prop, ufbx_find_shader_prop_bindings, ufbx_find_shader_prop_bindings_len,
    ufbx_find_shader_prop_len, ufbx_find_shader_texture_input, ufbx_find_shader_texture_input_len,
    ufbx_find_string, ufbx_find_string_len, ufbx_find_vec3, ufbx_find_vec3_len, ufbx_format_error,
    ufbx_free_anim, ufbx_free_baked_anim, ufbx_free_geometry_cache, ufbx_free_line_curve,
    ufbx_free_mesh, ufbx_free_scene, ufbx_generate_indices, ufbx_generate_normal_mapping,
    ufbx_get_blend_shape_offset_index, ufbx_get_blend_shape_vertex_offset,
    ufbx_get_blend_vertex_offset, ufbx_get_bone_pose, ufbx_get_compatible_matrix_for_normals,
    ufbx_get_prop_element, ufbx_get_weighted_face_normal, ufbx_identity_matrix, ufbx_identity_quat,
    ufbx_identity_transform, ufbx_inflate, ufbx_is_thread_safe, ufbx_load_file, ufbx_load_file_len,
    ufbx_load_geometry_cache, ufbx_load_geometry_cache_len, ufbx_load_memory, ufbx_load_stdio,
    ufbx_load_stdio_prefix, ufbx_load_stream, ufbx_load_stream_prefix, ufbx_matrix_determinant,
    ufbx_matrix_for_normals, ufbx_matrix_invert, ufbx_matrix_mul, ufbx_matrix_to_transform,
    ufbx_open_file, ufbx_open_file_ctx, ufbx_open_memory, ufbx_open_memory_ctx, ufbx_quat_dot,
    ufbx_quat_fix_antipodal, ufbx_quat_mul, ufbx_quat_normalize, ufbx_quat_rotate_vec3,
    ufbx_quat_slerp, ufbx_quat_to_euler, ufbx_read_geometry_cache_real,
    ufbx_read_geometry_cache_vec3, ufbx_retain_anim, ufbx_retain_baked_anim,
    ufbx_retain_geometry_cache, ufbx_retain_line_curve, ufbx_retain_mesh, ufbx_retain_scene,
    ufbx_sample_geometry_cache_real, ufbx_sample_geometry_cache_vec3, ufbx_source_version,
    ufbx_subdivide_mesh, ufbx_tessellate_nurbs_curve, ufbx_tessellate_nurbs_surface,
    ufbx_thread_pool_get_user_ptr, ufbx_thread_pool_run_task, ufbx_thread_pool_set_user_ptr,
    ufbx_topo_next_vertex_edge, ufbx_topo_prev_vertex_edge, ufbx_transform_direction,
    ufbx_transform_position, ufbx_transform_to_matrix, ufbx_triangulate_face, ufbx_vec3_normalize,
    ufbx_zero_vec2, ufbx_zero_vec3, ufbx_zero_vec4,
};
#[allow(unused_imports)]
pub(crate) use crate::capi::{ufbx_empty_blob, ufbx_empty_string};
pub struct SceneRoot {
    scene: *mut Scene,
    _marker: marker::PhantomData<Scene>,
}

pub struct MeshRoot {
    mesh: *mut Mesh,
    _marker: marker::PhantomData<Mesh>,
}

pub struct LineCurveRoot {
    line_curve: *mut LineCurve,
    _marker: marker::PhantomData<LineCurve>,
}

pub struct GeometryCacheRoot {
    cache: *mut GeometryCache,
    _marker: marker::PhantomData<GeometryCache>,
}

pub struct AnimRoot {
    anim: *mut Anim,
    _marker: marker::PhantomData<Anim>,
}

pub struct BakedAnimRoot {
    anim: *mut BakedAnim,
    _marker: marker::PhantomData<BakedAnim>,
}

impl SceneRoot {
    fn new(scene: *mut Scene) -> SceneRoot {
        SceneRoot {
            scene,
            _marker: marker::PhantomData,
        }
    }
}

impl MeshRoot {
    fn new(mesh: *mut Mesh) -> MeshRoot {
        MeshRoot {
            mesh,
            _marker: marker::PhantomData,
        }
    }
}

impl LineCurveRoot {
    fn new(line_curve: *mut LineCurve) -> LineCurveRoot {
        LineCurveRoot {
            line_curve,
            _marker: marker::PhantomData,
        }
    }
}

impl GeometryCacheRoot {
    fn new(cache: *mut GeometryCache) -> GeometryCacheRoot {
        GeometryCacheRoot {
            cache,
            _marker: marker::PhantomData,
        }
    }
}

impl AnimRoot {
    fn new(anim: *mut Anim) -> AnimRoot {
        AnimRoot {
            anim,
            _marker: marker::PhantomData,
        }
    }
}

impl BakedAnimRoot {
    fn new(anim: *mut BakedAnim) -> BakedAnimRoot {
        BakedAnimRoot {
            anim,
            _marker: marker::PhantomData,
        }
    }
}

impl Drop for SceneRoot {
    fn drop(&mut self) {
        unsafe { crate::native::api::free_scene(self.scene) }
    }
}

impl Drop for MeshRoot {
    fn drop(&mut self) {
        unsafe { crate::native::api::free_mesh(self.mesh) }
    }
}

impl Drop for LineCurveRoot {
    fn drop(&mut self) {
        unsafe { crate::native::api::free_line_curve(self.line_curve) }
    }
}

impl Drop for GeometryCacheRoot {
    fn drop(&mut self) {
        unsafe { crate::native::api::free_geometry_cache(self.cache) }
    }
}

impl Drop for AnimRoot {
    fn drop(&mut self) {
        unsafe { crate::native::api::free_anim(self.anim) }
    }
}

impl Drop for BakedAnimRoot {
    fn drop(&mut self) {
        unsafe { crate::native::api::free_baked_anim(self.anim) }
    }
}

impl Clone for SceneRoot {
    fn clone(&self) -> Self {
        unsafe { crate::native::api::retain_scene(self.scene) }
        SceneRoot::new(self.scene)
    }
}

impl Clone for MeshRoot {
    fn clone(&self) -> Self {
        unsafe { crate::native::api::retain_mesh(self.mesh) }
        MeshRoot::new(self.mesh)
    }
}

impl Clone for LineCurveRoot {
    fn clone(&self) -> Self {
        unsafe { crate::native::api::retain_line_curve(self.line_curve) }
        LineCurveRoot::new(self.line_curve)
    }
}

impl Clone for GeometryCacheRoot {
    fn clone(&self) -> Self {
        unsafe { crate::native::api::retain_geometry_cache(self.cache) }
        GeometryCacheRoot::new(self.cache)
    }
}

impl Clone for AnimRoot {
    fn clone(&self) -> Self {
        unsafe { crate::native::api::retain_anim(self.anim) }
        AnimRoot::new(self.anim)
    }
}

impl Clone for BakedAnimRoot {
    fn clone(&self) -> Self {
        unsafe { crate::native::api::retain_baked_anim(self.anim) }
        BakedAnimRoot::new(self.anim)
    }
}

impl Deref for SceneRoot {
    type Target = Scene;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.scene }
    }
}

impl Deref for MeshRoot {
    type Target = Mesh;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.mesh }
    }
}

impl Deref for LineCurveRoot {
    type Target = LineCurve;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.line_curve }
    }
}

impl Deref for GeometryCacheRoot {
    type Target = GeometryCache;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.cache }
    }
}

impl Deref for AnimRoot {
    type Target = Anim;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.anim }
    }
}

impl Deref for BakedAnimRoot {
    type Target = BakedAnim;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.anim }
    }
}

unsafe impl Send for SceneRoot {}
unsafe impl Sync for SceneRoot {}

unsafe impl Send for MeshRoot {}
unsafe impl Sync for MeshRoot {}

unsafe impl Send for LineCurveRoot {}
unsafe impl Sync for LineCurveRoot {}

unsafe impl Send for GeometryCacheRoot {}
unsafe impl Sync for GeometryCacheRoot {}

unsafe impl Send for AnimRoot {}
unsafe impl Sync for AnimRoot {}

unsafe impl Send for BakedAnimRoot {}
unsafe impl Sync for BakedAnimRoot {}

#[allow(clippy::let_and_return)]
pub fn is_thread_safe() -> bool {
    let result = crate::native::api::is_thread_safe();
    result
}

pub unsafe fn load_memory_raw(data: &[u8], opts: &RawLoadOpts) -> Result<SceneRoot> {
    let result = {
        crate::native::api::load_memory(
            data.as_ptr() as *const c_void,
            data.len(),
            opts as *const RawLoadOpts,
        )
    };
    result.map(SceneRoot::new)
}

pub fn load_memory(data: &[u8], opts: LoadOpts) -> Result<SceneRoot> {
    let mut arena = Arena::new();
    let mut opts_mut = opts;
    let opts_raw = opts_mut.to_raw_mut(&mut arena);
    unsafe { load_memory_raw(data, &opts_raw) }
}

pub unsafe fn load_file_raw(filename: &str, opts: &RawLoadOpts) -> Result<SceneRoot> {
    let result = {
        crate::native::api::load_file_len(
            filename.as_ptr(),
            filename.len(),
            opts as *const RawLoadOpts,
        )
    };
    result.map(SceneRoot::new)
}

pub fn load_file(filename: &str, opts: LoadOpts) -> Result<SceneRoot> {
    let mut arena = Arena::new();
    let mut opts_mut = opts;
    let opts_raw = opts_mut.to_raw_mut(&mut arena);
    unsafe { load_file_raw(filename, &opts_raw) }
}

pub unsafe fn load_stdio_raw(file: *mut c_void, opts: &RawLoadOpts) -> Result<SceneRoot> {
    let result = { crate::native::api::load_stdio(file, opts as *const RawLoadOpts) };
    result.map(SceneRoot::new)
}

pub fn load_stdio(file: *mut c_void, opts: LoadOpts) -> Result<SceneRoot> {
    let mut arena = Arena::new();
    let mut opts_mut = opts;
    let opts_raw = opts_mut.to_raw_mut(&mut arena);
    unsafe { load_stdio_raw(file, &opts_raw) }
}

pub unsafe fn load_stdio_prefix_raw(
    file: *mut c_void,
    prefix: &[u8],
    opts: &RawLoadOpts,
) -> Result<SceneRoot> {
    let result = {
        crate::native::api::load_stdio_prefix(
            file,
            prefix.as_ptr() as *const c_void,
            prefix.len(),
            opts as *const RawLoadOpts,
        )
    };
    result.map(SceneRoot::new)
}

pub fn load_stdio_prefix(file: *mut c_void, prefix: &[u8], opts: LoadOpts) -> Result<SceneRoot> {
    let mut arena = Arena::new();
    let mut opts_mut = opts;
    let opts_raw = opts_mut.to_raw_mut(&mut arena);
    unsafe { load_stdio_prefix_raw(file, prefix, &opts_raw) }
}

pub unsafe fn load_stream_raw(stream: &RawStream, opts: &RawLoadOpts) -> Result<SceneRoot> {
    let result =
        { crate::native::api::load_stream(stream as *const RawStream, opts as *const RawLoadOpts) };
    result.map(SceneRoot::new)
}

pub fn load_stream(stream: Stream, opts: LoadOpts) -> Result<SceneRoot> {
    let mut stream_mut = stream;
    let stream_raw = stream_mut.to_raw_mut();
    let mut arena = Arena::new();
    let mut opts_mut = opts;
    let opts_raw = opts_mut.to_raw_mut(&mut arena);
    unsafe { load_stream_raw(&stream_raw, &opts_raw) }
}

pub unsafe fn load_stream_prefix_raw(
    stream: &RawStream,
    prefix: &[u8],
    opts: &RawLoadOpts,
) -> Result<SceneRoot> {
    let result = {
        crate::native::api::load_stream_prefix(
            stream as *const RawStream,
            prefix.as_ptr() as *const c_void,
            prefix.len(),
            opts as *const RawLoadOpts,
        )
    };
    result.map(SceneRoot::new)
}

pub fn load_stream_prefix(stream: Stream, prefix: &[u8], opts: LoadOpts) -> Result<SceneRoot> {
    let mut stream_mut = stream;
    let stream_raw = stream_mut.to_raw_mut();
    let mut arena = Arena::new();
    let mut opts_mut = opts;
    let opts_raw = opts_mut.to_raw_mut(&mut arena);
    unsafe { load_stream_prefix_raw(&stream_raw, prefix, &opts_raw) }
}

#[allow(clippy::let_and_return)]
pub fn format_error(dst: &mut [u8], error: &Error) -> usize {
    let result = unsafe {
        crate::native::api::format_error(dst.as_mut_ptr(), dst.len(), error as *const Error)
    };
    result
}

#[allow(clippy::needless_lifetimes)]
pub fn find_prop<'a>(props: &'a Props, name: &str) -> Option<&'a Prop> {
    let result = crate::native::api::find_prop_len(
        crate::native::view::View::<Props, crate::native::view::Const>::from_ref(props),
        name.as_bytes(),
    );
    result.map(|prop| unsafe { &*prop.as_ptr() })
}

// TODO: Property find functions

// TODO: Property find functions

// TODO: Property find functions

// TODO: Property find functions

// TODO: Property find functions

pub fn find_blob(props: &Props, name: &str, def: Blob) -> Blob {
    crate::native::api::find_blob_len(
        crate::native::view::View::<Props, crate::native::view::Const>::from_ref(props),
        name.as_bytes(),
        def,
    )
}

// TODO: ufbx_find_prop_concat()

#[allow(clippy::needless_lifetimes)]
pub fn get_prop_element<'a>(
    element: &'a Element,
    prop: &Prop,
    type_: ElementType,
) -> Option<&'a Element> {
    let result = unsafe {
        crate::native::api::get_prop_element(element as *const Element, prop as *const Prop, type_)
    };
    if result.is_null() {
        None
    } else {
        unsafe { Some(&*result) }
    }
}

#[allow(clippy::needless_lifetimes)]
pub fn find_prop_element<'a>(
    element: &'a Element,
    name: &str,
    type_: ElementType,
) -> Option<&'a Element> {
    let result = crate::native::api::find_prop_element_len(
        crate::native::view::View::<Element, crate::native::view::Const>::from_ref(element),
        name.as_bytes(),
        type_,
    );
    if result.is_null() {
        None
    } else {
        unsafe { Some(&*result) }
    }
}

#[allow(clippy::needless_lifetimes)]
pub fn find_element<'a>(scene: &'a Scene, type_: ElementType, name: &str) -> Option<&'a Element> {
    let result = crate::native::api::find_element_len(
        Some(crate::native::view::View::<Scene, crate::native::view::Const>::from_ref(scene)),
        type_,
        name.as_bytes(),
    );
    if result.is_null() {
        None
    } else {
        unsafe { Some(&*result) }
    }
}

#[allow(clippy::needless_lifetimes)]
pub fn find_node<'a>(scene: &'a Scene, name: &str) -> Option<&'a Node> {
    let result = crate::native::api::find_node_len(
        Some(crate::native::view::View::<Scene, crate::native::view::Const>::from_ref(scene)),
        name.as_bytes(),
    );
    if result.is_null() {
        None
    } else {
        unsafe { Some(&*result) }
    }
}

#[allow(clippy::needless_lifetimes)]
pub fn find_anim_stack<'a>(scene: &'a Scene, name: &str) -> Option<&'a AnimStack> {
    let result = crate::native::api::find_anim_stack_len(
        Some(crate::native::view::View::<Scene, crate::native::view::Const>::from_ref(scene)),
        name.as_bytes(),
    );
    if result.is_null() {
        None
    } else {
        unsafe { Some(&*result) }
    }
}

#[allow(clippy::needless_lifetimes)]
pub fn find_material<'a>(scene: &'a Scene, name: &str) -> Option<&'a Material> {
    let result = crate::native::api::find_material_len(
        Some(crate::native::view::View::<Scene, crate::native::view::Const>::from_ref(scene)),
        name.as_bytes(),
    );
    if result.is_null() {
        None
    } else {
        unsafe { Some(&*result) }
    }
}

#[allow(clippy::needless_lifetimes)]
pub fn find_anim_prop<'a>(
    layer: &'a AnimLayer,
    element: &Element,
    prop: &str,
) -> Option<&'a AnimProp> {
    let result = crate::native::api::find_anim_prop_len(
        Some(crate::native::view::View::<
            AnimLayer,
            crate::native::view::Const,
        >::from_ref(layer)),
        element as *const Element,
        prop.as_bytes(),
    );
    result.map(|prop| unsafe { &*prop.as_ptr() })
}

#[allow(clippy::needless_lifetimes)]
pub fn find_anim_props<'a>(layer: &'a AnimLayer, element: &Element) -> &'a [AnimProp] {
    let result = crate::native::api::find_anim_props(
        Some(crate::native::view::View::<
            AnimLayer,
            crate::native::view::Const,
        >::from_ref(layer)),
        element as *const Element,
    );
    unsafe { result.as_static_ref() }
}

pub fn get_compatible_matrix_for_normals(node: &Node) -> Matrix {
    let node = crate::native::view::View::<Node, crate::native::view::Const>::from_ref(node);
    crate::native::api::get_compatible_matrix_for_normals(Some(node))
}

#[allow(clippy::let_and_return)]
pub fn inflate(dst: &mut [u8], input: &InflateInput, retain: &mut InflateRetain) -> isize {
    let result = unsafe {
        crate::native::deflate::inflate(
            dst.as_mut_ptr() as *mut c_void,
            dst.len(),
            input as *const InflateInput,
            retain as *mut InflateRetain,
        )
    };
    result
}

#[allow(clippy::let_and_return)]
pub unsafe fn default_open_file_raw(
    user: *mut c_void,
    stream: &mut RawStream,
    path: &str,
    info: &OpenFileInfo,
) -> bool {
    let result = {
        ufbx_default_open_file(
            user,
            stream as *mut RawStream,
            path.as_ptr(),
            path.len(),
            info as *const OpenFileInfo,
        )
    };
    result
}

pub unsafe fn open_file_raw(
    stream: &mut RawStream,
    path: &str,
    opts: &RawOpenFileOpts,
) -> Result<bool> {
    let result = {
        crate::native::api::open_file(
            stream as *mut RawStream,
            path.as_ptr(),
            path.len(),
            opts as *const RawOpenFileOpts,
        )
    };
    result.map(|()| true)
}

pub unsafe fn open_file_ctx_raw(
    stream: &mut RawStream,
    ctx: OpenFileContext,
    path: &str,
    opts: &RawOpenFileOpts,
) -> Result<bool> {
    let result = {
        crate::native::api::open_file_ctx(
            stream as *mut RawStream,
            ctx,
            path.as_ptr(),
            path.len(),
            opts as *const RawOpenFileOpts,
        )
    };
    result.map(|()| true)
}

pub unsafe fn open_memory_raw(
    stream: &mut RawStream,
    data: &[u8],
    opts: &RawOpenMemoryOpts,
) -> Result<bool> {
    let result = {
        crate::native::api::open_memory(
            stream as *mut RawStream,
            data.as_ptr() as *const c_void,
            data.len(),
            opts as *const RawOpenMemoryOpts,
        )
    };
    result.map(|()| true)
}

pub unsafe fn open_memory_ctx_raw(
    stream: &mut RawStream,
    ctx: OpenFileContext,
    data: &[u8],
    opts: &RawOpenMemoryOpts,
) -> Result<bool> {
    let result = {
        crate::native::api::open_memory_ctx(
            stream as *mut RawStream,
            ctx,
            data.as_ptr() as *const c_void,
            data.len(),
            opts as *const RawOpenMemoryOpts,
        )
    };
    result.map(|()| true)
}

pub fn evaluate_curve(curve: &AnimCurve, time: f64, default_value: Real) -> Real {
    crate::native::api::evaluate_curve(
        Some(crate::native::view::View::<
            AnimCurve,
            crate::native::view::Const,
        >::from_ref(curve)),
        time,
        default_value,
    )
}

pub fn evaluate_curve_flags(curve: &AnimCurve, time: f64, default_value: Real, flags: u32) -> Real {
    crate::native::api::evaluate_curve_flags(
        Some(crate::native::view::View::<
            AnimCurve,
            crate::native::view::Const,
        >::from_ref(curve)),
        time,
        default_value,
        flags,
    )
}

pub fn evaluate_anim_value_real(anim_value: &AnimValue, time: f64) -> Real {
    crate::native::api::evaluate_anim_value_real(
        Some(crate::native::view::View::<
            AnimValue,
            crate::native::view::Const,
        >::from_ref(anim_value)),
        time,
    )
}

pub fn evaluate_anim_value_vec3(anim_value: &AnimValue, time: f64) -> Vec3 {
    crate::native::api::evaluate_anim_value_vec3(
        Some(crate::native::view::View::<
            AnimValue,
            crate::native::view::Const,
        >::from_ref(anim_value)),
        time,
    )
}

pub fn evaluate_anim_value_real_flags(anim_value: &AnimValue, time: f64, flags: u32) -> Real {
    crate::native::api::evaluate_anim_value_real_flags(
        Some(crate::native::view::View::<
            AnimValue,
            crate::native::view::Const,
        >::from_ref(anim_value)),
        time,
        flags,
    )
}

pub fn evaluate_anim_value_vec3_flags(anim_value: &AnimValue, time: f64, flags: u32) -> Vec3 {
    crate::native::api::evaluate_anim_value_vec3_flags(
        Some(crate::native::view::View::<
            AnimValue,
            crate::native::view::Const,
        >::from_ref(anim_value)),
        time,
        flags,
    )
}

pub fn evaluate_prop<'a, 'b>(
    anim: &'a Anim,
    element: &'a Element,
    name: &'b str,
    time: f64,
) -> ExternalRef<'b, Prop>
where
    'a: 'b,
{
    let result = unsafe {
        ufbx_evaluate_prop_len(
            anim as *const Anim,
            element as *const Element,
            name.as_ptr(),
            name.len(),
            time,
        )
    };
    unsafe { ExternalRef::new(result) }
}

#[allow(clippy::let_and_return)]
pub fn evaluate_prop_flags(
    anim: &Anim,
    element: &Element,
    name: &str,
    time: f64,
    flags: u32,
) -> Prop {
    let result = unsafe {
        crate::native::api::evaluate_prop_flags_len(
            anim as *const Anim,
            element as *const Element,
            name.as_ptr(),
            name.len(),
            time,
            flags,
        )
    };
    result
}

pub fn evaluate_props<'a, 'b>(
    anim: &'a Anim,
    element: &'a Element,
    time: f64,
    buffer: &'b mut [ExternalRef<'b, Prop>],
) -> ExternalRef<'b, Props>
where
    'a: 'b,
{
    let result = unsafe {
        ufbx_evaluate_props(
            anim as *const Anim,
            element as *const Element,
            time,
            buffer.as_mut_ptr().cast::<Prop>(),
            buffer.len(),
        )
    };
    unsafe { ExternalRef::new(result) }
}

pub fn evaluate_props_flags<'a, 'b>(
    anim: &'a Anim,
    element: &'a Element,
    time: f64,
    buffer: &'b mut [ExternalRef<'b, Prop>],
    flags: u32,
) -> ExternalRef<'b, Props>
where
    'a: 'b,
{
    let result = unsafe {
        ufbx_evaluate_props_flags(
            anim as *const Anim,
            element as *const Element,
            time,
            buffer.as_mut_ptr().cast::<Prop>(),
            buffer.len(),
            flags,
        )
    };
    unsafe { ExternalRef::new(result) }
}

#[allow(clippy::let_and_return)]
pub fn evaluate_transform(anim: &Anim, node: &Node, time: f64) -> Transform {
    let result = unsafe {
        crate::native::api::evaluate_transform(anim as *const Anim, node as *const Node, time)
    };
    result
}

#[allow(clippy::let_and_return)]
pub fn evaluate_transform_flags(anim: &Anim, node: &Node, time: f64, flags: u32) -> Transform {
    let result = unsafe {
        crate::native::api::evaluate_transform_flags(
            anim as *const Anim,
            node as *const Node,
            time,
            flags,
        )
    };
    result
}

#[allow(clippy::let_and_return)]
pub fn evaluate_blend_weight(anim: &Anim, channel: &BlendChannel, time: f64) -> Real {
    let result = crate::native::api::evaluate_blend_weight(
        crate::native::view::View::<Anim, crate::native::view::Const>::from_ref(anim),
        crate::native::view::View::<BlendChannel, crate::native::view::Const>::from_ref(channel),
        time,
    );
    result
}

#[allow(clippy::let_and_return)]
pub fn evaluate_blend_weight_flags(
    anim: &Anim,
    channel: &BlendChannel,
    time: f64,
    flags: u32,
) -> Real {
    let result = crate::native::api::evaluate_blend_weight_flags(
        crate::native::view::View::<Anim, crate::native::view::Const>::from_ref(anim),
        crate::native::view::View::<BlendChannel, crate::native::view::Const>::from_ref(channel),
        time,
        flags,
    );
    result
}

pub unsafe fn evaluate_scene_raw(
    scene: &Scene,
    anim: &Anim,
    time: f64,
    opts: &RawEvaluateOpts,
) -> Result<SceneRoot> {
    let result = {
        crate::native::api::evaluate_scene(
            scene as *const Scene,
            anim as *const Anim,
            time,
            opts as *const RawEvaluateOpts,
        )
    };
    result.map(SceneRoot::new)
}

pub fn evaluate_scene(
    scene: &Scene,
    anim: &Anim,
    time: f64,
    opts: EvaluateOpts,
) -> Result<SceneRoot> {
    let mut arena = Arena::new();
    let mut opts_mut = opts;
    let opts_raw = opts_mut.to_raw_mut(&mut arena);
    unsafe { evaluate_scene_raw(scene, anim, time, &opts_raw) }
}

pub unsafe fn create_anim_raw(scene: &Scene, opts: &RawAnimOpts) -> Result<AnimRoot> {
    let result =
        { crate::native::api::create_anim(scene as *const Scene, opts as *const RawAnimOpts) };
    result.map(AnimRoot::new)
}

pub fn create_anim(scene: &Scene, opts: AnimOpts) -> Result<AnimRoot> {
    let mut arena = Arena::new();
    let mut opts_mut = opts;
    let opts_raw = opts_mut.to_raw_mut(&mut arena);
    unsafe { create_anim_raw(scene, &opts_raw) }
}

pub unsafe fn bake_anim_raw(
    scene: &Scene,
    anim: &Anim,
    opts: &RawBakeOpts,
) -> Result<BakedAnimRoot> {
    let result = {
        crate::native::api::bake_anim(
            scene as *const Scene,
            anim as *const Anim,
            opts as *const RawBakeOpts,
        )
    };
    result.map(BakedAnimRoot::new)
}

pub fn bake_anim(scene: &Scene, anim: &Anim, opts: BakeOpts) -> Result<BakedAnimRoot> {
    let mut arena = Arena::new();
    let mut opts_mut = opts;
    let opts_raw = opts_mut.to_raw_mut(&mut arena);
    unsafe { bake_anim_raw(scene, anim, &opts_raw) }
}

pub fn find_baked_node_by_typed_id(bake: &mut BakedAnim, typed_id: u32) -> Option<&BakedNode> {
    let result = crate::native::api::find_baked_node_by_typed_id(
        crate::native::view::View::<BakedAnim, crate::native::view::Const>::from_ref(bake),
        typed_id,
    );
    result.map(|node| unsafe { &*node.as_ptr() })
}

pub fn find_baked_node<'a>(bake: &'a mut BakedAnim, node: &mut Node) -> Option<&'a BakedNode> {
    let result = crate::native::api::find_baked_node(
        Some(crate::native::view::View::<
            BakedAnim,
            crate::native::view::Const,
        >::from_ref(bake)),
        Some(crate::native::view::View::<Node, crate::native::view::Const>::from_ref(node)),
    );
    result.map(|node| unsafe { &*node.as_ptr() })
}

pub fn find_baked_element_by_element_id(
    bake: &mut BakedAnim,
    element_id: u32,
) -> Option<&BakedElement> {
    let result = crate::native::api::find_baked_element_by_element_id(
        crate::native::view::View::<BakedAnim, crate::native::view::Const>::from_ref(bake),
        element_id,
    );
    result.map(|elem| unsafe { &*elem.as_ptr() })
}

pub fn find_baked_element<'a>(
    bake: &'a mut BakedAnim,
    element: &mut Element,
) -> Option<&'a BakedElement> {
    let result = crate::native::api::find_baked_element(
        Some(crate::native::view::View::<
            BakedAnim,
            crate::native::view::Const,
        >::from_ref(bake)),
        Some(crate::native::view::View::<
            Element,
            crate::native::view::Const,
        >::from_ref(element)),
    );
    result.map(|elem| unsafe { &*elem.as_ptr() })
}

#[allow(clippy::let_and_return)]
pub fn evaluate_baked_vec3(keyframes: &[BakedVec3], time: f64) -> Vec3 {
    let result =
        unsafe { crate::native::api::evaluate_baked_vec3(List::from_slice(keyframes), time) };
    result
}

#[allow(clippy::let_and_return)]
pub fn evaluate_baked_quat(keyframes: &[BakedQuat], time: f64) -> Quat {
    let result =
        unsafe { crate::native::api::evaluate_baked_quat(List::from_slice(keyframes), time) };
    result
}

#[allow(clippy::needless_lifetimes)]
pub fn get_bone_pose<'a>(pose: &'a Pose, node: &Node) -> Option<&'a BonePose> {
    let result = crate::native::api::get_bone_pose_entry(
        Some(crate::native::view::View::<Pose, crate::native::view::Const>::from_ref(pose)),
        Some(crate::native::view::View::<Node, crate::native::view::Const>::from_ref(node)),
    );
    result.map(|bone_pose| unsafe { &*bone_pose.as_ptr() })
}

#[allow(clippy::needless_lifetimes)]
pub fn find_prop_texture<'a>(material: &'a Material, name: &str) -> Option<&'a Texture> {
    let result = unsafe {
        crate::native::api::find_prop_texture_len(material as *const Material, name.as_bytes())
    };
    if result.is_null() {
        None
    } else {
        unsafe { Some(&*result) }
    }
}

pub fn find_shader_prop<'a>(shader: &'a Shader, name: &str) -> &'a str {
    let result = crate::native::api::find_shader_prop_len(
        Some(crate::native::view::View::<
            Shader,
            crate::native::view::Const,
        >::from_ref(shader)),
        name.as_bytes(),
    );
    unsafe { result.as_static_ref() }
}

#[allow(clippy::needless_lifetimes)]
pub fn find_shader_prop_bindings<'a>(shader: &'a Shader, name: &str) -> &'a [ShaderPropBinding] {
    let result = crate::native::api::find_shader_prop_bindings_len(
        Some(crate::native::view::View::<
            Shader,
            crate::native::view::Const,
        >::from_ref(shader)),
        name.as_bytes(),
    );
    unsafe { result.as_static_ref() }
}

pub fn find_shader_texture_input<'a>(
    shader: &'a ShaderTexture,
    name: &str,
) -> Option<&'a ShaderTextureInput> {
    let result = crate::native::api::find_shader_texture_input_len(
        crate::native::view::View::<ShaderTexture, crate::native::view::Const>::from_ref(shader),
        name.as_bytes(),
    );
    result.map(|input| unsafe { &*input.as_ptr() })
}

#[allow(clippy::let_and_return)]
pub fn coordinate_axes_valid(axes: CoordinateAxes) -> bool {
    let result = crate::native::api::coordinate_axes_valid(axes);
    result
}

#[allow(clippy::let_and_return)]
pub fn vec3_normalize(v: Vec3) -> Vec3 {
    let result = crate::native::api::vec3_normalize(v);
    result
}

#[allow(clippy::let_and_return)]
pub fn quat_dot(a: Quat, b: Quat) -> Real {
    let result = crate::native::api::quat_dot(a, b);
    result
}

#[allow(clippy::let_and_return)]
pub fn quat_mul(a: Quat, b: Quat) -> Quat {
    let result = crate::native::api::quat_mul(a, b);
    result
}

#[allow(clippy::let_and_return)]
pub fn quat_normalize(q: Quat) -> Quat {
    let result = crate::native::api::quat_normalize(q);
    result
}

#[allow(clippy::let_and_return)]
pub fn quat_fix_antipodal(q: Quat, reference: Quat) -> Quat {
    let result = crate::native::api::quat_fix_antipodal(q, reference);
    result
}

#[allow(clippy::let_and_return)]
pub fn quat_slerp(a: Quat, b: Quat, t: Real) -> Quat {
    let result = crate::native::api::quat_slerp(a, b, t);
    result
}

#[allow(clippy::let_and_return)]
pub fn quat_rotate_vec3(q: Quat, v: Vec3) -> Vec3 {
    let result = crate::native::api::quat_rotate_vec3(q, v);
    result
}

#[allow(clippy::let_and_return)]
pub fn quat_to_euler(q: Quat, order: RotationOrder) -> Vec3 {
    let result = crate::native::api::quat_to_euler(q, order);
    result
}

#[allow(clippy::let_and_return)]
pub fn euler_to_quat(v: Vec3, order: RotationOrder) -> Quat {
    let result = crate::native::api::euler_to_quat(v, order);
    result
}

#[allow(clippy::let_and_return)]
pub fn matrix_mul(a: &Matrix, b: &Matrix) -> Matrix {
    let result = unsafe { crate::native::api::matrix_mul(a as *const Matrix, b as *const Matrix) };
    result
}

#[allow(clippy::let_and_return)]
pub fn matrix_determinant(m: &Matrix) -> Real {
    let result = unsafe { crate::native::api::matrix_determinant(m as *const Matrix) };
    result
}

#[allow(clippy::let_and_return)]
pub fn matrix_invert(m: &Matrix) -> Matrix {
    let result = unsafe { crate::native::api::matrix_invert(m as *const Matrix) };
    result
}

#[allow(clippy::let_and_return)]
pub fn matrix_for_normals(m: &Matrix) -> Matrix {
    let result = unsafe { crate::native::api::matrix_for_normals(m as *const Matrix) };
    result
}

#[allow(clippy::let_and_return)]
pub fn transform_position(m: &Matrix, v: Vec3) -> Vec3 {
    let result = unsafe { crate::native::api::transform_position(m as *const Matrix, v) };
    result
}

#[allow(clippy::let_and_return)]
pub fn transform_direction(m: &Matrix, v: Vec3) -> Vec3 {
    let result = unsafe { crate::native::api::transform_direction(m as *const Matrix, v) };
    result
}

#[allow(clippy::let_and_return)]
pub fn transform_to_matrix(t: &Transform) -> Matrix {
    let result = unsafe { crate::native::api::transform_to_matrix(t as *const Transform) };
    result
}

#[allow(clippy::let_and_return)]
pub fn matrix_to_transform(m: &Matrix) -> Transform {
    let result = unsafe { crate::native::api::matrix_to_transform(m as *const Matrix) };
    result
}

pub fn get_skin_vertex_matrix(skin: &SkinDeformer, vertex: usize, fallback: &Matrix) -> Matrix {
    let mut panic: Panic = Default::default();
    let result = unsafe {
        crate::native::api::catch_get_skin_vertex_matrix(
            Some(&mut panic),
            crate::native::view::View::<SkinDeformer, crate::native::view::Const>::from_ref(skin),
            vertex,
            fallback as *const Matrix,
        )
    };
    if panic.did_panic {
        panic!("ufbx::get_skin_vertex_matrix() {}", panic.message());
    }
    result
}

pub fn get_blend_shape_offset_index(shape: &BlendShape, vertex: usize) -> u32 {
    let shape =
        crate::native::view::View::<BlendShape, crate::native::view::Const>::from_ref(shape);
    crate::native::api::get_blend_shape_offset_index(Some(shape), vertex)
}

#[allow(clippy::let_and_return)]
pub fn get_blend_shape_vertex_offset(shape: &BlendShape, vertex: usize) -> Vec3 {
    let result = unsafe {
        crate::native::api::get_blend_shape_vertex_offset(shape as *const BlendShape, vertex)
    };
    result
}

#[allow(clippy::let_and_return)]
pub fn get_blend_vertex_offset(blend: &BlendDeformer, vertex: usize) -> Vec3 {
    let result = unsafe {
        crate::native::api::get_blend_vertex_offset(blend as *const BlendDeformer, vertex)
    };
    result
}

pub fn add_blend_shape_vertex_offsets(shape: &BlendShape, vertices: &mut [Vec3], weight: Real) {
    unsafe {
        crate::native::api::add_blend_shape_vertex_offsets(
            shape as *const BlendShape,
            vertices.as_mut_ptr(),
            vertices.len(),
            weight,
        )
    };
}

pub fn add_blend_vertex_offsets(blend: &BlendDeformer, vertices: &mut [Vec3], weight: Real) {
    unsafe {
        crate::native::api::add_blend_vertex_offsets(
            blend as *const BlendDeformer,
            vertices.as_mut_ptr(),
            vertices.len(),
            weight,
        )
    };
}

#[allow(clippy::let_and_return)]
pub fn evaluate_nurbs_basis(
    basis: &NurbsBasis,
    u: Real,
    weights: &mut [Real],
    derivatives: &mut [Real],
) -> usize {
    let result = unsafe {
        crate::native::api::evaluate_nurbs_basis(
            basis as *const NurbsBasis,
            u,
            weights.as_mut_ptr(),
            weights.len(),
            derivatives.as_mut_ptr(),
            derivatives.len(),
        )
    };
    result
}

#[allow(clippy::let_and_return)]
pub fn evaluate_nurbs_curve(curve: &NurbsCurve, u: Real) -> CurvePoint {
    let result = unsafe { crate::native::api::evaluate_nurbs_curve(curve as *const NurbsCurve, u) };
    result
}

#[allow(clippy::let_and_return)]
pub fn evaluate_nurbs_surface(surface: &NurbsSurface, u: Real, v: Real) -> SurfacePoint {
    let result =
        unsafe { crate::native::api::evaluate_nurbs_surface(surface as *const NurbsSurface, u, v) };
    result
}

pub unsafe fn tessellate_nurbs_curve_raw(
    curve: &NurbsCurve,
    opts: &RawTessellateCurveOpts,
) -> Result<LineCurveRoot> {
    let result = {
        crate::native::api::tessellate_nurbs_curve(
            curve as *const NurbsCurve,
            opts as *const RawTessellateCurveOpts,
        )
    };
    result.map(LineCurveRoot::new)
}

pub fn tessellate_nurbs_curve(
    curve: &NurbsCurve,
    opts: TessellateCurveOpts,
) -> Result<LineCurveRoot> {
    let mut arena = Arena::new();
    let mut opts_mut = opts;
    let opts_raw = opts_mut.to_raw_mut(&mut arena);
    unsafe { tessellate_nurbs_curve_raw(curve, &opts_raw) }
}

pub unsafe fn tessellate_nurbs_surface_raw(
    surface: &NurbsSurface,
    opts: &RawTessellateSurfaceOpts,
) -> Result<MeshRoot> {
    let result = {
        crate::native::api::tessellate_nurbs_surface(
            surface as *const NurbsSurface,
            opts as *const RawTessellateSurfaceOpts,
        )
    };
    result.map(MeshRoot::new)
}

pub fn tessellate_nurbs_surface(
    surface: &NurbsSurface,
    opts: TessellateSurfaceOpts,
) -> Result<MeshRoot> {
    let mut arena = Arena::new();
    let mut opts_mut = opts;
    let opts_raw = opts_mut.to_raw_mut(&mut arena);
    unsafe { tessellate_nurbs_surface_raw(surface, &opts_raw) }
}

#[allow(clippy::let_and_return)]
pub fn find_face_index(mesh: &mut Mesh, index: usize) -> u32 {
    let result = unsafe { crate::native::api::find_face_index(mesh as *mut Mesh, index) };
    result
}

pub fn triangulate_face(indices: &mut [u32], mesh: &Mesh, face: Face) -> u32 {
    let mut panic: Panic = Default::default();
    let result = unsafe {
        crate::native::api::catch_triangulate_face(
            Some(&mut panic),
            indices.as_mut_ptr(),
            indices.len(),
            crate::native::view::View::<Mesh, crate::native::view::Const>::from_ref(mesh),
            face,
        )
    };
    if panic.did_panic {
        panic!("ufbx::triangulate_face() {}", panic.message());
    }
    result
}

pub fn compute_topology(mesh: &Mesh, topo: &mut [TopoEdge]) {
    let mut panic: Panic = Default::default();
    unsafe {
        crate::native::api::catch_compute_topology(
            Some(&mut panic),
            crate::native::view::View::<Mesh, crate::native::view::Const>::from_ref(mesh),
            topo.as_mut_ptr(),
            topo.len(),
        )
    };
    if panic.did_panic {
        panic!("ufbx::compute_topology() {}", panic.message());
    }
}

pub fn topo_next_vertex_edge(topo: &[TopoEdge], index: u32) -> u32 {
    let mut panic: Panic = Default::default();
    let result = unsafe {
        crate::native::api::catch_topo_next_vertex_edge(
            Some(&mut panic),
            topo.as_ptr(),
            topo.len(),
            index,
        )
    };
    if panic.did_panic {
        panic!("ufbx::topo_next_vertex_edge() {}", panic.message());
    }
    result
}

pub fn topo_prev_vertex_edge(topo: &[TopoEdge], index: u32) -> u32 {
    let mut panic: Panic = Default::default();
    let result = unsafe {
        crate::native::api::catch_topo_prev_vertex_edge(
            Some(&mut panic),
            topo.as_ptr(),
            topo.len(),
            index,
        )
    };
    if panic.did_panic {
        panic!("ufbx::topo_prev_vertex_edge() {}", panic.message());
    }
    result
}

pub fn get_weighted_face_normal(positions: &VertexVec3, face: Face) -> Vec3 {
    let mut panic: Panic = Default::default();
    let result = crate::native::api::catch_get_weighted_face_normal(
        Some(&mut panic),
        crate::native::view::View::<VertexVec3, crate::native::view::Const>::from_ref(positions),
        face,
    );
    if panic.did_panic {
        panic!("ufbx::get_weighted_face_normal() {}", panic.message());
    }
    result
}

pub fn generate_normal_mapping(
    mesh: &Mesh,
    topo: &[TopoEdge],
    normal_indices: &mut [u32],
    assume_smooth: bool,
) -> usize {
    let mut panic: Panic = Default::default();
    let result = unsafe {
        crate::native::api::catch_generate_normal_mapping(
            Some(&mut panic),
            crate::native::view::View::<Mesh, crate::native::view::Const>::from_ref(mesh),
            topo.as_ptr(),
            topo.len(),
            normal_indices.as_mut_ptr(),
            normal_indices.len(),
            assume_smooth,
        )
    };
    if panic.did_panic {
        panic!("ufbx::generate_normal_mapping() {}", panic.message());
    }
    result
}

pub fn compute_normals(
    mesh: &Mesh,
    positions: &VertexVec3,
    normal_indices: &[u32],
    normals: &mut [Vec3],
) {
    let mut panic: Panic = Default::default();
    unsafe {
        crate::native::api::catch_compute_normals(
            Some(&mut panic),
            crate::native::view::View::<Mesh, crate::native::view::Const>::from_ref(mesh),
            crate::native::view::View::<VertexVec3, crate::native::view::Const>::from_ref(
                positions,
            ),
            normal_indices.as_ptr(),
            normal_indices.len(),
            normals.as_mut_ptr(),
            normals.len(),
        )
    };
    if panic.did_panic {
        panic!("ufbx::compute_normals() {}", panic.message());
    }
}

pub unsafe fn subdivide_mesh_raw(
    mesh: &Mesh,
    level: usize,
    opts: &RawSubdivideOpts,
) -> Result<MeshRoot> {
    let result = {
        crate::native::api::subdivide_mesh(
            mesh as *const Mesh,
            level,
            opts as *const RawSubdivideOpts,
        )
    };
    result.map(MeshRoot::new)
}

pub fn subdivide_mesh(mesh: &Mesh, level: usize, opts: SubdivideOpts) -> Result<MeshRoot> {
    let mut arena = Arena::new();
    let mut opts_mut = opts;
    let opts_raw = opts_mut.to_raw_mut(&mut arena);
    unsafe { subdivide_mesh_raw(mesh, level, &opts_raw) }
}

pub unsafe fn load_geometry_cache_raw(
    filename: &str,
    opts: &RawGeometryCacheOpts,
) -> Result<GeometryCacheRoot> {
    let result = {
        crate::native::api::load_geometry_cache_len(
            filename.as_ptr(),
            filename.len(),
            opts as *const RawGeometryCacheOpts,
        )
    };
    result.map(GeometryCacheRoot::new)
}

pub fn load_geometry_cache(filename: &str, opts: GeometryCacheOpts) -> Result<GeometryCacheRoot> {
    let mut arena = Arena::new();
    let mut opts_mut = opts;
    let opts_raw = opts_mut.to_raw_mut(&mut arena);
    unsafe { load_geometry_cache_raw(filename, &opts_raw) }
}

#[allow(clippy::let_and_return)]
pub unsafe fn read_geometry_cache_real_raw(
    frame: &CacheFrame,
    data: &mut [Real],
    opts: &RawGeometryCacheDataOpts,
) -> usize {
    let result = {
        crate::native::api::read_geometry_cache_real(
            frame as *const CacheFrame,
            data.as_mut_ptr(),
            data.len(),
            opts as *const RawGeometryCacheDataOpts,
        )
    };
    result
}

pub fn read_geometry_cache_real(
    frame: &CacheFrame,
    data: &mut [Real],
    opts: GeometryCacheDataOpts,
) -> usize {
    let mut arena = Arena::new();
    let mut opts_mut = opts;
    let opts_raw = opts_mut.to_raw_mut(&mut arena);
    unsafe { read_geometry_cache_real_raw(frame, data, &opts_raw) }
}

#[allow(clippy::let_and_return)]
pub unsafe fn read_geometry_cache_vec3_raw(
    frame: &CacheFrame,
    data: &mut [Vec3],
    opts: &RawGeometryCacheDataOpts,
) -> usize {
    let result = {
        crate::native::api::read_geometry_cache_vec3(
            frame as *const CacheFrame,
            data.as_mut_ptr(),
            data.len(),
            opts as *const RawGeometryCacheDataOpts,
        )
    };
    result
}

pub fn read_geometry_cache_vec3(
    frame: &CacheFrame,
    data: &mut [Vec3],
    opts: GeometryCacheDataOpts,
) -> usize {
    let mut arena = Arena::new();
    let mut opts_mut = opts;
    let opts_raw = opts_mut.to_raw_mut(&mut arena);
    unsafe { read_geometry_cache_vec3_raw(frame, data, &opts_raw) }
}

#[allow(clippy::let_and_return)]
pub unsafe fn sample_geometry_cache_real_raw(
    channel: &CacheChannel,
    time: f64,
    data: &mut [Real],
    opts: &RawGeometryCacheDataOpts,
) -> usize {
    let result = {
        crate::native::api::sample_geometry_cache_real(
            channel as *const CacheChannel,
            time,
            data.as_mut_ptr(),
            data.len(),
            opts as *const RawGeometryCacheDataOpts,
        )
    };
    result
}

pub fn sample_geometry_cache_real(
    channel: &CacheChannel,
    time: f64,
    data: &mut [Real],
    opts: GeometryCacheDataOpts,
) -> usize {
    let mut arena = Arena::new();
    let mut opts_mut = opts;
    let opts_raw = opts_mut.to_raw_mut(&mut arena);
    unsafe { sample_geometry_cache_real_raw(channel, time, data, &opts_raw) }
}

#[allow(clippy::let_and_return)]
pub unsafe fn sample_geometry_cache_vec3_raw(
    channel: &CacheChannel,
    time: f64,
    data: &mut [Vec3],
    opts: &RawGeometryCacheDataOpts,
) -> usize {
    let result = {
        crate::native::api::sample_geometry_cache_vec3(
            channel as *const CacheChannel,
            time,
            data.as_mut_ptr(),
            data.len(),
            opts as *const RawGeometryCacheDataOpts,
        )
    };
    result
}

pub fn sample_geometry_cache_vec3(
    channel: &CacheChannel,
    time: f64,
    data: &mut [Vec3],
    opts: GeometryCacheDataOpts,
) -> usize {
    let mut arena = Arena::new();
    let mut opts_mut = opts;
    let opts_raw = opts_mut.to_raw_mut(&mut arena);
    unsafe { sample_geometry_cache_vec3_raw(channel, time, data, &opts_raw) }
}

pub fn dom_find<'a>(parent: &'a DomNode, name: &str) -> Option<&'a DomNode> {
    let result = crate::native::api::dom_find_len(
        crate::native::view::View::<DomNode, crate::native::view::Const>::from_ref(parent),
        name.as_bytes(),
    );
    result.map(|node| unsafe { &*node.as_ptr() })
}

#[allow(clippy::let_and_return)]
pub unsafe fn generate_indices_raw(
    streams: &[RawVertexStream],
    indices: &mut [u32],
    allocator: &RawAllocatorOpts,
) -> Result<usize> {
    let result = {
        crate::native::api::generate_indices(
            streams.as_ptr(),
            streams.len(),
            indices.as_mut_ptr(),
            indices.len(),
            allocator as *const RawAllocatorOpts,
        )
    };
    result
}

pub fn generate_indices(
    streams: &mut [VertexStream],
    indices: &mut [u32],
    allocator: AllocatorOpts,
) -> Result<usize> {
    let mut arena = Arena::new();
    let streams_raw = streams.to_raw_mut(&mut arena);
    let mut allocator_mut = allocator;
    let allocator_raw = allocator_mut.to_raw_mut(&mut arena);
    unsafe { generate_indices_raw(&streams_raw, indices, &allocator_raw) }
}

pub unsafe fn thread_pool_run_task(ctx: ThreadPoolContext, index: u32) {
    crate::native::api::thread_pool_run_task(ctx, index);
}

pub unsafe fn thread_pool_set_user_ptr(ctx: ThreadPoolContext, user_ptr: *mut c_void) {
    ufbx_thread_pool_set_user_ptr(ctx, user_ptr)
}

pub unsafe fn thread_pool_get_user_ptr(ctx: ThreadPoolContext) -> *mut c_void {
    ufbx_thread_pool_get_user_ptr(ctx)
}

pub fn get_vertex_real(v: &VertexReal, index: usize) -> Real {
    let mut panic: Panic = Default::default();
    let result = crate::native::api::catch_get_vertex_real(
        Some(&mut panic),
        crate::native::view::View::<VertexReal, crate::native::view::Const>::from_ref(v),
        index,
    );
    if panic.did_panic {
        panic!("ufbx::get_vertex_real() {}", panic.message());
    }
    result
}

pub fn get_vertex_vec2(v: &VertexVec2, index: usize) -> Vec2 {
    let mut panic: Panic = Default::default();
    let result = crate::native::api::catch_get_vertex_vec2(
        Some(&mut panic),
        crate::native::view::View::<VertexVec2, crate::native::view::Const>::from_ref(v),
        index,
    );
    if panic.did_panic {
        panic!("ufbx::get_vertex_vec2() {}", panic.message());
    }
    result
}

pub fn get_vertex_vec3(v: &VertexVec3, index: usize) -> Vec3 {
    let mut panic: Panic = Default::default();
    let result = crate::native::api::catch_get_vertex_vec3(
        Some(&mut panic),
        crate::native::view::View::<VertexVec3, crate::native::view::Const>::from_ref(v),
        index,
    );
    if panic.did_panic {
        panic!("ufbx::get_vertex_vec3() {}", panic.message());
    }
    result
}

pub fn get_vertex_vec4(v: &VertexVec4, index: usize) -> Vec4 {
    let mut panic: Panic = Default::default();
    let result = crate::native::api::catch_get_vertex_vec4(
        Some(&mut panic),
        crate::native::view::View::<VertexVec4, crate::native::view::Const>::from_ref(v),
        index,
    );
    if panic.did_panic {
        panic!("ufbx::get_vertex_vec4() {}", panic.message());
    }
    result
}

pub fn get_vertex_w_vec3(v: &VertexVec3, index: usize) -> Real {
    let mut panic: Panic = Default::default();
    let result = crate::native::api::catch_get_vertex_w_vec3(
        Some(&mut panic),
        crate::native::view::View::<VertexVec3, crate::native::view::Const>::from_ref(v),
        index,
    );
    if panic.did_panic {
        panic!("ufbx::get_vertex_w_vec3() {}", panic.message());
    }
    result
}

#[allow(clippy::needless_lifetimes)]
pub fn as_unknown<'a>(element: &'a Element) -> Option<&'a Unknown> {
    let result = unsafe { crate::native::api::as_unknown(element as *const Element) };
    if result.is_null() {
        None
    } else {
        unsafe { Some(&*result) }
    }
}

#[allow(clippy::needless_lifetimes)]
pub fn as_node<'a>(element: &'a Element) -> Option<&'a Node> {
    let result = unsafe { crate::native::api::as_node(element as *const Element) };
    if result.is_null() {
        None
    } else {
        unsafe { Some(&*result) }
    }
}

#[allow(clippy::needless_lifetimes)]
pub fn as_mesh<'a>(element: &'a Element) -> Option<&'a Mesh> {
    let result = unsafe { crate::native::api::as_mesh(element as *const Element) };
    if result.is_null() {
        None
    } else {
        unsafe { Some(&*result) }
    }
}

#[allow(clippy::needless_lifetimes)]
pub fn as_light<'a>(element: &'a Element) -> Option<&'a Light> {
    let result = unsafe { crate::native::api::as_light(element as *const Element) };
    if result.is_null() {
        None
    } else {
        unsafe { Some(&*result) }
    }
}

#[allow(clippy::needless_lifetimes)]
pub fn as_camera<'a>(element: &'a Element) -> Option<&'a Camera> {
    let result = unsafe { crate::native::api::as_camera(element as *const Element) };
    if result.is_null() {
        None
    } else {
        unsafe { Some(&*result) }
    }
}

#[allow(clippy::needless_lifetimes)]
pub fn as_bone<'a>(element: &'a Element) -> Option<&'a Bone> {
    let result = unsafe { crate::native::api::as_bone(element as *const Element) };
    if result.is_null() {
        None
    } else {
        unsafe { Some(&*result) }
    }
}

#[allow(clippy::needless_lifetimes)]
pub fn as_empty<'a>(element: &'a Element) -> Option<&'a Empty> {
    let result = unsafe { crate::native::api::as_empty(element as *const Element) };
    if result.is_null() {
        None
    } else {
        unsafe { Some(&*result) }
    }
}

#[allow(clippy::needless_lifetimes)]
pub fn as_line_curve<'a>(element: &'a Element) -> Option<&'a LineCurve> {
    let result = unsafe { crate::native::api::as_line_curve(element as *const Element) };
    if result.is_null() {
        None
    } else {
        unsafe { Some(&*result) }
    }
}

#[allow(clippy::needless_lifetimes)]
pub fn as_nurbs_curve<'a>(element: &'a Element) -> Option<&'a NurbsCurve> {
    let result = unsafe { crate::native::api::as_nurbs_curve(element as *const Element) };
    if result.is_null() {
        None
    } else {
        unsafe { Some(&*result) }
    }
}

#[allow(clippy::needless_lifetimes)]
pub fn as_nurbs_surface<'a>(element: &'a Element) -> Option<&'a NurbsSurface> {
    let result = unsafe { crate::native::api::as_nurbs_surface(element as *const Element) };
    if result.is_null() {
        None
    } else {
        unsafe { Some(&*result) }
    }
}

#[allow(clippy::needless_lifetimes)]
pub fn as_nurbs_trim_surface<'a>(element: &'a Element) -> Option<&'a NurbsTrimSurface> {
    let result = unsafe { crate::native::api::as_nurbs_trim_surface(element as *const Element) };
    if result.is_null() {
        None
    } else {
        unsafe { Some(&*result) }
    }
}

#[allow(clippy::needless_lifetimes)]
pub fn as_nurbs_trim_boundary<'a>(element: &'a Element) -> Option<&'a NurbsTrimBoundary> {
    let result = unsafe { crate::native::api::as_nurbs_trim_boundary(element as *const Element) };
    if result.is_null() {
        None
    } else {
        unsafe { Some(&*result) }
    }
}

#[allow(clippy::needless_lifetimes)]
pub fn as_procedural_geometry<'a>(element: &'a Element) -> Option<&'a ProceduralGeometry> {
    let result = unsafe { crate::native::api::as_procedural_geometry(element as *const Element) };
    if result.is_null() {
        None
    } else {
        unsafe { Some(&*result) }
    }
}

#[allow(clippy::needless_lifetimes)]
pub fn as_stereo_camera<'a>(element: &'a Element) -> Option<&'a StereoCamera> {
    let result = unsafe { crate::native::api::as_stereo_camera(element as *const Element) };
    if result.is_null() {
        None
    } else {
        unsafe { Some(&*result) }
    }
}

#[allow(clippy::needless_lifetimes)]
pub fn as_camera_switcher<'a>(element: &'a Element) -> Option<&'a CameraSwitcher> {
    let result = unsafe { crate::native::api::as_camera_switcher(element as *const Element) };
    if result.is_null() {
        None
    } else {
        unsafe { Some(&*result) }
    }
}

#[allow(clippy::needless_lifetimes)]
pub fn as_marker<'a>(element: &'a Element) -> Option<&'a Marker> {
    let result = unsafe { crate::native::api::as_marker(element as *const Element) };
    if result.is_null() {
        None
    } else {
        unsafe { Some(&*result) }
    }
}

#[allow(clippy::needless_lifetimes)]
pub fn as_lod_group<'a>(element: &'a Element) -> Option<&'a LodGroup> {
    let result = unsafe { crate::native::api::as_lod_group(element as *const Element) };
    if result.is_null() {
        None
    } else {
        unsafe { Some(&*result) }
    }
}

#[allow(clippy::needless_lifetimes)]
pub fn as_skin_deformer<'a>(element: &'a Element) -> Option<&'a SkinDeformer> {
    let result = unsafe { crate::native::api::as_skin_deformer(element as *const Element) };
    if result.is_null() {
        None
    } else {
        unsafe { Some(&*result) }
    }
}

#[allow(clippy::needless_lifetimes)]
pub fn as_skin_cluster<'a>(element: &'a Element) -> Option<&'a SkinCluster> {
    let result = unsafe { crate::native::api::as_skin_cluster(element as *const Element) };
    if result.is_null() {
        None
    } else {
        unsafe { Some(&*result) }
    }
}

#[allow(clippy::needless_lifetimes)]
pub fn as_blend_deformer<'a>(element: &'a Element) -> Option<&'a BlendDeformer> {
    let result = unsafe { crate::native::api::as_blend_deformer(element as *const Element) };
    if result.is_null() {
        None
    } else {
        unsafe { Some(&*result) }
    }
}

#[allow(clippy::needless_lifetimes)]
pub fn as_blend_channel<'a>(element: &'a Element) -> Option<&'a BlendChannel> {
    let result = unsafe { crate::native::api::as_blend_channel(element as *const Element) };
    if result.is_null() {
        None
    } else {
        unsafe { Some(&*result) }
    }
}

#[allow(clippy::needless_lifetimes)]
pub fn as_blend_shape<'a>(element: &'a Element) -> Option<&'a BlendShape> {
    let result = unsafe { crate::native::api::as_blend_shape(element as *const Element) };
    if result.is_null() {
        None
    } else {
        unsafe { Some(&*result) }
    }
}

#[allow(clippy::needless_lifetimes)]
pub fn as_cache_deformer<'a>(element: &'a Element) -> Option<&'a CacheDeformer> {
    let result = unsafe { crate::native::api::as_cache_deformer(element as *const Element) };
    if result.is_null() {
        None
    } else {
        unsafe { Some(&*result) }
    }
}

#[allow(clippy::needless_lifetimes)]
pub fn as_cache_file<'a>(element: &'a Element) -> Option<&'a CacheFile> {
    let result = unsafe { crate::native::api::as_cache_file(element as *const Element) };
    if result.is_null() {
        None
    } else {
        unsafe { Some(&*result) }
    }
}

#[allow(clippy::needless_lifetimes)]
pub fn as_material<'a>(element: &'a Element) -> Option<&'a Material> {
    let result = unsafe { crate::native::api::as_material(element as *const Element) };
    if result.is_null() {
        None
    } else {
        unsafe { Some(&*result) }
    }
}

#[allow(clippy::needless_lifetimes)]
pub fn as_texture<'a>(element: &'a Element) -> Option<&'a Texture> {
    let result = unsafe { crate::native::api::as_texture(element as *const Element) };
    if result.is_null() {
        None
    } else {
        unsafe { Some(&*result) }
    }
}

#[allow(clippy::needless_lifetimes)]
pub fn as_video<'a>(element: &'a Element) -> Option<&'a Video> {
    let result = unsafe { crate::native::api::as_video(element as *const Element) };
    if result.is_null() {
        None
    } else {
        unsafe { Some(&*result) }
    }
}

#[allow(clippy::needless_lifetimes)]
pub fn as_shader<'a>(element: &'a Element) -> Option<&'a Shader> {
    let result = unsafe { crate::native::api::as_shader(element as *const Element) };
    if result.is_null() {
        None
    } else {
        unsafe { Some(&*result) }
    }
}

#[allow(clippy::needless_lifetimes)]
pub fn as_shader_binding<'a>(element: &'a Element) -> Option<&'a ShaderBinding> {
    let result = unsafe { crate::native::api::as_shader_binding(element as *const Element) };
    if result.is_null() {
        None
    } else {
        unsafe { Some(&*result) }
    }
}

#[allow(clippy::needless_lifetimes)]
pub fn as_anim_stack<'a>(element: &'a Element) -> Option<&'a AnimStack> {
    let result = unsafe { crate::native::api::as_anim_stack(element as *const Element) };
    if result.is_null() {
        None
    } else {
        unsafe { Some(&*result) }
    }
}

#[allow(clippy::needless_lifetimes)]
pub fn as_anim_layer<'a>(element: &'a Element) -> Option<&'a AnimLayer> {
    let result = unsafe { crate::native::api::as_anim_layer(element as *const Element) };
    if result.is_null() {
        None
    } else {
        unsafe { Some(&*result) }
    }
}

#[allow(clippy::needless_lifetimes)]
pub fn as_anim_value<'a>(element: &'a Element) -> Option<&'a AnimValue> {
    let result = unsafe { crate::native::api::as_anim_value(element as *const Element) };
    if result.is_null() {
        None
    } else {
        unsafe { Some(&*result) }
    }
}

#[allow(clippy::needless_lifetimes)]
pub fn as_anim_curve<'a>(element: &'a Element) -> Option<&'a AnimCurve> {
    let result = unsafe { crate::native::api::as_anim_curve(element as *const Element) };
    if result.is_null() {
        None
    } else {
        unsafe { Some(&*result) }
    }
}

#[allow(clippy::needless_lifetimes)]
pub fn as_display_layer<'a>(element: &'a Element) -> Option<&'a DisplayLayer> {
    let result = unsafe { crate::native::api::as_display_layer(element as *const Element) };
    if result.is_null() {
        None
    } else {
        unsafe { Some(&*result) }
    }
}

#[allow(clippy::needless_lifetimes)]
pub fn as_selection_set<'a>(element: &'a Element) -> Option<&'a SelectionSet> {
    let result = unsafe { crate::native::api::as_selection_set(element as *const Element) };
    if result.is_null() {
        None
    } else {
        unsafe { Some(&*result) }
    }
}

#[allow(clippy::needless_lifetimes)]
pub fn as_selection_node<'a>(element: &'a Element) -> Option<&'a SelectionNode> {
    let result = unsafe { crate::native::api::as_selection_node(element as *const Element) };
    if result.is_null() {
        None
    } else {
        unsafe { Some(&*result) }
    }
}

#[allow(clippy::needless_lifetimes)]
pub fn as_character<'a>(element: &'a Element) -> Option<&'a Character> {
    let result = unsafe { crate::native::api::as_character(element as *const Element) };
    if result.is_null() {
        None
    } else {
        unsafe { Some(&*result) }
    }
}

#[allow(clippy::needless_lifetimes)]
pub fn as_constraint<'a>(element: &'a Element) -> Option<&'a Constraint> {
    let result = unsafe { crate::native::api::as_constraint(element as *const Element) };
    if result.is_null() {
        None
    } else {
        unsafe { Some(&*result) }
    }
}

#[allow(clippy::needless_lifetimes)]
pub fn as_audio_layer<'a>(element: &'a Element) -> Option<&'a AudioLayer> {
    let result = unsafe { crate::native::api::as_audio_layer(element as *const Element) };
    if result.is_null() {
        None
    } else {
        unsafe { Some(&*result) }
    }
}

#[allow(clippy::needless_lifetimes)]
pub fn as_audio_clip<'a>(element: &'a Element) -> Option<&'a AudioClip> {
    let result = unsafe { crate::native::api::as_audio_clip(element as *const Element) };
    if result.is_null() {
        None
    } else {
        unsafe { Some(&*result) }
    }
}

#[allow(clippy::needless_lifetimes)]
pub fn as_pose<'a>(element: &'a Element) -> Option<&'a Pose> {
    let result = unsafe { crate::native::api::as_pose(element as *const Element) };
    if result.is_null() {
        None
    } else {
        unsafe { Some(&*result) }
    }
}

#[allow(clippy::needless_lifetimes)]
pub fn as_metadata_object<'a>(element: &'a Element) -> Option<&'a MetadataObject> {
    let result = unsafe { crate::native::api::as_metadata_object(element as *const Element) };
    if result.is_null() {
        None
    } else {
        unsafe { Some(&*result) }
    }
}

pub fn dom_is_array(node: &DomNode) -> bool {
    crate::native::api::dom_is_array(Some(crate::native::view::View::<
        DomNode,
        crate::native::view::Const,
    >::from_ref(node)))
}

pub fn dom_array_size(node: &DomNode) -> usize {
    crate::native::api::dom_array_size(Some(crate::native::view::View::<
        DomNode,
        crate::native::view::Const,
    >::from_ref(node)))
}

pub fn dom_as_int32_list(node: &DomNode) -> &[i32] {
    let result = crate::native::api::dom_as_int32_list(Some(crate::native::view::View::<
        DomNode,
        crate::native::view::Const,
    >::from_ref(node)));
    unsafe { result.as_static_ref() }
}

pub fn dom_as_int64_list(node: &DomNode) -> &[i64] {
    let result = crate::native::api::dom_as_int64_list(Some(crate::native::view::View::<
        DomNode,
        crate::native::view::Const,
    >::from_ref(node)));
    unsafe { result.as_static_ref() }
}

pub fn dom_as_float_list(node: &DomNode) -> &[f32] {
    let result = crate::native::api::dom_as_float_list(Some(crate::native::view::View::<
        DomNode,
        crate::native::view::Const,
    >::from_ref(node)));
    unsafe { result.as_static_ref() }
}

pub fn dom_as_double_list(node: &DomNode) -> &[f64] {
    let result = crate::native::api::dom_as_double_list(Some(crate::native::view::View::<
        DomNode,
        crate::native::view::Const,
    >::from_ref(node)));
    unsafe { result.as_static_ref() }
}

pub fn dom_as_real_list(node: &DomNode) -> &[Real] {
    let result = crate::native::api::dom_as_real_list(Some(crate::native::view::View::<
        DomNode,
        crate::native::view::Const,
    >::from_ref(node)));
    unsafe { result.as_static_ref() }
}

pub fn dom_as_blob_list(node: &DomNode) -> &[Blob] {
    let result = crate::native::api::dom_as_blob_list(Some(crate::native::view::View::<
        DomNode,
        crate::native::view::Const,
    >::from_ref(node)));
    unsafe { result.as_static_ref() }
}
pub fn identity_matrix() -> Matrix {
    ufbx_identity_matrix
}
pub fn identity_transform() -> Transform {
    ufbx_identity_transform
}
pub fn zero_vec2() -> Vec2 {
    ufbx_zero_vec2
}
pub fn zero_vec3() -> Vec3 {
    ufbx_zero_vec3
}
pub fn zero_vec4() -> Vec4 {
    ufbx_zero_vec4
}
pub fn identity_quat() -> Quat {
    ufbx_identity_quat
}
pub fn axes_right_handed_y_up() -> CoordinateAxes {
    ufbx_axes_right_handed_y_up
}
pub fn axes_right_handed_z_up() -> CoordinateAxes {
    ufbx_axes_right_handed_z_up
}
pub fn axes_left_handed_y_up() -> CoordinateAxes {
    ufbx_axes_left_handed_y_up
}
pub fn axes_left_handed_z_up() -> CoordinateAxes {
    ufbx_axes_left_handed_z_up
}
pub fn source_version() -> u32 {
    ufbx_source_version
}

impl Vec2 {
    pub fn zero() -> Vec2 {
        ufbx_zero_vec2
    }
}

impl Vec3 {
    pub fn zero() -> Vec3 {
        ufbx_zero_vec3
    }
}

impl Vec4 {
    pub fn zero() -> Vec4 {
        ufbx_zero_vec4
    }
}

impl Quat {
    pub fn identity() -> Quat {
        ufbx_identity_quat
    }
}

impl Transform {
    pub fn identity() -> Transform {
        ufbx_identity_transform
    }
}

impl Matrix {
    pub fn identity() -> Matrix {
        ufbx_identity_matrix
    }
}

impl DomNode {
    #[allow(clippy::needless_lifetimes)]
    pub fn find<'a>(&'a self, name: &str) -> Option<&'a DomNode> {
        dom_find(self, name)
    }

    pub fn is_array(&self) -> bool {
        dom_is_array(self)
    }

    pub fn array_size(&self) -> usize {
        dom_array_size(self)
    }

    #[allow(clippy::needless_lifetimes)]
    pub fn as_int32_list<'a>(&'a self) -> &'a [i32] {
        dom_as_int32_list(self)
    }

    #[allow(clippy::needless_lifetimes)]
    pub fn as_int64_list<'a>(&'a self) -> &'a [i64] {
        dom_as_int64_list(self)
    }

    #[allow(clippy::needless_lifetimes)]
    pub fn as_float_list<'a>(&'a self) -> &'a [f32] {
        dom_as_float_list(self)
    }

    #[allow(clippy::needless_lifetimes)]
    pub fn as_double_list<'a>(&'a self) -> &'a [f64] {
        dom_as_double_list(self)
    }

    #[allow(clippy::needless_lifetimes)]
    pub fn as_real_list<'a>(&'a self) -> &'a [Real] {
        dom_as_real_list(self)
    }

    #[allow(clippy::needless_lifetimes)]
    pub fn as_blob_list<'a>(&'a self) -> &'a [Blob] {
        dom_as_blob_list(self)
    }
}

impl Props {
    #[allow(clippy::needless_lifetimes)]
    pub fn find_prop<'a>(&'a self, name: &str) -> Option<&'a Prop> {
        find_prop(self, name)
    }

    // TODO: find_real()

    // TODO: find_vec3()

    // TODO: find_int()

    // TODO: find_bool()

    // TODO: find_string()
}

impl Node {
    pub fn get_compatible_matrix_for_normals(&self) -> Matrix {
        get_compatible_matrix_for_normals(self)
    }

    pub fn evaluate_transform(&self, anim: &Anim, time: f64) -> Transform {
        evaluate_transform(anim, self, time)
    }
}

impl Mesh {
    pub fn triangulate_face(&self, indices: &mut [u32], face: Face) -> u32 {
        triangulate_face(indices, self, face)
    }

    pub fn subdivide(&self, level: usize, opts: SubdivideOpts) -> Result<MeshRoot> {
        subdivide_mesh(self, level, opts)
    }
}

impl CoordinateAxes {
    pub fn right_handed_y_up() -> CoordinateAxes {
        ufbx_axes_right_handed_y_up
    }
    pub fn right_handed_z_up() -> CoordinateAxes {
        ufbx_axes_right_handed_z_up
    }
    pub fn left_handed_y_up() -> CoordinateAxes {
        ufbx_axes_left_handed_y_up
    }
    pub fn left_handed_z_up() -> CoordinateAxes {
        ufbx_axes_left_handed_z_up
    }
}

impl NurbsBasis {
    pub fn evaluate(&self, u: Real, weights: &mut [Real], derivatives: &mut [Real]) -> usize {
        evaluate_nurbs_basis(self, u, weights, derivatives)
    }
}

impl NurbsCurve {
    pub fn evaluate(&self, u: Real) -> CurvePoint {
        evaluate_nurbs_curve(self, u)
    }

    pub fn tessellate(&self, opts: TessellateCurveOpts) -> Result<LineCurveRoot> {
        tessellate_nurbs_curve(self, opts)
    }
}

impl NurbsSurface {
    pub fn evaluate(&self, u: Real, v: Real) -> SurfacePoint {
        evaluate_nurbs_surface(self, u, v)
    }

    pub fn tessellate(&self, opts: TessellateSurfaceOpts) -> Result<MeshRoot> {
        tessellate_nurbs_surface(self, opts)
    }
}

impl SkinDeformer {
    pub fn get_skin_vertex_matrix(&self, vertex: usize, fallback: &Matrix) -> Matrix {
        get_skin_vertex_matrix(self, vertex, fallback)
    }
}

impl BlendDeformer {
    pub fn get_vertex_offset(&self, vertex: usize) -> Vec3 {
        get_blend_vertex_offset(self, vertex)
    }

    pub fn add_vertex_offsets(&self, vertices: &mut [Vec3], weight: Real) {
        add_blend_vertex_offsets(self, vertices, weight)
    }
}

impl BlendChannel {
    pub fn evaluate_blend_weight(&self, anim: &Anim, time: f64) -> Real {
        evaluate_blend_weight(anim, self, time)
    }
}

impl BlendShape {
    pub fn get_vertex_offset(&self, vertex: usize) -> Vec3 {
        get_blend_shape_vertex_offset(self, vertex)
    }

    pub fn add_vertex_offsets(&self, vertices: &mut [Vec3], weight: Real) {
        add_blend_shape_vertex_offsets(self, vertices, weight)
    }
}

impl CacheFrame {
    pub fn read_real(&self, data: &mut [Real], opts: GeometryCacheDataOpts) -> usize {
        read_geometry_cache_real(self, data, opts)
    }

    pub fn read_vec3(&self, data: &mut [Vec3], opts: GeometryCacheDataOpts) -> usize {
        read_geometry_cache_vec3(self, data, opts)
    }
}

impl CacheChannel {
    pub fn sample_real(&self, time: f64, data: &mut [Real], opts: GeometryCacheDataOpts) -> usize {
        sample_geometry_cache_real(self, time, data, opts)
    }

    pub fn sample_vec3(&self, time: f64, data: &mut [Vec3], opts: GeometryCacheDataOpts) -> usize {
        sample_geometry_cache_vec3(self, time, data, opts)
    }
}

impl Material {
    #[allow(clippy::needless_lifetimes)]
    pub fn find_prop_texture<'a>(&'a self, name: &str) -> Option<&'a Texture> {
        find_prop_texture(self, name)
    }
}

impl Shader {
    pub fn find_shader_prop<'a>(&'a self, name: &str) -> &'a str {
        find_shader_prop(self, name)
    }
}

impl AnimLayer {
    #[allow(clippy::needless_lifetimes)]
    pub fn find_anim_prop<'a>(&'a self, element: &Element, prop: &str) -> Option<&'a AnimProp> {
        find_anim_prop(self, element, prop)
    }

    #[allow(clippy::needless_lifetimes)]
    pub fn find_anim_props<'a>(&'a self, element: &Element) -> &'a [AnimProp] {
        find_anim_props(self, element)
    }
}

impl AnimValue {
    pub fn evaluate_real(&self, time: f64) -> Real {
        evaluate_anim_value_real(self, time)
    }

    pub fn evaluate_vec3(&self, time: f64) -> Vec3 {
        evaluate_anim_value_vec3(self, time)
    }
}

impl AnimCurve {
    pub fn evaluate(&self, time: f64, default_value: Real) -> Real {
        evaluate_curve(self, time, default_value)
    }
}

impl Scene {
    #[allow(clippy::needless_lifetimes)]
    pub fn find_element<'a>(&'a self, type_: ElementType, name: &str) -> Option<&'a Element> {
        find_element(self, type_, name)
    }

    #[allow(clippy::needless_lifetimes)]
    pub fn find_node<'a>(&'a self, name: &str) -> Option<&'a Node> {
        find_node(self, name)
    }

    #[allow(clippy::needless_lifetimes)]
    pub fn find_anim_stack<'a>(&'a self, name: &str) -> Option<&'a AnimStack> {
        find_anim_stack(self, name)
    }

    pub fn evaluate(&self, anim: &Anim, time: f64, opts: EvaluateOpts) -> Result<SceneRoot> {
        evaluate_scene(self, anim, time, opts)
    }
}

pub enum ElementData<'a> {
    Unknown(&'a Unknown),
    Node(&'a Node),
    Mesh(&'a Mesh),
    Light(&'a Light),
    Camera(&'a Camera),
    Bone(&'a Bone),
    Empty(&'a Empty),
    LineCurve(&'a LineCurve),
    NurbsCurve(&'a NurbsCurve),
    NurbsSurface(&'a NurbsSurface),
    NurbsTrimSurface(&'a NurbsTrimSurface),
    NurbsTrimBoundary(&'a NurbsTrimBoundary),
    ProceduralGeometry(&'a ProceduralGeometry),
    StereoCamera(&'a StereoCamera),
    CameraSwitcher(&'a CameraSwitcher),
    Marker(&'a Marker),
    LodGroup(&'a LodGroup),
    SkinDeformer(&'a SkinDeformer),
    SkinCluster(&'a SkinCluster),
    BlendDeformer(&'a BlendDeformer),
    BlendChannel(&'a BlendChannel),
    BlendShape(&'a BlendShape),
    CacheDeformer(&'a CacheDeformer),
    CacheFile(&'a CacheFile),
    Material(&'a Material),
    Texture(&'a Texture),
    Video(&'a Video),
    Shader(&'a Shader),
    ShaderBinding(&'a ShaderBinding),
    AnimStack(&'a AnimStack),
    AnimLayer(&'a AnimLayer),
    AnimValue(&'a AnimValue),
    AnimCurve(&'a AnimCurve),
    DisplayLayer(&'a DisplayLayer),
    SelectionSet(&'a SelectionSet),
    SelectionNode(&'a SelectionNode),
    Character(&'a Character),
    Constraint(&'a Constraint),
    AudioLayer(&'a AudioLayer),
    AudioClip(&'a AudioClip),
    Pose(&'a Pose),
    MetadataObject(&'a MetadataObject),
}

impl Element {
    pub fn as_data(&self) -> ElementData<'_> {
        unsafe {
            match self.type_ {
                ElementType::Unknown => {
                    ElementData::Unknown(&*(self as *const _ as *const Unknown))
                }
                ElementType::Node => ElementData::Node(&*(self as *const _ as *const Node)),
                ElementType::Mesh => ElementData::Mesh(&*(self as *const _ as *const Mesh)),
                ElementType::Light => ElementData::Light(&*(self as *const _ as *const Light)),
                ElementType::Camera => ElementData::Camera(&*(self as *const _ as *const Camera)),
                ElementType::Bone => ElementData::Bone(&*(self as *const _ as *const Bone)),
                ElementType::Empty => ElementData::Empty(&*(self as *const _ as *const Empty)),
                ElementType::LineCurve => {
                    ElementData::LineCurve(&*(self as *const _ as *const LineCurve))
                }
                ElementType::NurbsCurve => {
                    ElementData::NurbsCurve(&*(self as *const _ as *const NurbsCurve))
                }
                ElementType::NurbsSurface => {
                    ElementData::NurbsSurface(&*(self as *const _ as *const NurbsSurface))
                }
                ElementType::NurbsTrimSurface => {
                    ElementData::NurbsTrimSurface(&*(self as *const _ as *const NurbsTrimSurface))
                }
                ElementType::NurbsTrimBoundary => {
                    ElementData::NurbsTrimBoundary(&*(self as *const _ as *const NurbsTrimBoundary))
                }
                ElementType::ProceduralGeometry => ElementData::ProceduralGeometry(
                    &*(self as *const _ as *const ProceduralGeometry),
                ),
                ElementType::StereoCamera => {
                    ElementData::StereoCamera(&*(self as *const _ as *const StereoCamera))
                }
                ElementType::CameraSwitcher => {
                    ElementData::CameraSwitcher(&*(self as *const _ as *const CameraSwitcher))
                }
                ElementType::Marker => ElementData::Marker(&*(self as *const _ as *const Marker)),
                ElementType::LodGroup => {
                    ElementData::LodGroup(&*(self as *const _ as *const LodGroup))
                }
                ElementType::SkinDeformer => {
                    ElementData::SkinDeformer(&*(self as *const _ as *const SkinDeformer))
                }
                ElementType::SkinCluster => {
                    ElementData::SkinCluster(&*(self as *const _ as *const SkinCluster))
                }
                ElementType::BlendDeformer => {
                    ElementData::BlendDeformer(&*(self as *const _ as *const BlendDeformer))
                }
                ElementType::BlendChannel => {
                    ElementData::BlendChannel(&*(self as *const _ as *const BlendChannel))
                }
                ElementType::BlendShape => {
                    ElementData::BlendShape(&*(self as *const _ as *const BlendShape))
                }
                ElementType::CacheDeformer => {
                    ElementData::CacheDeformer(&*(self as *const _ as *const CacheDeformer))
                }
                ElementType::CacheFile => {
                    ElementData::CacheFile(&*(self as *const _ as *const CacheFile))
                }
                ElementType::Material => {
                    ElementData::Material(&*(self as *const _ as *const Material))
                }
                ElementType::Texture => {
                    ElementData::Texture(&*(self as *const _ as *const Texture))
                }
                ElementType::Video => ElementData::Video(&*(self as *const _ as *const Video)),
                ElementType::Shader => ElementData::Shader(&*(self as *const _ as *const Shader)),
                ElementType::ShaderBinding => {
                    ElementData::ShaderBinding(&*(self as *const _ as *const ShaderBinding))
                }
                ElementType::AnimStack => {
                    ElementData::AnimStack(&*(self as *const _ as *const AnimStack))
                }
                ElementType::AnimLayer => {
                    ElementData::AnimLayer(&*(self as *const _ as *const AnimLayer))
                }
                ElementType::AnimValue => {
                    ElementData::AnimValue(&*(self as *const _ as *const AnimValue))
                }
                ElementType::AnimCurve => {
                    ElementData::AnimCurve(&*(self as *const _ as *const AnimCurve))
                }
                ElementType::DisplayLayer => {
                    ElementData::DisplayLayer(&*(self as *const _ as *const DisplayLayer))
                }
                ElementType::SelectionSet => {
                    ElementData::SelectionSet(&*(self as *const _ as *const SelectionSet))
                }
                ElementType::SelectionNode => {
                    ElementData::SelectionNode(&*(self as *const _ as *const SelectionNode))
                }
                ElementType::Character => {
                    ElementData::Character(&*(self as *const _ as *const Character))
                }
                ElementType::Constraint => {
                    ElementData::Constraint(&*(self as *const _ as *const Constraint))
                }
                ElementType::AudioLayer => {
                    ElementData::AudioLayer(&*(self as *const _ as *const AudioLayer))
                }
                ElementType::AudioClip => {
                    ElementData::AudioClip(&*(self as *const _ as *const AudioClip))
                }
                ElementType::Pose => ElementData::Pose(&*(self as *const _ as *const Pose)),
                ElementType::MetadataObject => {
                    ElementData::MetadataObject(&*(self as *const _ as *const MetadataObject))
                }
            }
        }
    }
}
