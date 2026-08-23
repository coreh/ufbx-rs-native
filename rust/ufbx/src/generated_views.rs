// GENERATED FILE — do not edit by hand. Produced by rust/regen.sh from
// ufbx.h via bindgen/ufbx_ir.py + rust/ufbx/bindgen/generate_rust.py.
// Fixes belong in the GENERATOR (see PORTING.md); hand edits are
// silently overwritten on the next regeneration and CI diffs this file.
//
// Crate-internal `View<T, M>` field accessors over the generated public
// structs (`view_accessor_structs` in generate_rust.py): a by-value read
// per leaf field, an in-place `&View` projection per aggregate and list
// (`*_view`) field, a `*_ptr` read-address projection per field, and
// `Mut`-only setters / raw field pointers. Soundness model (mint vouch,
// `Mut`/`Const` provenance): src/native/view.rs.

use crate::generated::*;
use crate::native::view::{
    view_project, view_raw_mut, view_raw_shared, view_read_shared, view_write, Mode, Mut, View,
};
use crate::prelude::*;

#[allow(dead_code)]
impl<M: Mode> View<Element, M> {
    #[inline(always)]
    pub(crate) fn name(&self) -> String {
        view_read_shared!(self, name)
    }
    #[inline(always)]
    pub(crate) fn name_view(&self) -> &View<String, M> {
        view_project!(self, name)
    }
    #[inline(always)]
    pub(crate) fn name_ptr(&self) -> *const String {
        view_raw_shared!(self, name)
    }
    #[inline(always)]
    pub(crate) fn props(&self) -> &View<Props, M> {
        view_project!(self, props)
    }
    #[inline(always)]
    pub(crate) fn props_ptr(&self) -> *const Props {
        view_raw_shared!(self, props)
    }
    #[inline(always)]
    pub(crate) fn element_id(&self) -> u32 {
        view_read_shared!(self, element_id)
    }
    #[inline(always)]
    pub(crate) fn element_id_ptr(&self) -> *const u32 {
        view_raw_shared!(self, element_id)
    }
    #[inline(always)]
    pub(crate) fn typed_id(&self) -> u32 {
        view_read_shared!(self, typed_id)
    }
    #[inline(always)]
    pub(crate) fn typed_id_ptr(&self) -> *const u32 {
        view_raw_shared!(self, typed_id)
    }
    #[inline(always)]
    pub(crate) fn instances(&self) -> RefList<Node> {
        view_read_shared!(self, instances)
    }
    #[inline(always)]
    pub(crate) fn instances_view(&self) -> &View<RefList<Node>, M> {
        view_project!(self, instances)
    }
    #[inline(always)]
    pub(crate) fn instances_ptr(&self) -> *const RefList<Node> {
        view_raw_shared!(self, instances)
    }
    #[inline(always)]
    pub(crate) fn type_(&self) -> ElementType {
        view_read_shared!(self, type_)
    }
    #[inline(always)]
    pub(crate) fn type_ptr(&self) -> *const ElementType {
        view_raw_shared!(self, type_)
    }
    #[inline(always)]
    pub(crate) fn connections_src(&self) -> List<Connection> {
        view_read_shared!(self, connections_src)
    }
    #[inline(always)]
    pub(crate) fn connections_src_view(&self) -> &View<List<Connection>, M> {
        view_project!(self, connections_src)
    }
    #[inline(always)]
    pub(crate) fn connections_src_ptr(&self) -> *const List<Connection> {
        view_raw_shared!(self, connections_src)
    }
    #[inline(always)]
    pub(crate) fn connections_dst(&self) -> List<Connection> {
        view_read_shared!(self, connections_dst)
    }
    #[inline(always)]
    pub(crate) fn connections_dst_view(&self) -> &View<List<Connection>, M> {
        view_project!(self, connections_dst)
    }
    #[inline(always)]
    pub(crate) fn connections_dst_ptr(&self) -> *const List<Connection> {
        view_raw_shared!(self, connections_dst)
    }
    #[inline(always)]
    pub(crate) fn dom_node(&self) -> Option<Ref<DomNode>> {
        view_read_shared!(self, dom_node)
    }
    #[inline(always)]
    pub(crate) fn dom_node_ptr(&self) -> *const Option<Ref<DomNode>> {
        view_raw_shared!(self, dom_node)
    }
    #[inline(always)]
    pub(crate) fn scene(&self) -> Ref<Scene> {
        view_read_shared!(self, scene)
    }
    #[inline(always)]
    pub(crate) fn scene_ptr(&self) -> *const Ref<Scene> {
        view_raw_shared!(self, scene)
    }
}

#[allow(dead_code)]
impl View<Element, Mut> {
    #[inline(always)]
    pub(crate) fn set_name(&self, value: String) {
        view_write!(self, name, value)
    }
    #[inline(always)]
    pub(crate) fn name_raw(&self) -> *mut String {
        view_raw_mut!(self, name)
    }
    #[inline(always)]
    pub(crate) fn set_props(&self, value: Props) {
        view_write!(self, props, value)
    }
    #[inline(always)]
    pub(crate) fn props_raw(&self) -> *mut Props {
        view_raw_mut!(self, props)
    }
    #[inline(always)]
    pub(crate) fn set_element_id(&self, value: u32) {
        view_write!(self, element_id, value)
    }
    #[inline(always)]
    pub(crate) fn element_id_raw(&self) -> *mut u32 {
        view_raw_mut!(self, element_id)
    }
    #[inline(always)]
    pub(crate) fn set_typed_id(&self, value: u32) {
        view_write!(self, typed_id, value)
    }
    #[inline(always)]
    pub(crate) fn typed_id_raw(&self) -> *mut u32 {
        view_raw_mut!(self, typed_id)
    }
    #[inline(always)]
    pub(crate) fn set_instances(&self, value: RefList<Node>) {
        view_write!(self, instances, value)
    }
    #[inline(always)]
    pub(crate) fn instances_raw(&self) -> *mut RefList<Node> {
        view_raw_mut!(self, instances)
    }
    #[inline(always)]
    pub(crate) fn set_type(&self, value: ElementType) {
        view_write!(self, type_, value)
    }
    #[inline(always)]
    pub(crate) fn type_raw(&self) -> *mut ElementType {
        view_raw_mut!(self, type_)
    }
    #[inline(always)]
    pub(crate) fn set_connections_src(&self, value: List<Connection>) {
        view_write!(self, connections_src, value)
    }
    #[inline(always)]
    pub(crate) fn connections_src_raw(&self) -> *mut List<Connection> {
        view_raw_mut!(self, connections_src)
    }
    #[inline(always)]
    pub(crate) fn set_connections_dst(&self, value: List<Connection>) {
        view_write!(self, connections_dst, value)
    }
    #[inline(always)]
    pub(crate) fn connections_dst_raw(&self) -> *mut List<Connection> {
        view_raw_mut!(self, connections_dst)
    }
    #[inline(always)]
    pub(crate) fn set_dom_node(&self, value: Option<Ref<DomNode>>) {
        view_write!(self, dom_node, value)
    }
    #[inline(always)]
    pub(crate) fn dom_node_raw(&self) -> *mut Option<Ref<DomNode>> {
        view_raw_mut!(self, dom_node)
    }
    #[inline(always)]
    pub(crate) fn set_scene(&self, value: Ref<Scene>) {
        view_write!(self, scene, value)
    }
    #[inline(always)]
    pub(crate) fn scene_raw(&self) -> *mut Ref<Scene> {
        view_raw_mut!(self, scene)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<Props, M> {
    #[inline(always)]
    pub(crate) fn props(&self) -> List<Prop> {
        view_read_shared!(self, props)
    }
    #[inline(always)]
    pub(crate) fn props_view(&self) -> &View<List<Prop>, M> {
        view_project!(self, props)
    }
    #[inline(always)]
    pub(crate) fn props_ptr(&self) -> *const List<Prop> {
        view_raw_shared!(self, props)
    }
    #[inline(always)]
    pub(crate) fn num_animated(&self) -> usize {
        view_read_shared!(self, num_animated)
    }
    #[inline(always)]
    pub(crate) fn num_animated_ptr(&self) -> *const usize {
        view_raw_shared!(self, num_animated)
    }
    #[inline(always)]
    pub(crate) fn defaults_ptr(&self) -> *const Option<Ref<Props>> {
        view_raw_shared!(self, defaults)
    }
}

#[allow(dead_code)]
impl View<Props, Mut> {
    #[inline(always)]
    pub(crate) fn set_props(&self, value: List<Prop>) {
        view_write!(self, props, value)
    }
    #[inline(always)]
    pub(crate) fn props_raw(&self) -> *mut List<Prop> {
        view_raw_mut!(self, props)
    }
    #[inline(always)]
    pub(crate) fn set_num_animated(&self, value: usize) {
        view_write!(self, num_animated, value)
    }
    #[inline(always)]
    pub(crate) fn num_animated_raw(&self) -> *mut usize {
        view_raw_mut!(self, num_animated)
    }
    #[inline(always)]
    pub(crate) fn set_defaults(&self, value: Option<Ref<Props>>) {
        view_write!(self, defaults, value)
    }
    #[inline(always)]
    pub(crate) fn defaults_raw(&self) -> *mut Option<Ref<Props>> {
        view_raw_mut!(self, defaults)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<Prop, M> {
    #[inline(always)]
    pub(crate) fn name(&self) -> String {
        view_read_shared!(self, name)
    }
    #[inline(always)]
    pub(crate) fn name_view(&self) -> &View<String, M> {
        view_project!(self, name)
    }
    #[inline(always)]
    pub(crate) fn name_ptr(&self) -> *const String {
        view_raw_shared!(self, name)
    }
    #[inline(always)]
    pub(crate) fn _internal_key(&self) -> u32 {
        view_read_shared!(self, _internal_key)
    }
    #[inline(always)]
    pub(crate) fn internal_key_ptr(&self) -> *const u32 {
        view_raw_shared!(self, _internal_key)
    }
    #[inline(always)]
    pub(crate) fn type_(&self) -> PropType {
        view_read_shared!(self, type_)
    }
    #[inline(always)]
    pub(crate) fn type_ptr(&self) -> *const PropType {
        view_raw_shared!(self, type_)
    }
    #[inline(always)]
    pub(crate) fn flags(&self) -> PropFlags {
        view_read_shared!(self, flags)
    }
    #[inline(always)]
    pub(crate) fn flags_ptr(&self) -> *const PropFlags {
        view_raw_shared!(self, flags)
    }
    #[inline(always)]
    pub(crate) fn value_str(&self) -> String {
        view_read_shared!(self, value_str)
    }
    #[inline(always)]
    pub(crate) fn value_str_view(&self) -> &View<String, M> {
        view_project!(self, value_str)
    }
    #[inline(always)]
    pub(crate) fn value_str_ptr(&self) -> *const String {
        view_raw_shared!(self, value_str)
    }
    #[inline(always)]
    pub(crate) fn value_blob(&self) -> Blob {
        view_read_shared!(self, value_blob)
    }
    #[inline(always)]
    pub(crate) fn value_blob_ptr(&self) -> *const Blob {
        view_raw_shared!(self, value_blob)
    }
    #[inline(always)]
    pub(crate) fn value_int(&self) -> i64 {
        view_read_shared!(self, value_int)
    }
    #[inline(always)]
    pub(crate) fn value_int_ptr(&self) -> *const i64 {
        view_raw_shared!(self, value_int)
    }
    #[inline(always)]
    pub(crate) fn value_vec4(&self) -> Vec4 {
        view_read_shared!(self, value_vec4)
    }
    #[inline(always)]
    pub(crate) fn value_vec4_ptr(&self) -> *const Vec4 {
        view_raw_shared!(self, value_vec4)
    }
}

#[allow(dead_code)]
impl View<Prop, Mut> {
    #[inline(always)]
    pub(crate) fn set_name(&self, value: String) {
        view_write!(self, name, value)
    }
    #[inline(always)]
    pub(crate) fn name_raw(&self) -> *mut String {
        view_raw_mut!(self, name)
    }
    #[inline(always)]
    pub(crate) fn set_internal_key(&self, value: u32) {
        view_write!(self, _internal_key, value)
    }
    #[inline(always)]
    pub(crate) fn internal_key_raw(&self) -> *mut u32 {
        view_raw_mut!(self, _internal_key)
    }
    #[inline(always)]
    pub(crate) fn set_type(&self, value: PropType) {
        view_write!(self, type_, value)
    }
    #[inline(always)]
    pub(crate) fn type_raw(&self) -> *mut PropType {
        view_raw_mut!(self, type_)
    }
    #[inline(always)]
    pub(crate) fn set_flags(&self, value: PropFlags) {
        view_write!(self, flags, value)
    }
    #[inline(always)]
    pub(crate) fn flags_raw(&self) -> *mut PropFlags {
        view_raw_mut!(self, flags)
    }
    #[inline(always)]
    pub(crate) fn set_value_str(&self, value: String) {
        view_write!(self, value_str, value)
    }
    #[inline(always)]
    pub(crate) fn value_str_raw(&self) -> *mut String {
        view_raw_mut!(self, value_str)
    }
    #[inline(always)]
    pub(crate) fn set_value_blob(&self, value: Blob) {
        view_write!(self, value_blob, value)
    }
    #[inline(always)]
    pub(crate) fn value_blob_raw(&self) -> *mut Blob {
        view_raw_mut!(self, value_blob)
    }
    #[inline(always)]
    pub(crate) fn set_value_int(&self, value: i64) {
        view_write!(self, value_int, value)
    }
    #[inline(always)]
    pub(crate) fn value_int_raw(&self) -> *mut i64 {
        view_raw_mut!(self, value_int)
    }
    #[inline(always)]
    pub(crate) fn set_value_vec4(&self, value: Vec4) {
        view_write!(self, value_vec4, value)
    }
    #[inline(always)]
    pub(crate) fn value_vec4_raw(&self) -> *mut Vec4 {
        view_raw_mut!(self, value_vec4)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<Face, M> {
    #[inline(always)]
    pub(crate) fn index_begin(&self) -> u32 {
        view_read_shared!(self, index_begin)
    }
    #[inline(always)]
    pub(crate) fn index_begin_ptr(&self) -> *const u32 {
        view_raw_shared!(self, index_begin)
    }
    #[inline(always)]
    pub(crate) fn num_indices(&self) -> u32 {
        view_read_shared!(self, num_indices)
    }
    #[inline(always)]
    pub(crate) fn num_indices_ptr(&self) -> *const u32 {
        view_raw_shared!(self, num_indices)
    }
}

#[allow(dead_code)]
impl View<Face, Mut> {
    #[inline(always)]
    pub(crate) fn set_index_begin(&self, value: u32) {
        view_write!(self, index_begin, value)
    }
    #[inline(always)]
    pub(crate) fn index_begin_raw(&self) -> *mut u32 {
        view_raw_mut!(self, index_begin)
    }
    #[inline(always)]
    pub(crate) fn set_num_indices(&self, value: u32) {
        view_write!(self, num_indices, value)
    }
    #[inline(always)]
    pub(crate) fn num_indices_raw(&self) -> *mut u32 {
        view_raw_mut!(self, num_indices)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<Edge, M> {
    #[inline(always)]
    pub(crate) fn a(&self) -> u32 {
        view_read_shared!(self, a)
    }
    #[inline(always)]
    pub(crate) fn a_ptr(&self) -> *const u32 {
        view_raw_shared!(self, a)
    }
    #[inline(always)]
    pub(crate) fn b(&self) -> u32 {
        view_read_shared!(self, b)
    }
    #[inline(always)]
    pub(crate) fn b_ptr(&self) -> *const u32 {
        view_raw_shared!(self, b)
    }
}

#[allow(dead_code)]
impl View<Edge, Mut> {
    #[inline(always)]
    pub(crate) fn set_a(&self, value: u32) {
        view_write!(self, a, value)
    }
    #[inline(always)]
    pub(crate) fn a_raw(&self) -> *mut u32 {
        view_raw_mut!(self, a)
    }
    #[inline(always)]
    pub(crate) fn set_b(&self, value: u32) {
        view_write!(self, b, value)
    }
    #[inline(always)]
    pub(crate) fn b_raw(&self) -> *mut u32 {
        view_raw_mut!(self, b)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<VertexAttrib, M> {
    #[inline(always)]
    pub(crate) fn exists(&self) -> bool {
        view_read_shared!(self, exists)
    }
    #[inline(always)]
    pub(crate) fn exists_ptr(&self) -> *const bool {
        view_raw_shared!(self, exists)
    }
    #[inline(always)]
    pub(crate) fn values(&self) -> VoidList {
        view_read_shared!(self, values)
    }
    #[inline(always)]
    pub(crate) fn values_ptr(&self) -> *const VoidList {
        view_raw_shared!(self, values)
    }
    #[inline(always)]
    pub(crate) fn indices(&self) -> List<u32> {
        view_read_shared!(self, indices)
    }
    #[inline(always)]
    pub(crate) fn indices_view(&self) -> &View<List<u32>, M> {
        view_project!(self, indices)
    }
    #[inline(always)]
    pub(crate) fn indices_ptr(&self) -> *const List<u32> {
        view_raw_shared!(self, indices)
    }
    #[inline(always)]
    pub(crate) fn value_reals(&self) -> usize {
        view_read_shared!(self, value_reals)
    }
    #[inline(always)]
    pub(crate) fn value_reals_ptr(&self) -> *const usize {
        view_raw_shared!(self, value_reals)
    }
    #[inline(always)]
    pub(crate) fn unique_per_vertex(&self) -> bool {
        view_read_shared!(self, unique_per_vertex)
    }
    #[inline(always)]
    pub(crate) fn unique_per_vertex_ptr(&self) -> *const bool {
        view_raw_shared!(self, unique_per_vertex)
    }
    #[inline(always)]
    pub(crate) fn values_w(&self) -> List<Real> {
        view_read_shared!(self, values_w)
    }
    #[inline(always)]
    pub(crate) fn values_w_view(&self) -> &View<List<Real>, M> {
        view_project!(self, values_w)
    }
    #[inline(always)]
    pub(crate) fn values_w_ptr(&self) -> *const List<Real> {
        view_raw_shared!(self, values_w)
    }
}

#[allow(dead_code)]
impl View<VertexAttrib, Mut> {
    #[inline(always)]
    pub(crate) fn set_exists(&self, value: bool) {
        view_write!(self, exists, value)
    }
    #[inline(always)]
    pub(crate) fn exists_raw(&self) -> *mut bool {
        view_raw_mut!(self, exists)
    }
    #[inline(always)]
    pub(crate) fn set_values(&self, value: VoidList) {
        view_write!(self, values, value)
    }
    #[inline(always)]
    pub(crate) fn values_raw(&self) -> *mut VoidList {
        view_raw_mut!(self, values)
    }
    #[inline(always)]
    pub(crate) fn set_indices(&self, value: List<u32>) {
        view_write!(self, indices, value)
    }
    #[inline(always)]
    pub(crate) fn indices_raw(&self) -> *mut List<u32> {
        view_raw_mut!(self, indices)
    }
    #[inline(always)]
    pub(crate) fn set_value_reals(&self, value: usize) {
        view_write!(self, value_reals, value)
    }
    #[inline(always)]
    pub(crate) fn value_reals_raw(&self) -> *mut usize {
        view_raw_mut!(self, value_reals)
    }
    #[inline(always)]
    pub(crate) fn set_unique_per_vertex(&self, value: bool) {
        view_write!(self, unique_per_vertex, value)
    }
    #[inline(always)]
    pub(crate) fn unique_per_vertex_raw(&self) -> *mut bool {
        view_raw_mut!(self, unique_per_vertex)
    }
    #[inline(always)]
    pub(crate) fn set_values_w(&self, value: List<Real>) {
        view_write!(self, values_w, value)
    }
    #[inline(always)]
    pub(crate) fn values_w_raw(&self) -> *mut List<Real> {
        view_raw_mut!(self, values_w)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<VertexReal, M> {
    #[inline(always)]
    pub(crate) fn exists(&self) -> bool {
        view_read_shared!(self, exists)
    }
    #[inline(always)]
    pub(crate) fn exists_ptr(&self) -> *const bool {
        view_raw_shared!(self, exists)
    }
    #[inline(always)]
    pub(crate) fn values(&self) -> List<Real> {
        view_read_shared!(self, values)
    }
    #[inline(always)]
    pub(crate) fn values_view(&self) -> &View<List<Real>, M> {
        view_project!(self, values)
    }
    #[inline(always)]
    pub(crate) fn values_ptr(&self) -> *const List<Real> {
        view_raw_shared!(self, values)
    }
    #[inline(always)]
    pub(crate) fn indices(&self) -> List<u32> {
        view_read_shared!(self, indices)
    }
    #[inline(always)]
    pub(crate) fn indices_view(&self) -> &View<List<u32>, M> {
        view_project!(self, indices)
    }
    #[inline(always)]
    pub(crate) fn indices_ptr(&self) -> *const List<u32> {
        view_raw_shared!(self, indices)
    }
    #[inline(always)]
    pub(crate) fn value_reals(&self) -> usize {
        view_read_shared!(self, value_reals)
    }
    #[inline(always)]
    pub(crate) fn value_reals_ptr(&self) -> *const usize {
        view_raw_shared!(self, value_reals)
    }
    #[inline(always)]
    pub(crate) fn unique_per_vertex(&self) -> bool {
        view_read_shared!(self, unique_per_vertex)
    }
    #[inline(always)]
    pub(crate) fn unique_per_vertex_ptr(&self) -> *const bool {
        view_raw_shared!(self, unique_per_vertex)
    }
    #[inline(always)]
    pub(crate) fn values_w(&self) -> List<Real> {
        view_read_shared!(self, values_w)
    }
    #[inline(always)]
    pub(crate) fn values_w_view(&self) -> &View<List<Real>, M> {
        view_project!(self, values_w)
    }
    #[inline(always)]
    pub(crate) fn values_w_ptr(&self) -> *const List<Real> {
        view_raw_shared!(self, values_w)
    }
}

#[allow(dead_code)]
impl View<VertexReal, Mut> {
    #[inline(always)]
    pub(crate) fn set_exists(&self, value: bool) {
        view_write!(self, exists, value)
    }
    #[inline(always)]
    pub(crate) fn exists_raw(&self) -> *mut bool {
        view_raw_mut!(self, exists)
    }
    #[inline(always)]
    pub(crate) fn set_values(&self, value: List<Real>) {
        view_write!(self, values, value)
    }
    #[inline(always)]
    pub(crate) fn values_raw(&self) -> *mut List<Real> {
        view_raw_mut!(self, values)
    }
    #[inline(always)]
    pub(crate) fn set_indices(&self, value: List<u32>) {
        view_write!(self, indices, value)
    }
    #[inline(always)]
    pub(crate) fn indices_raw(&self) -> *mut List<u32> {
        view_raw_mut!(self, indices)
    }
    #[inline(always)]
    pub(crate) fn set_value_reals(&self, value: usize) {
        view_write!(self, value_reals, value)
    }
    #[inline(always)]
    pub(crate) fn value_reals_raw(&self) -> *mut usize {
        view_raw_mut!(self, value_reals)
    }
    #[inline(always)]
    pub(crate) fn set_unique_per_vertex(&self, value: bool) {
        view_write!(self, unique_per_vertex, value)
    }
    #[inline(always)]
    pub(crate) fn unique_per_vertex_raw(&self) -> *mut bool {
        view_raw_mut!(self, unique_per_vertex)
    }
    #[inline(always)]
    pub(crate) fn set_values_w(&self, value: List<Real>) {
        view_write!(self, values_w, value)
    }
    #[inline(always)]
    pub(crate) fn values_w_raw(&self) -> *mut List<Real> {
        view_raw_mut!(self, values_w)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<VertexVec2, M> {
    #[inline(always)]
    pub(crate) fn exists(&self) -> bool {
        view_read_shared!(self, exists)
    }
    #[inline(always)]
    pub(crate) fn exists_ptr(&self) -> *const bool {
        view_raw_shared!(self, exists)
    }
    #[inline(always)]
    pub(crate) fn values(&self) -> List<Vec2> {
        view_read_shared!(self, values)
    }
    #[inline(always)]
    pub(crate) fn values_view(&self) -> &View<List<Vec2>, M> {
        view_project!(self, values)
    }
    #[inline(always)]
    pub(crate) fn values_ptr(&self) -> *const List<Vec2> {
        view_raw_shared!(self, values)
    }
    #[inline(always)]
    pub(crate) fn indices(&self) -> List<u32> {
        view_read_shared!(self, indices)
    }
    #[inline(always)]
    pub(crate) fn indices_view(&self) -> &View<List<u32>, M> {
        view_project!(self, indices)
    }
    #[inline(always)]
    pub(crate) fn indices_ptr(&self) -> *const List<u32> {
        view_raw_shared!(self, indices)
    }
    #[inline(always)]
    pub(crate) fn value_reals(&self) -> usize {
        view_read_shared!(self, value_reals)
    }
    #[inline(always)]
    pub(crate) fn value_reals_ptr(&self) -> *const usize {
        view_raw_shared!(self, value_reals)
    }
    #[inline(always)]
    pub(crate) fn unique_per_vertex(&self) -> bool {
        view_read_shared!(self, unique_per_vertex)
    }
    #[inline(always)]
    pub(crate) fn unique_per_vertex_ptr(&self) -> *const bool {
        view_raw_shared!(self, unique_per_vertex)
    }
    #[inline(always)]
    pub(crate) fn values_w(&self) -> List<Real> {
        view_read_shared!(self, values_w)
    }
    #[inline(always)]
    pub(crate) fn values_w_view(&self) -> &View<List<Real>, M> {
        view_project!(self, values_w)
    }
    #[inline(always)]
    pub(crate) fn values_w_ptr(&self) -> *const List<Real> {
        view_raw_shared!(self, values_w)
    }
}

#[allow(dead_code)]
impl View<VertexVec2, Mut> {
    #[inline(always)]
    pub(crate) fn set_exists(&self, value: bool) {
        view_write!(self, exists, value)
    }
    #[inline(always)]
    pub(crate) fn exists_raw(&self) -> *mut bool {
        view_raw_mut!(self, exists)
    }
    #[inline(always)]
    pub(crate) fn set_values(&self, value: List<Vec2>) {
        view_write!(self, values, value)
    }
    #[inline(always)]
    pub(crate) fn values_raw(&self) -> *mut List<Vec2> {
        view_raw_mut!(self, values)
    }
    #[inline(always)]
    pub(crate) fn set_indices(&self, value: List<u32>) {
        view_write!(self, indices, value)
    }
    #[inline(always)]
    pub(crate) fn indices_raw(&self) -> *mut List<u32> {
        view_raw_mut!(self, indices)
    }
    #[inline(always)]
    pub(crate) fn set_value_reals(&self, value: usize) {
        view_write!(self, value_reals, value)
    }
    #[inline(always)]
    pub(crate) fn value_reals_raw(&self) -> *mut usize {
        view_raw_mut!(self, value_reals)
    }
    #[inline(always)]
    pub(crate) fn set_unique_per_vertex(&self, value: bool) {
        view_write!(self, unique_per_vertex, value)
    }
    #[inline(always)]
    pub(crate) fn unique_per_vertex_raw(&self) -> *mut bool {
        view_raw_mut!(self, unique_per_vertex)
    }
    #[inline(always)]
    pub(crate) fn set_values_w(&self, value: List<Real>) {
        view_write!(self, values_w, value)
    }
    #[inline(always)]
    pub(crate) fn values_w_raw(&self) -> *mut List<Real> {
        view_raw_mut!(self, values_w)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<VertexVec3, M> {
    #[inline(always)]
    pub(crate) fn exists(&self) -> bool {
        view_read_shared!(self, exists)
    }
    #[inline(always)]
    pub(crate) fn exists_ptr(&self) -> *const bool {
        view_raw_shared!(self, exists)
    }
    #[inline(always)]
    pub(crate) fn values(&self) -> List<Vec3> {
        view_read_shared!(self, values)
    }
    #[inline(always)]
    pub(crate) fn values_view(&self) -> &View<List<Vec3>, M> {
        view_project!(self, values)
    }
    #[inline(always)]
    pub(crate) fn values_ptr(&self) -> *const List<Vec3> {
        view_raw_shared!(self, values)
    }
    #[inline(always)]
    pub(crate) fn indices(&self) -> List<u32> {
        view_read_shared!(self, indices)
    }
    #[inline(always)]
    pub(crate) fn indices_view(&self) -> &View<List<u32>, M> {
        view_project!(self, indices)
    }
    #[inline(always)]
    pub(crate) fn indices_ptr(&self) -> *const List<u32> {
        view_raw_shared!(self, indices)
    }
    #[inline(always)]
    pub(crate) fn value_reals(&self) -> usize {
        view_read_shared!(self, value_reals)
    }
    #[inline(always)]
    pub(crate) fn value_reals_ptr(&self) -> *const usize {
        view_raw_shared!(self, value_reals)
    }
    #[inline(always)]
    pub(crate) fn unique_per_vertex(&self) -> bool {
        view_read_shared!(self, unique_per_vertex)
    }
    #[inline(always)]
    pub(crate) fn unique_per_vertex_ptr(&self) -> *const bool {
        view_raw_shared!(self, unique_per_vertex)
    }
    #[inline(always)]
    pub(crate) fn values_w(&self) -> List<Real> {
        view_read_shared!(self, values_w)
    }
    #[inline(always)]
    pub(crate) fn values_w_view(&self) -> &View<List<Real>, M> {
        view_project!(self, values_w)
    }
    #[inline(always)]
    pub(crate) fn values_w_ptr(&self) -> *const List<Real> {
        view_raw_shared!(self, values_w)
    }
}

#[allow(dead_code)]
impl View<VertexVec3, Mut> {
    #[inline(always)]
    pub(crate) fn set_exists(&self, value: bool) {
        view_write!(self, exists, value)
    }
    #[inline(always)]
    pub(crate) fn exists_raw(&self) -> *mut bool {
        view_raw_mut!(self, exists)
    }
    #[inline(always)]
    pub(crate) fn set_values(&self, value: List<Vec3>) {
        view_write!(self, values, value)
    }
    #[inline(always)]
    pub(crate) fn values_raw(&self) -> *mut List<Vec3> {
        view_raw_mut!(self, values)
    }
    #[inline(always)]
    pub(crate) fn set_indices(&self, value: List<u32>) {
        view_write!(self, indices, value)
    }
    #[inline(always)]
    pub(crate) fn indices_raw(&self) -> *mut List<u32> {
        view_raw_mut!(self, indices)
    }
    #[inline(always)]
    pub(crate) fn set_value_reals(&self, value: usize) {
        view_write!(self, value_reals, value)
    }
    #[inline(always)]
    pub(crate) fn value_reals_raw(&self) -> *mut usize {
        view_raw_mut!(self, value_reals)
    }
    #[inline(always)]
    pub(crate) fn set_unique_per_vertex(&self, value: bool) {
        view_write!(self, unique_per_vertex, value)
    }
    #[inline(always)]
    pub(crate) fn unique_per_vertex_raw(&self) -> *mut bool {
        view_raw_mut!(self, unique_per_vertex)
    }
    #[inline(always)]
    pub(crate) fn set_values_w(&self, value: List<Real>) {
        view_write!(self, values_w, value)
    }
    #[inline(always)]
    pub(crate) fn values_w_raw(&self) -> *mut List<Real> {
        view_raw_mut!(self, values_w)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<VertexVec4, M> {
    #[inline(always)]
    pub(crate) fn exists(&self) -> bool {
        view_read_shared!(self, exists)
    }
    #[inline(always)]
    pub(crate) fn exists_ptr(&self) -> *const bool {
        view_raw_shared!(self, exists)
    }
    #[inline(always)]
    pub(crate) fn values(&self) -> List<Vec4> {
        view_read_shared!(self, values)
    }
    #[inline(always)]
    pub(crate) fn values_view(&self) -> &View<List<Vec4>, M> {
        view_project!(self, values)
    }
    #[inline(always)]
    pub(crate) fn values_ptr(&self) -> *const List<Vec4> {
        view_raw_shared!(self, values)
    }
    #[inline(always)]
    pub(crate) fn indices(&self) -> List<u32> {
        view_read_shared!(self, indices)
    }
    #[inline(always)]
    pub(crate) fn indices_view(&self) -> &View<List<u32>, M> {
        view_project!(self, indices)
    }
    #[inline(always)]
    pub(crate) fn indices_ptr(&self) -> *const List<u32> {
        view_raw_shared!(self, indices)
    }
    #[inline(always)]
    pub(crate) fn value_reals(&self) -> usize {
        view_read_shared!(self, value_reals)
    }
    #[inline(always)]
    pub(crate) fn value_reals_ptr(&self) -> *const usize {
        view_raw_shared!(self, value_reals)
    }
    #[inline(always)]
    pub(crate) fn unique_per_vertex(&self) -> bool {
        view_read_shared!(self, unique_per_vertex)
    }
    #[inline(always)]
    pub(crate) fn unique_per_vertex_ptr(&self) -> *const bool {
        view_raw_shared!(self, unique_per_vertex)
    }
    #[inline(always)]
    pub(crate) fn values_w(&self) -> List<Real> {
        view_read_shared!(self, values_w)
    }
    #[inline(always)]
    pub(crate) fn values_w_view(&self) -> &View<List<Real>, M> {
        view_project!(self, values_w)
    }
    #[inline(always)]
    pub(crate) fn values_w_ptr(&self) -> *const List<Real> {
        view_raw_shared!(self, values_w)
    }
}

#[allow(dead_code)]
impl View<VertexVec4, Mut> {
    #[inline(always)]
    pub(crate) fn set_exists(&self, value: bool) {
        view_write!(self, exists, value)
    }
    #[inline(always)]
    pub(crate) fn exists_raw(&self) -> *mut bool {
        view_raw_mut!(self, exists)
    }
    #[inline(always)]
    pub(crate) fn set_values(&self, value: List<Vec4>) {
        view_write!(self, values, value)
    }
    #[inline(always)]
    pub(crate) fn values_raw(&self) -> *mut List<Vec4> {
        view_raw_mut!(self, values)
    }
    #[inline(always)]
    pub(crate) fn set_indices(&self, value: List<u32>) {
        view_write!(self, indices, value)
    }
    #[inline(always)]
    pub(crate) fn indices_raw(&self) -> *mut List<u32> {
        view_raw_mut!(self, indices)
    }
    #[inline(always)]
    pub(crate) fn set_value_reals(&self, value: usize) {
        view_write!(self, value_reals, value)
    }
    #[inline(always)]
    pub(crate) fn value_reals_raw(&self) -> *mut usize {
        view_raw_mut!(self, value_reals)
    }
    #[inline(always)]
    pub(crate) fn set_unique_per_vertex(&self, value: bool) {
        view_write!(self, unique_per_vertex, value)
    }
    #[inline(always)]
    pub(crate) fn unique_per_vertex_raw(&self) -> *mut bool {
        view_raw_mut!(self, unique_per_vertex)
    }
    #[inline(always)]
    pub(crate) fn set_values_w(&self, value: List<Real>) {
        view_write!(self, values_w, value)
    }
    #[inline(always)]
    pub(crate) fn values_w_raw(&self) -> *mut List<Real> {
        view_raw_mut!(self, values_w)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<UvSet, M> {
    #[inline(always)]
    pub(crate) fn name(&self) -> String {
        view_read_shared!(self, name)
    }
    #[inline(always)]
    pub(crate) fn name_view(&self) -> &View<String, M> {
        view_project!(self, name)
    }
    #[inline(always)]
    pub(crate) fn name_ptr(&self) -> *const String {
        view_raw_shared!(self, name)
    }
    #[inline(always)]
    pub(crate) fn index(&self) -> u32 {
        view_read_shared!(self, index)
    }
    #[inline(always)]
    pub(crate) fn index_ptr(&self) -> *const u32 {
        view_raw_shared!(self, index)
    }
    #[inline(always)]
    pub(crate) fn vertex_uv(&self) -> &View<VertexVec2, M> {
        view_project!(self, vertex_uv)
    }
    #[inline(always)]
    pub(crate) fn vertex_uv_ptr(&self) -> *const VertexVec2 {
        view_raw_shared!(self, vertex_uv)
    }
    #[inline(always)]
    pub(crate) fn vertex_tangent(&self) -> &View<VertexVec3, M> {
        view_project!(self, vertex_tangent)
    }
    #[inline(always)]
    pub(crate) fn vertex_tangent_ptr(&self) -> *const VertexVec3 {
        view_raw_shared!(self, vertex_tangent)
    }
    #[inline(always)]
    pub(crate) fn vertex_bitangent(&self) -> &View<VertexVec3, M> {
        view_project!(self, vertex_bitangent)
    }
    #[inline(always)]
    pub(crate) fn vertex_bitangent_ptr(&self) -> *const VertexVec3 {
        view_raw_shared!(self, vertex_bitangent)
    }
}

#[allow(dead_code)]
impl View<UvSet, Mut> {
    #[inline(always)]
    pub(crate) fn set_name(&self, value: String) {
        view_write!(self, name, value)
    }
    #[inline(always)]
    pub(crate) fn name_raw(&self) -> *mut String {
        view_raw_mut!(self, name)
    }
    #[inline(always)]
    pub(crate) fn set_index(&self, value: u32) {
        view_write!(self, index, value)
    }
    #[inline(always)]
    pub(crate) fn index_raw(&self) -> *mut u32 {
        view_raw_mut!(self, index)
    }
    #[inline(always)]
    pub(crate) fn set_vertex_uv(&self, value: VertexVec2) {
        view_write!(self, vertex_uv, value)
    }
    #[inline(always)]
    pub(crate) fn vertex_uv_raw(&self) -> *mut VertexVec2 {
        view_raw_mut!(self, vertex_uv)
    }
    #[inline(always)]
    pub(crate) fn set_vertex_tangent(&self, value: VertexVec3) {
        view_write!(self, vertex_tangent, value)
    }
    #[inline(always)]
    pub(crate) fn vertex_tangent_raw(&self) -> *mut VertexVec3 {
        view_raw_mut!(self, vertex_tangent)
    }
    #[inline(always)]
    pub(crate) fn set_vertex_bitangent(&self, value: VertexVec3) {
        view_write!(self, vertex_bitangent, value)
    }
    #[inline(always)]
    pub(crate) fn vertex_bitangent_raw(&self) -> *mut VertexVec3 {
        view_raw_mut!(self, vertex_bitangent)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<ColorSet, M> {
    #[inline(always)]
    pub(crate) fn name(&self) -> String {
        view_read_shared!(self, name)
    }
    #[inline(always)]
    pub(crate) fn name_view(&self) -> &View<String, M> {
        view_project!(self, name)
    }
    #[inline(always)]
    pub(crate) fn name_ptr(&self) -> *const String {
        view_raw_shared!(self, name)
    }
    #[inline(always)]
    pub(crate) fn index(&self) -> u32 {
        view_read_shared!(self, index)
    }
    #[inline(always)]
    pub(crate) fn index_ptr(&self) -> *const u32 {
        view_raw_shared!(self, index)
    }
    #[inline(always)]
    pub(crate) fn vertex_color(&self) -> &View<VertexVec4, M> {
        view_project!(self, vertex_color)
    }
    #[inline(always)]
    pub(crate) fn vertex_color_ptr(&self) -> *const VertexVec4 {
        view_raw_shared!(self, vertex_color)
    }
}

#[allow(dead_code)]
impl View<ColorSet, Mut> {
    #[inline(always)]
    pub(crate) fn set_name(&self, value: String) {
        view_write!(self, name, value)
    }
    #[inline(always)]
    pub(crate) fn name_raw(&self) -> *mut String {
        view_raw_mut!(self, name)
    }
    #[inline(always)]
    pub(crate) fn set_index(&self, value: u32) {
        view_write!(self, index, value)
    }
    #[inline(always)]
    pub(crate) fn index_raw(&self) -> *mut u32 {
        view_raw_mut!(self, index)
    }
    #[inline(always)]
    pub(crate) fn set_vertex_color(&self, value: VertexVec4) {
        view_write!(self, vertex_color, value)
    }
    #[inline(always)]
    pub(crate) fn vertex_color_raw(&self) -> *mut VertexVec4 {
        view_raw_mut!(self, vertex_color)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<MeshPart, M> {
    #[inline(always)]
    pub(crate) fn index(&self) -> u32 {
        view_read_shared!(self, index)
    }
    #[inline(always)]
    pub(crate) fn index_ptr(&self) -> *const u32 {
        view_raw_shared!(self, index)
    }
    #[inline(always)]
    pub(crate) fn num_faces(&self) -> usize {
        view_read_shared!(self, num_faces)
    }
    #[inline(always)]
    pub(crate) fn num_faces_ptr(&self) -> *const usize {
        view_raw_shared!(self, num_faces)
    }
    #[inline(always)]
    pub(crate) fn num_triangles(&self) -> usize {
        view_read_shared!(self, num_triangles)
    }
    #[inline(always)]
    pub(crate) fn num_triangles_ptr(&self) -> *const usize {
        view_raw_shared!(self, num_triangles)
    }
    #[inline(always)]
    pub(crate) fn num_empty_faces(&self) -> usize {
        view_read_shared!(self, num_empty_faces)
    }
    #[inline(always)]
    pub(crate) fn num_empty_faces_ptr(&self) -> *const usize {
        view_raw_shared!(self, num_empty_faces)
    }
    #[inline(always)]
    pub(crate) fn num_point_faces(&self) -> usize {
        view_read_shared!(self, num_point_faces)
    }
    #[inline(always)]
    pub(crate) fn num_point_faces_ptr(&self) -> *const usize {
        view_raw_shared!(self, num_point_faces)
    }
    #[inline(always)]
    pub(crate) fn num_line_faces(&self) -> usize {
        view_read_shared!(self, num_line_faces)
    }
    #[inline(always)]
    pub(crate) fn num_line_faces_ptr(&self) -> *const usize {
        view_raw_shared!(self, num_line_faces)
    }
    #[inline(always)]
    pub(crate) fn face_indices(&self) -> List<u32> {
        view_read_shared!(self, face_indices)
    }
    #[inline(always)]
    pub(crate) fn face_indices_view(&self) -> &View<List<u32>, M> {
        view_project!(self, face_indices)
    }
    #[inline(always)]
    pub(crate) fn face_indices_ptr(&self) -> *const List<u32> {
        view_raw_shared!(self, face_indices)
    }
}

#[allow(dead_code)]
impl View<MeshPart, Mut> {
    #[inline(always)]
    pub(crate) fn set_index(&self, value: u32) {
        view_write!(self, index, value)
    }
    #[inline(always)]
    pub(crate) fn index_raw(&self) -> *mut u32 {
        view_raw_mut!(self, index)
    }
    #[inline(always)]
    pub(crate) fn set_num_faces(&self, value: usize) {
        view_write!(self, num_faces, value)
    }
    #[inline(always)]
    pub(crate) fn num_faces_raw(&self) -> *mut usize {
        view_raw_mut!(self, num_faces)
    }
    #[inline(always)]
    pub(crate) fn set_num_triangles(&self, value: usize) {
        view_write!(self, num_triangles, value)
    }
    #[inline(always)]
    pub(crate) fn num_triangles_raw(&self) -> *mut usize {
        view_raw_mut!(self, num_triangles)
    }
    #[inline(always)]
    pub(crate) fn set_num_empty_faces(&self, value: usize) {
        view_write!(self, num_empty_faces, value)
    }
    #[inline(always)]
    pub(crate) fn num_empty_faces_raw(&self) -> *mut usize {
        view_raw_mut!(self, num_empty_faces)
    }
    #[inline(always)]
    pub(crate) fn set_num_point_faces(&self, value: usize) {
        view_write!(self, num_point_faces, value)
    }
    #[inline(always)]
    pub(crate) fn num_point_faces_raw(&self) -> *mut usize {
        view_raw_mut!(self, num_point_faces)
    }
    #[inline(always)]
    pub(crate) fn set_num_line_faces(&self, value: usize) {
        view_write!(self, num_line_faces, value)
    }
    #[inline(always)]
    pub(crate) fn num_line_faces_raw(&self) -> *mut usize {
        view_raw_mut!(self, num_line_faces)
    }
    #[inline(always)]
    pub(crate) fn set_face_indices(&self, value: List<u32>) {
        view_write!(self, face_indices, value)
    }
    #[inline(always)]
    pub(crate) fn face_indices_raw(&self) -> *mut List<u32> {
        view_raw_mut!(self, face_indices)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<FaceGroup, M> {
    #[inline(always)]
    pub(crate) fn id(&self) -> i32 {
        view_read_shared!(self, id)
    }
    #[inline(always)]
    pub(crate) fn id_ptr(&self) -> *const i32 {
        view_raw_shared!(self, id)
    }
    #[inline(always)]
    pub(crate) fn name(&self) -> String {
        view_read_shared!(self, name)
    }
    #[inline(always)]
    pub(crate) fn name_view(&self) -> &View<String, M> {
        view_project!(self, name)
    }
    #[inline(always)]
    pub(crate) fn name_ptr(&self) -> *const String {
        view_raw_shared!(self, name)
    }
}

#[allow(dead_code)]
impl View<FaceGroup, Mut> {
    #[inline(always)]
    pub(crate) fn set_id(&self, value: i32) {
        view_write!(self, id, value)
    }
    #[inline(always)]
    pub(crate) fn id_raw(&self) -> *mut i32 {
        view_raw_mut!(self, id)
    }
    #[inline(always)]
    pub(crate) fn set_name(&self, value: String) {
        view_write!(self, name, value)
    }
    #[inline(always)]
    pub(crate) fn name_raw(&self) -> *mut String {
        view_raw_mut!(self, name)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<Mesh, M> {
    #[inline(always)]
    pub(crate) fn element(&self) -> &View<Element, M> {
        view_project!(self, element)
    }
    #[inline(always)]
    pub(crate) fn element_ptr(&self) -> *const Element {
        view_raw_shared!(self, element)
    }
    #[inline(always)]
    pub(crate) fn num_vertices(&self) -> usize {
        view_read_shared!(self, num_vertices)
    }
    #[inline(always)]
    pub(crate) fn num_vertices_ptr(&self) -> *const usize {
        view_raw_shared!(self, num_vertices)
    }
    #[inline(always)]
    pub(crate) fn num_indices(&self) -> usize {
        view_read_shared!(self, num_indices)
    }
    #[inline(always)]
    pub(crate) fn num_indices_ptr(&self) -> *const usize {
        view_raw_shared!(self, num_indices)
    }
    #[inline(always)]
    pub(crate) fn num_faces(&self) -> usize {
        view_read_shared!(self, num_faces)
    }
    #[inline(always)]
    pub(crate) fn num_faces_ptr(&self) -> *const usize {
        view_raw_shared!(self, num_faces)
    }
    #[inline(always)]
    pub(crate) fn num_triangles(&self) -> usize {
        view_read_shared!(self, num_triangles)
    }
    #[inline(always)]
    pub(crate) fn num_triangles_ptr(&self) -> *const usize {
        view_raw_shared!(self, num_triangles)
    }
    #[inline(always)]
    pub(crate) fn num_edges(&self) -> usize {
        view_read_shared!(self, num_edges)
    }
    #[inline(always)]
    pub(crate) fn num_edges_ptr(&self) -> *const usize {
        view_raw_shared!(self, num_edges)
    }
    #[inline(always)]
    pub(crate) fn max_face_triangles(&self) -> usize {
        view_read_shared!(self, max_face_triangles)
    }
    #[inline(always)]
    pub(crate) fn max_face_triangles_ptr(&self) -> *const usize {
        view_raw_shared!(self, max_face_triangles)
    }
    #[inline(always)]
    pub(crate) fn num_empty_faces(&self) -> usize {
        view_read_shared!(self, num_empty_faces)
    }
    #[inline(always)]
    pub(crate) fn num_empty_faces_ptr(&self) -> *const usize {
        view_raw_shared!(self, num_empty_faces)
    }
    #[inline(always)]
    pub(crate) fn num_point_faces(&self) -> usize {
        view_read_shared!(self, num_point_faces)
    }
    #[inline(always)]
    pub(crate) fn num_point_faces_ptr(&self) -> *const usize {
        view_raw_shared!(self, num_point_faces)
    }
    #[inline(always)]
    pub(crate) fn num_line_faces(&self) -> usize {
        view_read_shared!(self, num_line_faces)
    }
    #[inline(always)]
    pub(crate) fn num_line_faces_ptr(&self) -> *const usize {
        view_raw_shared!(self, num_line_faces)
    }
    #[inline(always)]
    pub(crate) fn faces(&self) -> List<Face> {
        view_read_shared!(self, faces)
    }
    #[inline(always)]
    pub(crate) fn faces_view(&self) -> &View<List<Face>, M> {
        view_project!(self, faces)
    }
    #[inline(always)]
    pub(crate) fn faces_ptr(&self) -> *const List<Face> {
        view_raw_shared!(self, faces)
    }
    #[inline(always)]
    pub(crate) fn face_smoothing(&self) -> List<bool> {
        view_read_shared!(self, face_smoothing)
    }
    #[inline(always)]
    pub(crate) fn face_smoothing_view(&self) -> &View<List<bool>, M> {
        view_project!(self, face_smoothing)
    }
    #[inline(always)]
    pub(crate) fn face_smoothing_ptr(&self) -> *const List<bool> {
        view_raw_shared!(self, face_smoothing)
    }
    #[inline(always)]
    pub(crate) fn face_material(&self) -> List<u32> {
        view_read_shared!(self, face_material)
    }
    #[inline(always)]
    pub(crate) fn face_material_view(&self) -> &View<List<u32>, M> {
        view_project!(self, face_material)
    }
    #[inline(always)]
    pub(crate) fn face_material_ptr(&self) -> *const List<u32> {
        view_raw_shared!(self, face_material)
    }
    #[inline(always)]
    pub(crate) fn face_group(&self) -> List<u32> {
        view_read_shared!(self, face_group)
    }
    #[inline(always)]
    pub(crate) fn face_group_view(&self) -> &View<List<u32>, M> {
        view_project!(self, face_group)
    }
    #[inline(always)]
    pub(crate) fn face_group_ptr(&self) -> *const List<u32> {
        view_raw_shared!(self, face_group)
    }
    #[inline(always)]
    pub(crate) fn face_hole(&self) -> List<bool> {
        view_read_shared!(self, face_hole)
    }
    #[inline(always)]
    pub(crate) fn face_hole_view(&self) -> &View<List<bool>, M> {
        view_project!(self, face_hole)
    }
    #[inline(always)]
    pub(crate) fn face_hole_ptr(&self) -> *const List<bool> {
        view_raw_shared!(self, face_hole)
    }
    #[inline(always)]
    pub(crate) fn edges(&self) -> List<Edge> {
        view_read_shared!(self, edges)
    }
    #[inline(always)]
    pub(crate) fn edges_view(&self) -> &View<List<Edge>, M> {
        view_project!(self, edges)
    }
    #[inline(always)]
    pub(crate) fn edges_ptr(&self) -> *const List<Edge> {
        view_raw_shared!(self, edges)
    }
    #[inline(always)]
    pub(crate) fn edge_smoothing(&self) -> List<bool> {
        view_read_shared!(self, edge_smoothing)
    }
    #[inline(always)]
    pub(crate) fn edge_smoothing_view(&self) -> &View<List<bool>, M> {
        view_project!(self, edge_smoothing)
    }
    #[inline(always)]
    pub(crate) fn edge_smoothing_ptr(&self) -> *const List<bool> {
        view_raw_shared!(self, edge_smoothing)
    }
    #[inline(always)]
    pub(crate) fn edge_crease(&self) -> List<Real> {
        view_read_shared!(self, edge_crease)
    }
    #[inline(always)]
    pub(crate) fn edge_crease_view(&self) -> &View<List<Real>, M> {
        view_project!(self, edge_crease)
    }
    #[inline(always)]
    pub(crate) fn edge_crease_ptr(&self) -> *const List<Real> {
        view_raw_shared!(self, edge_crease)
    }
    #[inline(always)]
    pub(crate) fn edge_visibility(&self) -> List<bool> {
        view_read_shared!(self, edge_visibility)
    }
    #[inline(always)]
    pub(crate) fn edge_visibility_view(&self) -> &View<List<bool>, M> {
        view_project!(self, edge_visibility)
    }
    #[inline(always)]
    pub(crate) fn edge_visibility_ptr(&self) -> *const List<bool> {
        view_raw_shared!(self, edge_visibility)
    }
    #[inline(always)]
    pub(crate) fn vertex_indices(&self) -> List<u32> {
        view_read_shared!(self, vertex_indices)
    }
    #[inline(always)]
    pub(crate) fn vertex_indices_view(&self) -> &View<List<u32>, M> {
        view_project!(self, vertex_indices)
    }
    #[inline(always)]
    pub(crate) fn vertex_indices_ptr(&self) -> *const List<u32> {
        view_raw_shared!(self, vertex_indices)
    }
    #[inline(always)]
    pub(crate) fn vertices(&self) -> List<Vec3> {
        view_read_shared!(self, vertices)
    }
    #[inline(always)]
    pub(crate) fn vertices_view(&self) -> &View<List<Vec3>, M> {
        view_project!(self, vertices)
    }
    #[inline(always)]
    pub(crate) fn vertices_ptr(&self) -> *const List<Vec3> {
        view_raw_shared!(self, vertices)
    }
    #[inline(always)]
    pub(crate) fn vertex_first_index(&self) -> List<u32> {
        view_read_shared!(self, vertex_first_index)
    }
    #[inline(always)]
    pub(crate) fn vertex_first_index_view(&self) -> &View<List<u32>, M> {
        view_project!(self, vertex_first_index)
    }
    #[inline(always)]
    pub(crate) fn vertex_first_index_ptr(&self) -> *const List<u32> {
        view_raw_shared!(self, vertex_first_index)
    }
    #[inline(always)]
    pub(crate) fn vertex_position(&self) -> &View<VertexVec3, M> {
        view_project!(self, vertex_position)
    }
    #[inline(always)]
    pub(crate) fn vertex_position_ptr(&self) -> *const VertexVec3 {
        view_raw_shared!(self, vertex_position)
    }
    #[inline(always)]
    pub(crate) fn vertex_normal(&self) -> &View<VertexVec3, M> {
        view_project!(self, vertex_normal)
    }
    #[inline(always)]
    pub(crate) fn vertex_normal_ptr(&self) -> *const VertexVec3 {
        view_raw_shared!(self, vertex_normal)
    }
    #[inline(always)]
    pub(crate) fn vertex_uv(&self) -> &View<VertexVec2, M> {
        view_project!(self, vertex_uv)
    }
    #[inline(always)]
    pub(crate) fn vertex_uv_ptr(&self) -> *const VertexVec2 {
        view_raw_shared!(self, vertex_uv)
    }
    #[inline(always)]
    pub(crate) fn vertex_tangent(&self) -> &View<VertexVec3, M> {
        view_project!(self, vertex_tangent)
    }
    #[inline(always)]
    pub(crate) fn vertex_tangent_ptr(&self) -> *const VertexVec3 {
        view_raw_shared!(self, vertex_tangent)
    }
    #[inline(always)]
    pub(crate) fn vertex_bitangent(&self) -> &View<VertexVec3, M> {
        view_project!(self, vertex_bitangent)
    }
    #[inline(always)]
    pub(crate) fn vertex_bitangent_ptr(&self) -> *const VertexVec3 {
        view_raw_shared!(self, vertex_bitangent)
    }
    #[inline(always)]
    pub(crate) fn vertex_color(&self) -> &View<VertexVec4, M> {
        view_project!(self, vertex_color)
    }
    #[inline(always)]
    pub(crate) fn vertex_color_ptr(&self) -> *const VertexVec4 {
        view_raw_shared!(self, vertex_color)
    }
    #[inline(always)]
    pub(crate) fn vertex_crease(&self) -> &View<VertexReal, M> {
        view_project!(self, vertex_crease)
    }
    #[inline(always)]
    pub(crate) fn vertex_crease_ptr(&self) -> *const VertexReal {
        view_raw_shared!(self, vertex_crease)
    }
    #[inline(always)]
    pub(crate) fn uv_sets(&self) -> List<UvSet> {
        view_read_shared!(self, uv_sets)
    }
    #[inline(always)]
    pub(crate) fn uv_sets_view(&self) -> &View<List<UvSet>, M> {
        view_project!(self, uv_sets)
    }
    #[inline(always)]
    pub(crate) fn uv_sets_ptr(&self) -> *const List<UvSet> {
        view_raw_shared!(self, uv_sets)
    }
    #[inline(always)]
    pub(crate) fn color_sets(&self) -> List<ColorSet> {
        view_read_shared!(self, color_sets)
    }
    #[inline(always)]
    pub(crate) fn color_sets_view(&self) -> &View<List<ColorSet>, M> {
        view_project!(self, color_sets)
    }
    #[inline(always)]
    pub(crate) fn color_sets_ptr(&self) -> *const List<ColorSet> {
        view_raw_shared!(self, color_sets)
    }
    #[inline(always)]
    pub(crate) fn materials(&self) -> RefList<Material> {
        view_read_shared!(self, materials)
    }
    #[inline(always)]
    pub(crate) fn materials_view(&self) -> &View<RefList<Material>, M> {
        view_project!(self, materials)
    }
    #[inline(always)]
    pub(crate) fn materials_ptr(&self) -> *const RefList<Material> {
        view_raw_shared!(self, materials)
    }
    #[inline(always)]
    pub(crate) fn face_groups(&self) -> List<FaceGroup> {
        view_read_shared!(self, face_groups)
    }
    #[inline(always)]
    pub(crate) fn face_groups_view(&self) -> &View<List<FaceGroup>, M> {
        view_project!(self, face_groups)
    }
    #[inline(always)]
    pub(crate) fn face_groups_ptr(&self) -> *const List<FaceGroup> {
        view_raw_shared!(self, face_groups)
    }
    #[inline(always)]
    pub(crate) fn material_parts(&self) -> List<MeshPart> {
        view_read_shared!(self, material_parts)
    }
    #[inline(always)]
    pub(crate) fn material_parts_view(&self) -> &View<List<MeshPart>, M> {
        view_project!(self, material_parts)
    }
    #[inline(always)]
    pub(crate) fn material_parts_ptr(&self) -> *const List<MeshPart> {
        view_raw_shared!(self, material_parts)
    }
    #[inline(always)]
    pub(crate) fn face_group_parts(&self) -> List<MeshPart> {
        view_read_shared!(self, face_group_parts)
    }
    #[inline(always)]
    pub(crate) fn face_group_parts_view(&self) -> &View<List<MeshPart>, M> {
        view_project!(self, face_group_parts)
    }
    #[inline(always)]
    pub(crate) fn face_group_parts_ptr(&self) -> *const List<MeshPart> {
        view_raw_shared!(self, face_group_parts)
    }
    #[inline(always)]
    pub(crate) fn material_part_usage_order(&self) -> List<u32> {
        view_read_shared!(self, material_part_usage_order)
    }
    #[inline(always)]
    pub(crate) fn material_part_usage_order_view(&self) -> &View<List<u32>, M> {
        view_project!(self, material_part_usage_order)
    }
    #[inline(always)]
    pub(crate) fn material_part_usage_order_ptr(&self) -> *const List<u32> {
        view_raw_shared!(self, material_part_usage_order)
    }
    #[inline(always)]
    pub(crate) fn skinned_is_local(&self) -> bool {
        view_read_shared!(self, skinned_is_local)
    }
    #[inline(always)]
    pub(crate) fn skinned_is_local_ptr(&self) -> *const bool {
        view_raw_shared!(self, skinned_is_local)
    }
    #[inline(always)]
    pub(crate) fn skinned_position(&self) -> &View<VertexVec3, M> {
        view_project!(self, skinned_position)
    }
    #[inline(always)]
    pub(crate) fn skinned_position_ptr(&self) -> *const VertexVec3 {
        view_raw_shared!(self, skinned_position)
    }
    #[inline(always)]
    pub(crate) fn skinned_normal(&self) -> &View<VertexVec3, M> {
        view_project!(self, skinned_normal)
    }
    #[inline(always)]
    pub(crate) fn skinned_normal_ptr(&self) -> *const VertexVec3 {
        view_raw_shared!(self, skinned_normal)
    }
    #[inline(always)]
    pub(crate) fn skin_deformers(&self) -> RefList<SkinDeformer> {
        view_read_shared!(self, skin_deformers)
    }
    #[inline(always)]
    pub(crate) fn skin_deformers_view(&self) -> &View<RefList<SkinDeformer>, M> {
        view_project!(self, skin_deformers)
    }
    #[inline(always)]
    pub(crate) fn skin_deformers_ptr(&self) -> *const RefList<SkinDeformer> {
        view_raw_shared!(self, skin_deformers)
    }
    #[inline(always)]
    pub(crate) fn blend_deformers(&self) -> RefList<BlendDeformer> {
        view_read_shared!(self, blend_deformers)
    }
    #[inline(always)]
    pub(crate) fn blend_deformers_view(&self) -> &View<RefList<BlendDeformer>, M> {
        view_project!(self, blend_deformers)
    }
    #[inline(always)]
    pub(crate) fn blend_deformers_ptr(&self) -> *const RefList<BlendDeformer> {
        view_raw_shared!(self, blend_deformers)
    }
    #[inline(always)]
    pub(crate) fn cache_deformers(&self) -> RefList<CacheDeformer> {
        view_read_shared!(self, cache_deformers)
    }
    #[inline(always)]
    pub(crate) fn cache_deformers_view(&self) -> &View<RefList<CacheDeformer>, M> {
        view_project!(self, cache_deformers)
    }
    #[inline(always)]
    pub(crate) fn cache_deformers_ptr(&self) -> *const RefList<CacheDeformer> {
        view_raw_shared!(self, cache_deformers)
    }
    #[inline(always)]
    pub(crate) fn all_deformers(&self) -> RefList<Element> {
        view_read_shared!(self, all_deformers)
    }
    #[inline(always)]
    pub(crate) fn all_deformers_view(&self) -> &View<RefList<Element>, M> {
        view_project!(self, all_deformers)
    }
    #[inline(always)]
    pub(crate) fn all_deformers_ptr(&self) -> *const RefList<Element> {
        view_raw_shared!(self, all_deformers)
    }
    #[inline(always)]
    pub(crate) fn subdivision_preview_levels(&self) -> u32 {
        view_read_shared!(self, subdivision_preview_levels)
    }
    #[inline(always)]
    pub(crate) fn subdivision_preview_levels_ptr(&self) -> *const u32 {
        view_raw_shared!(self, subdivision_preview_levels)
    }
    #[inline(always)]
    pub(crate) fn subdivision_render_levels(&self) -> u32 {
        view_read_shared!(self, subdivision_render_levels)
    }
    #[inline(always)]
    pub(crate) fn subdivision_render_levels_ptr(&self) -> *const u32 {
        view_raw_shared!(self, subdivision_render_levels)
    }
    #[inline(always)]
    pub(crate) fn subdivision_display_mode(&self) -> SubdivisionDisplayMode {
        view_read_shared!(self, subdivision_display_mode)
    }
    #[inline(always)]
    pub(crate) fn subdivision_display_mode_ptr(&self) -> *const SubdivisionDisplayMode {
        view_raw_shared!(self, subdivision_display_mode)
    }
    #[inline(always)]
    pub(crate) fn subdivision_boundary(&self) -> SubdivisionBoundary {
        view_read_shared!(self, subdivision_boundary)
    }
    #[inline(always)]
    pub(crate) fn subdivision_boundary_ptr(&self) -> *const SubdivisionBoundary {
        view_raw_shared!(self, subdivision_boundary)
    }
    #[inline(always)]
    pub(crate) fn subdivision_uv_boundary(&self) -> SubdivisionBoundary {
        view_read_shared!(self, subdivision_uv_boundary)
    }
    #[inline(always)]
    pub(crate) fn subdivision_uv_boundary_ptr(&self) -> *const SubdivisionBoundary {
        view_raw_shared!(self, subdivision_uv_boundary)
    }
    #[inline(always)]
    pub(crate) fn reversed_winding(&self) -> bool {
        view_read_shared!(self, reversed_winding)
    }
    #[inline(always)]
    pub(crate) fn reversed_winding_ptr(&self) -> *const bool {
        view_raw_shared!(self, reversed_winding)
    }
    #[inline(always)]
    pub(crate) fn generated_normals(&self) -> bool {
        view_read_shared!(self, generated_normals)
    }
    #[inline(always)]
    pub(crate) fn generated_normals_ptr(&self) -> *const bool {
        view_raw_shared!(self, generated_normals)
    }
    #[inline(always)]
    pub(crate) fn subdivision_evaluated(&self) -> bool {
        view_read_shared!(self, subdivision_evaluated)
    }
    #[inline(always)]
    pub(crate) fn subdivision_evaluated_ptr(&self) -> *const bool {
        view_raw_shared!(self, subdivision_evaluated)
    }
    #[inline(always)]
    pub(crate) fn subdivision_result(&self) -> Option<Ref<SubdivisionResult>> {
        view_read_shared!(self, subdivision_result)
    }
    #[inline(always)]
    pub(crate) fn subdivision_result_ptr(&self) -> *const Option<Ref<SubdivisionResult>> {
        view_raw_shared!(self, subdivision_result)
    }
    #[allow(clippy::wrong_self_convention)]
    #[inline(always)]
    pub(crate) fn from_tessellated_nurbs(&self) -> bool {
        view_read_shared!(self, from_tessellated_nurbs)
    }
    #[allow(clippy::wrong_self_convention)]
    #[inline(always)]
    pub(crate) fn from_tessellated_nurbs_ptr(&self) -> *const bool {
        view_raw_shared!(self, from_tessellated_nurbs)
    }
}

#[allow(dead_code)]
impl View<Mesh, Mut> {
    #[inline(always)]
    pub(crate) fn set_element(&self, value: Element) {
        view_write!(self, element, value)
    }
    #[inline(always)]
    pub(crate) fn element_raw(&self) -> *mut Element {
        view_raw_mut!(self, element)
    }
    #[inline(always)]
    pub(crate) fn set_num_vertices(&self, value: usize) {
        view_write!(self, num_vertices, value)
    }
    #[inline(always)]
    pub(crate) fn num_vertices_raw(&self) -> *mut usize {
        view_raw_mut!(self, num_vertices)
    }
    #[inline(always)]
    pub(crate) fn set_num_indices(&self, value: usize) {
        view_write!(self, num_indices, value)
    }
    #[inline(always)]
    pub(crate) fn num_indices_raw(&self) -> *mut usize {
        view_raw_mut!(self, num_indices)
    }
    #[inline(always)]
    pub(crate) fn set_num_faces(&self, value: usize) {
        view_write!(self, num_faces, value)
    }
    #[inline(always)]
    pub(crate) fn num_faces_raw(&self) -> *mut usize {
        view_raw_mut!(self, num_faces)
    }
    #[inline(always)]
    pub(crate) fn set_num_triangles(&self, value: usize) {
        view_write!(self, num_triangles, value)
    }
    #[inline(always)]
    pub(crate) fn num_triangles_raw(&self) -> *mut usize {
        view_raw_mut!(self, num_triangles)
    }
    #[inline(always)]
    pub(crate) fn set_num_edges(&self, value: usize) {
        view_write!(self, num_edges, value)
    }
    #[inline(always)]
    pub(crate) fn num_edges_raw(&self) -> *mut usize {
        view_raw_mut!(self, num_edges)
    }
    #[inline(always)]
    pub(crate) fn set_max_face_triangles(&self, value: usize) {
        view_write!(self, max_face_triangles, value)
    }
    #[inline(always)]
    pub(crate) fn max_face_triangles_raw(&self) -> *mut usize {
        view_raw_mut!(self, max_face_triangles)
    }
    #[inline(always)]
    pub(crate) fn set_num_empty_faces(&self, value: usize) {
        view_write!(self, num_empty_faces, value)
    }
    #[inline(always)]
    pub(crate) fn num_empty_faces_raw(&self) -> *mut usize {
        view_raw_mut!(self, num_empty_faces)
    }
    #[inline(always)]
    pub(crate) fn set_num_point_faces(&self, value: usize) {
        view_write!(self, num_point_faces, value)
    }
    #[inline(always)]
    pub(crate) fn num_point_faces_raw(&self) -> *mut usize {
        view_raw_mut!(self, num_point_faces)
    }
    #[inline(always)]
    pub(crate) fn set_num_line_faces(&self, value: usize) {
        view_write!(self, num_line_faces, value)
    }
    #[inline(always)]
    pub(crate) fn num_line_faces_raw(&self) -> *mut usize {
        view_raw_mut!(self, num_line_faces)
    }
    #[inline(always)]
    pub(crate) fn set_faces(&self, value: List<Face>) {
        view_write!(self, faces, value)
    }
    #[inline(always)]
    pub(crate) fn faces_raw(&self) -> *mut List<Face> {
        view_raw_mut!(self, faces)
    }
    #[inline(always)]
    pub(crate) fn set_face_smoothing(&self, value: List<bool>) {
        view_write!(self, face_smoothing, value)
    }
    #[inline(always)]
    pub(crate) fn face_smoothing_raw(&self) -> *mut List<bool> {
        view_raw_mut!(self, face_smoothing)
    }
    #[inline(always)]
    pub(crate) fn set_face_material(&self, value: List<u32>) {
        view_write!(self, face_material, value)
    }
    #[inline(always)]
    pub(crate) fn face_material_raw(&self) -> *mut List<u32> {
        view_raw_mut!(self, face_material)
    }
    #[inline(always)]
    pub(crate) fn set_face_group(&self, value: List<u32>) {
        view_write!(self, face_group, value)
    }
    #[inline(always)]
    pub(crate) fn face_group_raw(&self) -> *mut List<u32> {
        view_raw_mut!(self, face_group)
    }
    #[inline(always)]
    pub(crate) fn set_face_hole(&self, value: List<bool>) {
        view_write!(self, face_hole, value)
    }
    #[inline(always)]
    pub(crate) fn face_hole_raw(&self) -> *mut List<bool> {
        view_raw_mut!(self, face_hole)
    }
    #[inline(always)]
    pub(crate) fn set_edges(&self, value: List<Edge>) {
        view_write!(self, edges, value)
    }
    #[inline(always)]
    pub(crate) fn edges_raw(&self) -> *mut List<Edge> {
        view_raw_mut!(self, edges)
    }
    #[inline(always)]
    pub(crate) fn set_edge_smoothing(&self, value: List<bool>) {
        view_write!(self, edge_smoothing, value)
    }
    #[inline(always)]
    pub(crate) fn edge_smoothing_raw(&self) -> *mut List<bool> {
        view_raw_mut!(self, edge_smoothing)
    }
    #[inline(always)]
    pub(crate) fn set_edge_crease(&self, value: List<Real>) {
        view_write!(self, edge_crease, value)
    }
    #[inline(always)]
    pub(crate) fn edge_crease_raw(&self) -> *mut List<Real> {
        view_raw_mut!(self, edge_crease)
    }
    #[inline(always)]
    pub(crate) fn set_edge_visibility(&self, value: List<bool>) {
        view_write!(self, edge_visibility, value)
    }
    #[inline(always)]
    pub(crate) fn edge_visibility_raw(&self) -> *mut List<bool> {
        view_raw_mut!(self, edge_visibility)
    }
    #[inline(always)]
    pub(crate) fn set_vertex_indices(&self, value: List<u32>) {
        view_write!(self, vertex_indices, value)
    }
    #[inline(always)]
    pub(crate) fn vertex_indices_raw(&self) -> *mut List<u32> {
        view_raw_mut!(self, vertex_indices)
    }
    #[inline(always)]
    pub(crate) fn set_vertices(&self, value: List<Vec3>) {
        view_write!(self, vertices, value)
    }
    #[inline(always)]
    pub(crate) fn vertices_raw(&self) -> *mut List<Vec3> {
        view_raw_mut!(self, vertices)
    }
    #[inline(always)]
    pub(crate) fn set_vertex_first_index(&self, value: List<u32>) {
        view_write!(self, vertex_first_index, value)
    }
    #[inline(always)]
    pub(crate) fn vertex_first_index_raw(&self) -> *mut List<u32> {
        view_raw_mut!(self, vertex_first_index)
    }
    #[inline(always)]
    pub(crate) fn set_vertex_position(&self, value: VertexVec3) {
        view_write!(self, vertex_position, value)
    }
    #[inline(always)]
    pub(crate) fn vertex_position_raw(&self) -> *mut VertexVec3 {
        view_raw_mut!(self, vertex_position)
    }
    #[inline(always)]
    pub(crate) fn set_vertex_normal(&self, value: VertexVec3) {
        view_write!(self, vertex_normal, value)
    }
    #[inline(always)]
    pub(crate) fn vertex_normal_raw(&self) -> *mut VertexVec3 {
        view_raw_mut!(self, vertex_normal)
    }
    #[inline(always)]
    pub(crate) fn set_vertex_uv(&self, value: VertexVec2) {
        view_write!(self, vertex_uv, value)
    }
    #[inline(always)]
    pub(crate) fn vertex_uv_raw(&self) -> *mut VertexVec2 {
        view_raw_mut!(self, vertex_uv)
    }
    #[inline(always)]
    pub(crate) fn set_vertex_tangent(&self, value: VertexVec3) {
        view_write!(self, vertex_tangent, value)
    }
    #[inline(always)]
    pub(crate) fn vertex_tangent_raw(&self) -> *mut VertexVec3 {
        view_raw_mut!(self, vertex_tangent)
    }
    #[inline(always)]
    pub(crate) fn set_vertex_bitangent(&self, value: VertexVec3) {
        view_write!(self, vertex_bitangent, value)
    }
    #[inline(always)]
    pub(crate) fn vertex_bitangent_raw(&self) -> *mut VertexVec3 {
        view_raw_mut!(self, vertex_bitangent)
    }
    #[inline(always)]
    pub(crate) fn set_vertex_color(&self, value: VertexVec4) {
        view_write!(self, vertex_color, value)
    }
    #[inline(always)]
    pub(crate) fn vertex_color_raw(&self) -> *mut VertexVec4 {
        view_raw_mut!(self, vertex_color)
    }
    #[inline(always)]
    pub(crate) fn set_vertex_crease(&self, value: VertexReal) {
        view_write!(self, vertex_crease, value)
    }
    #[inline(always)]
    pub(crate) fn vertex_crease_raw(&self) -> *mut VertexReal {
        view_raw_mut!(self, vertex_crease)
    }
    #[inline(always)]
    pub(crate) fn set_uv_sets(&self, value: List<UvSet>) {
        view_write!(self, uv_sets, value)
    }
    #[inline(always)]
    pub(crate) fn uv_sets_raw(&self) -> *mut List<UvSet> {
        view_raw_mut!(self, uv_sets)
    }
    #[inline(always)]
    pub(crate) fn set_color_sets(&self, value: List<ColorSet>) {
        view_write!(self, color_sets, value)
    }
    #[inline(always)]
    pub(crate) fn color_sets_raw(&self) -> *mut List<ColorSet> {
        view_raw_mut!(self, color_sets)
    }
    #[inline(always)]
    pub(crate) fn set_materials(&self, value: RefList<Material>) {
        view_write!(self, materials, value)
    }
    #[inline(always)]
    pub(crate) fn materials_raw(&self) -> *mut RefList<Material> {
        view_raw_mut!(self, materials)
    }
    #[inline(always)]
    pub(crate) fn set_face_groups(&self, value: List<FaceGroup>) {
        view_write!(self, face_groups, value)
    }
    #[inline(always)]
    pub(crate) fn face_groups_raw(&self) -> *mut List<FaceGroup> {
        view_raw_mut!(self, face_groups)
    }
    #[inline(always)]
    pub(crate) fn set_material_parts(&self, value: List<MeshPart>) {
        view_write!(self, material_parts, value)
    }
    #[inline(always)]
    pub(crate) fn material_parts_raw(&self) -> *mut List<MeshPart> {
        view_raw_mut!(self, material_parts)
    }
    #[inline(always)]
    pub(crate) fn set_face_group_parts(&self, value: List<MeshPart>) {
        view_write!(self, face_group_parts, value)
    }
    #[inline(always)]
    pub(crate) fn face_group_parts_raw(&self) -> *mut List<MeshPart> {
        view_raw_mut!(self, face_group_parts)
    }
    #[inline(always)]
    pub(crate) fn set_material_part_usage_order(&self, value: List<u32>) {
        view_write!(self, material_part_usage_order, value)
    }
    #[inline(always)]
    pub(crate) fn material_part_usage_order_raw(&self) -> *mut List<u32> {
        view_raw_mut!(self, material_part_usage_order)
    }
    #[inline(always)]
    pub(crate) fn set_skinned_is_local(&self, value: bool) {
        view_write!(self, skinned_is_local, value)
    }
    #[inline(always)]
    pub(crate) fn skinned_is_local_raw(&self) -> *mut bool {
        view_raw_mut!(self, skinned_is_local)
    }
    #[inline(always)]
    pub(crate) fn set_skinned_position(&self, value: VertexVec3) {
        view_write!(self, skinned_position, value)
    }
    #[inline(always)]
    pub(crate) fn skinned_position_raw(&self) -> *mut VertexVec3 {
        view_raw_mut!(self, skinned_position)
    }
    #[inline(always)]
    pub(crate) fn set_skinned_normal(&self, value: VertexVec3) {
        view_write!(self, skinned_normal, value)
    }
    #[inline(always)]
    pub(crate) fn skinned_normal_raw(&self) -> *mut VertexVec3 {
        view_raw_mut!(self, skinned_normal)
    }
    #[inline(always)]
    pub(crate) fn set_skin_deformers(&self, value: RefList<SkinDeformer>) {
        view_write!(self, skin_deformers, value)
    }
    #[inline(always)]
    pub(crate) fn skin_deformers_raw(&self) -> *mut RefList<SkinDeformer> {
        view_raw_mut!(self, skin_deformers)
    }
    #[inline(always)]
    pub(crate) fn set_blend_deformers(&self, value: RefList<BlendDeformer>) {
        view_write!(self, blend_deformers, value)
    }
    #[inline(always)]
    pub(crate) fn blend_deformers_raw(&self) -> *mut RefList<BlendDeformer> {
        view_raw_mut!(self, blend_deformers)
    }
    #[inline(always)]
    pub(crate) fn set_cache_deformers(&self, value: RefList<CacheDeformer>) {
        view_write!(self, cache_deformers, value)
    }
    #[inline(always)]
    pub(crate) fn cache_deformers_raw(&self) -> *mut RefList<CacheDeformer> {
        view_raw_mut!(self, cache_deformers)
    }
    #[inline(always)]
    pub(crate) fn set_all_deformers(&self, value: RefList<Element>) {
        view_write!(self, all_deformers, value)
    }
    #[inline(always)]
    pub(crate) fn all_deformers_raw(&self) -> *mut RefList<Element> {
        view_raw_mut!(self, all_deformers)
    }
    #[inline(always)]
    pub(crate) fn set_subdivision_preview_levels(&self, value: u32) {
        view_write!(self, subdivision_preview_levels, value)
    }
    #[inline(always)]
    pub(crate) fn subdivision_preview_levels_raw(&self) -> *mut u32 {
        view_raw_mut!(self, subdivision_preview_levels)
    }
    #[inline(always)]
    pub(crate) fn set_subdivision_render_levels(&self, value: u32) {
        view_write!(self, subdivision_render_levels, value)
    }
    #[inline(always)]
    pub(crate) fn subdivision_render_levels_raw(&self) -> *mut u32 {
        view_raw_mut!(self, subdivision_render_levels)
    }
    #[inline(always)]
    pub(crate) fn set_subdivision_display_mode(&self, value: SubdivisionDisplayMode) {
        view_write!(self, subdivision_display_mode, value)
    }
    #[inline(always)]
    pub(crate) fn subdivision_display_mode_raw(&self) -> *mut SubdivisionDisplayMode {
        view_raw_mut!(self, subdivision_display_mode)
    }
    #[inline(always)]
    pub(crate) fn set_subdivision_boundary(&self, value: SubdivisionBoundary) {
        view_write!(self, subdivision_boundary, value)
    }
    #[inline(always)]
    pub(crate) fn subdivision_boundary_raw(&self) -> *mut SubdivisionBoundary {
        view_raw_mut!(self, subdivision_boundary)
    }
    #[inline(always)]
    pub(crate) fn set_subdivision_uv_boundary(&self, value: SubdivisionBoundary) {
        view_write!(self, subdivision_uv_boundary, value)
    }
    #[inline(always)]
    pub(crate) fn subdivision_uv_boundary_raw(&self) -> *mut SubdivisionBoundary {
        view_raw_mut!(self, subdivision_uv_boundary)
    }
    #[inline(always)]
    pub(crate) fn set_reversed_winding(&self, value: bool) {
        view_write!(self, reversed_winding, value)
    }
    #[inline(always)]
    pub(crate) fn reversed_winding_raw(&self) -> *mut bool {
        view_raw_mut!(self, reversed_winding)
    }
    #[inline(always)]
    pub(crate) fn set_generated_normals(&self, value: bool) {
        view_write!(self, generated_normals, value)
    }
    #[inline(always)]
    pub(crate) fn generated_normals_raw(&self) -> *mut bool {
        view_raw_mut!(self, generated_normals)
    }
    #[inline(always)]
    pub(crate) fn set_subdivision_evaluated(&self, value: bool) {
        view_write!(self, subdivision_evaluated, value)
    }
    #[inline(always)]
    pub(crate) fn subdivision_evaluated_raw(&self) -> *mut bool {
        view_raw_mut!(self, subdivision_evaluated)
    }
    #[inline(always)]
    pub(crate) fn set_subdivision_result(&self, value: Option<Ref<SubdivisionResult>>) {
        view_write!(self, subdivision_result, value)
    }
    #[inline(always)]
    pub(crate) fn subdivision_result_raw(&self) -> *mut Option<Ref<SubdivisionResult>> {
        view_raw_mut!(self, subdivision_result)
    }
    #[inline(always)]
    pub(crate) fn set_from_tessellated_nurbs(&self, value: bool) {
        view_write!(self, from_tessellated_nurbs, value)
    }
    #[allow(clippy::wrong_self_convention)]
    #[inline(always)]
    pub(crate) fn from_tessellated_nurbs_raw(&self) -> *mut bool {
        view_raw_mut!(self, from_tessellated_nurbs)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<SkinDeformer, M> {
    #[inline(always)]
    pub(crate) fn element(&self) -> &View<Element, M> {
        view_project!(self, element)
    }
    #[inline(always)]
    pub(crate) fn element_ptr(&self) -> *const Element {
        view_raw_shared!(self, element)
    }
    #[inline(always)]
    pub(crate) fn skinning_method(&self) -> SkinningMethod {
        view_read_shared!(self, skinning_method)
    }
    #[inline(always)]
    pub(crate) fn skinning_method_ptr(&self) -> *const SkinningMethod {
        view_raw_shared!(self, skinning_method)
    }
    #[inline(always)]
    pub(crate) fn clusters(&self) -> RefList<SkinCluster> {
        view_read_shared!(self, clusters)
    }
    #[inline(always)]
    pub(crate) fn clusters_view(&self) -> &View<RefList<SkinCluster>, M> {
        view_project!(self, clusters)
    }
    #[inline(always)]
    pub(crate) fn clusters_ptr(&self) -> *const RefList<SkinCluster> {
        view_raw_shared!(self, clusters)
    }
    #[inline(always)]
    pub(crate) fn vertices(&self) -> List<SkinVertex> {
        view_read_shared!(self, vertices)
    }
    #[inline(always)]
    pub(crate) fn vertices_view(&self) -> &View<List<SkinVertex>, M> {
        view_project!(self, vertices)
    }
    #[inline(always)]
    pub(crate) fn vertices_ptr(&self) -> *const List<SkinVertex> {
        view_raw_shared!(self, vertices)
    }
    #[inline(always)]
    pub(crate) fn weights(&self) -> List<SkinWeight> {
        view_read_shared!(self, weights)
    }
    #[inline(always)]
    pub(crate) fn weights_view(&self) -> &View<List<SkinWeight>, M> {
        view_project!(self, weights)
    }
    #[inline(always)]
    pub(crate) fn weights_ptr(&self) -> *const List<SkinWeight> {
        view_raw_shared!(self, weights)
    }
    #[inline(always)]
    pub(crate) fn max_weights_per_vertex(&self) -> usize {
        view_read_shared!(self, max_weights_per_vertex)
    }
    #[inline(always)]
    pub(crate) fn max_weights_per_vertex_ptr(&self) -> *const usize {
        view_raw_shared!(self, max_weights_per_vertex)
    }
    #[inline(always)]
    pub(crate) fn num_dq_weights(&self) -> usize {
        view_read_shared!(self, num_dq_weights)
    }
    #[inline(always)]
    pub(crate) fn num_dq_weights_ptr(&self) -> *const usize {
        view_raw_shared!(self, num_dq_weights)
    }
    #[inline(always)]
    pub(crate) fn dq_vertices(&self) -> List<u32> {
        view_read_shared!(self, dq_vertices)
    }
    #[inline(always)]
    pub(crate) fn dq_vertices_view(&self) -> &View<List<u32>, M> {
        view_project!(self, dq_vertices)
    }
    #[inline(always)]
    pub(crate) fn dq_vertices_ptr(&self) -> *const List<u32> {
        view_raw_shared!(self, dq_vertices)
    }
    #[inline(always)]
    pub(crate) fn dq_weights(&self) -> List<Real> {
        view_read_shared!(self, dq_weights)
    }
    #[inline(always)]
    pub(crate) fn dq_weights_view(&self) -> &View<List<Real>, M> {
        view_project!(self, dq_weights)
    }
    #[inline(always)]
    pub(crate) fn dq_weights_ptr(&self) -> *const List<Real> {
        view_raw_shared!(self, dq_weights)
    }
}

#[allow(dead_code)]
impl View<SkinDeformer, Mut> {
    #[inline(always)]
    pub(crate) fn set_element(&self, value: Element) {
        view_write!(self, element, value)
    }
    #[inline(always)]
    pub(crate) fn element_raw(&self) -> *mut Element {
        view_raw_mut!(self, element)
    }
    #[inline(always)]
    pub(crate) fn set_skinning_method(&self, value: SkinningMethod) {
        view_write!(self, skinning_method, value)
    }
    #[inline(always)]
    pub(crate) fn skinning_method_raw(&self) -> *mut SkinningMethod {
        view_raw_mut!(self, skinning_method)
    }
    #[inline(always)]
    pub(crate) fn set_clusters(&self, value: RefList<SkinCluster>) {
        view_write!(self, clusters, value)
    }
    #[inline(always)]
    pub(crate) fn clusters_raw(&self) -> *mut RefList<SkinCluster> {
        view_raw_mut!(self, clusters)
    }
    #[inline(always)]
    pub(crate) fn set_vertices(&self, value: List<SkinVertex>) {
        view_write!(self, vertices, value)
    }
    #[inline(always)]
    pub(crate) fn vertices_raw(&self) -> *mut List<SkinVertex> {
        view_raw_mut!(self, vertices)
    }
    #[inline(always)]
    pub(crate) fn set_weights(&self, value: List<SkinWeight>) {
        view_write!(self, weights, value)
    }
    #[inline(always)]
    pub(crate) fn weights_raw(&self) -> *mut List<SkinWeight> {
        view_raw_mut!(self, weights)
    }
    #[inline(always)]
    pub(crate) fn set_max_weights_per_vertex(&self, value: usize) {
        view_write!(self, max_weights_per_vertex, value)
    }
    #[inline(always)]
    pub(crate) fn max_weights_per_vertex_raw(&self) -> *mut usize {
        view_raw_mut!(self, max_weights_per_vertex)
    }
    #[inline(always)]
    pub(crate) fn set_num_dq_weights(&self, value: usize) {
        view_write!(self, num_dq_weights, value)
    }
    #[inline(always)]
    pub(crate) fn num_dq_weights_raw(&self) -> *mut usize {
        view_raw_mut!(self, num_dq_weights)
    }
    #[inline(always)]
    pub(crate) fn set_dq_vertices(&self, value: List<u32>) {
        view_write!(self, dq_vertices, value)
    }
    #[inline(always)]
    pub(crate) fn dq_vertices_raw(&self) -> *mut List<u32> {
        view_raw_mut!(self, dq_vertices)
    }
    #[inline(always)]
    pub(crate) fn set_dq_weights(&self, value: List<Real>) {
        view_write!(self, dq_weights, value)
    }
    #[inline(always)]
    pub(crate) fn dq_weights_raw(&self) -> *mut List<Real> {
        view_raw_mut!(self, dq_weights)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<SubdivisionResult, M> {
    #[inline(always)]
    pub(crate) fn result_memory_used(&self) -> usize {
        view_read_shared!(self, result_memory_used)
    }
    #[inline(always)]
    pub(crate) fn result_memory_used_ptr(&self) -> *const usize {
        view_raw_shared!(self, result_memory_used)
    }
    #[inline(always)]
    pub(crate) fn temp_memory_used(&self) -> usize {
        view_read_shared!(self, temp_memory_used)
    }
    #[inline(always)]
    pub(crate) fn temp_memory_used_ptr(&self) -> *const usize {
        view_raw_shared!(self, temp_memory_used)
    }
    #[inline(always)]
    pub(crate) fn result_allocs(&self) -> usize {
        view_read_shared!(self, result_allocs)
    }
    #[inline(always)]
    pub(crate) fn result_allocs_ptr(&self) -> *const usize {
        view_raw_shared!(self, result_allocs)
    }
    #[inline(always)]
    pub(crate) fn temp_allocs(&self) -> usize {
        view_read_shared!(self, temp_allocs)
    }
    #[inline(always)]
    pub(crate) fn temp_allocs_ptr(&self) -> *const usize {
        view_raw_shared!(self, temp_allocs)
    }
    #[inline(always)]
    pub(crate) fn source_vertex_ranges(&self) -> List<SubdivisionWeightRange> {
        view_read_shared!(self, source_vertex_ranges)
    }
    #[inline(always)]
    pub(crate) fn source_vertex_ranges_view(&self) -> &View<List<SubdivisionWeightRange>, M> {
        view_project!(self, source_vertex_ranges)
    }
    #[inline(always)]
    pub(crate) fn source_vertex_ranges_ptr(&self) -> *const List<SubdivisionWeightRange> {
        view_raw_shared!(self, source_vertex_ranges)
    }
    #[inline(always)]
    pub(crate) fn source_vertex_weights(&self) -> List<SubdivisionWeight> {
        view_read_shared!(self, source_vertex_weights)
    }
    #[inline(always)]
    pub(crate) fn source_vertex_weights_view(&self) -> &View<List<SubdivisionWeight>, M> {
        view_project!(self, source_vertex_weights)
    }
    #[inline(always)]
    pub(crate) fn source_vertex_weights_ptr(&self) -> *const List<SubdivisionWeight> {
        view_raw_shared!(self, source_vertex_weights)
    }
    #[inline(always)]
    pub(crate) fn skin_cluster_ranges(&self) -> List<SubdivisionWeightRange> {
        view_read_shared!(self, skin_cluster_ranges)
    }
    #[inline(always)]
    pub(crate) fn skin_cluster_ranges_view(&self) -> &View<List<SubdivisionWeightRange>, M> {
        view_project!(self, skin_cluster_ranges)
    }
    #[inline(always)]
    pub(crate) fn skin_cluster_ranges_ptr(&self) -> *const List<SubdivisionWeightRange> {
        view_raw_shared!(self, skin_cluster_ranges)
    }
    #[inline(always)]
    pub(crate) fn skin_cluster_weights(&self) -> List<SubdivisionWeight> {
        view_read_shared!(self, skin_cluster_weights)
    }
    #[inline(always)]
    pub(crate) fn skin_cluster_weights_view(&self) -> &View<List<SubdivisionWeight>, M> {
        view_project!(self, skin_cluster_weights)
    }
    #[inline(always)]
    pub(crate) fn skin_cluster_weights_ptr(&self) -> *const List<SubdivisionWeight> {
        view_raw_shared!(self, skin_cluster_weights)
    }
}

#[allow(dead_code)]
impl View<SubdivisionResult, Mut> {
    #[inline(always)]
    pub(crate) fn set_result_memory_used(&self, value: usize) {
        view_write!(self, result_memory_used, value)
    }
    #[inline(always)]
    pub(crate) fn result_memory_used_raw(&self) -> *mut usize {
        view_raw_mut!(self, result_memory_used)
    }
    #[inline(always)]
    pub(crate) fn set_temp_memory_used(&self, value: usize) {
        view_write!(self, temp_memory_used, value)
    }
    #[inline(always)]
    pub(crate) fn temp_memory_used_raw(&self) -> *mut usize {
        view_raw_mut!(self, temp_memory_used)
    }
    #[inline(always)]
    pub(crate) fn set_result_allocs(&self, value: usize) {
        view_write!(self, result_allocs, value)
    }
    #[inline(always)]
    pub(crate) fn result_allocs_raw(&self) -> *mut usize {
        view_raw_mut!(self, result_allocs)
    }
    #[inline(always)]
    pub(crate) fn set_temp_allocs(&self, value: usize) {
        view_write!(self, temp_allocs, value)
    }
    #[inline(always)]
    pub(crate) fn temp_allocs_raw(&self) -> *mut usize {
        view_raw_mut!(self, temp_allocs)
    }
    #[inline(always)]
    pub(crate) fn set_source_vertex_ranges(&self, value: List<SubdivisionWeightRange>) {
        view_write!(self, source_vertex_ranges, value)
    }
    #[inline(always)]
    pub(crate) fn source_vertex_ranges_raw(&self) -> *mut List<SubdivisionWeightRange> {
        view_raw_mut!(self, source_vertex_ranges)
    }
    #[inline(always)]
    pub(crate) fn set_source_vertex_weights(&self, value: List<SubdivisionWeight>) {
        view_write!(self, source_vertex_weights, value)
    }
    #[inline(always)]
    pub(crate) fn source_vertex_weights_raw(&self) -> *mut List<SubdivisionWeight> {
        view_raw_mut!(self, source_vertex_weights)
    }
    #[inline(always)]
    pub(crate) fn set_skin_cluster_ranges(&self, value: List<SubdivisionWeightRange>) {
        view_write!(self, skin_cluster_ranges, value)
    }
    #[inline(always)]
    pub(crate) fn skin_cluster_ranges_raw(&self) -> *mut List<SubdivisionWeightRange> {
        view_raw_mut!(self, skin_cluster_ranges)
    }
    #[inline(always)]
    pub(crate) fn set_skin_cluster_weights(&self, value: List<SubdivisionWeight>) {
        view_write!(self, skin_cluster_weights, value)
    }
    #[inline(always)]
    pub(crate) fn skin_cluster_weights_raw(&self) -> *mut List<SubdivisionWeight> {
        view_raw_mut!(self, skin_cluster_weights)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<CacheDeformer, M> {
    #[inline(always)]
    pub(crate) fn element(&self) -> &View<Element, M> {
        view_project!(self, element)
    }
    #[inline(always)]
    pub(crate) fn element_ptr(&self) -> *const Element {
        view_raw_shared!(self, element)
    }
    #[inline(always)]
    pub(crate) fn channel(&self) -> String {
        view_read_shared!(self, channel)
    }
    #[inline(always)]
    pub(crate) fn channel_view(&self) -> &View<String, M> {
        view_project!(self, channel)
    }
    #[inline(always)]
    pub(crate) fn channel_ptr(&self) -> *const String {
        view_raw_shared!(self, channel)
    }
    #[inline(always)]
    pub(crate) fn file(&self) -> Option<Ref<CacheFile>> {
        view_read_shared!(self, file)
    }
    #[inline(always)]
    pub(crate) fn file_ptr(&self) -> *const Option<Ref<CacheFile>> {
        view_raw_shared!(self, file)
    }
    #[inline(always)]
    pub(crate) fn external_cache(&self) -> Option<Ref<GeometryCache>> {
        view_read_shared!(self, external_cache)
    }
    #[inline(always)]
    pub(crate) fn external_cache_ptr(&self) -> *const Option<Ref<GeometryCache>> {
        view_raw_shared!(self, external_cache)
    }
    #[inline(always)]
    pub(crate) fn external_channel(&self) -> Option<Ref<CacheChannel>> {
        view_read_shared!(self, external_channel)
    }
    #[inline(always)]
    pub(crate) fn external_channel_ptr(&self) -> *const Option<Ref<CacheChannel>> {
        view_raw_shared!(self, external_channel)
    }
}

#[allow(dead_code)]
impl View<CacheDeformer, Mut> {
    #[inline(always)]
    pub(crate) fn set_element(&self, value: Element) {
        view_write!(self, element, value)
    }
    #[inline(always)]
    pub(crate) fn element_raw(&self) -> *mut Element {
        view_raw_mut!(self, element)
    }
    #[inline(always)]
    pub(crate) fn set_channel(&self, value: String) {
        view_write!(self, channel, value)
    }
    #[inline(always)]
    pub(crate) fn channel_raw(&self) -> *mut String {
        view_raw_mut!(self, channel)
    }
    #[inline(always)]
    pub(crate) fn set_file(&self, value: Option<Ref<CacheFile>>) {
        view_write!(self, file, value)
    }
    #[inline(always)]
    pub(crate) fn file_raw(&self) -> *mut Option<Ref<CacheFile>> {
        view_raw_mut!(self, file)
    }
    #[inline(always)]
    pub(crate) fn set_external_cache(&self, value: Option<Ref<GeometryCache>>) {
        view_write!(self, external_cache, value)
    }
    #[inline(always)]
    pub(crate) fn external_cache_raw(&self) -> *mut Option<Ref<GeometryCache>> {
        view_raw_mut!(self, external_cache)
    }
    #[inline(always)]
    pub(crate) fn set_external_channel(&self, value: Option<Ref<CacheChannel>>) {
        view_write!(self, external_channel, value)
    }
    #[inline(always)]
    pub(crate) fn external_channel_raw(&self) -> *mut Option<Ref<CacheChannel>> {
        view_raw_mut!(self, external_channel)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<MaterialMap, M> {
    #[inline(always)]
    pub(crate) fn value_vec4(&self) -> Vec4 {
        view_read_shared!(self, value_vec4)
    }
    #[inline(always)]
    pub(crate) fn value_vec4_ptr(&self) -> *const Vec4 {
        view_raw_shared!(self, value_vec4)
    }
    #[inline(always)]
    pub(crate) fn value_int(&self) -> i64 {
        view_read_shared!(self, value_int)
    }
    #[inline(always)]
    pub(crate) fn value_int_ptr(&self) -> *const i64 {
        view_raw_shared!(self, value_int)
    }
    #[inline(always)]
    pub(crate) fn texture(&self) -> Option<Ref<Texture>> {
        view_read_shared!(self, texture)
    }
    #[inline(always)]
    pub(crate) fn texture_ptr(&self) -> *const Option<Ref<Texture>> {
        view_raw_shared!(self, texture)
    }
    #[inline(always)]
    pub(crate) fn has_value(&self) -> bool {
        view_read_shared!(self, has_value)
    }
    #[inline(always)]
    pub(crate) fn has_value_ptr(&self) -> *const bool {
        view_raw_shared!(self, has_value)
    }
    #[inline(always)]
    pub(crate) fn texture_enabled(&self) -> bool {
        view_read_shared!(self, texture_enabled)
    }
    #[inline(always)]
    pub(crate) fn texture_enabled_ptr(&self) -> *const bool {
        view_raw_shared!(self, texture_enabled)
    }
    #[inline(always)]
    pub(crate) fn feature_disabled(&self) -> bool {
        view_read_shared!(self, feature_disabled)
    }
    #[inline(always)]
    pub(crate) fn feature_disabled_ptr(&self) -> *const bool {
        view_raw_shared!(self, feature_disabled)
    }
    #[inline(always)]
    pub(crate) fn value_components(&self) -> u8 {
        view_read_shared!(self, value_components)
    }
    #[inline(always)]
    pub(crate) fn value_components_ptr(&self) -> *const u8 {
        view_raw_shared!(self, value_components)
    }
}

#[allow(dead_code)]
impl View<MaterialMap, Mut> {
    #[inline(always)]
    pub(crate) fn set_value_vec4(&self, value: Vec4) {
        view_write!(self, value_vec4, value)
    }
    #[inline(always)]
    pub(crate) fn value_vec4_raw(&self) -> *mut Vec4 {
        view_raw_mut!(self, value_vec4)
    }
    #[inline(always)]
    pub(crate) fn set_value_int(&self, value: i64) {
        view_write!(self, value_int, value)
    }
    #[inline(always)]
    pub(crate) fn value_int_raw(&self) -> *mut i64 {
        view_raw_mut!(self, value_int)
    }
    #[inline(always)]
    pub(crate) fn set_texture(&self, value: Option<Ref<Texture>>) {
        view_write!(self, texture, value)
    }
    #[inline(always)]
    pub(crate) fn texture_raw(&self) -> *mut Option<Ref<Texture>> {
        view_raw_mut!(self, texture)
    }
    #[inline(always)]
    pub(crate) fn set_has_value(&self, value: bool) {
        view_write!(self, has_value, value)
    }
    #[inline(always)]
    pub(crate) fn has_value_raw(&self) -> *mut bool {
        view_raw_mut!(self, has_value)
    }
    #[inline(always)]
    pub(crate) fn set_texture_enabled(&self, value: bool) {
        view_write!(self, texture_enabled, value)
    }
    #[inline(always)]
    pub(crate) fn texture_enabled_raw(&self) -> *mut bool {
        view_raw_mut!(self, texture_enabled)
    }
    #[inline(always)]
    pub(crate) fn set_feature_disabled(&self, value: bool) {
        view_write!(self, feature_disabled, value)
    }
    #[inline(always)]
    pub(crate) fn feature_disabled_raw(&self) -> *mut bool {
        view_raw_mut!(self, feature_disabled)
    }
    #[inline(always)]
    pub(crate) fn set_value_components(&self, value: u8) {
        view_write!(self, value_components, value)
    }
    #[inline(always)]
    pub(crate) fn value_components_raw(&self) -> *mut u8 {
        view_raw_mut!(self, value_components)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<MaterialFeatureInfo, M> {
    #[inline(always)]
    pub(crate) fn enabled(&self) -> bool {
        view_read_shared!(self, enabled)
    }
    #[inline(always)]
    pub(crate) fn enabled_ptr(&self) -> *const bool {
        view_raw_shared!(self, enabled)
    }
    #[inline(always)]
    pub(crate) fn is_explicit(&self) -> bool {
        view_read_shared!(self, is_explicit)
    }
    #[inline(always)]
    pub(crate) fn is_explicit_ptr(&self) -> *const bool {
        view_raw_shared!(self, is_explicit)
    }
}

#[allow(dead_code)]
impl View<MaterialFeatureInfo, Mut> {
    #[inline(always)]
    pub(crate) fn set_enabled(&self, value: bool) {
        view_write!(self, enabled, value)
    }
    #[inline(always)]
    pub(crate) fn enabled_raw(&self) -> *mut bool {
        view_raw_mut!(self, enabled)
    }
    #[inline(always)]
    pub(crate) fn set_is_explicit(&self, value: bool) {
        view_write!(self, is_explicit, value)
    }
    #[inline(always)]
    pub(crate) fn is_explicit_raw(&self) -> *mut bool {
        view_raw_mut!(self, is_explicit)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<MaterialTexture, M> {
    #[inline(always)]
    pub(crate) fn material_prop(&self) -> String {
        view_read_shared!(self, material_prop)
    }
    #[inline(always)]
    pub(crate) fn material_prop_view(&self) -> &View<String, M> {
        view_project!(self, material_prop)
    }
    #[inline(always)]
    pub(crate) fn material_prop_ptr(&self) -> *const String {
        view_raw_shared!(self, material_prop)
    }
    #[inline(always)]
    pub(crate) fn shader_prop(&self) -> String {
        view_read_shared!(self, shader_prop)
    }
    #[inline(always)]
    pub(crate) fn shader_prop_view(&self) -> &View<String, M> {
        view_project!(self, shader_prop)
    }
    #[inline(always)]
    pub(crate) fn shader_prop_ptr(&self) -> *const String {
        view_raw_shared!(self, shader_prop)
    }
    #[inline(always)]
    pub(crate) fn texture(&self) -> Ref<Texture> {
        view_read_shared!(self, texture)
    }
    #[inline(always)]
    pub(crate) fn texture_ptr(&self) -> *const Ref<Texture> {
        view_raw_shared!(self, texture)
    }
}

#[allow(dead_code)]
impl View<MaterialTexture, Mut> {
    #[inline(always)]
    pub(crate) fn set_material_prop(&self, value: String) {
        view_write!(self, material_prop, value)
    }
    #[inline(always)]
    pub(crate) fn material_prop_raw(&self) -> *mut String {
        view_raw_mut!(self, material_prop)
    }
    #[inline(always)]
    pub(crate) fn set_shader_prop(&self, value: String) {
        view_write!(self, shader_prop, value)
    }
    #[inline(always)]
    pub(crate) fn shader_prop_raw(&self) -> *mut String {
        view_raw_mut!(self, shader_prop)
    }
    #[inline(always)]
    pub(crate) fn set_texture(&self, value: Ref<Texture>) {
        view_write!(self, texture, value)
    }
    #[inline(always)]
    pub(crate) fn texture_raw(&self) -> *mut Ref<Texture> {
        view_raw_mut!(self, texture)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<ShaderPropBinding, M> {
    #[inline(always)]
    pub(crate) fn shader_prop(&self) -> String {
        view_read_shared!(self, shader_prop)
    }
    #[inline(always)]
    pub(crate) fn shader_prop_view(&self) -> &View<String, M> {
        view_project!(self, shader_prop)
    }
    #[inline(always)]
    pub(crate) fn shader_prop_ptr(&self) -> *const String {
        view_raw_shared!(self, shader_prop)
    }
    #[inline(always)]
    pub(crate) fn material_prop(&self) -> String {
        view_read_shared!(self, material_prop)
    }
    #[inline(always)]
    pub(crate) fn material_prop_view(&self) -> &View<String, M> {
        view_project!(self, material_prop)
    }
    #[inline(always)]
    pub(crate) fn material_prop_ptr(&self) -> *const String {
        view_raw_shared!(self, material_prop)
    }
}

#[allow(dead_code)]
impl View<ShaderPropBinding, Mut> {
    #[inline(always)]
    pub(crate) fn set_shader_prop(&self, value: String) {
        view_write!(self, shader_prop, value)
    }
    #[inline(always)]
    pub(crate) fn shader_prop_raw(&self) -> *mut String {
        view_raw_mut!(self, shader_prop)
    }
    #[inline(always)]
    pub(crate) fn set_material_prop(&self, value: String) {
        view_write!(self, material_prop, value)
    }
    #[inline(always)]
    pub(crate) fn material_prop_raw(&self) -> *mut String {
        view_raw_mut!(self, material_prop)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<PropOverride, M> {
    #[inline(always)]
    pub(crate) fn element_id(&self) -> u32 {
        view_read_shared!(self, element_id)
    }
    #[inline(always)]
    pub(crate) fn element_id_ptr(&self) -> *const u32 {
        view_raw_shared!(self, element_id)
    }
    #[inline(always)]
    pub(crate) fn _internal_key(&self) -> u32 {
        view_read_shared!(self, _internal_key)
    }
    #[inline(always)]
    pub(crate) fn internal_key_ptr(&self) -> *const u32 {
        view_raw_shared!(self, _internal_key)
    }
    #[inline(always)]
    pub(crate) fn prop_name(&self) -> String {
        view_read_shared!(self, prop_name)
    }
    #[inline(always)]
    pub(crate) fn prop_name_view(&self) -> &View<String, M> {
        view_project!(self, prop_name)
    }
    #[inline(always)]
    pub(crate) fn prop_name_ptr(&self) -> *const String {
        view_raw_shared!(self, prop_name)
    }
    #[inline(always)]
    pub(crate) fn value(&self) -> Vec4 {
        view_read_shared!(self, value)
    }
    #[inline(always)]
    pub(crate) fn value_ptr(&self) -> *const Vec4 {
        view_raw_shared!(self, value)
    }
    #[inline(always)]
    pub(crate) fn value_str(&self) -> String {
        view_read_shared!(self, value_str)
    }
    #[inline(always)]
    pub(crate) fn value_str_view(&self) -> &View<String, M> {
        view_project!(self, value_str)
    }
    #[inline(always)]
    pub(crate) fn value_str_ptr(&self) -> *const String {
        view_raw_shared!(self, value_str)
    }
    #[inline(always)]
    pub(crate) fn value_int(&self) -> i64 {
        view_read_shared!(self, value_int)
    }
    #[inline(always)]
    pub(crate) fn value_int_ptr(&self) -> *const i64 {
        view_raw_shared!(self, value_int)
    }
}

#[allow(dead_code)]
impl View<PropOverride, Mut> {
    #[inline(always)]
    pub(crate) fn set_element_id(&self, value: u32) {
        view_write!(self, element_id, value)
    }
    #[inline(always)]
    pub(crate) fn element_id_raw(&self) -> *mut u32 {
        view_raw_mut!(self, element_id)
    }
    #[inline(always)]
    pub(crate) fn set_internal_key(&self, value: u32) {
        view_write!(self, _internal_key, value)
    }
    #[inline(always)]
    pub(crate) fn internal_key_raw(&self) -> *mut u32 {
        view_raw_mut!(self, _internal_key)
    }
    #[inline(always)]
    pub(crate) fn set_prop_name(&self, value: String) {
        view_write!(self, prop_name, value)
    }
    #[inline(always)]
    pub(crate) fn prop_name_raw(&self) -> *mut String {
        view_raw_mut!(self, prop_name)
    }
    #[inline(always)]
    pub(crate) fn set_value(&self, value: Vec4) {
        view_write!(self, value, value)
    }
    #[inline(always)]
    pub(crate) fn value_raw(&self) -> *mut Vec4 {
        view_raw_mut!(self, value)
    }
    #[inline(always)]
    pub(crate) fn set_value_str(&self, value: String) {
        view_write!(self, value_str, value)
    }
    #[inline(always)]
    pub(crate) fn value_str_raw(&self) -> *mut String {
        view_raw_mut!(self, value_str)
    }
    #[inline(always)]
    pub(crate) fn set_value_int(&self, value: i64) {
        view_write!(self, value_int, value)
    }
    #[inline(always)]
    pub(crate) fn value_int_raw(&self) -> *mut i64 {
        view_raw_mut!(self, value_int)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<DomNode, M> {
    #[inline(always)]
    pub(crate) fn name(&self) -> String {
        view_read_shared!(self, name)
    }
    #[inline(always)]
    pub(crate) fn name_view(&self) -> &View<String, M> {
        view_project!(self, name)
    }
    #[inline(always)]
    pub(crate) fn name_ptr(&self) -> *const String {
        view_raw_shared!(self, name)
    }
    #[inline(always)]
    pub(crate) fn children(&self) -> RefList<DomNode> {
        view_read_shared!(self, children)
    }
    #[inline(always)]
    pub(crate) fn children_view(&self) -> &View<RefList<DomNode>, M> {
        view_project!(self, children)
    }
    #[inline(always)]
    pub(crate) fn children_ptr(&self) -> *const RefList<DomNode> {
        view_raw_shared!(self, children)
    }
    #[inline(always)]
    pub(crate) fn values(&self) -> List<DomValue> {
        view_read_shared!(self, values)
    }
    #[inline(always)]
    pub(crate) fn values_view(&self) -> &View<List<DomValue>, M> {
        view_project!(self, values)
    }
    #[inline(always)]
    pub(crate) fn values_ptr(&self) -> *const List<DomValue> {
        view_raw_shared!(self, values)
    }
}

#[allow(dead_code)]
impl View<DomNode, Mut> {
    #[inline(always)]
    pub(crate) fn set_name(&self, value: String) {
        view_write!(self, name, value)
    }
    #[inline(always)]
    pub(crate) fn name_raw(&self) -> *mut String {
        view_raw_mut!(self, name)
    }
    #[inline(always)]
    pub(crate) fn set_children(&self, value: RefList<DomNode>) {
        view_write!(self, children, value)
    }
    #[inline(always)]
    pub(crate) fn children_raw(&self) -> *mut RefList<DomNode> {
        view_raw_mut!(self, children)
    }
    #[inline(always)]
    pub(crate) fn set_values(&self, value: List<DomValue>) {
        view_write!(self, values, value)
    }
    #[inline(always)]
    pub(crate) fn values_raw(&self) -> *mut List<DomValue> {
        view_raw_mut!(self, values)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<AnimCurve, M> {
    #[inline(always)]
    pub(crate) fn element(&self) -> &View<Element, M> {
        view_project!(self, element)
    }
    #[inline(always)]
    pub(crate) fn element_ptr(&self) -> *const Element {
        view_raw_shared!(self, element)
    }
    #[inline(always)]
    pub(crate) fn keyframes(&self) -> List<Keyframe> {
        view_read_shared!(self, keyframes)
    }
    #[inline(always)]
    pub(crate) fn keyframes_view(&self) -> &View<List<Keyframe>, M> {
        view_project!(self, keyframes)
    }
    #[inline(always)]
    pub(crate) fn keyframes_ptr(&self) -> *const List<Keyframe> {
        view_raw_shared!(self, keyframes)
    }
    #[inline(always)]
    pub(crate) fn pre_extrapolation(&self) -> Extrapolation {
        view_read_shared!(self, pre_extrapolation)
    }
    #[inline(always)]
    pub(crate) fn pre_extrapolation_ptr(&self) -> *const Extrapolation {
        view_raw_shared!(self, pre_extrapolation)
    }
    #[inline(always)]
    pub(crate) fn post_extrapolation(&self) -> Extrapolation {
        view_read_shared!(self, post_extrapolation)
    }
    #[inline(always)]
    pub(crate) fn post_extrapolation_ptr(&self) -> *const Extrapolation {
        view_raw_shared!(self, post_extrapolation)
    }
    #[inline(always)]
    pub(crate) fn min_value(&self) -> Real {
        view_read_shared!(self, min_value)
    }
    #[inline(always)]
    pub(crate) fn min_value_ptr(&self) -> *const Real {
        view_raw_shared!(self, min_value)
    }
    #[inline(always)]
    pub(crate) fn max_value(&self) -> Real {
        view_read_shared!(self, max_value)
    }
    #[inline(always)]
    pub(crate) fn max_value_ptr(&self) -> *const Real {
        view_raw_shared!(self, max_value)
    }
    #[inline(always)]
    pub(crate) fn min_time(&self) -> f64 {
        view_read_shared!(self, min_time)
    }
    #[inline(always)]
    pub(crate) fn min_time_ptr(&self) -> *const f64 {
        view_raw_shared!(self, min_time)
    }
    #[inline(always)]
    pub(crate) fn max_time(&self) -> f64 {
        view_read_shared!(self, max_time)
    }
    #[inline(always)]
    pub(crate) fn max_time_ptr(&self) -> *const f64 {
        view_raw_shared!(self, max_time)
    }
}

#[allow(dead_code)]
impl View<AnimCurve, Mut> {
    #[inline(always)]
    pub(crate) fn set_element(&self, value: Element) {
        view_write!(self, element, value)
    }
    #[inline(always)]
    pub(crate) fn element_raw(&self) -> *mut Element {
        view_raw_mut!(self, element)
    }
    #[inline(always)]
    pub(crate) fn set_keyframes(&self, value: List<Keyframe>) {
        view_write!(self, keyframes, value)
    }
    #[inline(always)]
    pub(crate) fn keyframes_raw(&self) -> *mut List<Keyframe> {
        view_raw_mut!(self, keyframes)
    }
    #[inline(always)]
    pub(crate) fn set_pre_extrapolation(&self, value: Extrapolation) {
        view_write!(self, pre_extrapolation, value)
    }
    #[inline(always)]
    pub(crate) fn pre_extrapolation_raw(&self) -> *mut Extrapolation {
        view_raw_mut!(self, pre_extrapolation)
    }
    #[inline(always)]
    pub(crate) fn set_post_extrapolation(&self, value: Extrapolation) {
        view_write!(self, post_extrapolation, value)
    }
    #[inline(always)]
    pub(crate) fn post_extrapolation_raw(&self) -> *mut Extrapolation {
        view_raw_mut!(self, post_extrapolation)
    }
    #[inline(always)]
    pub(crate) fn set_min_value(&self, value: Real) {
        view_write!(self, min_value, value)
    }
    #[inline(always)]
    pub(crate) fn min_value_raw(&self) -> *mut Real {
        view_raw_mut!(self, min_value)
    }
    #[inline(always)]
    pub(crate) fn set_max_value(&self, value: Real) {
        view_write!(self, max_value, value)
    }
    #[inline(always)]
    pub(crate) fn max_value_raw(&self) -> *mut Real {
        view_raw_mut!(self, max_value)
    }
    #[inline(always)]
    pub(crate) fn set_min_time(&self, value: f64) {
        view_write!(self, min_time, value)
    }
    #[inline(always)]
    pub(crate) fn min_time_raw(&self) -> *mut f64 {
        view_raw_mut!(self, min_time)
    }
    #[inline(always)]
    pub(crate) fn set_max_time(&self, value: f64) {
        view_write!(self, max_time, value)
    }
    #[inline(always)]
    pub(crate) fn max_time_raw(&self) -> *mut f64 {
        view_raw_mut!(self, max_time)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<AnimValue, M> {
    #[inline(always)]
    pub(crate) fn element(&self) -> &View<Element, M> {
        view_project!(self, element)
    }
    #[inline(always)]
    pub(crate) fn element_ptr(&self) -> *const Element {
        view_raw_shared!(self, element)
    }
    #[inline(always)]
    pub(crate) fn default_value(&self) -> Vec3 {
        view_read_shared!(self, default_value)
    }
    #[inline(always)]
    pub(crate) fn default_value_ptr(&self) -> *const Vec3 {
        view_raw_shared!(self, default_value)
    }
    #[inline(always)]
    pub(crate) fn curves_ptr(&self) -> *const [Option<Ref<AnimCurve>>; 3] {
        view_raw_shared!(self, curves)
    }
}

#[allow(dead_code)]
impl View<AnimValue, Mut> {
    #[inline(always)]
    pub(crate) fn set_element(&self, value: Element) {
        view_write!(self, element, value)
    }
    #[inline(always)]
    pub(crate) fn element_raw(&self) -> *mut Element {
        view_raw_mut!(self, element)
    }
    #[inline(always)]
    pub(crate) fn set_default_value(&self, value: Vec3) {
        view_write!(self, default_value, value)
    }
    #[inline(always)]
    pub(crate) fn default_value_raw(&self) -> *mut Vec3 {
        view_raw_mut!(self, default_value)
    }
    #[inline(always)]
    pub(crate) fn set_curves(&self, value: [Option<Ref<AnimCurve>>; 3]) {
        view_write!(self, curves, value)
    }
    #[inline(always)]
    pub(crate) fn curves_raw(&self) -> *mut [Option<Ref<AnimCurve>>; 3] {
        view_raw_mut!(self, curves)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<Keyframe, M> {
    #[inline(always)]
    pub(crate) fn time(&self) -> f64 {
        view_read_shared!(self, time)
    }
    #[inline(always)]
    pub(crate) fn time_ptr(&self) -> *const f64 {
        view_raw_shared!(self, time)
    }
    #[inline(always)]
    pub(crate) fn value(&self) -> Real {
        view_read_shared!(self, value)
    }
    #[inline(always)]
    pub(crate) fn value_ptr(&self) -> *const Real {
        view_raw_shared!(self, value)
    }
    #[inline(always)]
    pub(crate) fn interpolation(&self) -> Interpolation {
        view_read_shared!(self, interpolation)
    }
    #[inline(always)]
    pub(crate) fn interpolation_ptr(&self) -> *const Interpolation {
        view_raw_shared!(self, interpolation)
    }
    #[inline(always)]
    pub(crate) fn left(&self) -> Tangent {
        view_read_shared!(self, left)
    }
    #[inline(always)]
    pub(crate) fn left_ptr(&self) -> *const Tangent {
        view_raw_shared!(self, left)
    }
    #[inline(always)]
    pub(crate) fn right(&self) -> Tangent {
        view_read_shared!(self, right)
    }
    #[inline(always)]
    pub(crate) fn right_ptr(&self) -> *const Tangent {
        view_raw_shared!(self, right)
    }
}

#[allow(dead_code)]
impl View<Keyframe, Mut> {
    #[inline(always)]
    pub(crate) fn set_time(&self, value: f64) {
        view_write!(self, time, value)
    }
    #[inline(always)]
    pub(crate) fn time_raw(&self) -> *mut f64 {
        view_raw_mut!(self, time)
    }
    #[inline(always)]
    pub(crate) fn set_value(&self, value: Real) {
        view_write!(self, value, value)
    }
    #[inline(always)]
    pub(crate) fn value_raw(&self) -> *mut Real {
        view_raw_mut!(self, value)
    }
    #[inline(always)]
    pub(crate) fn set_interpolation(&self, value: Interpolation) {
        view_write!(self, interpolation, value)
    }
    #[inline(always)]
    pub(crate) fn interpolation_raw(&self) -> *mut Interpolation {
        view_raw_mut!(self, interpolation)
    }
    #[inline(always)]
    pub(crate) fn set_left(&self, value: Tangent) {
        view_write!(self, left, value)
    }
    #[inline(always)]
    pub(crate) fn left_raw(&self) -> *mut Tangent {
        view_raw_mut!(self, left)
    }
    #[inline(always)]
    pub(crate) fn set_right(&self, value: Tangent) {
        view_write!(self, right, value)
    }
    #[inline(always)]
    pub(crate) fn right_raw(&self) -> *mut Tangent {
        view_raw_mut!(self, right)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<Connection, M> {
    #[inline(always)]
    pub(crate) fn src(&self) -> Ref<Element> {
        view_read_shared!(self, src)
    }
    #[inline(always)]
    pub(crate) fn src_ptr(&self) -> *const Ref<Element> {
        view_raw_shared!(self, src)
    }
    #[inline(always)]
    pub(crate) fn dst(&self) -> Ref<Element> {
        view_read_shared!(self, dst)
    }
    #[inline(always)]
    pub(crate) fn dst_ptr(&self) -> *const Ref<Element> {
        view_raw_shared!(self, dst)
    }
    #[inline(always)]
    pub(crate) fn src_prop(&self) -> String {
        view_read_shared!(self, src_prop)
    }
    #[inline(always)]
    pub(crate) fn src_prop_view(&self) -> &View<String, M> {
        view_project!(self, src_prop)
    }
    #[inline(always)]
    pub(crate) fn src_prop_ptr(&self) -> *const String {
        view_raw_shared!(self, src_prop)
    }
    #[inline(always)]
    pub(crate) fn dst_prop(&self) -> String {
        view_read_shared!(self, dst_prop)
    }
    #[inline(always)]
    pub(crate) fn dst_prop_view(&self) -> &View<String, M> {
        view_project!(self, dst_prop)
    }
    #[inline(always)]
    pub(crate) fn dst_prop_ptr(&self) -> *const String {
        view_raw_shared!(self, dst_prop)
    }
}

#[allow(dead_code)]
impl View<Connection, Mut> {
    #[inline(always)]
    pub(crate) fn set_src(&self, value: Ref<Element>) {
        view_write!(self, src, value)
    }
    #[inline(always)]
    pub(crate) fn src_raw(&self) -> *mut Ref<Element> {
        view_raw_mut!(self, src)
    }
    #[inline(always)]
    pub(crate) fn set_dst(&self, value: Ref<Element>) {
        view_write!(self, dst, value)
    }
    #[inline(always)]
    pub(crate) fn dst_raw(&self) -> *mut Ref<Element> {
        view_raw_mut!(self, dst)
    }
    #[inline(always)]
    pub(crate) fn set_src_prop(&self, value: String) {
        view_write!(self, src_prop, value)
    }
    #[inline(always)]
    pub(crate) fn src_prop_raw(&self) -> *mut String {
        view_raw_mut!(self, src_prop)
    }
    #[inline(always)]
    pub(crate) fn set_dst_prop(&self, value: String) {
        view_write!(self, dst_prop, value)
    }
    #[inline(always)]
    pub(crate) fn dst_prop_raw(&self) -> *mut String {
        view_raw_mut!(self, dst_prop)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<AnimLayer, M> {
    #[inline(always)]
    pub(crate) fn element(&self) -> &View<Element, M> {
        view_project!(self, element)
    }
    #[inline(always)]
    pub(crate) fn element_ptr(&self) -> *const Element {
        view_raw_shared!(self, element)
    }
    #[inline(always)]
    pub(crate) fn weight(&self) -> Real {
        view_read_shared!(self, weight)
    }
    #[inline(always)]
    pub(crate) fn weight_ptr(&self) -> *const Real {
        view_raw_shared!(self, weight)
    }
    #[inline(always)]
    pub(crate) fn weight_is_animated(&self) -> bool {
        view_read_shared!(self, weight_is_animated)
    }
    #[inline(always)]
    pub(crate) fn weight_is_animated_ptr(&self) -> *const bool {
        view_raw_shared!(self, weight_is_animated)
    }
    #[inline(always)]
    pub(crate) fn blended(&self) -> bool {
        view_read_shared!(self, blended)
    }
    #[inline(always)]
    pub(crate) fn blended_ptr(&self) -> *const bool {
        view_raw_shared!(self, blended)
    }
    #[inline(always)]
    pub(crate) fn additive(&self) -> bool {
        view_read_shared!(self, additive)
    }
    #[inline(always)]
    pub(crate) fn additive_ptr(&self) -> *const bool {
        view_raw_shared!(self, additive)
    }
    #[inline(always)]
    pub(crate) fn compose_rotation(&self) -> bool {
        view_read_shared!(self, compose_rotation)
    }
    #[inline(always)]
    pub(crate) fn compose_rotation_ptr(&self) -> *const bool {
        view_raw_shared!(self, compose_rotation)
    }
    #[inline(always)]
    pub(crate) fn compose_scale(&self) -> bool {
        view_read_shared!(self, compose_scale)
    }
    #[inline(always)]
    pub(crate) fn compose_scale_ptr(&self) -> *const bool {
        view_raw_shared!(self, compose_scale)
    }
    #[inline(always)]
    pub(crate) fn anim_values(&self) -> RefList<AnimValue> {
        view_read_shared!(self, anim_values)
    }
    #[inline(always)]
    pub(crate) fn anim_values_view(&self) -> &View<RefList<AnimValue>, M> {
        view_project!(self, anim_values)
    }
    #[inline(always)]
    pub(crate) fn anim_values_ptr(&self) -> *const RefList<AnimValue> {
        view_raw_shared!(self, anim_values)
    }
    #[inline(always)]
    pub(crate) fn anim_props(&self) -> List<AnimProp> {
        view_read_shared!(self, anim_props)
    }
    #[inline(always)]
    pub(crate) fn anim_props_view(&self) -> &View<List<AnimProp>, M> {
        view_project!(self, anim_props)
    }
    #[inline(always)]
    pub(crate) fn anim_props_ptr(&self) -> *const List<AnimProp> {
        view_raw_shared!(self, anim_props)
    }
    #[inline(always)]
    pub(crate) fn anim(&self) -> Ref<Anim> {
        view_read_shared!(self, anim)
    }
    #[inline(always)]
    pub(crate) fn anim_ptr(&self) -> *const Ref<Anim> {
        view_raw_shared!(self, anim)
    }
    #[inline(always)]
    pub(crate) fn _min_element_id(&self) -> u32 {
        view_read_shared!(self, _min_element_id)
    }
    #[inline(always)]
    pub(crate) fn min_element_id_ptr(&self) -> *const u32 {
        view_raw_shared!(self, _min_element_id)
    }
    #[inline(always)]
    pub(crate) fn _max_element_id(&self) -> u32 {
        view_read_shared!(self, _max_element_id)
    }
    #[inline(always)]
    pub(crate) fn max_element_id_ptr(&self) -> *const u32 {
        view_raw_shared!(self, _max_element_id)
    }
    #[inline(always)]
    pub(crate) fn _element_id_bitmask(&self) -> [u32; 4] {
        view_read_shared!(self, _element_id_bitmask)
    }
    #[inline(always)]
    pub(crate) fn element_id_bitmask_ptr(&self) -> *const [u32; 4] {
        view_raw_shared!(self, _element_id_bitmask)
    }
}

#[allow(dead_code)]
impl View<AnimLayer, Mut> {
    #[inline(always)]
    pub(crate) fn set_element(&self, value: Element) {
        view_write!(self, element, value)
    }
    #[inline(always)]
    pub(crate) fn element_raw(&self) -> *mut Element {
        view_raw_mut!(self, element)
    }
    #[inline(always)]
    pub(crate) fn set_weight(&self, value: Real) {
        view_write!(self, weight, value)
    }
    #[inline(always)]
    pub(crate) fn weight_raw(&self) -> *mut Real {
        view_raw_mut!(self, weight)
    }
    #[inline(always)]
    pub(crate) fn set_weight_is_animated(&self, value: bool) {
        view_write!(self, weight_is_animated, value)
    }
    #[inline(always)]
    pub(crate) fn weight_is_animated_raw(&self) -> *mut bool {
        view_raw_mut!(self, weight_is_animated)
    }
    #[inline(always)]
    pub(crate) fn set_blended(&self, value: bool) {
        view_write!(self, blended, value)
    }
    #[inline(always)]
    pub(crate) fn blended_raw(&self) -> *mut bool {
        view_raw_mut!(self, blended)
    }
    #[inline(always)]
    pub(crate) fn set_additive(&self, value: bool) {
        view_write!(self, additive, value)
    }
    #[inline(always)]
    pub(crate) fn additive_raw(&self) -> *mut bool {
        view_raw_mut!(self, additive)
    }
    #[inline(always)]
    pub(crate) fn set_compose_rotation(&self, value: bool) {
        view_write!(self, compose_rotation, value)
    }
    #[inline(always)]
    pub(crate) fn compose_rotation_raw(&self) -> *mut bool {
        view_raw_mut!(self, compose_rotation)
    }
    #[inline(always)]
    pub(crate) fn set_compose_scale(&self, value: bool) {
        view_write!(self, compose_scale, value)
    }
    #[inline(always)]
    pub(crate) fn compose_scale_raw(&self) -> *mut bool {
        view_raw_mut!(self, compose_scale)
    }
    #[inline(always)]
    pub(crate) fn set_anim_values(&self, value: RefList<AnimValue>) {
        view_write!(self, anim_values, value)
    }
    #[inline(always)]
    pub(crate) fn anim_values_raw(&self) -> *mut RefList<AnimValue> {
        view_raw_mut!(self, anim_values)
    }
    #[inline(always)]
    pub(crate) fn set_anim_props(&self, value: List<AnimProp>) {
        view_write!(self, anim_props, value)
    }
    #[inline(always)]
    pub(crate) fn anim_props_raw(&self) -> *mut List<AnimProp> {
        view_raw_mut!(self, anim_props)
    }
    #[inline(always)]
    pub(crate) fn set_anim(&self, value: Ref<Anim>) {
        view_write!(self, anim, value)
    }
    #[inline(always)]
    pub(crate) fn anim_raw(&self) -> *mut Ref<Anim> {
        view_raw_mut!(self, anim)
    }
    #[inline(always)]
    pub(crate) fn set_min_element_id(&self, value: u32) {
        view_write!(self, _min_element_id, value)
    }
    #[inline(always)]
    pub(crate) fn min_element_id_raw(&self) -> *mut u32 {
        view_raw_mut!(self, _min_element_id)
    }
    #[inline(always)]
    pub(crate) fn set_max_element_id(&self, value: u32) {
        view_write!(self, _max_element_id, value)
    }
    #[inline(always)]
    pub(crate) fn max_element_id_raw(&self) -> *mut u32 {
        view_raw_mut!(self, _max_element_id)
    }
    #[inline(always)]
    pub(crate) fn set_element_id_bitmask(&self, value: [u32; 4]) {
        view_write!(self, _element_id_bitmask, value)
    }
    #[inline(always)]
    pub(crate) fn element_id_bitmask_raw(&self) -> *mut [u32; 4] {
        view_raw_mut!(self, _element_id_bitmask)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<AnimProp, M> {
    #[inline(always)]
    pub(crate) fn element(&self) -> Ref<Element> {
        view_read_shared!(self, element)
    }
    #[inline(always)]
    pub(crate) fn element_ptr(&self) -> *const Ref<Element> {
        view_raw_shared!(self, element)
    }
    #[inline(always)]
    pub(crate) fn _internal_key(&self) -> u32 {
        view_read_shared!(self, _internal_key)
    }
    #[inline(always)]
    pub(crate) fn internal_key_ptr(&self) -> *const u32 {
        view_raw_shared!(self, _internal_key)
    }
    #[inline(always)]
    pub(crate) fn prop_name(&self) -> String {
        view_read_shared!(self, prop_name)
    }
    #[inline(always)]
    pub(crate) fn prop_name_view(&self) -> &View<String, M> {
        view_project!(self, prop_name)
    }
    #[inline(always)]
    pub(crate) fn prop_name_ptr(&self) -> *const String {
        view_raw_shared!(self, prop_name)
    }
    #[inline(always)]
    pub(crate) fn anim_value(&self) -> Ref<AnimValue> {
        view_read_shared!(self, anim_value)
    }
    #[inline(always)]
    pub(crate) fn anim_value_ptr(&self) -> *const Ref<AnimValue> {
        view_raw_shared!(self, anim_value)
    }
}

#[allow(dead_code)]
impl View<AnimProp, Mut> {
    #[inline(always)]
    pub(crate) fn set_element(&self, value: Ref<Element>) {
        view_write!(self, element, value)
    }
    #[inline(always)]
    pub(crate) fn element_raw(&self) -> *mut Ref<Element> {
        view_raw_mut!(self, element)
    }
    #[inline(always)]
    pub(crate) fn set_internal_key(&self, value: u32) {
        view_write!(self, _internal_key, value)
    }
    #[inline(always)]
    pub(crate) fn internal_key_raw(&self) -> *mut u32 {
        view_raw_mut!(self, _internal_key)
    }
    #[inline(always)]
    pub(crate) fn set_prop_name(&self, value: String) {
        view_write!(self, prop_name, value)
    }
    #[inline(always)]
    pub(crate) fn prop_name_raw(&self) -> *mut String {
        view_raw_mut!(self, prop_name)
    }
    #[inline(always)]
    pub(crate) fn set_anim_value(&self, value: Ref<AnimValue>) {
        view_write!(self, anim_value, value)
    }
    #[inline(always)]
    pub(crate) fn anim_value_raw(&self) -> *mut Ref<AnimValue> {
        view_raw_mut!(self, anim_value)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<Shader, M> {
    #[inline(always)]
    pub(crate) fn element(&self) -> &View<Element, M> {
        view_project!(self, element)
    }
    #[inline(always)]
    pub(crate) fn element_ptr(&self) -> *const Element {
        view_raw_shared!(self, element)
    }
    #[inline(always)]
    pub(crate) fn type_(&self) -> ShaderType {
        view_read_shared!(self, type_)
    }
    #[inline(always)]
    pub(crate) fn type_ptr(&self) -> *const ShaderType {
        view_raw_shared!(self, type_)
    }
    #[inline(always)]
    pub(crate) fn bindings(&self) -> RefList<ShaderBinding> {
        view_read_shared!(self, bindings)
    }
    #[inline(always)]
    pub(crate) fn bindings_view(&self) -> &View<RefList<ShaderBinding>, M> {
        view_project!(self, bindings)
    }
    #[inline(always)]
    pub(crate) fn bindings_ptr(&self) -> *const RefList<ShaderBinding> {
        view_raw_shared!(self, bindings)
    }
}

#[allow(dead_code)]
impl View<Shader, Mut> {
    #[inline(always)]
    pub(crate) fn set_element(&self, value: Element) {
        view_write!(self, element, value)
    }
    #[inline(always)]
    pub(crate) fn element_raw(&self) -> *mut Element {
        view_raw_mut!(self, element)
    }
    #[inline(always)]
    pub(crate) fn set_type(&self, value: ShaderType) {
        view_write!(self, type_, value)
    }
    #[inline(always)]
    pub(crate) fn type_raw(&self) -> *mut ShaderType {
        view_raw_mut!(self, type_)
    }
    #[inline(always)]
    pub(crate) fn set_bindings(&self, value: RefList<ShaderBinding>) {
        view_write!(self, bindings, value)
    }
    #[inline(always)]
    pub(crate) fn bindings_raw(&self) -> *mut RefList<ShaderBinding> {
        view_raw_mut!(self, bindings)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<ShaderBinding, M> {
    #[inline(always)]
    pub(crate) fn element(&self) -> &View<Element, M> {
        view_project!(self, element)
    }
    #[inline(always)]
    pub(crate) fn element_ptr(&self) -> *const Element {
        view_raw_shared!(self, element)
    }
    #[inline(always)]
    pub(crate) fn prop_bindings(&self) -> List<ShaderPropBinding> {
        view_read_shared!(self, prop_bindings)
    }
    #[inline(always)]
    pub(crate) fn prop_bindings_view(&self) -> &View<List<ShaderPropBinding>, M> {
        view_project!(self, prop_bindings)
    }
    #[inline(always)]
    pub(crate) fn prop_bindings_ptr(&self) -> *const List<ShaderPropBinding> {
        view_raw_shared!(self, prop_bindings)
    }
}

#[allow(dead_code)]
impl View<ShaderBinding, Mut> {
    #[inline(always)]
    pub(crate) fn set_element(&self, value: Element) {
        view_write!(self, element, value)
    }
    #[inline(always)]
    pub(crate) fn element_raw(&self) -> *mut Element {
        view_raw_mut!(self, element)
    }
    #[inline(always)]
    pub(crate) fn set_prop_bindings(&self, value: List<ShaderPropBinding>) {
        view_write!(self, prop_bindings, value)
    }
    #[inline(always)]
    pub(crate) fn prop_bindings_raw(&self) -> *mut List<ShaderPropBinding> {
        view_raw_mut!(self, prop_bindings)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<ShaderTexture, M> {
    #[inline(always)]
    pub(crate) fn type_(&self) -> ShaderTextureType {
        view_read_shared!(self, type_)
    }
    #[inline(always)]
    pub(crate) fn type_ptr(&self) -> *const ShaderTextureType {
        view_raw_shared!(self, type_)
    }
    #[inline(always)]
    pub(crate) fn shader_name(&self) -> String {
        view_read_shared!(self, shader_name)
    }
    #[inline(always)]
    pub(crate) fn shader_name_view(&self) -> &View<String, M> {
        view_project!(self, shader_name)
    }
    #[inline(always)]
    pub(crate) fn shader_name_ptr(&self) -> *const String {
        view_raw_shared!(self, shader_name)
    }
    #[inline(always)]
    pub(crate) fn shader_type_id(&self) -> u64 {
        view_read_shared!(self, shader_type_id)
    }
    #[inline(always)]
    pub(crate) fn shader_type_id_ptr(&self) -> *const u64 {
        view_raw_shared!(self, shader_type_id)
    }
    #[inline(always)]
    pub(crate) fn inputs(&self) -> List<ShaderTextureInput> {
        view_read_shared!(self, inputs)
    }
    #[inline(always)]
    pub(crate) fn inputs_view(&self) -> &View<List<ShaderTextureInput>, M> {
        view_project!(self, inputs)
    }
    #[inline(always)]
    pub(crate) fn inputs_ptr(&self) -> *const List<ShaderTextureInput> {
        view_raw_shared!(self, inputs)
    }
    #[inline(always)]
    pub(crate) fn shader_source(&self) -> String {
        view_read_shared!(self, shader_source)
    }
    #[inline(always)]
    pub(crate) fn shader_source_view(&self) -> &View<String, M> {
        view_project!(self, shader_source)
    }
    #[inline(always)]
    pub(crate) fn shader_source_ptr(&self) -> *const String {
        view_raw_shared!(self, shader_source)
    }
    #[inline(always)]
    pub(crate) fn raw_shader_source(&self) -> Blob {
        view_read_shared!(self, raw_shader_source)
    }
    #[inline(always)]
    pub(crate) fn raw_shader_source_ptr(&self) -> *const Blob {
        view_raw_shared!(self, raw_shader_source)
    }
    #[inline(always)]
    pub(crate) fn main_texture(&self) -> Option<Ref<Texture>> {
        view_read_shared!(self, main_texture)
    }
    #[inline(always)]
    pub(crate) fn main_texture_ptr(&self) -> *const Option<Ref<Texture>> {
        view_raw_shared!(self, main_texture)
    }
    #[inline(always)]
    pub(crate) fn main_texture_output_index(&self) -> i64 {
        view_read_shared!(self, main_texture_output_index)
    }
    #[inline(always)]
    pub(crate) fn main_texture_output_index_ptr(&self) -> *const i64 {
        view_raw_shared!(self, main_texture_output_index)
    }
    #[inline(always)]
    pub(crate) fn prop_prefix(&self) -> String {
        view_read_shared!(self, prop_prefix)
    }
    #[inline(always)]
    pub(crate) fn prop_prefix_view(&self) -> &View<String, M> {
        view_project!(self, prop_prefix)
    }
    #[inline(always)]
    pub(crate) fn prop_prefix_ptr(&self) -> *const String {
        view_raw_shared!(self, prop_prefix)
    }
}

#[allow(dead_code)]
impl View<ShaderTexture, Mut> {
    #[inline(always)]
    pub(crate) fn set_type(&self, value: ShaderTextureType) {
        view_write!(self, type_, value)
    }
    #[inline(always)]
    pub(crate) fn type_raw(&self) -> *mut ShaderTextureType {
        view_raw_mut!(self, type_)
    }
    #[inline(always)]
    pub(crate) fn set_shader_name(&self, value: String) {
        view_write!(self, shader_name, value)
    }
    #[inline(always)]
    pub(crate) fn shader_name_raw(&self) -> *mut String {
        view_raw_mut!(self, shader_name)
    }
    #[inline(always)]
    pub(crate) fn set_shader_type_id(&self, value: u64) {
        view_write!(self, shader_type_id, value)
    }
    #[inline(always)]
    pub(crate) fn shader_type_id_raw(&self) -> *mut u64 {
        view_raw_mut!(self, shader_type_id)
    }
    #[inline(always)]
    pub(crate) fn set_inputs(&self, value: List<ShaderTextureInput>) {
        view_write!(self, inputs, value)
    }
    #[inline(always)]
    pub(crate) fn inputs_raw(&self) -> *mut List<ShaderTextureInput> {
        view_raw_mut!(self, inputs)
    }
    #[inline(always)]
    pub(crate) fn set_shader_source(&self, value: String) {
        view_write!(self, shader_source, value)
    }
    #[inline(always)]
    pub(crate) fn shader_source_raw(&self) -> *mut String {
        view_raw_mut!(self, shader_source)
    }
    #[inline(always)]
    pub(crate) fn set_raw_shader_source(&self, value: Blob) {
        view_write!(self, raw_shader_source, value)
    }
    #[inline(always)]
    pub(crate) fn raw_shader_source_raw(&self) -> *mut Blob {
        view_raw_mut!(self, raw_shader_source)
    }
    #[inline(always)]
    pub(crate) fn set_main_texture(&self, value: Option<Ref<Texture>>) {
        view_write!(self, main_texture, value)
    }
    #[inline(always)]
    pub(crate) fn main_texture_raw(&self) -> *mut Option<Ref<Texture>> {
        view_raw_mut!(self, main_texture)
    }
    #[inline(always)]
    pub(crate) fn set_main_texture_output_index(&self, value: i64) {
        view_write!(self, main_texture_output_index, value)
    }
    #[inline(always)]
    pub(crate) fn main_texture_output_index_raw(&self) -> *mut i64 {
        view_raw_mut!(self, main_texture_output_index)
    }
    #[inline(always)]
    pub(crate) fn set_prop_prefix(&self, value: String) {
        view_write!(self, prop_prefix, value)
    }
    #[inline(always)]
    pub(crate) fn prop_prefix_raw(&self) -> *mut String {
        view_raw_mut!(self, prop_prefix)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<ShaderTextureInput, M> {
    #[inline(always)]
    pub(crate) fn name(&self) -> String {
        view_read_shared!(self, name)
    }
    #[inline(always)]
    pub(crate) fn name_view(&self) -> &View<String, M> {
        view_project!(self, name)
    }
    #[inline(always)]
    pub(crate) fn name_ptr(&self) -> *const String {
        view_raw_shared!(self, name)
    }
    #[inline(always)]
    pub(crate) fn value_vec4(&self) -> Vec4 {
        view_read_shared!(self, value_vec4)
    }
    #[inline(always)]
    pub(crate) fn value_vec4_ptr(&self) -> *const Vec4 {
        view_raw_shared!(self, value_vec4)
    }
    #[inline(always)]
    pub(crate) fn value_int(&self) -> i64 {
        view_read_shared!(self, value_int)
    }
    #[inline(always)]
    pub(crate) fn value_int_ptr(&self) -> *const i64 {
        view_raw_shared!(self, value_int)
    }
    #[inline(always)]
    pub(crate) fn value_str(&self) -> String {
        view_read_shared!(self, value_str)
    }
    #[inline(always)]
    pub(crate) fn value_str_view(&self) -> &View<String, M> {
        view_project!(self, value_str)
    }
    #[inline(always)]
    pub(crate) fn value_str_ptr(&self) -> *const String {
        view_raw_shared!(self, value_str)
    }
    #[inline(always)]
    pub(crate) fn value_blob(&self) -> Blob {
        view_read_shared!(self, value_blob)
    }
    #[inline(always)]
    pub(crate) fn value_blob_ptr(&self) -> *const Blob {
        view_raw_shared!(self, value_blob)
    }
    #[inline(always)]
    pub(crate) fn texture(&self) -> Option<Ref<Texture>> {
        view_read_shared!(self, texture)
    }
    #[inline(always)]
    pub(crate) fn texture_ptr(&self) -> *const Option<Ref<Texture>> {
        view_raw_shared!(self, texture)
    }
    #[inline(always)]
    pub(crate) fn texture_output_index(&self) -> i64 {
        view_read_shared!(self, texture_output_index)
    }
    #[inline(always)]
    pub(crate) fn texture_output_index_ptr(&self) -> *const i64 {
        view_raw_shared!(self, texture_output_index)
    }
    #[inline(always)]
    pub(crate) fn texture_enabled(&self) -> bool {
        view_read_shared!(self, texture_enabled)
    }
    #[inline(always)]
    pub(crate) fn texture_enabled_ptr(&self) -> *const bool {
        view_raw_shared!(self, texture_enabled)
    }
    #[inline(always)]
    pub(crate) fn prop(&self) -> Option<Ref<Prop>> {
        view_read_shared!(self, prop)
    }
    #[inline(always)]
    pub(crate) fn prop_ptr(&self) -> *const Option<Ref<Prop>> {
        view_raw_shared!(self, prop)
    }
    #[inline(always)]
    pub(crate) fn texture_prop(&self) -> Option<Ref<Prop>> {
        view_read_shared!(self, texture_prop)
    }
    #[inline(always)]
    pub(crate) fn texture_prop_ptr(&self) -> *const Option<Ref<Prop>> {
        view_raw_shared!(self, texture_prop)
    }
    #[inline(always)]
    pub(crate) fn texture_enabled_prop(&self) -> Option<Ref<Prop>> {
        view_read_shared!(self, texture_enabled_prop)
    }
    #[inline(always)]
    pub(crate) fn texture_enabled_prop_ptr(&self) -> *const Option<Ref<Prop>> {
        view_raw_shared!(self, texture_enabled_prop)
    }
}

#[allow(dead_code)]
impl View<ShaderTextureInput, Mut> {
    #[inline(always)]
    pub(crate) fn set_name(&self, value: String) {
        view_write!(self, name, value)
    }
    #[inline(always)]
    pub(crate) fn name_raw(&self) -> *mut String {
        view_raw_mut!(self, name)
    }
    #[inline(always)]
    pub(crate) fn set_value_vec4(&self, value: Vec4) {
        view_write!(self, value_vec4, value)
    }
    #[inline(always)]
    pub(crate) fn value_vec4_raw(&self) -> *mut Vec4 {
        view_raw_mut!(self, value_vec4)
    }
    #[inline(always)]
    pub(crate) fn set_value_int(&self, value: i64) {
        view_write!(self, value_int, value)
    }
    #[inline(always)]
    pub(crate) fn value_int_raw(&self) -> *mut i64 {
        view_raw_mut!(self, value_int)
    }
    #[inline(always)]
    pub(crate) fn set_value_str(&self, value: String) {
        view_write!(self, value_str, value)
    }
    #[inline(always)]
    pub(crate) fn value_str_raw(&self) -> *mut String {
        view_raw_mut!(self, value_str)
    }
    #[inline(always)]
    pub(crate) fn set_value_blob(&self, value: Blob) {
        view_write!(self, value_blob, value)
    }
    #[inline(always)]
    pub(crate) fn value_blob_raw(&self) -> *mut Blob {
        view_raw_mut!(self, value_blob)
    }
    #[inline(always)]
    pub(crate) fn set_texture(&self, value: Option<Ref<Texture>>) {
        view_write!(self, texture, value)
    }
    #[inline(always)]
    pub(crate) fn texture_raw(&self) -> *mut Option<Ref<Texture>> {
        view_raw_mut!(self, texture)
    }
    #[inline(always)]
    pub(crate) fn set_texture_output_index(&self, value: i64) {
        view_write!(self, texture_output_index, value)
    }
    #[inline(always)]
    pub(crate) fn texture_output_index_raw(&self) -> *mut i64 {
        view_raw_mut!(self, texture_output_index)
    }
    #[inline(always)]
    pub(crate) fn set_texture_enabled(&self, value: bool) {
        view_write!(self, texture_enabled, value)
    }
    #[inline(always)]
    pub(crate) fn texture_enabled_raw(&self) -> *mut bool {
        view_raw_mut!(self, texture_enabled)
    }
    #[inline(always)]
    pub(crate) fn set_prop(&self, value: Option<Ref<Prop>>) {
        view_write!(self, prop, value)
    }
    #[inline(always)]
    pub(crate) fn prop_raw(&self) -> *mut Option<Ref<Prop>> {
        view_raw_mut!(self, prop)
    }
    #[inline(always)]
    pub(crate) fn set_texture_prop(&self, value: Option<Ref<Prop>>) {
        view_write!(self, texture_prop, value)
    }
    #[inline(always)]
    pub(crate) fn texture_prop_raw(&self) -> *mut Option<Ref<Prop>> {
        view_raw_mut!(self, texture_prop)
    }
    #[inline(always)]
    pub(crate) fn set_texture_enabled_prop(&self, value: Option<Ref<Prop>>) {
        view_write!(self, texture_enabled_prop, value)
    }
    #[inline(always)]
    pub(crate) fn texture_enabled_prop_raw(&self) -> *mut Option<Ref<Prop>> {
        view_raw_mut!(self, texture_enabled_prop)
    }
}
