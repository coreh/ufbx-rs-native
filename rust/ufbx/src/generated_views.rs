// GENERATED FILE — do not edit by hand. Produced by rust/regen.sh from
// ufbx.h via bindgen/ufbx_ir.py + rust/ufbx/bindgen/generate_rust.py.
// Fixes belong in the GENERATOR (see PORTING.md); hand edits are
// silently overwritten on the next regeneration and CI diffs this file.
//
// Crate-internal `View<T, M>` field accessors over the generated public
// structs (`view_accessor_structs` in generate_rust.py): a by-value read
// per leaf field, an in-place `&View` projection per aggregate and list
// (`*_view`) field, a followed-`Ref` projection per element-reference
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
    pub(crate) fn dom_node_view(&self) -> Option<&View<DomNode, M>> {
        self.dom_node().map(Ref::view)
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
    pub(crate) fn defaults(&self) -> Option<Ref<Props>> {
        view_read_shared!(self, defaults)
    }
    #[inline(always)]
    pub(crate) fn defaults_view(&self) -> Option<&View<Props, M>> {
        self.defaults().map(Ref::view)
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
    pub(crate) fn subdivision_result_view(&self) -> Option<&View<SubdivisionResult, M>> {
        self.subdivision_result().map(Ref::view)
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
    pub(crate) fn file_view(&self) -> Option<&View<CacheFile, M>> {
        self.file().map(Ref::view)
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
    pub(crate) fn external_cache_view(&self) -> Option<&View<GeometryCache, M>> {
        self.external_cache().map(Ref::view)
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
    pub(crate) fn external_channel_view(&self) -> Option<&View<CacheChannel, M>> {
        self.external_channel().map(Ref::view)
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
    pub(crate) fn texture_view(&self) -> Option<&View<Texture, M>> {
        self.texture().map(Ref::view)
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
    pub(crate) fn texture_view(&self) -> &View<Texture, M> {
        self.texture().view()
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
    pub(crate) fn src_view(&self) -> &View<Element, M> {
        self.src().view()
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
    pub(crate) fn dst_view(&self) -> &View<Element, M> {
        self.dst().view()
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
    pub(crate) fn anim_view(&self) -> &View<Anim, M> {
        self.anim().view()
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
    pub(crate) fn anim_value_view(&self) -> &View<AnimValue, M> {
        self.anim_value().view()
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
    pub(crate) fn main_texture_view(&self) -> Option<&View<Texture, M>> {
        self.main_texture().map(Ref::view)
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
    pub(crate) fn texture_view(&self) -> Option<&View<Texture, M>> {
        self.texture().map(Ref::view)
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
    pub(crate) fn prop_view(&self) -> Option<&View<Prop, M>> {
        self.prop().map(Ref::view)
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
    pub(crate) fn texture_prop_view(&self) -> Option<&View<Prop, M>> {
        self.texture_prop().map(Ref::view)
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
    pub(crate) fn texture_enabled_prop_view(&self) -> Option<&View<Prop, M>> {
        self.texture_enabled_prop().map(Ref::view)
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

#[allow(dead_code)]
impl<M: Mode> View<NameElement, M> {
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
    pub(crate) fn type_(&self) -> ElementType {
        view_read_shared!(self, type_)
    }
    #[inline(always)]
    pub(crate) fn type_ptr(&self) -> *const ElementType {
        view_raw_shared!(self, type_)
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
    pub(crate) fn element(&self) -> Ref<Element> {
        view_read_shared!(self, element)
    }
    #[inline(always)]
    pub(crate) fn element_view(&self) -> &View<Element, M> {
        self.element().view()
    }
    #[inline(always)]
    pub(crate) fn element_ptr(&self) -> *const Ref<Element> {
        view_raw_shared!(self, element)
    }
}

#[allow(dead_code)]
impl View<NameElement, Mut> {
    #[inline(always)]
    pub(crate) fn set_name(&self, value: String) {
        view_write!(self, name, value)
    }
    #[inline(always)]
    pub(crate) fn name_raw(&self) -> *mut String {
        view_raw_mut!(self, name)
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
    pub(crate) fn set_internal_key(&self, value: u32) {
        view_write!(self, _internal_key, value)
    }
    #[inline(always)]
    pub(crate) fn internal_key_raw(&self) -> *mut u32 {
        view_raw_mut!(self, _internal_key)
    }
    #[inline(always)]
    pub(crate) fn set_element(&self, value: Ref<Element>) {
        view_write!(self, element, value)
    }
    #[inline(always)]
    pub(crate) fn element_raw(&self) -> *mut Ref<Element> {
        view_raw_mut!(self, element)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<Node, M> {
    #[inline(always)]
    pub(crate) fn element(&self) -> &View<Element, M> {
        view_project!(self, element)
    }
    #[inline(always)]
    pub(crate) fn element_ptr(&self) -> *const Element {
        view_raw_shared!(self, element)
    }
    #[inline(always)]
    pub(crate) fn parent(&self) -> Option<Ref<Node>> {
        view_read_shared!(self, parent)
    }
    #[inline(always)]
    pub(crate) fn parent_view(&self) -> Option<&View<Node, M>> {
        self.parent().map(Ref::view)
    }
    #[inline(always)]
    pub(crate) fn parent_ptr(&self) -> *const Option<Ref<Node>> {
        view_raw_shared!(self, parent)
    }
    #[inline(always)]
    pub(crate) fn children(&self) -> RefList<Node> {
        view_read_shared!(self, children)
    }
    #[inline(always)]
    pub(crate) fn children_view(&self) -> &View<RefList<Node>, M> {
        view_project!(self, children)
    }
    #[inline(always)]
    pub(crate) fn children_ptr(&self) -> *const RefList<Node> {
        view_raw_shared!(self, children)
    }
    #[inline(always)]
    pub(crate) fn mesh(&self) -> Option<Ref<Mesh>> {
        view_read_shared!(self, mesh)
    }
    #[inline(always)]
    pub(crate) fn mesh_view(&self) -> Option<&View<Mesh, M>> {
        self.mesh().map(Ref::view)
    }
    #[inline(always)]
    pub(crate) fn mesh_ptr(&self) -> *const Option<Ref<Mesh>> {
        view_raw_shared!(self, mesh)
    }
    #[inline(always)]
    pub(crate) fn light(&self) -> Option<Ref<Light>> {
        view_read_shared!(self, light)
    }
    #[inline(always)]
    pub(crate) fn light_view(&self) -> Option<&View<Light, M>> {
        self.light().map(Ref::view)
    }
    #[inline(always)]
    pub(crate) fn light_ptr(&self) -> *const Option<Ref<Light>> {
        view_raw_shared!(self, light)
    }
    #[inline(always)]
    pub(crate) fn camera(&self) -> Option<Ref<Camera>> {
        view_read_shared!(self, camera)
    }
    #[inline(always)]
    pub(crate) fn camera_view(&self) -> Option<&View<Camera, M>> {
        self.camera().map(Ref::view)
    }
    #[inline(always)]
    pub(crate) fn camera_ptr(&self) -> *const Option<Ref<Camera>> {
        view_raw_shared!(self, camera)
    }
    #[inline(always)]
    pub(crate) fn bone(&self) -> Option<Ref<Bone>> {
        view_read_shared!(self, bone)
    }
    #[inline(always)]
    pub(crate) fn bone_view(&self) -> Option<&View<Bone, M>> {
        self.bone().map(Ref::view)
    }
    #[inline(always)]
    pub(crate) fn bone_ptr(&self) -> *const Option<Ref<Bone>> {
        view_raw_shared!(self, bone)
    }
    #[inline(always)]
    pub(crate) fn attrib(&self) -> Option<Ref<Element>> {
        view_read_shared!(self, attrib)
    }
    #[inline(always)]
    pub(crate) fn attrib_view(&self) -> Option<&View<Element, M>> {
        self.attrib().map(Ref::view)
    }
    #[inline(always)]
    pub(crate) fn attrib_ptr(&self) -> *const Option<Ref<Element>> {
        view_raw_shared!(self, attrib)
    }
    #[inline(always)]
    pub(crate) fn geometry_transform_helper(&self) -> Option<Ref<Node>> {
        view_read_shared!(self, geometry_transform_helper)
    }
    #[inline(always)]
    pub(crate) fn geometry_transform_helper_view(&self) -> Option<&View<Node, M>> {
        self.geometry_transform_helper().map(Ref::view)
    }
    #[inline(always)]
    pub(crate) fn geometry_transform_helper_ptr(&self) -> *const Option<Ref<Node>> {
        view_raw_shared!(self, geometry_transform_helper)
    }
    #[inline(always)]
    pub(crate) fn scale_helper(&self) -> Option<Ref<Node>> {
        view_read_shared!(self, scale_helper)
    }
    #[inline(always)]
    pub(crate) fn scale_helper_view(&self) -> Option<&View<Node, M>> {
        self.scale_helper().map(Ref::view)
    }
    #[inline(always)]
    pub(crate) fn scale_helper_ptr(&self) -> *const Option<Ref<Node>> {
        view_raw_shared!(self, scale_helper)
    }
    #[inline(always)]
    pub(crate) fn attrib_type(&self) -> ElementType {
        view_read_shared!(self, attrib_type)
    }
    #[inline(always)]
    pub(crate) fn attrib_type_ptr(&self) -> *const ElementType {
        view_raw_shared!(self, attrib_type)
    }
    #[inline(always)]
    pub(crate) fn all_attribs(&self) -> RefList<Element> {
        view_read_shared!(self, all_attribs)
    }
    #[inline(always)]
    pub(crate) fn all_attribs_view(&self) -> &View<RefList<Element>, M> {
        view_project!(self, all_attribs)
    }
    #[inline(always)]
    pub(crate) fn all_attribs_ptr(&self) -> *const RefList<Element> {
        view_raw_shared!(self, all_attribs)
    }
    #[inline(always)]
    pub(crate) fn inherit_mode(&self) -> InheritMode {
        view_read_shared!(self, inherit_mode)
    }
    #[inline(always)]
    pub(crate) fn inherit_mode_ptr(&self) -> *const InheritMode {
        view_raw_shared!(self, inherit_mode)
    }
    #[inline(always)]
    pub(crate) fn original_inherit_mode(&self) -> InheritMode {
        view_read_shared!(self, original_inherit_mode)
    }
    #[inline(always)]
    pub(crate) fn original_inherit_mode_ptr(&self) -> *const InheritMode {
        view_raw_shared!(self, original_inherit_mode)
    }
    #[inline(always)]
    pub(crate) fn local_transform(&self) -> Transform {
        view_read_shared!(self, local_transform)
    }
    #[inline(always)]
    pub(crate) fn local_transform_ptr(&self) -> *const Transform {
        view_raw_shared!(self, local_transform)
    }
    #[inline(always)]
    pub(crate) fn geometry_transform(&self) -> Transform {
        view_read_shared!(self, geometry_transform)
    }
    #[inline(always)]
    pub(crate) fn geometry_transform_ptr(&self) -> *const Transform {
        view_raw_shared!(self, geometry_transform)
    }
    #[inline(always)]
    pub(crate) fn inherit_scale(&self) -> Vec3 {
        view_read_shared!(self, inherit_scale)
    }
    #[inline(always)]
    pub(crate) fn inherit_scale_ptr(&self) -> *const Vec3 {
        view_raw_shared!(self, inherit_scale)
    }
    #[inline(always)]
    pub(crate) fn inherit_scale_node(&self) -> Option<Ref<Node>> {
        view_read_shared!(self, inherit_scale_node)
    }
    #[inline(always)]
    pub(crate) fn inherit_scale_node_view(&self) -> Option<&View<Node, M>> {
        self.inherit_scale_node().map(Ref::view)
    }
    #[inline(always)]
    pub(crate) fn inherit_scale_node_ptr(&self) -> *const Option<Ref<Node>> {
        view_raw_shared!(self, inherit_scale_node)
    }
    #[inline(always)]
    pub(crate) fn rotation_order(&self) -> RotationOrder {
        view_read_shared!(self, rotation_order)
    }
    #[inline(always)]
    pub(crate) fn rotation_order_ptr(&self) -> *const RotationOrder {
        view_raw_shared!(self, rotation_order)
    }
    #[inline(always)]
    pub(crate) fn euler_rotation(&self) -> Vec3 {
        view_read_shared!(self, euler_rotation)
    }
    #[inline(always)]
    pub(crate) fn euler_rotation_ptr(&self) -> *const Vec3 {
        view_raw_shared!(self, euler_rotation)
    }
    #[inline(always)]
    pub(crate) fn node_to_parent(&self) -> Matrix {
        view_read_shared!(self, node_to_parent)
    }
    #[inline(always)]
    pub(crate) fn node_to_parent_ptr(&self) -> *const Matrix {
        view_raw_shared!(self, node_to_parent)
    }
    #[inline(always)]
    pub(crate) fn node_to_world(&self) -> Matrix {
        view_read_shared!(self, node_to_world)
    }
    #[inline(always)]
    pub(crate) fn node_to_world_ptr(&self) -> *const Matrix {
        view_raw_shared!(self, node_to_world)
    }
    #[inline(always)]
    pub(crate) fn geometry_to_node(&self) -> Matrix {
        view_read_shared!(self, geometry_to_node)
    }
    #[inline(always)]
    pub(crate) fn geometry_to_node_ptr(&self) -> *const Matrix {
        view_raw_shared!(self, geometry_to_node)
    }
    #[inline(always)]
    pub(crate) fn geometry_to_world(&self) -> Matrix {
        view_read_shared!(self, geometry_to_world)
    }
    #[inline(always)]
    pub(crate) fn geometry_to_world_ptr(&self) -> *const Matrix {
        view_raw_shared!(self, geometry_to_world)
    }
    #[inline(always)]
    pub(crate) fn unscaled_node_to_world(&self) -> Matrix {
        view_read_shared!(self, unscaled_node_to_world)
    }
    #[inline(always)]
    pub(crate) fn unscaled_node_to_world_ptr(&self) -> *const Matrix {
        view_raw_shared!(self, unscaled_node_to_world)
    }
    #[inline(always)]
    pub(crate) fn adjust_pre_translation(&self) -> Vec3 {
        view_read_shared!(self, adjust_pre_translation)
    }
    #[inline(always)]
    pub(crate) fn adjust_pre_translation_ptr(&self) -> *const Vec3 {
        view_raw_shared!(self, adjust_pre_translation)
    }
    #[inline(always)]
    pub(crate) fn adjust_pre_rotation(&self) -> Quat {
        view_read_shared!(self, adjust_pre_rotation)
    }
    #[inline(always)]
    pub(crate) fn adjust_pre_rotation_ptr(&self) -> *const Quat {
        view_raw_shared!(self, adjust_pre_rotation)
    }
    #[inline(always)]
    pub(crate) fn adjust_pre_scale(&self) -> Real {
        view_read_shared!(self, adjust_pre_scale)
    }
    #[inline(always)]
    pub(crate) fn adjust_pre_scale_ptr(&self) -> *const Real {
        view_raw_shared!(self, adjust_pre_scale)
    }
    #[inline(always)]
    pub(crate) fn adjust_post_rotation(&self) -> Quat {
        view_read_shared!(self, adjust_post_rotation)
    }
    #[inline(always)]
    pub(crate) fn adjust_post_rotation_ptr(&self) -> *const Quat {
        view_raw_shared!(self, adjust_post_rotation)
    }
    #[inline(always)]
    pub(crate) fn adjust_post_scale(&self) -> Real {
        view_read_shared!(self, adjust_post_scale)
    }
    #[inline(always)]
    pub(crate) fn adjust_post_scale_ptr(&self) -> *const Real {
        view_raw_shared!(self, adjust_post_scale)
    }
    #[inline(always)]
    pub(crate) fn adjust_translation_scale(&self) -> Real {
        view_read_shared!(self, adjust_translation_scale)
    }
    #[inline(always)]
    pub(crate) fn adjust_translation_scale_ptr(&self) -> *const Real {
        view_raw_shared!(self, adjust_translation_scale)
    }
    #[inline(always)]
    pub(crate) fn adjust_mirror_axis(&self) -> MirrorAxis {
        view_read_shared!(self, adjust_mirror_axis)
    }
    #[inline(always)]
    pub(crate) fn adjust_mirror_axis_ptr(&self) -> *const MirrorAxis {
        view_raw_shared!(self, adjust_mirror_axis)
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
    pub(crate) fn bind_pose(&self) -> Option<Ref<Pose>> {
        view_read_shared!(self, bind_pose)
    }
    #[inline(always)]
    pub(crate) fn bind_pose_view(&self) -> Option<&View<Pose, M>> {
        self.bind_pose().map(Ref::view)
    }
    #[inline(always)]
    pub(crate) fn bind_pose_ptr(&self) -> *const Option<Ref<Pose>> {
        view_raw_shared!(self, bind_pose)
    }
    #[inline(always)]
    pub(crate) fn visible(&self) -> bool {
        view_read_shared!(self, visible)
    }
    #[inline(always)]
    pub(crate) fn visible_ptr(&self) -> *const bool {
        view_raw_shared!(self, visible)
    }
    #[inline(always)]
    pub(crate) fn is_root(&self) -> bool {
        view_read_shared!(self, is_root)
    }
    #[inline(always)]
    pub(crate) fn is_root_ptr(&self) -> *const bool {
        view_raw_shared!(self, is_root)
    }
    #[inline(always)]
    pub(crate) fn has_geometry_transform(&self) -> bool {
        view_read_shared!(self, has_geometry_transform)
    }
    #[inline(always)]
    pub(crate) fn has_geometry_transform_ptr(&self) -> *const bool {
        view_raw_shared!(self, has_geometry_transform)
    }
    #[inline(always)]
    pub(crate) fn use_rotation_space(&self) -> bool {
        view_read_shared!(self, use_rotation_space)
    }
    #[inline(always)]
    pub(crate) fn use_rotation_space_ptr(&self) -> *const bool {
        view_raw_shared!(self, use_rotation_space)
    }
    #[inline(always)]
    pub(crate) fn has_adjust_transform(&self) -> bool {
        view_read_shared!(self, has_adjust_transform)
    }
    #[inline(always)]
    pub(crate) fn has_adjust_transform_ptr(&self) -> *const bool {
        view_raw_shared!(self, has_adjust_transform)
    }
    #[inline(always)]
    pub(crate) fn has_root_adjust_transform(&self) -> bool {
        view_read_shared!(self, has_root_adjust_transform)
    }
    #[inline(always)]
    pub(crate) fn has_root_adjust_transform_ptr(&self) -> *const bool {
        view_raw_shared!(self, has_root_adjust_transform)
    }
    #[inline(always)]
    pub(crate) fn is_geometry_transform_helper(&self) -> bool {
        view_read_shared!(self, is_geometry_transform_helper)
    }
    #[inline(always)]
    pub(crate) fn is_geometry_transform_helper_ptr(&self) -> *const bool {
        view_raw_shared!(self, is_geometry_transform_helper)
    }
    #[inline(always)]
    pub(crate) fn is_scale_helper(&self) -> bool {
        view_read_shared!(self, is_scale_helper)
    }
    #[inline(always)]
    pub(crate) fn is_scale_helper_ptr(&self) -> *const bool {
        view_raw_shared!(self, is_scale_helper)
    }
    #[inline(always)]
    pub(crate) fn is_scale_compensate_parent(&self) -> bool {
        view_read_shared!(self, is_scale_compensate_parent)
    }
    #[inline(always)]
    pub(crate) fn is_scale_compensate_parent_ptr(&self) -> *const bool {
        view_raw_shared!(self, is_scale_compensate_parent)
    }
    #[inline(always)]
    pub(crate) fn node_depth(&self) -> u32 {
        view_read_shared!(self, node_depth)
    }
    #[inline(always)]
    pub(crate) fn node_depth_ptr(&self) -> *const u32 {
        view_raw_shared!(self, node_depth)
    }
}

#[allow(dead_code)]
impl View<Node, Mut> {
    #[inline(always)]
    pub(crate) fn set_element(&self, value: Element) {
        view_write!(self, element, value)
    }
    #[inline(always)]
    pub(crate) fn element_raw(&self) -> *mut Element {
        view_raw_mut!(self, element)
    }
    #[inline(always)]
    pub(crate) fn set_parent(&self, value: Option<Ref<Node>>) {
        view_write!(self, parent, value)
    }
    #[inline(always)]
    pub(crate) fn parent_raw(&self) -> *mut Option<Ref<Node>> {
        view_raw_mut!(self, parent)
    }
    #[inline(always)]
    pub(crate) fn set_children(&self, value: RefList<Node>) {
        view_write!(self, children, value)
    }
    #[inline(always)]
    pub(crate) fn children_raw(&self) -> *mut RefList<Node> {
        view_raw_mut!(self, children)
    }
    #[inline(always)]
    pub(crate) fn set_mesh(&self, value: Option<Ref<Mesh>>) {
        view_write!(self, mesh, value)
    }
    #[inline(always)]
    pub(crate) fn mesh_raw(&self) -> *mut Option<Ref<Mesh>> {
        view_raw_mut!(self, mesh)
    }
    #[inline(always)]
    pub(crate) fn set_light(&self, value: Option<Ref<Light>>) {
        view_write!(self, light, value)
    }
    #[inline(always)]
    pub(crate) fn light_raw(&self) -> *mut Option<Ref<Light>> {
        view_raw_mut!(self, light)
    }
    #[inline(always)]
    pub(crate) fn set_camera(&self, value: Option<Ref<Camera>>) {
        view_write!(self, camera, value)
    }
    #[inline(always)]
    pub(crate) fn camera_raw(&self) -> *mut Option<Ref<Camera>> {
        view_raw_mut!(self, camera)
    }
    #[inline(always)]
    pub(crate) fn set_bone(&self, value: Option<Ref<Bone>>) {
        view_write!(self, bone, value)
    }
    #[inline(always)]
    pub(crate) fn bone_raw(&self) -> *mut Option<Ref<Bone>> {
        view_raw_mut!(self, bone)
    }
    #[inline(always)]
    pub(crate) fn set_attrib(&self, value: Option<Ref<Element>>) {
        view_write!(self, attrib, value)
    }
    #[inline(always)]
    pub(crate) fn attrib_raw(&self) -> *mut Option<Ref<Element>> {
        view_raw_mut!(self, attrib)
    }
    #[inline(always)]
    pub(crate) fn set_geometry_transform_helper(&self, value: Option<Ref<Node>>) {
        view_write!(self, geometry_transform_helper, value)
    }
    #[inline(always)]
    pub(crate) fn geometry_transform_helper_raw(&self) -> *mut Option<Ref<Node>> {
        view_raw_mut!(self, geometry_transform_helper)
    }
    #[inline(always)]
    pub(crate) fn set_scale_helper(&self, value: Option<Ref<Node>>) {
        view_write!(self, scale_helper, value)
    }
    #[inline(always)]
    pub(crate) fn scale_helper_raw(&self) -> *mut Option<Ref<Node>> {
        view_raw_mut!(self, scale_helper)
    }
    #[inline(always)]
    pub(crate) fn set_attrib_type(&self, value: ElementType) {
        view_write!(self, attrib_type, value)
    }
    #[inline(always)]
    pub(crate) fn attrib_type_raw(&self) -> *mut ElementType {
        view_raw_mut!(self, attrib_type)
    }
    #[inline(always)]
    pub(crate) fn set_all_attribs(&self, value: RefList<Element>) {
        view_write!(self, all_attribs, value)
    }
    #[inline(always)]
    pub(crate) fn all_attribs_raw(&self) -> *mut RefList<Element> {
        view_raw_mut!(self, all_attribs)
    }
    #[inline(always)]
    pub(crate) fn set_inherit_mode(&self, value: InheritMode) {
        view_write!(self, inherit_mode, value)
    }
    #[inline(always)]
    pub(crate) fn inherit_mode_raw(&self) -> *mut InheritMode {
        view_raw_mut!(self, inherit_mode)
    }
    #[inline(always)]
    pub(crate) fn set_original_inherit_mode(&self, value: InheritMode) {
        view_write!(self, original_inherit_mode, value)
    }
    #[inline(always)]
    pub(crate) fn original_inherit_mode_raw(&self) -> *mut InheritMode {
        view_raw_mut!(self, original_inherit_mode)
    }
    #[inline(always)]
    pub(crate) fn set_local_transform(&self, value: Transform) {
        view_write!(self, local_transform, value)
    }
    #[inline(always)]
    pub(crate) fn local_transform_raw(&self) -> *mut Transform {
        view_raw_mut!(self, local_transform)
    }
    #[inline(always)]
    pub(crate) fn set_geometry_transform(&self, value: Transform) {
        view_write!(self, geometry_transform, value)
    }
    #[inline(always)]
    pub(crate) fn geometry_transform_raw(&self) -> *mut Transform {
        view_raw_mut!(self, geometry_transform)
    }
    #[inline(always)]
    pub(crate) fn set_inherit_scale(&self, value: Vec3) {
        view_write!(self, inherit_scale, value)
    }
    #[inline(always)]
    pub(crate) fn inherit_scale_raw(&self) -> *mut Vec3 {
        view_raw_mut!(self, inherit_scale)
    }
    #[inline(always)]
    pub(crate) fn set_inherit_scale_node(&self, value: Option<Ref<Node>>) {
        view_write!(self, inherit_scale_node, value)
    }
    #[inline(always)]
    pub(crate) fn inherit_scale_node_raw(&self) -> *mut Option<Ref<Node>> {
        view_raw_mut!(self, inherit_scale_node)
    }
    #[inline(always)]
    pub(crate) fn set_rotation_order(&self, value: RotationOrder) {
        view_write!(self, rotation_order, value)
    }
    #[inline(always)]
    pub(crate) fn rotation_order_raw(&self) -> *mut RotationOrder {
        view_raw_mut!(self, rotation_order)
    }
    #[inline(always)]
    pub(crate) fn set_euler_rotation(&self, value: Vec3) {
        view_write!(self, euler_rotation, value)
    }
    #[inline(always)]
    pub(crate) fn euler_rotation_raw(&self) -> *mut Vec3 {
        view_raw_mut!(self, euler_rotation)
    }
    #[inline(always)]
    pub(crate) fn set_node_to_parent(&self, value: Matrix) {
        view_write!(self, node_to_parent, value)
    }
    #[inline(always)]
    pub(crate) fn node_to_parent_raw(&self) -> *mut Matrix {
        view_raw_mut!(self, node_to_parent)
    }
    #[inline(always)]
    pub(crate) fn set_node_to_world(&self, value: Matrix) {
        view_write!(self, node_to_world, value)
    }
    #[inline(always)]
    pub(crate) fn node_to_world_raw(&self) -> *mut Matrix {
        view_raw_mut!(self, node_to_world)
    }
    #[inline(always)]
    pub(crate) fn set_geometry_to_node(&self, value: Matrix) {
        view_write!(self, geometry_to_node, value)
    }
    #[inline(always)]
    pub(crate) fn geometry_to_node_raw(&self) -> *mut Matrix {
        view_raw_mut!(self, geometry_to_node)
    }
    #[inline(always)]
    pub(crate) fn set_geometry_to_world(&self, value: Matrix) {
        view_write!(self, geometry_to_world, value)
    }
    #[inline(always)]
    pub(crate) fn geometry_to_world_raw(&self) -> *mut Matrix {
        view_raw_mut!(self, geometry_to_world)
    }
    #[inline(always)]
    pub(crate) fn set_unscaled_node_to_world(&self, value: Matrix) {
        view_write!(self, unscaled_node_to_world, value)
    }
    #[inline(always)]
    pub(crate) fn unscaled_node_to_world_raw(&self) -> *mut Matrix {
        view_raw_mut!(self, unscaled_node_to_world)
    }
    #[inline(always)]
    pub(crate) fn set_adjust_pre_translation(&self, value: Vec3) {
        view_write!(self, adjust_pre_translation, value)
    }
    #[inline(always)]
    pub(crate) fn adjust_pre_translation_raw(&self) -> *mut Vec3 {
        view_raw_mut!(self, adjust_pre_translation)
    }
    #[inline(always)]
    pub(crate) fn set_adjust_pre_rotation(&self, value: Quat) {
        view_write!(self, adjust_pre_rotation, value)
    }
    #[inline(always)]
    pub(crate) fn adjust_pre_rotation_raw(&self) -> *mut Quat {
        view_raw_mut!(self, adjust_pre_rotation)
    }
    #[inline(always)]
    pub(crate) fn set_adjust_pre_scale(&self, value: Real) {
        view_write!(self, adjust_pre_scale, value)
    }
    #[inline(always)]
    pub(crate) fn adjust_pre_scale_raw(&self) -> *mut Real {
        view_raw_mut!(self, adjust_pre_scale)
    }
    #[inline(always)]
    pub(crate) fn set_adjust_post_rotation(&self, value: Quat) {
        view_write!(self, adjust_post_rotation, value)
    }
    #[inline(always)]
    pub(crate) fn adjust_post_rotation_raw(&self) -> *mut Quat {
        view_raw_mut!(self, adjust_post_rotation)
    }
    #[inline(always)]
    pub(crate) fn set_adjust_post_scale(&self, value: Real) {
        view_write!(self, adjust_post_scale, value)
    }
    #[inline(always)]
    pub(crate) fn adjust_post_scale_raw(&self) -> *mut Real {
        view_raw_mut!(self, adjust_post_scale)
    }
    #[inline(always)]
    pub(crate) fn set_adjust_translation_scale(&self, value: Real) {
        view_write!(self, adjust_translation_scale, value)
    }
    #[inline(always)]
    pub(crate) fn adjust_translation_scale_raw(&self) -> *mut Real {
        view_raw_mut!(self, adjust_translation_scale)
    }
    #[inline(always)]
    pub(crate) fn set_adjust_mirror_axis(&self, value: MirrorAxis) {
        view_write!(self, adjust_mirror_axis, value)
    }
    #[inline(always)]
    pub(crate) fn adjust_mirror_axis_raw(&self) -> *mut MirrorAxis {
        view_raw_mut!(self, adjust_mirror_axis)
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
    pub(crate) fn set_bind_pose(&self, value: Option<Ref<Pose>>) {
        view_write!(self, bind_pose, value)
    }
    #[inline(always)]
    pub(crate) fn bind_pose_raw(&self) -> *mut Option<Ref<Pose>> {
        view_raw_mut!(self, bind_pose)
    }
    #[inline(always)]
    pub(crate) fn set_visible(&self, value: bool) {
        view_write!(self, visible, value)
    }
    #[inline(always)]
    pub(crate) fn visible_raw(&self) -> *mut bool {
        view_raw_mut!(self, visible)
    }
    #[inline(always)]
    pub(crate) fn set_is_root(&self, value: bool) {
        view_write!(self, is_root, value)
    }
    #[inline(always)]
    pub(crate) fn is_root_raw(&self) -> *mut bool {
        view_raw_mut!(self, is_root)
    }
    #[inline(always)]
    pub(crate) fn set_has_geometry_transform(&self, value: bool) {
        view_write!(self, has_geometry_transform, value)
    }
    #[inline(always)]
    pub(crate) fn has_geometry_transform_raw(&self) -> *mut bool {
        view_raw_mut!(self, has_geometry_transform)
    }
    #[inline(always)]
    pub(crate) fn set_use_rotation_space(&self, value: bool) {
        view_write!(self, use_rotation_space, value)
    }
    #[inline(always)]
    pub(crate) fn use_rotation_space_raw(&self) -> *mut bool {
        view_raw_mut!(self, use_rotation_space)
    }
    #[inline(always)]
    pub(crate) fn set_has_adjust_transform(&self, value: bool) {
        view_write!(self, has_adjust_transform, value)
    }
    #[inline(always)]
    pub(crate) fn has_adjust_transform_raw(&self) -> *mut bool {
        view_raw_mut!(self, has_adjust_transform)
    }
    #[inline(always)]
    pub(crate) fn set_has_root_adjust_transform(&self, value: bool) {
        view_write!(self, has_root_adjust_transform, value)
    }
    #[inline(always)]
    pub(crate) fn has_root_adjust_transform_raw(&self) -> *mut bool {
        view_raw_mut!(self, has_root_adjust_transform)
    }
    #[inline(always)]
    pub(crate) fn set_is_geometry_transform_helper(&self, value: bool) {
        view_write!(self, is_geometry_transform_helper, value)
    }
    #[inline(always)]
    pub(crate) fn is_geometry_transform_helper_raw(&self) -> *mut bool {
        view_raw_mut!(self, is_geometry_transform_helper)
    }
    #[inline(always)]
    pub(crate) fn set_is_scale_helper(&self, value: bool) {
        view_write!(self, is_scale_helper, value)
    }
    #[inline(always)]
    pub(crate) fn is_scale_helper_raw(&self) -> *mut bool {
        view_raw_mut!(self, is_scale_helper)
    }
    #[inline(always)]
    pub(crate) fn set_is_scale_compensate_parent(&self, value: bool) {
        view_write!(self, is_scale_compensate_parent, value)
    }
    #[inline(always)]
    pub(crate) fn is_scale_compensate_parent_raw(&self) -> *mut bool {
        view_raw_mut!(self, is_scale_compensate_parent)
    }
    #[inline(always)]
    pub(crate) fn set_node_depth(&self, value: u32) {
        view_write!(self, node_depth, value)
    }
    #[inline(always)]
    pub(crate) fn node_depth_raw(&self) -> *mut u32 {
        view_raw_mut!(self, node_depth)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<Texture, M> {
    #[inline(always)]
    pub(crate) fn element(&self) -> &View<Element, M> {
        view_project!(self, element)
    }
    #[inline(always)]
    pub(crate) fn element_ptr(&self) -> *const Element {
        view_raw_shared!(self, element)
    }
    #[inline(always)]
    pub(crate) fn type_(&self) -> TextureType {
        view_read_shared!(self, type_)
    }
    #[inline(always)]
    pub(crate) fn type_ptr(&self) -> *const TextureType {
        view_raw_shared!(self, type_)
    }
    #[inline(always)]
    pub(crate) fn filename(&self) -> String {
        view_read_shared!(self, filename)
    }
    #[inline(always)]
    pub(crate) fn filename_view(&self) -> &View<String, M> {
        view_project!(self, filename)
    }
    #[inline(always)]
    pub(crate) fn filename_ptr(&self) -> *const String {
        view_raw_shared!(self, filename)
    }
    #[inline(always)]
    pub(crate) fn absolute_filename(&self) -> String {
        view_read_shared!(self, absolute_filename)
    }
    #[inline(always)]
    pub(crate) fn absolute_filename_view(&self) -> &View<String, M> {
        view_project!(self, absolute_filename)
    }
    #[inline(always)]
    pub(crate) fn absolute_filename_ptr(&self) -> *const String {
        view_raw_shared!(self, absolute_filename)
    }
    #[inline(always)]
    pub(crate) fn relative_filename(&self) -> String {
        view_read_shared!(self, relative_filename)
    }
    #[inline(always)]
    pub(crate) fn relative_filename_view(&self) -> &View<String, M> {
        view_project!(self, relative_filename)
    }
    #[inline(always)]
    pub(crate) fn relative_filename_ptr(&self) -> *const String {
        view_raw_shared!(self, relative_filename)
    }
    #[inline(always)]
    pub(crate) fn raw_filename(&self) -> Blob {
        view_read_shared!(self, raw_filename)
    }
    #[inline(always)]
    pub(crate) fn raw_filename_ptr(&self) -> *const Blob {
        view_raw_shared!(self, raw_filename)
    }
    #[inline(always)]
    pub(crate) fn raw_absolute_filename(&self) -> Blob {
        view_read_shared!(self, raw_absolute_filename)
    }
    #[inline(always)]
    pub(crate) fn raw_absolute_filename_ptr(&self) -> *const Blob {
        view_raw_shared!(self, raw_absolute_filename)
    }
    #[inline(always)]
    pub(crate) fn raw_relative_filename(&self) -> Blob {
        view_read_shared!(self, raw_relative_filename)
    }
    #[inline(always)]
    pub(crate) fn raw_relative_filename_ptr(&self) -> *const Blob {
        view_raw_shared!(self, raw_relative_filename)
    }
    #[inline(always)]
    pub(crate) fn content(&self) -> Blob {
        view_read_shared!(self, content)
    }
    #[inline(always)]
    pub(crate) fn content_ptr(&self) -> *const Blob {
        view_raw_shared!(self, content)
    }
    #[inline(always)]
    pub(crate) fn video(&self) -> Option<Ref<Video>> {
        view_read_shared!(self, video)
    }
    #[inline(always)]
    pub(crate) fn video_view(&self) -> Option<&View<Video, M>> {
        self.video().map(Ref::view)
    }
    #[inline(always)]
    pub(crate) fn video_ptr(&self) -> *const Option<Ref<Video>> {
        view_raw_shared!(self, video)
    }
    #[inline(always)]
    pub(crate) fn file_index(&self) -> u32 {
        view_read_shared!(self, file_index)
    }
    #[inline(always)]
    pub(crate) fn file_index_ptr(&self) -> *const u32 {
        view_raw_shared!(self, file_index)
    }
    #[inline(always)]
    pub(crate) fn has_file(&self) -> bool {
        view_read_shared!(self, has_file)
    }
    #[inline(always)]
    pub(crate) fn has_file_ptr(&self) -> *const bool {
        view_raw_shared!(self, has_file)
    }
    #[inline(always)]
    pub(crate) fn layers(&self) -> List<TextureLayer> {
        view_read_shared!(self, layers)
    }
    #[inline(always)]
    pub(crate) fn layers_view(&self) -> &View<List<TextureLayer>, M> {
        view_project!(self, layers)
    }
    #[inline(always)]
    pub(crate) fn layers_ptr(&self) -> *const List<TextureLayer> {
        view_raw_shared!(self, layers)
    }
    #[inline(always)]
    pub(crate) fn shader(&self) -> Option<Ref<ShaderTexture>> {
        view_read_shared!(self, shader)
    }
    #[inline(always)]
    pub(crate) fn shader_view(&self) -> Option<&View<ShaderTexture, M>> {
        self.shader().map(Ref::view)
    }
    #[inline(always)]
    pub(crate) fn shader_ptr(&self) -> *const Option<Ref<ShaderTexture>> {
        view_raw_shared!(self, shader)
    }
    #[inline(always)]
    pub(crate) fn file_textures(&self) -> RefList<Texture> {
        view_read_shared!(self, file_textures)
    }
    #[inline(always)]
    pub(crate) fn file_textures_view(&self) -> &View<RefList<Texture>, M> {
        view_project!(self, file_textures)
    }
    #[inline(always)]
    pub(crate) fn file_textures_ptr(&self) -> *const RefList<Texture> {
        view_raw_shared!(self, file_textures)
    }
    #[inline(always)]
    pub(crate) fn uv_set(&self) -> String {
        view_read_shared!(self, uv_set)
    }
    #[inline(always)]
    pub(crate) fn uv_set_view(&self) -> &View<String, M> {
        view_project!(self, uv_set)
    }
    #[inline(always)]
    pub(crate) fn uv_set_ptr(&self) -> *const String {
        view_raw_shared!(self, uv_set)
    }
    #[inline(always)]
    pub(crate) fn wrap_u(&self) -> WrapMode {
        view_read_shared!(self, wrap_u)
    }
    #[inline(always)]
    pub(crate) fn wrap_u_ptr(&self) -> *const WrapMode {
        view_raw_shared!(self, wrap_u)
    }
    #[inline(always)]
    pub(crate) fn wrap_v(&self) -> WrapMode {
        view_read_shared!(self, wrap_v)
    }
    #[inline(always)]
    pub(crate) fn wrap_v_ptr(&self) -> *const WrapMode {
        view_raw_shared!(self, wrap_v)
    }
    #[inline(always)]
    pub(crate) fn has_uv_transform(&self) -> bool {
        view_read_shared!(self, has_uv_transform)
    }
    #[inline(always)]
    pub(crate) fn has_uv_transform_ptr(&self) -> *const bool {
        view_raw_shared!(self, has_uv_transform)
    }
    #[inline(always)]
    pub(crate) fn uv_transform(&self) -> Transform {
        view_read_shared!(self, uv_transform)
    }
    #[inline(always)]
    pub(crate) fn uv_transform_ptr(&self) -> *const Transform {
        view_raw_shared!(self, uv_transform)
    }
    #[inline(always)]
    pub(crate) fn texture_to_uv(&self) -> Matrix {
        view_read_shared!(self, texture_to_uv)
    }
    #[inline(always)]
    pub(crate) fn texture_to_uv_ptr(&self) -> *const Matrix {
        view_raw_shared!(self, texture_to_uv)
    }
    #[inline(always)]
    pub(crate) fn uv_to_texture(&self) -> Matrix {
        view_read_shared!(self, uv_to_texture)
    }
    #[inline(always)]
    pub(crate) fn uv_to_texture_ptr(&self) -> *const Matrix {
        view_raw_shared!(self, uv_to_texture)
    }
}

#[allow(dead_code)]
impl View<Texture, Mut> {
    #[inline(always)]
    pub(crate) fn set_element(&self, value: Element) {
        view_write!(self, element, value)
    }
    #[inline(always)]
    pub(crate) fn element_raw(&self) -> *mut Element {
        view_raw_mut!(self, element)
    }
    #[inline(always)]
    pub(crate) fn set_type(&self, value: TextureType) {
        view_write!(self, type_, value)
    }
    #[inline(always)]
    pub(crate) fn type_raw(&self) -> *mut TextureType {
        view_raw_mut!(self, type_)
    }
    #[inline(always)]
    pub(crate) fn set_filename(&self, value: String) {
        view_write!(self, filename, value)
    }
    #[inline(always)]
    pub(crate) fn filename_raw(&self) -> *mut String {
        view_raw_mut!(self, filename)
    }
    #[inline(always)]
    pub(crate) fn set_absolute_filename(&self, value: String) {
        view_write!(self, absolute_filename, value)
    }
    #[inline(always)]
    pub(crate) fn absolute_filename_raw(&self) -> *mut String {
        view_raw_mut!(self, absolute_filename)
    }
    #[inline(always)]
    pub(crate) fn set_relative_filename(&self, value: String) {
        view_write!(self, relative_filename, value)
    }
    #[inline(always)]
    pub(crate) fn relative_filename_raw(&self) -> *mut String {
        view_raw_mut!(self, relative_filename)
    }
    #[inline(always)]
    pub(crate) fn set_raw_filename(&self, value: Blob) {
        view_write!(self, raw_filename, value)
    }
    #[inline(always)]
    pub(crate) fn raw_filename_raw(&self) -> *mut Blob {
        view_raw_mut!(self, raw_filename)
    }
    #[inline(always)]
    pub(crate) fn set_raw_absolute_filename(&self, value: Blob) {
        view_write!(self, raw_absolute_filename, value)
    }
    #[inline(always)]
    pub(crate) fn raw_absolute_filename_raw(&self) -> *mut Blob {
        view_raw_mut!(self, raw_absolute_filename)
    }
    #[inline(always)]
    pub(crate) fn set_raw_relative_filename(&self, value: Blob) {
        view_write!(self, raw_relative_filename, value)
    }
    #[inline(always)]
    pub(crate) fn raw_relative_filename_raw(&self) -> *mut Blob {
        view_raw_mut!(self, raw_relative_filename)
    }
    #[inline(always)]
    pub(crate) fn set_content(&self, value: Blob) {
        view_write!(self, content, value)
    }
    #[inline(always)]
    pub(crate) fn content_raw(&self) -> *mut Blob {
        view_raw_mut!(self, content)
    }
    #[inline(always)]
    pub(crate) fn set_video(&self, value: Option<Ref<Video>>) {
        view_write!(self, video, value)
    }
    #[inline(always)]
    pub(crate) fn video_raw(&self) -> *mut Option<Ref<Video>> {
        view_raw_mut!(self, video)
    }
    #[inline(always)]
    pub(crate) fn set_file_index(&self, value: u32) {
        view_write!(self, file_index, value)
    }
    #[inline(always)]
    pub(crate) fn file_index_raw(&self) -> *mut u32 {
        view_raw_mut!(self, file_index)
    }
    #[inline(always)]
    pub(crate) fn set_has_file(&self, value: bool) {
        view_write!(self, has_file, value)
    }
    #[inline(always)]
    pub(crate) fn has_file_raw(&self) -> *mut bool {
        view_raw_mut!(self, has_file)
    }
    #[inline(always)]
    pub(crate) fn set_layers(&self, value: List<TextureLayer>) {
        view_write!(self, layers, value)
    }
    #[inline(always)]
    pub(crate) fn layers_raw(&self) -> *mut List<TextureLayer> {
        view_raw_mut!(self, layers)
    }
    #[inline(always)]
    pub(crate) fn set_shader(&self, value: Option<Ref<ShaderTexture>>) {
        view_write!(self, shader, value)
    }
    #[inline(always)]
    pub(crate) fn shader_raw(&self) -> *mut Option<Ref<ShaderTexture>> {
        view_raw_mut!(self, shader)
    }
    #[inline(always)]
    pub(crate) fn set_file_textures(&self, value: RefList<Texture>) {
        view_write!(self, file_textures, value)
    }
    #[inline(always)]
    pub(crate) fn file_textures_raw(&self) -> *mut RefList<Texture> {
        view_raw_mut!(self, file_textures)
    }
    #[inline(always)]
    pub(crate) fn set_uv_set(&self, value: String) {
        view_write!(self, uv_set, value)
    }
    #[inline(always)]
    pub(crate) fn uv_set_raw(&self) -> *mut String {
        view_raw_mut!(self, uv_set)
    }
    #[inline(always)]
    pub(crate) fn set_wrap_u(&self, value: WrapMode) {
        view_write!(self, wrap_u, value)
    }
    #[inline(always)]
    pub(crate) fn wrap_u_raw(&self) -> *mut WrapMode {
        view_raw_mut!(self, wrap_u)
    }
    #[inline(always)]
    pub(crate) fn set_wrap_v(&self, value: WrapMode) {
        view_write!(self, wrap_v, value)
    }
    #[inline(always)]
    pub(crate) fn wrap_v_raw(&self) -> *mut WrapMode {
        view_raw_mut!(self, wrap_v)
    }
    #[inline(always)]
    pub(crate) fn set_has_uv_transform(&self, value: bool) {
        view_write!(self, has_uv_transform, value)
    }
    #[inline(always)]
    pub(crate) fn has_uv_transform_raw(&self) -> *mut bool {
        view_raw_mut!(self, has_uv_transform)
    }
    #[inline(always)]
    pub(crate) fn set_uv_transform(&self, value: Transform) {
        view_write!(self, uv_transform, value)
    }
    #[inline(always)]
    pub(crate) fn uv_transform_raw(&self) -> *mut Transform {
        view_raw_mut!(self, uv_transform)
    }
    #[inline(always)]
    pub(crate) fn set_texture_to_uv(&self, value: Matrix) {
        view_write!(self, texture_to_uv, value)
    }
    #[inline(always)]
    pub(crate) fn texture_to_uv_raw(&self) -> *mut Matrix {
        view_raw_mut!(self, texture_to_uv)
    }
    #[inline(always)]
    pub(crate) fn set_uv_to_texture(&self, value: Matrix) {
        view_write!(self, uv_to_texture, value)
    }
    #[inline(always)]
    pub(crate) fn uv_to_texture_raw(&self) -> *mut Matrix {
        view_raw_mut!(self, uv_to_texture)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<SkinCluster, M> {
    #[inline(always)]
    pub(crate) fn element(&self) -> &View<Element, M> {
        view_project!(self, element)
    }
    #[inline(always)]
    pub(crate) fn element_ptr(&self) -> *const Element {
        view_raw_shared!(self, element)
    }
    #[inline(always)]
    pub(crate) fn bone_node(&self) -> Option<Ref<Node>> {
        view_read_shared!(self, bone_node)
    }
    #[inline(always)]
    pub(crate) fn bone_node_view(&self) -> Option<&View<Node, M>> {
        self.bone_node().map(Ref::view)
    }
    #[inline(always)]
    pub(crate) fn bone_node_ptr(&self) -> *const Option<Ref<Node>> {
        view_raw_shared!(self, bone_node)
    }
    #[inline(always)]
    pub(crate) fn geometry_to_bone(&self) -> Matrix {
        view_read_shared!(self, geometry_to_bone)
    }
    #[inline(always)]
    pub(crate) fn geometry_to_bone_ptr(&self) -> *const Matrix {
        view_raw_shared!(self, geometry_to_bone)
    }
    #[inline(always)]
    pub(crate) fn mesh_node_to_bone(&self) -> Matrix {
        view_read_shared!(self, mesh_node_to_bone)
    }
    #[inline(always)]
    pub(crate) fn mesh_node_to_bone_ptr(&self) -> *const Matrix {
        view_raw_shared!(self, mesh_node_to_bone)
    }
    #[inline(always)]
    pub(crate) fn bind_to_world(&self) -> Matrix {
        view_read_shared!(self, bind_to_world)
    }
    #[inline(always)]
    pub(crate) fn bind_to_world_ptr(&self) -> *const Matrix {
        view_raw_shared!(self, bind_to_world)
    }
    #[inline(always)]
    pub(crate) fn geometry_to_world(&self) -> Matrix {
        view_read_shared!(self, geometry_to_world)
    }
    #[inline(always)]
    pub(crate) fn geometry_to_world_ptr(&self) -> *const Matrix {
        view_raw_shared!(self, geometry_to_world)
    }
    #[inline(always)]
    pub(crate) fn geometry_to_world_transform(&self) -> Transform {
        view_read_shared!(self, geometry_to_world_transform)
    }
    #[inline(always)]
    pub(crate) fn geometry_to_world_transform_ptr(&self) -> *const Transform {
        view_raw_shared!(self, geometry_to_world_transform)
    }
    #[inline(always)]
    pub(crate) fn num_weights(&self) -> usize {
        view_read_shared!(self, num_weights)
    }
    #[inline(always)]
    pub(crate) fn num_weights_ptr(&self) -> *const usize {
        view_raw_shared!(self, num_weights)
    }
    #[inline(always)]
    pub(crate) fn vertices(&self) -> List<u32> {
        view_read_shared!(self, vertices)
    }
    #[inline(always)]
    pub(crate) fn vertices_view(&self) -> &View<List<u32>, M> {
        view_project!(self, vertices)
    }
    #[inline(always)]
    pub(crate) fn vertices_ptr(&self) -> *const List<u32> {
        view_raw_shared!(self, vertices)
    }
    #[inline(always)]
    pub(crate) fn weights(&self) -> List<Real> {
        view_read_shared!(self, weights)
    }
    #[inline(always)]
    pub(crate) fn weights_view(&self) -> &View<List<Real>, M> {
        view_project!(self, weights)
    }
    #[inline(always)]
    pub(crate) fn weights_ptr(&self) -> *const List<Real> {
        view_raw_shared!(self, weights)
    }
}

#[allow(dead_code)]
impl View<SkinCluster, Mut> {
    #[inline(always)]
    pub(crate) fn set_element(&self, value: Element) {
        view_write!(self, element, value)
    }
    #[inline(always)]
    pub(crate) fn element_raw(&self) -> *mut Element {
        view_raw_mut!(self, element)
    }
    #[inline(always)]
    pub(crate) fn set_bone_node(&self, value: Option<Ref<Node>>) {
        view_write!(self, bone_node, value)
    }
    #[inline(always)]
    pub(crate) fn bone_node_raw(&self) -> *mut Option<Ref<Node>> {
        view_raw_mut!(self, bone_node)
    }
    #[inline(always)]
    pub(crate) fn set_geometry_to_bone(&self, value: Matrix) {
        view_write!(self, geometry_to_bone, value)
    }
    #[inline(always)]
    pub(crate) fn geometry_to_bone_raw(&self) -> *mut Matrix {
        view_raw_mut!(self, geometry_to_bone)
    }
    #[inline(always)]
    pub(crate) fn set_mesh_node_to_bone(&self, value: Matrix) {
        view_write!(self, mesh_node_to_bone, value)
    }
    #[inline(always)]
    pub(crate) fn mesh_node_to_bone_raw(&self) -> *mut Matrix {
        view_raw_mut!(self, mesh_node_to_bone)
    }
    #[inline(always)]
    pub(crate) fn set_bind_to_world(&self, value: Matrix) {
        view_write!(self, bind_to_world, value)
    }
    #[inline(always)]
    pub(crate) fn bind_to_world_raw(&self) -> *mut Matrix {
        view_raw_mut!(self, bind_to_world)
    }
    #[inline(always)]
    pub(crate) fn set_geometry_to_world(&self, value: Matrix) {
        view_write!(self, geometry_to_world, value)
    }
    #[inline(always)]
    pub(crate) fn geometry_to_world_raw(&self) -> *mut Matrix {
        view_raw_mut!(self, geometry_to_world)
    }
    #[inline(always)]
    pub(crate) fn set_geometry_to_world_transform(&self, value: Transform) {
        view_write!(self, geometry_to_world_transform, value)
    }
    #[inline(always)]
    pub(crate) fn geometry_to_world_transform_raw(&self) -> *mut Transform {
        view_raw_mut!(self, geometry_to_world_transform)
    }
    #[inline(always)]
    pub(crate) fn set_num_weights(&self, value: usize) {
        view_write!(self, num_weights, value)
    }
    #[inline(always)]
    pub(crate) fn num_weights_raw(&self) -> *mut usize {
        view_raw_mut!(self, num_weights)
    }
    #[inline(always)]
    pub(crate) fn set_vertices(&self, value: List<u32>) {
        view_write!(self, vertices, value)
    }
    #[inline(always)]
    pub(crate) fn vertices_raw(&self) -> *mut List<u32> {
        view_raw_mut!(self, vertices)
    }
    #[inline(always)]
    pub(crate) fn set_weights(&self, value: List<Real>) {
        view_write!(self, weights, value)
    }
    #[inline(always)]
    pub(crate) fn weights_raw(&self) -> *mut List<Real> {
        view_raw_mut!(self, weights)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<Material, M> {
    #[inline(always)]
    pub(crate) fn element(&self) -> &View<Element, M> {
        view_project!(self, element)
    }
    #[inline(always)]
    pub(crate) fn element_ptr(&self) -> *const Element {
        view_raw_shared!(self, element)
    }
    #[inline(always)]
    pub(crate) fn fbx(&self) -> MaterialFbxMaps {
        view_read_shared!(self, fbx)
    }
    #[inline(always)]
    pub(crate) fn fbx_ptr(&self) -> *const MaterialFbxMaps {
        view_raw_shared!(self, fbx)
    }
    #[inline(always)]
    pub(crate) fn pbr(&self) -> MaterialPbrMaps {
        view_read_shared!(self, pbr)
    }
    #[inline(always)]
    pub(crate) fn pbr_ptr(&self) -> *const MaterialPbrMaps {
        view_raw_shared!(self, pbr)
    }
    #[inline(always)]
    pub(crate) fn features(&self) -> MaterialFeatures {
        view_read_shared!(self, features)
    }
    #[inline(always)]
    pub(crate) fn features_ptr(&self) -> *const MaterialFeatures {
        view_raw_shared!(self, features)
    }
    #[inline(always)]
    pub(crate) fn shader_type(&self) -> ShaderType {
        view_read_shared!(self, shader_type)
    }
    #[inline(always)]
    pub(crate) fn shader_type_ptr(&self) -> *const ShaderType {
        view_raw_shared!(self, shader_type)
    }
    #[inline(always)]
    pub(crate) fn shader(&self) -> Option<Ref<Shader>> {
        view_read_shared!(self, shader)
    }
    #[inline(always)]
    pub(crate) fn shader_view(&self) -> Option<&View<Shader, M>> {
        self.shader().map(Ref::view)
    }
    #[inline(always)]
    pub(crate) fn shader_ptr(&self) -> *const Option<Ref<Shader>> {
        view_raw_shared!(self, shader)
    }
    #[inline(always)]
    pub(crate) fn shading_model_name(&self) -> String {
        view_read_shared!(self, shading_model_name)
    }
    #[inline(always)]
    pub(crate) fn shading_model_name_view(&self) -> &View<String, M> {
        view_project!(self, shading_model_name)
    }
    #[inline(always)]
    pub(crate) fn shading_model_name_ptr(&self) -> *const String {
        view_raw_shared!(self, shading_model_name)
    }
    #[inline(always)]
    pub(crate) fn shader_prop_prefix(&self) -> String {
        view_read_shared!(self, shader_prop_prefix)
    }
    #[inline(always)]
    pub(crate) fn shader_prop_prefix_view(&self) -> &View<String, M> {
        view_project!(self, shader_prop_prefix)
    }
    #[inline(always)]
    pub(crate) fn shader_prop_prefix_ptr(&self) -> *const String {
        view_raw_shared!(self, shader_prop_prefix)
    }
    #[inline(always)]
    pub(crate) fn textures(&self) -> List<MaterialTexture> {
        view_read_shared!(self, textures)
    }
    #[inline(always)]
    pub(crate) fn textures_view(&self) -> &View<List<MaterialTexture>, M> {
        view_project!(self, textures)
    }
    #[inline(always)]
    pub(crate) fn textures_ptr(&self) -> *const List<MaterialTexture> {
        view_raw_shared!(self, textures)
    }
}

#[allow(dead_code)]
impl View<Material, Mut> {
    #[inline(always)]
    pub(crate) fn set_element(&self, value: Element) {
        view_write!(self, element, value)
    }
    #[inline(always)]
    pub(crate) fn element_raw(&self) -> *mut Element {
        view_raw_mut!(self, element)
    }
    #[inline(always)]
    pub(crate) fn set_fbx(&self, value: MaterialFbxMaps) {
        view_write!(self, fbx, value)
    }
    #[inline(always)]
    pub(crate) fn fbx_raw(&self) -> *mut MaterialFbxMaps {
        view_raw_mut!(self, fbx)
    }
    #[inline(always)]
    pub(crate) fn set_pbr(&self, value: MaterialPbrMaps) {
        view_write!(self, pbr, value)
    }
    #[inline(always)]
    pub(crate) fn pbr_raw(&self) -> *mut MaterialPbrMaps {
        view_raw_mut!(self, pbr)
    }
    #[inline(always)]
    pub(crate) fn set_features(&self, value: MaterialFeatures) {
        view_write!(self, features, value)
    }
    #[inline(always)]
    pub(crate) fn features_raw(&self) -> *mut MaterialFeatures {
        view_raw_mut!(self, features)
    }
    #[inline(always)]
    pub(crate) fn set_shader_type(&self, value: ShaderType) {
        view_write!(self, shader_type, value)
    }
    #[inline(always)]
    pub(crate) fn shader_type_raw(&self) -> *mut ShaderType {
        view_raw_mut!(self, shader_type)
    }
    #[inline(always)]
    pub(crate) fn set_shader(&self, value: Option<Ref<Shader>>) {
        view_write!(self, shader, value)
    }
    #[inline(always)]
    pub(crate) fn shader_raw(&self) -> *mut Option<Ref<Shader>> {
        view_raw_mut!(self, shader)
    }
    #[inline(always)]
    pub(crate) fn set_shading_model_name(&self, value: String) {
        view_write!(self, shading_model_name, value)
    }
    #[inline(always)]
    pub(crate) fn shading_model_name_raw(&self) -> *mut String {
        view_raw_mut!(self, shading_model_name)
    }
    #[inline(always)]
    pub(crate) fn set_shader_prop_prefix(&self, value: String) {
        view_write!(self, shader_prop_prefix, value)
    }
    #[inline(always)]
    pub(crate) fn shader_prop_prefix_raw(&self) -> *mut String {
        view_raw_mut!(self, shader_prop_prefix)
    }
    #[inline(always)]
    pub(crate) fn set_textures(&self, value: List<MaterialTexture>) {
        view_write!(self, textures, value)
    }
    #[inline(always)]
    pub(crate) fn textures_raw(&self) -> *mut List<MaterialTexture> {
        view_raw_mut!(self, textures)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<Video, M> {
    #[inline(always)]
    pub(crate) fn element(&self) -> &View<Element, M> {
        view_project!(self, element)
    }
    #[inline(always)]
    pub(crate) fn element_ptr(&self) -> *const Element {
        view_raw_shared!(self, element)
    }
    #[inline(always)]
    pub(crate) fn filename(&self) -> String {
        view_read_shared!(self, filename)
    }
    #[inline(always)]
    pub(crate) fn filename_view(&self) -> &View<String, M> {
        view_project!(self, filename)
    }
    #[inline(always)]
    pub(crate) fn filename_ptr(&self) -> *const String {
        view_raw_shared!(self, filename)
    }
    #[inline(always)]
    pub(crate) fn absolute_filename(&self) -> String {
        view_read_shared!(self, absolute_filename)
    }
    #[inline(always)]
    pub(crate) fn absolute_filename_view(&self) -> &View<String, M> {
        view_project!(self, absolute_filename)
    }
    #[inline(always)]
    pub(crate) fn absolute_filename_ptr(&self) -> *const String {
        view_raw_shared!(self, absolute_filename)
    }
    #[inline(always)]
    pub(crate) fn relative_filename(&self) -> String {
        view_read_shared!(self, relative_filename)
    }
    #[inline(always)]
    pub(crate) fn relative_filename_view(&self) -> &View<String, M> {
        view_project!(self, relative_filename)
    }
    #[inline(always)]
    pub(crate) fn relative_filename_ptr(&self) -> *const String {
        view_raw_shared!(self, relative_filename)
    }
    #[inline(always)]
    pub(crate) fn raw_filename(&self) -> Blob {
        view_read_shared!(self, raw_filename)
    }
    #[inline(always)]
    pub(crate) fn raw_filename_ptr(&self) -> *const Blob {
        view_raw_shared!(self, raw_filename)
    }
    #[inline(always)]
    pub(crate) fn raw_absolute_filename(&self) -> Blob {
        view_read_shared!(self, raw_absolute_filename)
    }
    #[inline(always)]
    pub(crate) fn raw_absolute_filename_ptr(&self) -> *const Blob {
        view_raw_shared!(self, raw_absolute_filename)
    }
    #[inline(always)]
    pub(crate) fn raw_relative_filename(&self) -> Blob {
        view_read_shared!(self, raw_relative_filename)
    }
    #[inline(always)]
    pub(crate) fn raw_relative_filename_ptr(&self) -> *const Blob {
        view_raw_shared!(self, raw_relative_filename)
    }
    #[inline(always)]
    pub(crate) fn content(&self) -> Blob {
        view_read_shared!(self, content)
    }
    #[inline(always)]
    pub(crate) fn content_ptr(&self) -> *const Blob {
        view_raw_shared!(self, content)
    }
}

#[allow(dead_code)]
impl View<Video, Mut> {
    #[inline(always)]
    pub(crate) fn set_element(&self, value: Element) {
        view_write!(self, element, value)
    }
    #[inline(always)]
    pub(crate) fn element_raw(&self) -> *mut Element {
        view_raw_mut!(self, element)
    }
    #[inline(always)]
    pub(crate) fn set_filename(&self, value: String) {
        view_write!(self, filename, value)
    }
    #[inline(always)]
    pub(crate) fn filename_raw(&self) -> *mut String {
        view_raw_mut!(self, filename)
    }
    #[inline(always)]
    pub(crate) fn set_absolute_filename(&self, value: String) {
        view_write!(self, absolute_filename, value)
    }
    #[inline(always)]
    pub(crate) fn absolute_filename_raw(&self) -> *mut String {
        view_raw_mut!(self, absolute_filename)
    }
    #[inline(always)]
    pub(crate) fn set_relative_filename(&self, value: String) {
        view_write!(self, relative_filename, value)
    }
    #[inline(always)]
    pub(crate) fn relative_filename_raw(&self) -> *mut String {
        view_raw_mut!(self, relative_filename)
    }
    #[inline(always)]
    pub(crate) fn set_raw_filename(&self, value: Blob) {
        view_write!(self, raw_filename, value)
    }
    #[inline(always)]
    pub(crate) fn raw_filename_raw(&self) -> *mut Blob {
        view_raw_mut!(self, raw_filename)
    }
    #[inline(always)]
    pub(crate) fn set_raw_absolute_filename(&self, value: Blob) {
        view_write!(self, raw_absolute_filename, value)
    }
    #[inline(always)]
    pub(crate) fn raw_absolute_filename_raw(&self) -> *mut Blob {
        view_raw_mut!(self, raw_absolute_filename)
    }
    #[inline(always)]
    pub(crate) fn set_raw_relative_filename(&self, value: Blob) {
        view_write!(self, raw_relative_filename, value)
    }
    #[inline(always)]
    pub(crate) fn raw_relative_filename_raw(&self) -> *mut Blob {
        view_raw_mut!(self, raw_relative_filename)
    }
    #[inline(always)]
    pub(crate) fn set_content(&self, value: Blob) {
        view_write!(self, content, value)
    }
    #[inline(always)]
    pub(crate) fn content_raw(&self) -> *mut Blob {
        view_raw_mut!(self, content)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<Pose, M> {
    #[inline(always)]
    pub(crate) fn element(&self) -> &View<Element, M> {
        view_project!(self, element)
    }
    #[inline(always)]
    pub(crate) fn element_ptr(&self) -> *const Element {
        view_raw_shared!(self, element)
    }
    #[inline(always)]
    pub(crate) fn is_bind_pose(&self) -> bool {
        view_read_shared!(self, is_bind_pose)
    }
    #[inline(always)]
    pub(crate) fn is_bind_pose_ptr(&self) -> *const bool {
        view_raw_shared!(self, is_bind_pose)
    }
    #[inline(always)]
    pub(crate) fn bone_poses(&self) -> List<BonePose> {
        view_read_shared!(self, bone_poses)
    }
    #[inline(always)]
    pub(crate) fn bone_poses_view(&self) -> &View<List<BonePose>, M> {
        view_project!(self, bone_poses)
    }
    #[inline(always)]
    pub(crate) fn bone_poses_ptr(&self) -> *const List<BonePose> {
        view_raw_shared!(self, bone_poses)
    }
}

#[allow(dead_code)]
impl View<Pose, Mut> {
    #[inline(always)]
    pub(crate) fn set_element(&self, value: Element) {
        view_write!(self, element, value)
    }
    #[inline(always)]
    pub(crate) fn element_raw(&self) -> *mut Element {
        view_raw_mut!(self, element)
    }
    #[inline(always)]
    pub(crate) fn set_is_bind_pose(&self, value: bool) {
        view_write!(self, is_bind_pose, value)
    }
    #[inline(always)]
    pub(crate) fn is_bind_pose_raw(&self) -> *mut bool {
        view_raw_mut!(self, is_bind_pose)
    }
    #[inline(always)]
    pub(crate) fn set_bone_poses(&self, value: List<BonePose>) {
        view_write!(self, bone_poses, value)
    }
    #[inline(always)]
    pub(crate) fn bone_poses_raw(&self) -> *mut List<BonePose> {
        view_raw_mut!(self, bone_poses)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<BonePose, M> {
    #[inline(always)]
    pub(crate) fn bone_node(&self) -> Ref<Node> {
        view_read_shared!(self, bone_node)
    }
    #[inline(always)]
    pub(crate) fn bone_node_view(&self) -> &View<Node, M> {
        self.bone_node().view()
    }
    #[inline(always)]
    pub(crate) fn bone_node_ptr(&self) -> *const Ref<Node> {
        view_raw_shared!(self, bone_node)
    }
    #[inline(always)]
    pub(crate) fn bone_to_world(&self) -> Matrix {
        view_read_shared!(self, bone_to_world)
    }
    #[inline(always)]
    pub(crate) fn bone_to_world_ptr(&self) -> *const Matrix {
        view_raw_shared!(self, bone_to_world)
    }
    #[inline(always)]
    pub(crate) fn bone_to_parent(&self) -> Matrix {
        view_read_shared!(self, bone_to_parent)
    }
    #[inline(always)]
    pub(crate) fn bone_to_parent_ptr(&self) -> *const Matrix {
        view_raw_shared!(self, bone_to_parent)
    }
}

#[allow(dead_code)]
impl View<BonePose, Mut> {
    #[inline(always)]
    pub(crate) fn set_bone_node(&self, value: Ref<Node>) {
        view_write!(self, bone_node, value)
    }
    #[inline(always)]
    pub(crate) fn bone_node_raw(&self) -> *mut Ref<Node> {
        view_raw_mut!(self, bone_node)
    }
    #[inline(always)]
    pub(crate) fn set_bone_to_world(&self, value: Matrix) {
        view_write!(self, bone_to_world, value)
    }
    #[inline(always)]
    pub(crate) fn bone_to_world_raw(&self) -> *mut Matrix {
        view_raw_mut!(self, bone_to_world)
    }
    #[inline(always)]
    pub(crate) fn set_bone_to_parent(&self, value: Matrix) {
        view_write!(self, bone_to_parent, value)
    }
    #[inline(always)]
    pub(crate) fn bone_to_parent_raw(&self) -> *mut Matrix {
        view_raw_mut!(self, bone_to_parent)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<BlendKeyframe, M> {
    #[inline(always)]
    pub(crate) fn shape(&self) -> Ref<BlendShape> {
        view_read_shared!(self, shape)
    }
    #[inline(always)]
    pub(crate) fn shape_view(&self) -> &View<BlendShape, M> {
        self.shape().view()
    }
    #[inline(always)]
    pub(crate) fn shape_ptr(&self) -> *const Ref<BlendShape> {
        view_raw_shared!(self, shape)
    }
    #[inline(always)]
    pub(crate) fn target_weight(&self) -> Real {
        view_read_shared!(self, target_weight)
    }
    #[inline(always)]
    pub(crate) fn target_weight_ptr(&self) -> *const Real {
        view_raw_shared!(self, target_weight)
    }
    #[inline(always)]
    pub(crate) fn effective_weight(&self) -> Real {
        view_read_shared!(self, effective_weight)
    }
    #[inline(always)]
    pub(crate) fn effective_weight_ptr(&self) -> *const Real {
        view_raw_shared!(self, effective_weight)
    }
}

#[allow(dead_code)]
impl View<BlendKeyframe, Mut> {
    #[inline(always)]
    pub(crate) fn set_shape(&self, value: Ref<BlendShape>) {
        view_write!(self, shape, value)
    }
    #[inline(always)]
    pub(crate) fn shape_raw(&self) -> *mut Ref<BlendShape> {
        view_raw_mut!(self, shape)
    }
    #[inline(always)]
    pub(crate) fn set_target_weight(&self, value: Real) {
        view_write!(self, target_weight, value)
    }
    #[inline(always)]
    pub(crate) fn target_weight_raw(&self) -> *mut Real {
        view_raw_mut!(self, target_weight)
    }
    #[inline(always)]
    pub(crate) fn set_effective_weight(&self, value: Real) {
        view_write!(self, effective_weight, value)
    }
    #[inline(always)]
    pub(crate) fn effective_weight_raw(&self) -> *mut Real {
        view_raw_mut!(self, effective_weight)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<BlendChannel, M> {
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
    pub(crate) fn keyframes(&self) -> List<BlendKeyframe> {
        view_read_shared!(self, keyframes)
    }
    #[inline(always)]
    pub(crate) fn keyframes_view(&self) -> &View<List<BlendKeyframe>, M> {
        view_project!(self, keyframes)
    }
    #[inline(always)]
    pub(crate) fn keyframes_ptr(&self) -> *const List<BlendKeyframe> {
        view_raw_shared!(self, keyframes)
    }
    #[inline(always)]
    pub(crate) fn target_shape(&self) -> Option<Ref<BlendShape>> {
        view_read_shared!(self, target_shape)
    }
    #[inline(always)]
    pub(crate) fn target_shape_view(&self) -> Option<&View<BlendShape, M>> {
        self.target_shape().map(Ref::view)
    }
    #[inline(always)]
    pub(crate) fn target_shape_ptr(&self) -> *const Option<Ref<BlendShape>> {
        view_raw_shared!(self, target_shape)
    }
}

#[allow(dead_code)]
impl View<BlendChannel, Mut> {
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
    pub(crate) fn set_keyframes(&self, value: List<BlendKeyframe>) {
        view_write!(self, keyframes, value)
    }
    #[inline(always)]
    pub(crate) fn keyframes_raw(&self) -> *mut List<BlendKeyframe> {
        view_raw_mut!(self, keyframes)
    }
    #[inline(always)]
    pub(crate) fn set_target_shape(&self, value: Option<Ref<BlendShape>>) {
        view_write!(self, target_shape, value)
    }
    #[inline(always)]
    pub(crate) fn target_shape_raw(&self) -> *mut Option<Ref<BlendShape>> {
        view_raw_mut!(self, target_shape)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<BlendShape, M> {
    #[inline(always)]
    pub(crate) fn element(&self) -> &View<Element, M> {
        view_project!(self, element)
    }
    #[inline(always)]
    pub(crate) fn element_ptr(&self) -> *const Element {
        view_raw_shared!(self, element)
    }
    #[inline(always)]
    pub(crate) fn num_offsets(&self) -> usize {
        view_read_shared!(self, num_offsets)
    }
    #[inline(always)]
    pub(crate) fn num_offsets_ptr(&self) -> *const usize {
        view_raw_shared!(self, num_offsets)
    }
    #[inline(always)]
    pub(crate) fn offset_vertices(&self) -> List<u32> {
        view_read_shared!(self, offset_vertices)
    }
    #[inline(always)]
    pub(crate) fn offset_vertices_view(&self) -> &View<List<u32>, M> {
        view_project!(self, offset_vertices)
    }
    #[inline(always)]
    pub(crate) fn offset_vertices_ptr(&self) -> *const List<u32> {
        view_raw_shared!(self, offset_vertices)
    }
    #[inline(always)]
    pub(crate) fn position_offsets(&self) -> List<Vec3> {
        view_read_shared!(self, position_offsets)
    }
    #[inline(always)]
    pub(crate) fn position_offsets_view(&self) -> &View<List<Vec3>, M> {
        view_project!(self, position_offsets)
    }
    #[inline(always)]
    pub(crate) fn position_offsets_ptr(&self) -> *const List<Vec3> {
        view_raw_shared!(self, position_offsets)
    }
    #[inline(always)]
    pub(crate) fn normal_offsets(&self) -> List<Vec3> {
        view_read_shared!(self, normal_offsets)
    }
    #[inline(always)]
    pub(crate) fn normal_offsets_view(&self) -> &View<List<Vec3>, M> {
        view_project!(self, normal_offsets)
    }
    #[inline(always)]
    pub(crate) fn normal_offsets_ptr(&self) -> *const List<Vec3> {
        view_raw_shared!(self, normal_offsets)
    }
    #[inline(always)]
    pub(crate) fn offset_weights(&self) -> List<Real> {
        view_read_shared!(self, offset_weights)
    }
    #[inline(always)]
    pub(crate) fn offset_weights_view(&self) -> &View<List<Real>, M> {
        view_project!(self, offset_weights)
    }
    #[inline(always)]
    pub(crate) fn offset_weights_ptr(&self) -> *const List<Real> {
        view_raw_shared!(self, offset_weights)
    }
}

#[allow(dead_code)]
impl View<BlendShape, Mut> {
    #[inline(always)]
    pub(crate) fn set_element(&self, value: Element) {
        view_write!(self, element, value)
    }
    #[inline(always)]
    pub(crate) fn element_raw(&self) -> *mut Element {
        view_raw_mut!(self, element)
    }
    #[inline(always)]
    pub(crate) fn set_num_offsets(&self, value: usize) {
        view_write!(self, num_offsets, value)
    }
    #[inline(always)]
    pub(crate) fn num_offsets_raw(&self) -> *mut usize {
        view_raw_mut!(self, num_offsets)
    }
    #[inline(always)]
    pub(crate) fn set_offset_vertices(&self, value: List<u32>) {
        view_write!(self, offset_vertices, value)
    }
    #[inline(always)]
    pub(crate) fn offset_vertices_raw(&self) -> *mut List<u32> {
        view_raw_mut!(self, offset_vertices)
    }
    #[inline(always)]
    pub(crate) fn set_position_offsets(&self, value: List<Vec3>) {
        view_write!(self, position_offsets, value)
    }
    #[inline(always)]
    pub(crate) fn position_offsets_raw(&self) -> *mut List<Vec3> {
        view_raw_mut!(self, position_offsets)
    }
    #[inline(always)]
    pub(crate) fn set_normal_offsets(&self, value: List<Vec3>) {
        view_write!(self, normal_offsets, value)
    }
    #[inline(always)]
    pub(crate) fn normal_offsets_raw(&self) -> *mut List<Vec3> {
        view_raw_mut!(self, normal_offsets)
    }
    #[inline(always)]
    pub(crate) fn set_offset_weights(&self, value: List<Real>) {
        view_write!(self, offset_weights, value)
    }
    #[inline(always)]
    pub(crate) fn offset_weights_raw(&self) -> *mut List<Real> {
        view_raw_mut!(self, offset_weights)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<BlendDeformer, M> {
    #[inline(always)]
    pub(crate) fn element(&self) -> &View<Element, M> {
        view_project!(self, element)
    }
    #[inline(always)]
    pub(crate) fn element_ptr(&self) -> *const Element {
        view_raw_shared!(self, element)
    }
    #[inline(always)]
    pub(crate) fn channels(&self) -> RefList<BlendChannel> {
        view_read_shared!(self, channels)
    }
    #[inline(always)]
    pub(crate) fn channels_view(&self) -> &View<RefList<BlendChannel>, M> {
        view_project!(self, channels)
    }
    #[inline(always)]
    pub(crate) fn channels_ptr(&self) -> *const RefList<BlendChannel> {
        view_raw_shared!(self, channels)
    }
}

#[allow(dead_code)]
impl View<BlendDeformer, Mut> {
    #[inline(always)]
    pub(crate) fn set_element(&self, value: Element) {
        view_write!(self, element, value)
    }
    #[inline(always)]
    pub(crate) fn element_raw(&self) -> *mut Element {
        view_raw_mut!(self, element)
    }
    #[inline(always)]
    pub(crate) fn set_channels(&self, value: RefList<BlendChannel>) {
        view_write!(self, channels, value)
    }
    #[inline(always)]
    pub(crate) fn channels_raw(&self) -> *mut RefList<BlendChannel> {
        view_raw_mut!(self, channels)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<AnimStack, M> {
    #[inline(always)]
    pub(crate) fn element(&self) -> &View<Element, M> {
        view_project!(self, element)
    }
    #[inline(always)]
    pub(crate) fn element_ptr(&self) -> *const Element {
        view_raw_shared!(self, element)
    }
    #[inline(always)]
    pub(crate) fn time_begin(&self) -> f64 {
        view_read_shared!(self, time_begin)
    }
    #[inline(always)]
    pub(crate) fn time_begin_ptr(&self) -> *const f64 {
        view_raw_shared!(self, time_begin)
    }
    #[inline(always)]
    pub(crate) fn time_end(&self) -> f64 {
        view_read_shared!(self, time_end)
    }
    #[inline(always)]
    pub(crate) fn time_end_ptr(&self) -> *const f64 {
        view_raw_shared!(self, time_end)
    }
    #[inline(always)]
    pub(crate) fn layers(&self) -> RefList<AnimLayer> {
        view_read_shared!(self, layers)
    }
    #[inline(always)]
    pub(crate) fn layers_view(&self) -> &View<RefList<AnimLayer>, M> {
        view_project!(self, layers)
    }
    #[inline(always)]
    pub(crate) fn layers_ptr(&self) -> *const RefList<AnimLayer> {
        view_raw_shared!(self, layers)
    }
    #[inline(always)]
    pub(crate) fn anim(&self) -> Ref<Anim> {
        view_read_shared!(self, anim)
    }
    #[inline(always)]
    pub(crate) fn anim_view(&self) -> &View<Anim, M> {
        self.anim().view()
    }
    #[inline(always)]
    pub(crate) fn anim_ptr(&self) -> *const Ref<Anim> {
        view_raw_shared!(self, anim)
    }
}

#[allow(dead_code)]
impl View<AnimStack, Mut> {
    #[inline(always)]
    pub(crate) fn set_element(&self, value: Element) {
        view_write!(self, element, value)
    }
    #[inline(always)]
    pub(crate) fn element_raw(&self) -> *mut Element {
        view_raw_mut!(self, element)
    }
    #[inline(always)]
    pub(crate) fn set_time_begin(&self, value: f64) {
        view_write!(self, time_begin, value)
    }
    #[inline(always)]
    pub(crate) fn time_begin_raw(&self) -> *mut f64 {
        view_raw_mut!(self, time_begin)
    }
    #[inline(always)]
    pub(crate) fn set_time_end(&self, value: f64) {
        view_write!(self, time_end, value)
    }
    #[inline(always)]
    pub(crate) fn time_end_raw(&self) -> *mut f64 {
        view_raw_mut!(self, time_end)
    }
    #[inline(always)]
    pub(crate) fn set_layers(&self, value: RefList<AnimLayer>) {
        view_write!(self, layers, value)
    }
    #[inline(always)]
    pub(crate) fn layers_raw(&self) -> *mut RefList<AnimLayer> {
        view_raw_mut!(self, layers)
    }
    #[inline(always)]
    pub(crate) fn set_anim(&self, value: Ref<Anim>) {
        view_write!(self, anim, value)
    }
    #[inline(always)]
    pub(crate) fn anim_raw(&self) -> *mut Ref<Anim> {
        view_raw_mut!(self, anim)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<Anim, M> {
    #[inline(always)]
    pub(crate) fn time_begin(&self) -> f64 {
        view_read_shared!(self, time_begin)
    }
    #[inline(always)]
    pub(crate) fn time_begin_ptr(&self) -> *const f64 {
        view_raw_shared!(self, time_begin)
    }
    #[inline(always)]
    pub(crate) fn time_end(&self) -> f64 {
        view_read_shared!(self, time_end)
    }
    #[inline(always)]
    pub(crate) fn time_end_ptr(&self) -> *const f64 {
        view_raw_shared!(self, time_end)
    }
    #[inline(always)]
    pub(crate) fn layers(&self) -> RefList<AnimLayer> {
        view_read_shared!(self, layers)
    }
    #[inline(always)]
    pub(crate) fn layers_view(&self) -> &View<RefList<AnimLayer>, M> {
        view_project!(self, layers)
    }
    #[inline(always)]
    pub(crate) fn layers_ptr(&self) -> *const RefList<AnimLayer> {
        view_raw_shared!(self, layers)
    }
    #[inline(always)]
    pub(crate) fn override_layer_weights(&self) -> List<Real> {
        view_read_shared!(self, override_layer_weights)
    }
    #[inline(always)]
    pub(crate) fn override_layer_weights_view(&self) -> &View<List<Real>, M> {
        view_project!(self, override_layer_weights)
    }
    #[inline(always)]
    pub(crate) fn override_layer_weights_ptr(&self) -> *const List<Real> {
        view_raw_shared!(self, override_layer_weights)
    }
    #[inline(always)]
    pub(crate) fn prop_overrides(&self) -> List<PropOverride> {
        view_read_shared!(self, prop_overrides)
    }
    #[inline(always)]
    pub(crate) fn prop_overrides_view(&self) -> &View<List<PropOverride>, M> {
        view_project!(self, prop_overrides)
    }
    #[inline(always)]
    pub(crate) fn prop_overrides_ptr(&self) -> *const List<PropOverride> {
        view_raw_shared!(self, prop_overrides)
    }
    #[inline(always)]
    pub(crate) fn transform_overrides(&self) -> List<TransformOverride> {
        view_read_shared!(self, transform_overrides)
    }
    #[inline(always)]
    pub(crate) fn transform_overrides_view(&self) -> &View<List<TransformOverride>, M> {
        view_project!(self, transform_overrides)
    }
    #[inline(always)]
    pub(crate) fn transform_overrides_ptr(&self) -> *const List<TransformOverride> {
        view_raw_shared!(self, transform_overrides)
    }
    #[inline(always)]
    pub(crate) fn ignore_connections(&self) -> bool {
        view_read_shared!(self, ignore_connections)
    }
    #[inline(always)]
    pub(crate) fn ignore_connections_ptr(&self) -> *const bool {
        view_raw_shared!(self, ignore_connections)
    }
    #[inline(always)]
    pub(crate) fn custom(&self) -> bool {
        view_read_shared!(self, custom)
    }
    #[inline(always)]
    pub(crate) fn custom_ptr(&self) -> *const bool {
        view_raw_shared!(self, custom)
    }
}

#[allow(dead_code)]
impl View<Anim, Mut> {
    #[inline(always)]
    pub(crate) fn set_time_begin(&self, value: f64) {
        view_write!(self, time_begin, value)
    }
    #[inline(always)]
    pub(crate) fn time_begin_raw(&self) -> *mut f64 {
        view_raw_mut!(self, time_begin)
    }
    #[inline(always)]
    pub(crate) fn set_time_end(&self, value: f64) {
        view_write!(self, time_end, value)
    }
    #[inline(always)]
    pub(crate) fn time_end_raw(&self) -> *mut f64 {
        view_raw_mut!(self, time_end)
    }
    #[inline(always)]
    pub(crate) fn set_layers(&self, value: RefList<AnimLayer>) {
        view_write!(self, layers, value)
    }
    #[inline(always)]
    pub(crate) fn layers_raw(&self) -> *mut RefList<AnimLayer> {
        view_raw_mut!(self, layers)
    }
    #[inline(always)]
    pub(crate) fn set_override_layer_weights(&self, value: List<Real>) {
        view_write!(self, override_layer_weights, value)
    }
    #[inline(always)]
    pub(crate) fn override_layer_weights_raw(&self) -> *mut List<Real> {
        view_raw_mut!(self, override_layer_weights)
    }
    #[inline(always)]
    pub(crate) fn set_prop_overrides(&self, value: List<PropOverride>) {
        view_write!(self, prop_overrides, value)
    }
    #[inline(always)]
    pub(crate) fn prop_overrides_raw(&self) -> *mut List<PropOverride> {
        view_raw_mut!(self, prop_overrides)
    }
    #[inline(always)]
    pub(crate) fn set_transform_overrides(&self, value: List<TransformOverride>) {
        view_write!(self, transform_overrides, value)
    }
    #[inline(always)]
    pub(crate) fn transform_overrides_raw(&self) -> *mut List<TransformOverride> {
        view_raw_mut!(self, transform_overrides)
    }
    #[inline(always)]
    pub(crate) fn set_ignore_connections(&self, value: bool) {
        view_write!(self, ignore_connections, value)
    }
    #[inline(always)]
    pub(crate) fn ignore_connections_raw(&self) -> *mut bool {
        view_raw_mut!(self, ignore_connections)
    }
    #[inline(always)]
    pub(crate) fn set_custom(&self, value: bool) {
        view_write!(self, custom, value)
    }
    #[inline(always)]
    pub(crate) fn custom_raw(&self) -> *mut bool {
        view_raw_mut!(self, custom)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<Light, M> {
    #[inline(always)]
    pub(crate) fn element(&self) -> &View<Element, M> {
        view_project!(self, element)
    }
    #[inline(always)]
    pub(crate) fn element_ptr(&self) -> *const Element {
        view_raw_shared!(self, element)
    }
    #[inline(always)]
    pub(crate) fn color(&self) -> Vec3 {
        view_read_shared!(self, color)
    }
    #[inline(always)]
    pub(crate) fn color_ptr(&self) -> *const Vec3 {
        view_raw_shared!(self, color)
    }
    #[inline(always)]
    pub(crate) fn intensity(&self) -> Real {
        view_read_shared!(self, intensity)
    }
    #[inline(always)]
    pub(crate) fn intensity_ptr(&self) -> *const Real {
        view_raw_shared!(self, intensity)
    }
    #[inline(always)]
    pub(crate) fn local_direction(&self) -> Vec3 {
        view_read_shared!(self, local_direction)
    }
    #[inline(always)]
    pub(crate) fn local_direction_ptr(&self) -> *const Vec3 {
        view_raw_shared!(self, local_direction)
    }
    #[inline(always)]
    pub(crate) fn type_(&self) -> LightType {
        view_read_shared!(self, type_)
    }
    #[inline(always)]
    pub(crate) fn type_ptr(&self) -> *const LightType {
        view_raw_shared!(self, type_)
    }
    #[inline(always)]
    pub(crate) fn decay(&self) -> LightDecay {
        view_read_shared!(self, decay)
    }
    #[inline(always)]
    pub(crate) fn decay_ptr(&self) -> *const LightDecay {
        view_raw_shared!(self, decay)
    }
    #[inline(always)]
    pub(crate) fn area_shape(&self) -> LightAreaShape {
        view_read_shared!(self, area_shape)
    }
    #[inline(always)]
    pub(crate) fn area_shape_ptr(&self) -> *const LightAreaShape {
        view_raw_shared!(self, area_shape)
    }
    #[inline(always)]
    pub(crate) fn inner_angle(&self) -> Real {
        view_read_shared!(self, inner_angle)
    }
    #[inline(always)]
    pub(crate) fn inner_angle_ptr(&self) -> *const Real {
        view_raw_shared!(self, inner_angle)
    }
    #[inline(always)]
    pub(crate) fn outer_angle(&self) -> Real {
        view_read_shared!(self, outer_angle)
    }
    #[inline(always)]
    pub(crate) fn outer_angle_ptr(&self) -> *const Real {
        view_raw_shared!(self, outer_angle)
    }
    #[inline(always)]
    pub(crate) fn cast_light(&self) -> bool {
        view_read_shared!(self, cast_light)
    }
    #[inline(always)]
    pub(crate) fn cast_light_ptr(&self) -> *const bool {
        view_raw_shared!(self, cast_light)
    }
    #[inline(always)]
    pub(crate) fn cast_shadows(&self) -> bool {
        view_read_shared!(self, cast_shadows)
    }
    #[inline(always)]
    pub(crate) fn cast_shadows_ptr(&self) -> *const bool {
        view_raw_shared!(self, cast_shadows)
    }
}

#[allow(dead_code)]
impl View<Light, Mut> {
    #[inline(always)]
    pub(crate) fn set_element(&self, value: Element) {
        view_write!(self, element, value)
    }
    #[inline(always)]
    pub(crate) fn element_raw(&self) -> *mut Element {
        view_raw_mut!(self, element)
    }
    #[inline(always)]
    pub(crate) fn set_color(&self, value: Vec3) {
        view_write!(self, color, value)
    }
    #[inline(always)]
    pub(crate) fn color_raw(&self) -> *mut Vec3 {
        view_raw_mut!(self, color)
    }
    #[inline(always)]
    pub(crate) fn set_intensity(&self, value: Real) {
        view_write!(self, intensity, value)
    }
    #[inline(always)]
    pub(crate) fn intensity_raw(&self) -> *mut Real {
        view_raw_mut!(self, intensity)
    }
    #[inline(always)]
    pub(crate) fn set_local_direction(&self, value: Vec3) {
        view_write!(self, local_direction, value)
    }
    #[inline(always)]
    pub(crate) fn local_direction_raw(&self) -> *mut Vec3 {
        view_raw_mut!(self, local_direction)
    }
    #[inline(always)]
    pub(crate) fn set_type(&self, value: LightType) {
        view_write!(self, type_, value)
    }
    #[inline(always)]
    pub(crate) fn type_raw(&self) -> *mut LightType {
        view_raw_mut!(self, type_)
    }
    #[inline(always)]
    pub(crate) fn set_decay(&self, value: LightDecay) {
        view_write!(self, decay, value)
    }
    #[inline(always)]
    pub(crate) fn decay_raw(&self) -> *mut LightDecay {
        view_raw_mut!(self, decay)
    }
    #[inline(always)]
    pub(crate) fn set_area_shape(&self, value: LightAreaShape) {
        view_write!(self, area_shape, value)
    }
    #[inline(always)]
    pub(crate) fn area_shape_raw(&self) -> *mut LightAreaShape {
        view_raw_mut!(self, area_shape)
    }
    #[inline(always)]
    pub(crate) fn set_inner_angle(&self, value: Real) {
        view_write!(self, inner_angle, value)
    }
    #[inline(always)]
    pub(crate) fn inner_angle_raw(&self) -> *mut Real {
        view_raw_mut!(self, inner_angle)
    }
    #[inline(always)]
    pub(crate) fn set_outer_angle(&self, value: Real) {
        view_write!(self, outer_angle, value)
    }
    #[inline(always)]
    pub(crate) fn outer_angle_raw(&self) -> *mut Real {
        view_raw_mut!(self, outer_angle)
    }
    #[inline(always)]
    pub(crate) fn set_cast_light(&self, value: bool) {
        view_write!(self, cast_light, value)
    }
    #[inline(always)]
    pub(crate) fn cast_light_raw(&self) -> *mut bool {
        view_raw_mut!(self, cast_light)
    }
    #[inline(always)]
    pub(crate) fn set_cast_shadows(&self, value: bool) {
        view_write!(self, cast_shadows, value)
    }
    #[inline(always)]
    pub(crate) fn cast_shadows_raw(&self) -> *mut bool {
        view_raw_mut!(self, cast_shadows)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<Camera, M> {
    #[inline(always)]
    pub(crate) fn element(&self) -> &View<Element, M> {
        view_project!(self, element)
    }
    #[inline(always)]
    pub(crate) fn element_ptr(&self) -> *const Element {
        view_raw_shared!(self, element)
    }
    #[inline(always)]
    pub(crate) fn projection_mode(&self) -> ProjectionMode {
        view_read_shared!(self, projection_mode)
    }
    #[inline(always)]
    pub(crate) fn projection_mode_ptr(&self) -> *const ProjectionMode {
        view_raw_shared!(self, projection_mode)
    }
    #[inline(always)]
    pub(crate) fn resolution_is_pixels(&self) -> bool {
        view_read_shared!(self, resolution_is_pixels)
    }
    #[inline(always)]
    pub(crate) fn resolution_is_pixels_ptr(&self) -> *const bool {
        view_raw_shared!(self, resolution_is_pixels)
    }
    #[inline(always)]
    pub(crate) fn resolution(&self) -> Vec2 {
        view_read_shared!(self, resolution)
    }
    #[inline(always)]
    pub(crate) fn resolution_ptr(&self) -> *const Vec2 {
        view_raw_shared!(self, resolution)
    }
    #[inline(always)]
    pub(crate) fn field_of_view_deg(&self) -> Vec2 {
        view_read_shared!(self, field_of_view_deg)
    }
    #[inline(always)]
    pub(crate) fn field_of_view_deg_ptr(&self) -> *const Vec2 {
        view_raw_shared!(self, field_of_view_deg)
    }
    #[inline(always)]
    pub(crate) fn field_of_view_tan(&self) -> Vec2 {
        view_read_shared!(self, field_of_view_tan)
    }
    #[inline(always)]
    pub(crate) fn field_of_view_tan_ptr(&self) -> *const Vec2 {
        view_raw_shared!(self, field_of_view_tan)
    }
    #[inline(always)]
    pub(crate) fn orthographic_extent(&self) -> Real {
        view_read_shared!(self, orthographic_extent)
    }
    #[inline(always)]
    pub(crate) fn orthographic_extent_ptr(&self) -> *const Real {
        view_raw_shared!(self, orthographic_extent)
    }
    #[inline(always)]
    pub(crate) fn orthographic_size(&self) -> Vec2 {
        view_read_shared!(self, orthographic_size)
    }
    #[inline(always)]
    pub(crate) fn orthographic_size_ptr(&self) -> *const Vec2 {
        view_raw_shared!(self, orthographic_size)
    }
    #[inline(always)]
    pub(crate) fn projection_plane(&self) -> Vec2 {
        view_read_shared!(self, projection_plane)
    }
    #[inline(always)]
    pub(crate) fn projection_plane_ptr(&self) -> *const Vec2 {
        view_raw_shared!(self, projection_plane)
    }
    #[inline(always)]
    pub(crate) fn aspect_ratio(&self) -> Real {
        view_read_shared!(self, aspect_ratio)
    }
    #[inline(always)]
    pub(crate) fn aspect_ratio_ptr(&self) -> *const Real {
        view_raw_shared!(self, aspect_ratio)
    }
    #[inline(always)]
    pub(crate) fn near_plane(&self) -> Real {
        view_read_shared!(self, near_plane)
    }
    #[inline(always)]
    pub(crate) fn near_plane_ptr(&self) -> *const Real {
        view_raw_shared!(self, near_plane)
    }
    #[inline(always)]
    pub(crate) fn far_plane(&self) -> Real {
        view_read_shared!(self, far_plane)
    }
    #[inline(always)]
    pub(crate) fn far_plane_ptr(&self) -> *const Real {
        view_raw_shared!(self, far_plane)
    }
    #[inline(always)]
    pub(crate) fn projection_axes(&self) -> CoordinateAxes {
        view_read_shared!(self, projection_axes)
    }
    #[inline(always)]
    pub(crate) fn projection_axes_ptr(&self) -> *const CoordinateAxes {
        view_raw_shared!(self, projection_axes)
    }
    #[inline(always)]
    pub(crate) fn aspect_mode(&self) -> AspectMode {
        view_read_shared!(self, aspect_mode)
    }
    #[inline(always)]
    pub(crate) fn aspect_mode_ptr(&self) -> *const AspectMode {
        view_raw_shared!(self, aspect_mode)
    }
    #[inline(always)]
    pub(crate) fn aperture_mode(&self) -> ApertureMode {
        view_read_shared!(self, aperture_mode)
    }
    #[inline(always)]
    pub(crate) fn aperture_mode_ptr(&self) -> *const ApertureMode {
        view_raw_shared!(self, aperture_mode)
    }
    #[inline(always)]
    pub(crate) fn gate_fit(&self) -> GateFit {
        view_read_shared!(self, gate_fit)
    }
    #[inline(always)]
    pub(crate) fn gate_fit_ptr(&self) -> *const GateFit {
        view_raw_shared!(self, gate_fit)
    }
    #[inline(always)]
    pub(crate) fn aperture_format(&self) -> ApertureFormat {
        view_read_shared!(self, aperture_format)
    }
    #[inline(always)]
    pub(crate) fn aperture_format_ptr(&self) -> *const ApertureFormat {
        view_raw_shared!(self, aperture_format)
    }
    #[inline(always)]
    pub(crate) fn focal_length_mm(&self) -> Real {
        view_read_shared!(self, focal_length_mm)
    }
    #[inline(always)]
    pub(crate) fn focal_length_mm_ptr(&self) -> *const Real {
        view_raw_shared!(self, focal_length_mm)
    }
    #[inline(always)]
    pub(crate) fn film_size_inch(&self) -> Vec2 {
        view_read_shared!(self, film_size_inch)
    }
    #[inline(always)]
    pub(crate) fn film_size_inch_ptr(&self) -> *const Vec2 {
        view_raw_shared!(self, film_size_inch)
    }
    #[inline(always)]
    pub(crate) fn aperture_size_inch(&self) -> Vec2 {
        view_read_shared!(self, aperture_size_inch)
    }
    #[inline(always)]
    pub(crate) fn aperture_size_inch_ptr(&self) -> *const Vec2 {
        view_raw_shared!(self, aperture_size_inch)
    }
    #[inline(always)]
    pub(crate) fn squeeze_ratio(&self) -> Real {
        view_read_shared!(self, squeeze_ratio)
    }
    #[inline(always)]
    pub(crate) fn squeeze_ratio_ptr(&self) -> *const Real {
        view_raw_shared!(self, squeeze_ratio)
    }
}

#[allow(dead_code)]
impl View<Camera, Mut> {
    #[inline(always)]
    pub(crate) fn set_element(&self, value: Element) {
        view_write!(self, element, value)
    }
    #[inline(always)]
    pub(crate) fn element_raw(&self) -> *mut Element {
        view_raw_mut!(self, element)
    }
    #[inline(always)]
    pub(crate) fn set_projection_mode(&self, value: ProjectionMode) {
        view_write!(self, projection_mode, value)
    }
    #[inline(always)]
    pub(crate) fn projection_mode_raw(&self) -> *mut ProjectionMode {
        view_raw_mut!(self, projection_mode)
    }
    #[inline(always)]
    pub(crate) fn set_resolution_is_pixels(&self, value: bool) {
        view_write!(self, resolution_is_pixels, value)
    }
    #[inline(always)]
    pub(crate) fn resolution_is_pixels_raw(&self) -> *mut bool {
        view_raw_mut!(self, resolution_is_pixels)
    }
    #[inline(always)]
    pub(crate) fn set_resolution(&self, value: Vec2) {
        view_write!(self, resolution, value)
    }
    #[inline(always)]
    pub(crate) fn resolution_raw(&self) -> *mut Vec2 {
        view_raw_mut!(self, resolution)
    }
    #[inline(always)]
    pub(crate) fn set_field_of_view_deg(&self, value: Vec2) {
        view_write!(self, field_of_view_deg, value)
    }
    #[inline(always)]
    pub(crate) fn field_of_view_deg_raw(&self) -> *mut Vec2 {
        view_raw_mut!(self, field_of_view_deg)
    }
    #[inline(always)]
    pub(crate) fn set_field_of_view_tan(&self, value: Vec2) {
        view_write!(self, field_of_view_tan, value)
    }
    #[inline(always)]
    pub(crate) fn field_of_view_tan_raw(&self) -> *mut Vec2 {
        view_raw_mut!(self, field_of_view_tan)
    }
    #[inline(always)]
    pub(crate) fn set_orthographic_extent(&self, value: Real) {
        view_write!(self, orthographic_extent, value)
    }
    #[inline(always)]
    pub(crate) fn orthographic_extent_raw(&self) -> *mut Real {
        view_raw_mut!(self, orthographic_extent)
    }
    #[inline(always)]
    pub(crate) fn set_orthographic_size(&self, value: Vec2) {
        view_write!(self, orthographic_size, value)
    }
    #[inline(always)]
    pub(crate) fn orthographic_size_raw(&self) -> *mut Vec2 {
        view_raw_mut!(self, orthographic_size)
    }
    #[inline(always)]
    pub(crate) fn set_projection_plane(&self, value: Vec2) {
        view_write!(self, projection_plane, value)
    }
    #[inline(always)]
    pub(crate) fn projection_plane_raw(&self) -> *mut Vec2 {
        view_raw_mut!(self, projection_plane)
    }
    #[inline(always)]
    pub(crate) fn set_aspect_ratio(&self, value: Real) {
        view_write!(self, aspect_ratio, value)
    }
    #[inline(always)]
    pub(crate) fn aspect_ratio_raw(&self) -> *mut Real {
        view_raw_mut!(self, aspect_ratio)
    }
    #[inline(always)]
    pub(crate) fn set_near_plane(&self, value: Real) {
        view_write!(self, near_plane, value)
    }
    #[inline(always)]
    pub(crate) fn near_plane_raw(&self) -> *mut Real {
        view_raw_mut!(self, near_plane)
    }
    #[inline(always)]
    pub(crate) fn set_far_plane(&self, value: Real) {
        view_write!(self, far_plane, value)
    }
    #[inline(always)]
    pub(crate) fn far_plane_raw(&self) -> *mut Real {
        view_raw_mut!(self, far_plane)
    }
    #[inline(always)]
    pub(crate) fn set_projection_axes(&self, value: CoordinateAxes) {
        view_write!(self, projection_axes, value)
    }
    #[inline(always)]
    pub(crate) fn projection_axes_raw(&self) -> *mut CoordinateAxes {
        view_raw_mut!(self, projection_axes)
    }
    #[inline(always)]
    pub(crate) fn set_aspect_mode(&self, value: AspectMode) {
        view_write!(self, aspect_mode, value)
    }
    #[inline(always)]
    pub(crate) fn aspect_mode_raw(&self) -> *mut AspectMode {
        view_raw_mut!(self, aspect_mode)
    }
    #[inline(always)]
    pub(crate) fn set_aperture_mode(&self, value: ApertureMode) {
        view_write!(self, aperture_mode, value)
    }
    #[inline(always)]
    pub(crate) fn aperture_mode_raw(&self) -> *mut ApertureMode {
        view_raw_mut!(self, aperture_mode)
    }
    #[inline(always)]
    pub(crate) fn set_gate_fit(&self, value: GateFit) {
        view_write!(self, gate_fit, value)
    }
    #[inline(always)]
    pub(crate) fn gate_fit_raw(&self) -> *mut GateFit {
        view_raw_mut!(self, gate_fit)
    }
    #[inline(always)]
    pub(crate) fn set_aperture_format(&self, value: ApertureFormat) {
        view_write!(self, aperture_format, value)
    }
    #[inline(always)]
    pub(crate) fn aperture_format_raw(&self) -> *mut ApertureFormat {
        view_raw_mut!(self, aperture_format)
    }
    #[inline(always)]
    pub(crate) fn set_focal_length_mm(&self, value: Real) {
        view_write!(self, focal_length_mm, value)
    }
    #[inline(always)]
    pub(crate) fn focal_length_mm_raw(&self) -> *mut Real {
        view_raw_mut!(self, focal_length_mm)
    }
    #[inline(always)]
    pub(crate) fn set_film_size_inch(&self, value: Vec2) {
        view_write!(self, film_size_inch, value)
    }
    #[inline(always)]
    pub(crate) fn film_size_inch_raw(&self) -> *mut Vec2 {
        view_raw_mut!(self, film_size_inch)
    }
    #[inline(always)]
    pub(crate) fn set_aperture_size_inch(&self, value: Vec2) {
        view_write!(self, aperture_size_inch, value)
    }
    #[inline(always)]
    pub(crate) fn aperture_size_inch_raw(&self) -> *mut Vec2 {
        view_raw_mut!(self, aperture_size_inch)
    }
    #[inline(always)]
    pub(crate) fn set_squeeze_ratio(&self, value: Real) {
        view_write!(self, squeeze_ratio, value)
    }
    #[inline(always)]
    pub(crate) fn squeeze_ratio_raw(&self) -> *mut Real {
        view_raw_mut!(self, squeeze_ratio)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<TextureLayer, M> {
    #[inline(always)]
    pub(crate) fn texture(&self) -> Ref<Texture> {
        view_read_shared!(self, texture)
    }
    #[inline(always)]
    pub(crate) fn texture_view(&self) -> &View<Texture, M> {
        self.texture().view()
    }
    #[inline(always)]
    pub(crate) fn texture_ptr(&self) -> *const Ref<Texture> {
        view_raw_shared!(self, texture)
    }
    #[inline(always)]
    pub(crate) fn blend_mode(&self) -> BlendMode {
        view_read_shared!(self, blend_mode)
    }
    #[inline(always)]
    pub(crate) fn blend_mode_ptr(&self) -> *const BlendMode {
        view_raw_shared!(self, blend_mode)
    }
    #[inline(always)]
    pub(crate) fn alpha(&self) -> Real {
        view_read_shared!(self, alpha)
    }
    #[inline(always)]
    pub(crate) fn alpha_ptr(&self) -> *const Real {
        view_raw_shared!(self, alpha)
    }
}

#[allow(dead_code)]
impl View<TextureLayer, Mut> {
    #[inline(always)]
    pub(crate) fn set_texture(&self, value: Ref<Texture>) {
        view_write!(self, texture, value)
    }
    #[inline(always)]
    pub(crate) fn texture_raw(&self) -> *mut Ref<Texture> {
        view_raw_mut!(self, texture)
    }
    #[inline(always)]
    pub(crate) fn set_blend_mode(&self, value: BlendMode) {
        view_write!(self, blend_mode, value)
    }
    #[inline(always)]
    pub(crate) fn blend_mode_raw(&self) -> *mut BlendMode {
        view_raw_mut!(self, blend_mode)
    }
    #[inline(always)]
    pub(crate) fn set_alpha(&self, value: Real) {
        view_write!(self, alpha, value)
    }
    #[inline(always)]
    pub(crate) fn alpha_raw(&self) -> *mut Real {
        view_raw_mut!(self, alpha)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<TextureFile, M> {
    #[inline(always)]
    pub(crate) fn index(&self) -> u32 {
        view_read_shared!(self, index)
    }
    #[inline(always)]
    pub(crate) fn index_ptr(&self) -> *const u32 {
        view_raw_shared!(self, index)
    }
    #[inline(always)]
    pub(crate) fn filename(&self) -> String {
        view_read_shared!(self, filename)
    }
    #[inline(always)]
    pub(crate) fn filename_view(&self) -> &View<String, M> {
        view_project!(self, filename)
    }
    #[inline(always)]
    pub(crate) fn filename_ptr(&self) -> *const String {
        view_raw_shared!(self, filename)
    }
    #[inline(always)]
    pub(crate) fn absolute_filename(&self) -> String {
        view_read_shared!(self, absolute_filename)
    }
    #[inline(always)]
    pub(crate) fn absolute_filename_view(&self) -> &View<String, M> {
        view_project!(self, absolute_filename)
    }
    #[inline(always)]
    pub(crate) fn absolute_filename_ptr(&self) -> *const String {
        view_raw_shared!(self, absolute_filename)
    }
    #[inline(always)]
    pub(crate) fn relative_filename(&self) -> String {
        view_read_shared!(self, relative_filename)
    }
    #[inline(always)]
    pub(crate) fn relative_filename_view(&self) -> &View<String, M> {
        view_project!(self, relative_filename)
    }
    #[inline(always)]
    pub(crate) fn relative_filename_ptr(&self) -> *const String {
        view_raw_shared!(self, relative_filename)
    }
    #[inline(always)]
    pub(crate) fn raw_filename(&self) -> Blob {
        view_read_shared!(self, raw_filename)
    }
    #[inline(always)]
    pub(crate) fn raw_filename_ptr(&self) -> *const Blob {
        view_raw_shared!(self, raw_filename)
    }
    #[inline(always)]
    pub(crate) fn raw_absolute_filename(&self) -> Blob {
        view_read_shared!(self, raw_absolute_filename)
    }
    #[inline(always)]
    pub(crate) fn raw_absolute_filename_ptr(&self) -> *const Blob {
        view_raw_shared!(self, raw_absolute_filename)
    }
    #[inline(always)]
    pub(crate) fn raw_relative_filename(&self) -> Blob {
        view_read_shared!(self, raw_relative_filename)
    }
    #[inline(always)]
    pub(crate) fn raw_relative_filename_ptr(&self) -> *const Blob {
        view_raw_shared!(self, raw_relative_filename)
    }
    #[inline(always)]
    pub(crate) fn content(&self) -> Blob {
        view_read_shared!(self, content)
    }
    #[inline(always)]
    pub(crate) fn content_ptr(&self) -> *const Blob {
        view_raw_shared!(self, content)
    }
}

#[allow(dead_code)]
impl View<TextureFile, Mut> {
    #[inline(always)]
    pub(crate) fn set_index(&self, value: u32) {
        view_write!(self, index, value)
    }
    #[inline(always)]
    pub(crate) fn index_raw(&self) -> *mut u32 {
        view_raw_mut!(self, index)
    }
    #[inline(always)]
    pub(crate) fn set_filename(&self, value: String) {
        view_write!(self, filename, value)
    }
    #[inline(always)]
    pub(crate) fn filename_raw(&self) -> *mut String {
        view_raw_mut!(self, filename)
    }
    #[inline(always)]
    pub(crate) fn set_absolute_filename(&self, value: String) {
        view_write!(self, absolute_filename, value)
    }
    #[inline(always)]
    pub(crate) fn absolute_filename_raw(&self) -> *mut String {
        view_raw_mut!(self, absolute_filename)
    }
    #[inline(always)]
    pub(crate) fn set_relative_filename(&self, value: String) {
        view_write!(self, relative_filename, value)
    }
    #[inline(always)]
    pub(crate) fn relative_filename_raw(&self) -> *mut String {
        view_raw_mut!(self, relative_filename)
    }
    #[inline(always)]
    pub(crate) fn set_raw_filename(&self, value: Blob) {
        view_write!(self, raw_filename, value)
    }
    #[inline(always)]
    pub(crate) fn raw_filename_raw(&self) -> *mut Blob {
        view_raw_mut!(self, raw_filename)
    }
    #[inline(always)]
    pub(crate) fn set_raw_absolute_filename(&self, value: Blob) {
        view_write!(self, raw_absolute_filename, value)
    }
    #[inline(always)]
    pub(crate) fn raw_absolute_filename_raw(&self) -> *mut Blob {
        view_raw_mut!(self, raw_absolute_filename)
    }
    #[inline(always)]
    pub(crate) fn set_raw_relative_filename(&self, value: Blob) {
        view_write!(self, raw_relative_filename, value)
    }
    #[inline(always)]
    pub(crate) fn raw_relative_filename_raw(&self) -> *mut Blob {
        view_raw_mut!(self, raw_relative_filename)
    }
    #[inline(always)]
    pub(crate) fn set_content(&self, value: Blob) {
        view_write!(self, content, value)
    }
    #[inline(always)]
    pub(crate) fn content_raw(&self) -> *mut Blob {
        view_raw_mut!(self, content)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<Bone, M> {
    #[inline(always)]
    pub(crate) fn element(&self) -> &View<Element, M> {
        view_project!(self, element)
    }
    #[inline(always)]
    pub(crate) fn element_ptr(&self) -> *const Element {
        view_raw_shared!(self, element)
    }
    #[inline(always)]
    pub(crate) fn radius(&self) -> Real {
        view_read_shared!(self, radius)
    }
    #[inline(always)]
    pub(crate) fn radius_ptr(&self) -> *const Real {
        view_raw_shared!(self, radius)
    }
    #[inline(always)]
    pub(crate) fn relative_length(&self) -> Real {
        view_read_shared!(self, relative_length)
    }
    #[inline(always)]
    pub(crate) fn relative_length_ptr(&self) -> *const Real {
        view_raw_shared!(self, relative_length)
    }
    #[inline(always)]
    pub(crate) fn is_root(&self) -> bool {
        view_read_shared!(self, is_root)
    }
    #[inline(always)]
    pub(crate) fn is_root_ptr(&self) -> *const bool {
        view_raw_shared!(self, is_root)
    }
}

#[allow(dead_code)]
impl View<Bone, Mut> {
    #[inline(always)]
    pub(crate) fn set_element(&self, value: Element) {
        view_write!(self, element, value)
    }
    #[inline(always)]
    pub(crate) fn element_raw(&self) -> *mut Element {
        view_raw_mut!(self, element)
    }
    #[inline(always)]
    pub(crate) fn set_radius(&self, value: Real) {
        view_write!(self, radius, value)
    }
    #[inline(always)]
    pub(crate) fn radius_raw(&self) -> *mut Real {
        view_raw_mut!(self, radius)
    }
    #[inline(always)]
    pub(crate) fn set_relative_length(&self, value: Real) {
        view_write!(self, relative_length, value)
    }
    #[inline(always)]
    pub(crate) fn relative_length_raw(&self) -> *mut Real {
        view_raw_mut!(self, relative_length)
    }
    #[inline(always)]
    pub(crate) fn set_is_root(&self, value: bool) {
        view_write!(self, is_root, value)
    }
    #[inline(always)]
    pub(crate) fn is_root_raw(&self) -> *mut bool {
        view_raw_mut!(self, is_root)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<CacheFile, M> {
    #[inline(always)]
    pub(crate) fn element(&self) -> &View<Element, M> {
        view_project!(self, element)
    }
    #[inline(always)]
    pub(crate) fn element_ptr(&self) -> *const Element {
        view_raw_shared!(self, element)
    }
    #[inline(always)]
    pub(crate) fn filename(&self) -> String {
        view_read_shared!(self, filename)
    }
    #[inline(always)]
    pub(crate) fn filename_view(&self) -> &View<String, M> {
        view_project!(self, filename)
    }
    #[inline(always)]
    pub(crate) fn filename_ptr(&self) -> *const String {
        view_raw_shared!(self, filename)
    }
    #[inline(always)]
    pub(crate) fn absolute_filename(&self) -> String {
        view_read_shared!(self, absolute_filename)
    }
    #[inline(always)]
    pub(crate) fn absolute_filename_view(&self) -> &View<String, M> {
        view_project!(self, absolute_filename)
    }
    #[inline(always)]
    pub(crate) fn absolute_filename_ptr(&self) -> *const String {
        view_raw_shared!(self, absolute_filename)
    }
    #[inline(always)]
    pub(crate) fn relative_filename(&self) -> String {
        view_read_shared!(self, relative_filename)
    }
    #[inline(always)]
    pub(crate) fn relative_filename_view(&self) -> &View<String, M> {
        view_project!(self, relative_filename)
    }
    #[inline(always)]
    pub(crate) fn relative_filename_ptr(&self) -> *const String {
        view_raw_shared!(self, relative_filename)
    }
    #[inline(always)]
    pub(crate) fn raw_filename(&self) -> Blob {
        view_read_shared!(self, raw_filename)
    }
    #[inline(always)]
    pub(crate) fn raw_filename_ptr(&self) -> *const Blob {
        view_raw_shared!(self, raw_filename)
    }
    #[inline(always)]
    pub(crate) fn raw_absolute_filename(&self) -> Blob {
        view_read_shared!(self, raw_absolute_filename)
    }
    #[inline(always)]
    pub(crate) fn raw_absolute_filename_ptr(&self) -> *const Blob {
        view_raw_shared!(self, raw_absolute_filename)
    }
    #[inline(always)]
    pub(crate) fn raw_relative_filename(&self) -> Blob {
        view_read_shared!(self, raw_relative_filename)
    }
    #[inline(always)]
    pub(crate) fn raw_relative_filename_ptr(&self) -> *const Blob {
        view_raw_shared!(self, raw_relative_filename)
    }
    #[inline(always)]
    pub(crate) fn format(&self) -> CacheFileFormat {
        view_read_shared!(self, format)
    }
    #[inline(always)]
    pub(crate) fn format_ptr(&self) -> *const CacheFileFormat {
        view_raw_shared!(self, format)
    }
    #[inline(always)]
    pub(crate) fn external_cache(&self) -> Option<Ref<GeometryCache>> {
        view_read_shared!(self, external_cache)
    }
    #[inline(always)]
    pub(crate) fn external_cache_view(&self) -> Option<&View<GeometryCache, M>> {
        self.external_cache().map(Ref::view)
    }
    #[inline(always)]
    pub(crate) fn external_cache_ptr(&self) -> *const Option<Ref<GeometryCache>> {
        view_raw_shared!(self, external_cache)
    }
}

#[allow(dead_code)]
impl View<CacheFile, Mut> {
    #[inline(always)]
    pub(crate) fn set_element(&self, value: Element) {
        view_write!(self, element, value)
    }
    #[inline(always)]
    pub(crate) fn element_raw(&self) -> *mut Element {
        view_raw_mut!(self, element)
    }
    #[inline(always)]
    pub(crate) fn set_filename(&self, value: String) {
        view_write!(self, filename, value)
    }
    #[inline(always)]
    pub(crate) fn filename_raw(&self) -> *mut String {
        view_raw_mut!(self, filename)
    }
    #[inline(always)]
    pub(crate) fn set_absolute_filename(&self, value: String) {
        view_write!(self, absolute_filename, value)
    }
    #[inline(always)]
    pub(crate) fn absolute_filename_raw(&self) -> *mut String {
        view_raw_mut!(self, absolute_filename)
    }
    #[inline(always)]
    pub(crate) fn set_relative_filename(&self, value: String) {
        view_write!(self, relative_filename, value)
    }
    #[inline(always)]
    pub(crate) fn relative_filename_raw(&self) -> *mut String {
        view_raw_mut!(self, relative_filename)
    }
    #[inline(always)]
    pub(crate) fn set_raw_filename(&self, value: Blob) {
        view_write!(self, raw_filename, value)
    }
    #[inline(always)]
    pub(crate) fn raw_filename_raw(&self) -> *mut Blob {
        view_raw_mut!(self, raw_filename)
    }
    #[inline(always)]
    pub(crate) fn set_raw_absolute_filename(&self, value: Blob) {
        view_write!(self, raw_absolute_filename, value)
    }
    #[inline(always)]
    pub(crate) fn raw_absolute_filename_raw(&self) -> *mut Blob {
        view_raw_mut!(self, raw_absolute_filename)
    }
    #[inline(always)]
    pub(crate) fn set_raw_relative_filename(&self, value: Blob) {
        view_write!(self, raw_relative_filename, value)
    }
    #[inline(always)]
    pub(crate) fn raw_relative_filename_raw(&self) -> *mut Blob {
        view_raw_mut!(self, raw_relative_filename)
    }
    #[inline(always)]
    pub(crate) fn set_format(&self, value: CacheFileFormat) {
        view_write!(self, format, value)
    }
    #[inline(always)]
    pub(crate) fn format_raw(&self) -> *mut CacheFileFormat {
        view_raw_mut!(self, format)
    }
    #[inline(always)]
    pub(crate) fn set_external_cache(&self, value: Option<Ref<GeometryCache>>) {
        view_write!(self, external_cache, value)
    }
    #[inline(always)]
    pub(crate) fn external_cache_raw(&self) -> *mut Option<Ref<GeometryCache>> {
        view_raw_mut!(self, external_cache)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<Constraint, M> {
    #[inline(always)]
    pub(crate) fn element(&self) -> &View<Element, M> {
        view_project!(self, element)
    }
    #[inline(always)]
    pub(crate) fn element_ptr(&self) -> *const Element {
        view_raw_shared!(self, element)
    }
    #[inline(always)]
    pub(crate) fn type_(&self) -> ConstraintType {
        view_read_shared!(self, type_)
    }
    #[inline(always)]
    pub(crate) fn type_ptr(&self) -> *const ConstraintType {
        view_raw_shared!(self, type_)
    }
    #[inline(always)]
    pub(crate) fn type_name(&self) -> String {
        view_read_shared!(self, type_name)
    }
    #[inline(always)]
    pub(crate) fn type_name_view(&self) -> &View<String, M> {
        view_project!(self, type_name)
    }
    #[inline(always)]
    pub(crate) fn type_name_ptr(&self) -> *const String {
        view_raw_shared!(self, type_name)
    }
    #[inline(always)]
    pub(crate) fn node(&self) -> Option<Ref<Node>> {
        view_read_shared!(self, node)
    }
    #[inline(always)]
    pub(crate) fn node_view(&self) -> Option<&View<Node, M>> {
        self.node().map(Ref::view)
    }
    #[inline(always)]
    pub(crate) fn node_ptr(&self) -> *const Option<Ref<Node>> {
        view_raw_shared!(self, node)
    }
    #[inline(always)]
    pub(crate) fn targets(&self) -> List<ConstraintTarget> {
        view_read_shared!(self, targets)
    }
    #[inline(always)]
    pub(crate) fn targets_view(&self) -> &View<List<ConstraintTarget>, M> {
        view_project!(self, targets)
    }
    #[inline(always)]
    pub(crate) fn targets_ptr(&self) -> *const List<ConstraintTarget> {
        view_raw_shared!(self, targets)
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
    pub(crate) fn active(&self) -> bool {
        view_read_shared!(self, active)
    }
    #[inline(always)]
    pub(crate) fn active_ptr(&self) -> *const bool {
        view_raw_shared!(self, active)
    }
    #[inline(always)]
    pub(crate) fn constrain_translation(&self) -> [bool; 3] {
        view_read_shared!(self, constrain_translation)
    }
    #[inline(always)]
    pub(crate) fn constrain_translation_ptr(&self) -> *const [bool; 3] {
        view_raw_shared!(self, constrain_translation)
    }
    #[inline(always)]
    pub(crate) fn constrain_rotation(&self) -> [bool; 3] {
        view_read_shared!(self, constrain_rotation)
    }
    #[inline(always)]
    pub(crate) fn constrain_rotation_ptr(&self) -> *const [bool; 3] {
        view_raw_shared!(self, constrain_rotation)
    }
    #[inline(always)]
    pub(crate) fn constrain_scale(&self) -> [bool; 3] {
        view_read_shared!(self, constrain_scale)
    }
    #[inline(always)]
    pub(crate) fn constrain_scale_ptr(&self) -> *const [bool; 3] {
        view_raw_shared!(self, constrain_scale)
    }
    #[inline(always)]
    pub(crate) fn transform_offset(&self) -> Transform {
        view_read_shared!(self, transform_offset)
    }
    #[inline(always)]
    pub(crate) fn transform_offset_ptr(&self) -> *const Transform {
        view_raw_shared!(self, transform_offset)
    }
    #[inline(always)]
    pub(crate) fn aim_vector(&self) -> Vec3 {
        view_read_shared!(self, aim_vector)
    }
    #[inline(always)]
    pub(crate) fn aim_vector_ptr(&self) -> *const Vec3 {
        view_raw_shared!(self, aim_vector)
    }
    #[inline(always)]
    pub(crate) fn aim_up_type(&self) -> ConstraintAimUpType {
        view_read_shared!(self, aim_up_type)
    }
    #[inline(always)]
    pub(crate) fn aim_up_type_ptr(&self) -> *const ConstraintAimUpType {
        view_raw_shared!(self, aim_up_type)
    }
    #[inline(always)]
    pub(crate) fn aim_up_node(&self) -> Option<Ref<Node>> {
        view_read_shared!(self, aim_up_node)
    }
    #[inline(always)]
    pub(crate) fn aim_up_node_view(&self) -> Option<&View<Node, M>> {
        self.aim_up_node().map(Ref::view)
    }
    #[inline(always)]
    pub(crate) fn aim_up_node_ptr(&self) -> *const Option<Ref<Node>> {
        view_raw_shared!(self, aim_up_node)
    }
    #[inline(always)]
    pub(crate) fn aim_up_vector(&self) -> Vec3 {
        view_read_shared!(self, aim_up_vector)
    }
    #[inline(always)]
    pub(crate) fn aim_up_vector_ptr(&self) -> *const Vec3 {
        view_raw_shared!(self, aim_up_vector)
    }
    #[inline(always)]
    pub(crate) fn ik_effector(&self) -> Option<Ref<Node>> {
        view_read_shared!(self, ik_effector)
    }
    #[inline(always)]
    pub(crate) fn ik_effector_view(&self) -> Option<&View<Node, M>> {
        self.ik_effector().map(Ref::view)
    }
    #[inline(always)]
    pub(crate) fn ik_effector_ptr(&self) -> *const Option<Ref<Node>> {
        view_raw_shared!(self, ik_effector)
    }
    #[inline(always)]
    pub(crate) fn ik_end_node(&self) -> Option<Ref<Node>> {
        view_read_shared!(self, ik_end_node)
    }
    #[inline(always)]
    pub(crate) fn ik_end_node_view(&self) -> Option<&View<Node, M>> {
        self.ik_end_node().map(Ref::view)
    }
    #[inline(always)]
    pub(crate) fn ik_end_node_ptr(&self) -> *const Option<Ref<Node>> {
        view_raw_shared!(self, ik_end_node)
    }
    #[inline(always)]
    pub(crate) fn ik_pole_vector(&self) -> Vec3 {
        view_read_shared!(self, ik_pole_vector)
    }
    #[inline(always)]
    pub(crate) fn ik_pole_vector_ptr(&self) -> *const Vec3 {
        view_raw_shared!(self, ik_pole_vector)
    }
}

#[allow(dead_code)]
impl View<Constraint, Mut> {
    #[inline(always)]
    pub(crate) fn set_element(&self, value: Element) {
        view_write!(self, element, value)
    }
    #[inline(always)]
    pub(crate) fn element_raw(&self) -> *mut Element {
        view_raw_mut!(self, element)
    }
    #[inline(always)]
    pub(crate) fn set_type(&self, value: ConstraintType) {
        view_write!(self, type_, value)
    }
    #[inline(always)]
    pub(crate) fn type_raw(&self) -> *mut ConstraintType {
        view_raw_mut!(self, type_)
    }
    #[inline(always)]
    pub(crate) fn set_type_name(&self, value: String) {
        view_write!(self, type_name, value)
    }
    #[inline(always)]
    pub(crate) fn type_name_raw(&self) -> *mut String {
        view_raw_mut!(self, type_name)
    }
    #[inline(always)]
    pub(crate) fn set_node(&self, value: Option<Ref<Node>>) {
        view_write!(self, node, value)
    }
    #[inline(always)]
    pub(crate) fn node_raw(&self) -> *mut Option<Ref<Node>> {
        view_raw_mut!(self, node)
    }
    #[inline(always)]
    pub(crate) fn set_targets(&self, value: List<ConstraintTarget>) {
        view_write!(self, targets, value)
    }
    #[inline(always)]
    pub(crate) fn targets_raw(&self) -> *mut List<ConstraintTarget> {
        view_raw_mut!(self, targets)
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
    pub(crate) fn set_active(&self, value: bool) {
        view_write!(self, active, value)
    }
    #[inline(always)]
    pub(crate) fn active_raw(&self) -> *mut bool {
        view_raw_mut!(self, active)
    }
    #[inline(always)]
    pub(crate) fn set_constrain_translation(&self, value: [bool; 3]) {
        view_write!(self, constrain_translation, value)
    }
    #[inline(always)]
    pub(crate) fn constrain_translation_raw(&self) -> *mut [bool; 3] {
        view_raw_mut!(self, constrain_translation)
    }
    #[inline(always)]
    pub(crate) fn set_constrain_rotation(&self, value: [bool; 3]) {
        view_write!(self, constrain_rotation, value)
    }
    #[inline(always)]
    pub(crate) fn constrain_rotation_raw(&self) -> *mut [bool; 3] {
        view_raw_mut!(self, constrain_rotation)
    }
    #[inline(always)]
    pub(crate) fn set_constrain_scale(&self, value: [bool; 3]) {
        view_write!(self, constrain_scale, value)
    }
    #[inline(always)]
    pub(crate) fn constrain_scale_raw(&self) -> *mut [bool; 3] {
        view_raw_mut!(self, constrain_scale)
    }
    #[inline(always)]
    pub(crate) fn set_transform_offset(&self, value: Transform) {
        view_write!(self, transform_offset, value)
    }
    #[inline(always)]
    pub(crate) fn transform_offset_raw(&self) -> *mut Transform {
        view_raw_mut!(self, transform_offset)
    }
    #[inline(always)]
    pub(crate) fn set_aim_vector(&self, value: Vec3) {
        view_write!(self, aim_vector, value)
    }
    #[inline(always)]
    pub(crate) fn aim_vector_raw(&self) -> *mut Vec3 {
        view_raw_mut!(self, aim_vector)
    }
    #[inline(always)]
    pub(crate) fn set_aim_up_type(&self, value: ConstraintAimUpType) {
        view_write!(self, aim_up_type, value)
    }
    #[inline(always)]
    pub(crate) fn aim_up_type_raw(&self) -> *mut ConstraintAimUpType {
        view_raw_mut!(self, aim_up_type)
    }
    #[inline(always)]
    pub(crate) fn set_aim_up_node(&self, value: Option<Ref<Node>>) {
        view_write!(self, aim_up_node, value)
    }
    #[inline(always)]
    pub(crate) fn aim_up_node_raw(&self) -> *mut Option<Ref<Node>> {
        view_raw_mut!(self, aim_up_node)
    }
    #[inline(always)]
    pub(crate) fn set_aim_up_vector(&self, value: Vec3) {
        view_write!(self, aim_up_vector, value)
    }
    #[inline(always)]
    pub(crate) fn aim_up_vector_raw(&self) -> *mut Vec3 {
        view_raw_mut!(self, aim_up_vector)
    }
    #[inline(always)]
    pub(crate) fn set_ik_effector(&self, value: Option<Ref<Node>>) {
        view_write!(self, ik_effector, value)
    }
    #[inline(always)]
    pub(crate) fn ik_effector_raw(&self) -> *mut Option<Ref<Node>> {
        view_raw_mut!(self, ik_effector)
    }
    #[inline(always)]
    pub(crate) fn set_ik_end_node(&self, value: Option<Ref<Node>>) {
        view_write!(self, ik_end_node, value)
    }
    #[inline(always)]
    pub(crate) fn ik_end_node_raw(&self) -> *mut Option<Ref<Node>> {
        view_raw_mut!(self, ik_end_node)
    }
    #[inline(always)]
    pub(crate) fn set_ik_pole_vector(&self, value: Vec3) {
        view_write!(self, ik_pole_vector, value)
    }
    #[inline(always)]
    pub(crate) fn ik_pole_vector_raw(&self) -> *mut Vec3 {
        view_raw_mut!(self, ik_pole_vector)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<ConstraintTarget, M> {
    #[inline(always)]
    pub(crate) fn node(&self) -> Ref<Node> {
        view_read_shared!(self, node)
    }
    #[inline(always)]
    pub(crate) fn node_view(&self) -> &View<Node, M> {
        self.node().view()
    }
    #[inline(always)]
    pub(crate) fn node_ptr(&self) -> *const Ref<Node> {
        view_raw_shared!(self, node)
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
    pub(crate) fn transform(&self) -> Transform {
        view_read_shared!(self, transform)
    }
    #[inline(always)]
    pub(crate) fn transform_ptr(&self) -> *const Transform {
        view_raw_shared!(self, transform)
    }
}

#[allow(dead_code)]
impl View<ConstraintTarget, Mut> {
    #[inline(always)]
    pub(crate) fn set_node(&self, value: Ref<Node>) {
        view_write!(self, node, value)
    }
    #[inline(always)]
    pub(crate) fn node_raw(&self) -> *mut Ref<Node> {
        view_raw_mut!(self, node)
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
    pub(crate) fn set_transform(&self, value: Transform) {
        view_write!(self, transform, value)
    }
    #[inline(always)]
    pub(crate) fn transform_raw(&self) -> *mut Transform {
        view_raw_mut!(self, transform)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<NurbsBasis, M> {
    #[inline(always)]
    pub(crate) fn order(&self) -> u32 {
        view_read_shared!(self, order)
    }
    #[inline(always)]
    pub(crate) fn order_ptr(&self) -> *const u32 {
        view_raw_shared!(self, order)
    }
    #[inline(always)]
    pub(crate) fn topology(&self) -> NurbsTopology {
        view_read_shared!(self, topology)
    }
    #[inline(always)]
    pub(crate) fn topology_ptr(&self) -> *const NurbsTopology {
        view_raw_shared!(self, topology)
    }
    #[inline(always)]
    pub(crate) fn knot_vector(&self) -> List<Real> {
        view_read_shared!(self, knot_vector)
    }
    #[inline(always)]
    pub(crate) fn knot_vector_view(&self) -> &View<List<Real>, M> {
        view_project!(self, knot_vector)
    }
    #[inline(always)]
    pub(crate) fn knot_vector_ptr(&self) -> *const List<Real> {
        view_raw_shared!(self, knot_vector)
    }
    #[inline(always)]
    pub(crate) fn t_min(&self) -> Real {
        view_read_shared!(self, t_min)
    }
    #[inline(always)]
    pub(crate) fn t_min_ptr(&self) -> *const Real {
        view_raw_shared!(self, t_min)
    }
    #[inline(always)]
    pub(crate) fn t_max(&self) -> Real {
        view_read_shared!(self, t_max)
    }
    #[inline(always)]
    pub(crate) fn t_max_ptr(&self) -> *const Real {
        view_raw_shared!(self, t_max)
    }
    #[inline(always)]
    pub(crate) fn spans(&self) -> List<Real> {
        view_read_shared!(self, spans)
    }
    #[inline(always)]
    pub(crate) fn spans_view(&self) -> &View<List<Real>, M> {
        view_project!(self, spans)
    }
    #[inline(always)]
    pub(crate) fn spans_ptr(&self) -> *const List<Real> {
        view_raw_shared!(self, spans)
    }
    #[inline(always)]
    pub(crate) fn is_2d(&self) -> bool {
        view_read_shared!(self, is_2d)
    }
    #[inline(always)]
    pub(crate) fn is_2d_ptr(&self) -> *const bool {
        view_raw_shared!(self, is_2d)
    }
    #[inline(always)]
    pub(crate) fn num_wrap_control_points(&self) -> usize {
        view_read_shared!(self, num_wrap_control_points)
    }
    #[inline(always)]
    pub(crate) fn num_wrap_control_points_ptr(&self) -> *const usize {
        view_raw_shared!(self, num_wrap_control_points)
    }
    #[inline(always)]
    pub(crate) fn valid(&self) -> bool {
        view_read_shared!(self, valid)
    }
    #[inline(always)]
    pub(crate) fn valid_ptr(&self) -> *const bool {
        view_raw_shared!(self, valid)
    }
}

#[allow(dead_code)]
impl View<NurbsBasis, Mut> {
    #[inline(always)]
    pub(crate) fn set_order(&self, value: u32) {
        view_write!(self, order, value)
    }
    #[inline(always)]
    pub(crate) fn order_raw(&self) -> *mut u32 {
        view_raw_mut!(self, order)
    }
    #[inline(always)]
    pub(crate) fn set_topology(&self, value: NurbsTopology) {
        view_write!(self, topology, value)
    }
    #[inline(always)]
    pub(crate) fn topology_raw(&self) -> *mut NurbsTopology {
        view_raw_mut!(self, topology)
    }
    #[inline(always)]
    pub(crate) fn set_knot_vector(&self, value: List<Real>) {
        view_write!(self, knot_vector, value)
    }
    #[inline(always)]
    pub(crate) fn knot_vector_raw(&self) -> *mut List<Real> {
        view_raw_mut!(self, knot_vector)
    }
    #[inline(always)]
    pub(crate) fn set_t_min(&self, value: Real) {
        view_write!(self, t_min, value)
    }
    #[inline(always)]
    pub(crate) fn t_min_raw(&self) -> *mut Real {
        view_raw_mut!(self, t_min)
    }
    #[inline(always)]
    pub(crate) fn set_t_max(&self, value: Real) {
        view_write!(self, t_max, value)
    }
    #[inline(always)]
    pub(crate) fn t_max_raw(&self) -> *mut Real {
        view_raw_mut!(self, t_max)
    }
    #[inline(always)]
    pub(crate) fn set_spans(&self, value: List<Real>) {
        view_write!(self, spans, value)
    }
    #[inline(always)]
    pub(crate) fn spans_raw(&self) -> *mut List<Real> {
        view_raw_mut!(self, spans)
    }
    #[inline(always)]
    pub(crate) fn set_is_2d(&self, value: bool) {
        view_write!(self, is_2d, value)
    }
    #[inline(always)]
    pub(crate) fn is_2d_raw(&self) -> *mut bool {
        view_raw_mut!(self, is_2d)
    }
    #[inline(always)]
    pub(crate) fn set_num_wrap_control_points(&self, value: usize) {
        view_write!(self, num_wrap_control_points, value)
    }
    #[inline(always)]
    pub(crate) fn num_wrap_control_points_raw(&self) -> *mut usize {
        view_raw_mut!(self, num_wrap_control_points)
    }
    #[inline(always)]
    pub(crate) fn set_valid(&self, value: bool) {
        view_write!(self, valid, value)
    }
    #[inline(always)]
    pub(crate) fn valid_raw(&self) -> *mut bool {
        view_raw_mut!(self, valid)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<NurbsCurve, M> {
    #[inline(always)]
    pub(crate) fn element(&self) -> &View<Element, M> {
        view_project!(self, element)
    }
    #[inline(always)]
    pub(crate) fn element_ptr(&self) -> *const Element {
        view_raw_shared!(self, element)
    }
    #[inline(always)]
    pub(crate) fn basis(&self) -> &View<NurbsBasis, M> {
        view_project!(self, basis)
    }
    #[inline(always)]
    pub(crate) fn basis_ptr(&self) -> *const NurbsBasis {
        view_raw_shared!(self, basis)
    }
    #[inline(always)]
    pub(crate) fn control_points(&self) -> List<Vec4> {
        view_read_shared!(self, control_points)
    }
    #[inline(always)]
    pub(crate) fn control_points_view(&self) -> &View<List<Vec4>, M> {
        view_project!(self, control_points)
    }
    #[inline(always)]
    pub(crate) fn control_points_ptr(&self) -> *const List<Vec4> {
        view_raw_shared!(self, control_points)
    }
}

#[allow(dead_code)]
impl View<NurbsCurve, Mut> {
    #[inline(always)]
    pub(crate) fn set_element(&self, value: Element) {
        view_write!(self, element, value)
    }
    #[inline(always)]
    pub(crate) fn element_raw(&self) -> *mut Element {
        view_raw_mut!(self, element)
    }
    #[inline(always)]
    pub(crate) fn set_basis(&self, value: NurbsBasis) {
        view_write!(self, basis, value)
    }
    #[inline(always)]
    pub(crate) fn basis_raw(&self) -> *mut NurbsBasis {
        view_raw_mut!(self, basis)
    }
    #[inline(always)]
    pub(crate) fn set_control_points(&self, value: List<Vec4>) {
        view_write!(self, control_points, value)
    }
    #[inline(always)]
    pub(crate) fn control_points_raw(&self) -> *mut List<Vec4> {
        view_raw_mut!(self, control_points)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<NurbsSurface, M> {
    #[inline(always)]
    pub(crate) fn element(&self) -> &View<Element, M> {
        view_project!(self, element)
    }
    #[inline(always)]
    pub(crate) fn element_ptr(&self) -> *const Element {
        view_raw_shared!(self, element)
    }
    #[inline(always)]
    pub(crate) fn basis_u(&self) -> &View<NurbsBasis, M> {
        view_project!(self, basis_u)
    }
    #[inline(always)]
    pub(crate) fn basis_u_ptr(&self) -> *const NurbsBasis {
        view_raw_shared!(self, basis_u)
    }
    #[inline(always)]
    pub(crate) fn basis_v(&self) -> &View<NurbsBasis, M> {
        view_project!(self, basis_v)
    }
    #[inline(always)]
    pub(crate) fn basis_v_ptr(&self) -> *const NurbsBasis {
        view_raw_shared!(self, basis_v)
    }
    #[inline(always)]
    pub(crate) fn num_control_points_u(&self) -> usize {
        view_read_shared!(self, num_control_points_u)
    }
    #[inline(always)]
    pub(crate) fn num_control_points_u_ptr(&self) -> *const usize {
        view_raw_shared!(self, num_control_points_u)
    }
    #[inline(always)]
    pub(crate) fn num_control_points_v(&self) -> usize {
        view_read_shared!(self, num_control_points_v)
    }
    #[inline(always)]
    pub(crate) fn num_control_points_v_ptr(&self) -> *const usize {
        view_raw_shared!(self, num_control_points_v)
    }
    #[inline(always)]
    pub(crate) fn control_points(&self) -> List<Vec4> {
        view_read_shared!(self, control_points)
    }
    #[inline(always)]
    pub(crate) fn control_points_view(&self) -> &View<List<Vec4>, M> {
        view_project!(self, control_points)
    }
    #[inline(always)]
    pub(crate) fn control_points_ptr(&self) -> *const List<Vec4> {
        view_raw_shared!(self, control_points)
    }
    #[inline(always)]
    pub(crate) fn span_subdivision_u(&self) -> u32 {
        view_read_shared!(self, span_subdivision_u)
    }
    #[inline(always)]
    pub(crate) fn span_subdivision_u_ptr(&self) -> *const u32 {
        view_raw_shared!(self, span_subdivision_u)
    }
    #[inline(always)]
    pub(crate) fn span_subdivision_v(&self) -> u32 {
        view_read_shared!(self, span_subdivision_v)
    }
    #[inline(always)]
    pub(crate) fn span_subdivision_v_ptr(&self) -> *const u32 {
        view_raw_shared!(self, span_subdivision_v)
    }
    #[inline(always)]
    pub(crate) fn flip_normals(&self) -> bool {
        view_read_shared!(self, flip_normals)
    }
    #[inline(always)]
    pub(crate) fn flip_normals_ptr(&self) -> *const bool {
        view_raw_shared!(self, flip_normals)
    }
    #[inline(always)]
    pub(crate) fn material(&self) -> Option<Ref<Material>> {
        view_read_shared!(self, material)
    }
    #[inline(always)]
    pub(crate) fn material_view(&self) -> Option<&View<Material, M>> {
        self.material().map(Ref::view)
    }
    #[inline(always)]
    pub(crate) fn material_ptr(&self) -> *const Option<Ref<Material>> {
        view_raw_shared!(self, material)
    }
}

#[allow(dead_code)]
impl View<NurbsSurface, Mut> {
    #[inline(always)]
    pub(crate) fn set_element(&self, value: Element) {
        view_write!(self, element, value)
    }
    #[inline(always)]
    pub(crate) fn element_raw(&self) -> *mut Element {
        view_raw_mut!(self, element)
    }
    #[inline(always)]
    pub(crate) fn set_basis_u(&self, value: NurbsBasis) {
        view_write!(self, basis_u, value)
    }
    #[inline(always)]
    pub(crate) fn basis_u_raw(&self) -> *mut NurbsBasis {
        view_raw_mut!(self, basis_u)
    }
    #[inline(always)]
    pub(crate) fn set_basis_v(&self, value: NurbsBasis) {
        view_write!(self, basis_v, value)
    }
    #[inline(always)]
    pub(crate) fn basis_v_raw(&self) -> *mut NurbsBasis {
        view_raw_mut!(self, basis_v)
    }
    #[inline(always)]
    pub(crate) fn set_num_control_points_u(&self, value: usize) {
        view_write!(self, num_control_points_u, value)
    }
    #[inline(always)]
    pub(crate) fn num_control_points_u_raw(&self) -> *mut usize {
        view_raw_mut!(self, num_control_points_u)
    }
    #[inline(always)]
    pub(crate) fn set_num_control_points_v(&self, value: usize) {
        view_write!(self, num_control_points_v, value)
    }
    #[inline(always)]
    pub(crate) fn num_control_points_v_raw(&self) -> *mut usize {
        view_raw_mut!(self, num_control_points_v)
    }
    #[inline(always)]
    pub(crate) fn set_control_points(&self, value: List<Vec4>) {
        view_write!(self, control_points, value)
    }
    #[inline(always)]
    pub(crate) fn control_points_raw(&self) -> *mut List<Vec4> {
        view_raw_mut!(self, control_points)
    }
    #[inline(always)]
    pub(crate) fn set_span_subdivision_u(&self, value: u32) {
        view_write!(self, span_subdivision_u, value)
    }
    #[inline(always)]
    pub(crate) fn span_subdivision_u_raw(&self) -> *mut u32 {
        view_raw_mut!(self, span_subdivision_u)
    }
    #[inline(always)]
    pub(crate) fn set_span_subdivision_v(&self, value: u32) {
        view_write!(self, span_subdivision_v, value)
    }
    #[inline(always)]
    pub(crate) fn span_subdivision_v_raw(&self) -> *mut u32 {
        view_raw_mut!(self, span_subdivision_v)
    }
    #[inline(always)]
    pub(crate) fn set_flip_normals(&self, value: bool) {
        view_write!(self, flip_normals, value)
    }
    #[inline(always)]
    pub(crate) fn flip_normals_raw(&self) -> *mut bool {
        view_raw_mut!(self, flip_normals)
    }
    #[inline(always)]
    pub(crate) fn set_material(&self, value: Option<Ref<Material>>) {
        view_write!(self, material, value)
    }
    #[inline(always)]
    pub(crate) fn material_raw(&self) -> *mut Option<Ref<Material>> {
        view_raw_mut!(self, material)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<LineCurve, M> {
    #[inline(always)]
    pub(crate) fn element(&self) -> &View<Element, M> {
        view_project!(self, element)
    }
    #[inline(always)]
    pub(crate) fn element_ptr(&self) -> *const Element {
        view_raw_shared!(self, element)
    }
    #[inline(always)]
    pub(crate) fn color(&self) -> Vec3 {
        view_read_shared!(self, color)
    }
    #[inline(always)]
    pub(crate) fn color_ptr(&self) -> *const Vec3 {
        view_raw_shared!(self, color)
    }
    #[inline(always)]
    pub(crate) fn control_points(&self) -> List<Vec3> {
        view_read_shared!(self, control_points)
    }
    #[inline(always)]
    pub(crate) fn control_points_view(&self) -> &View<List<Vec3>, M> {
        view_project!(self, control_points)
    }
    #[inline(always)]
    pub(crate) fn control_points_ptr(&self) -> *const List<Vec3> {
        view_raw_shared!(self, control_points)
    }
    #[inline(always)]
    pub(crate) fn point_indices(&self) -> List<u32> {
        view_read_shared!(self, point_indices)
    }
    #[inline(always)]
    pub(crate) fn point_indices_view(&self) -> &View<List<u32>, M> {
        view_project!(self, point_indices)
    }
    #[inline(always)]
    pub(crate) fn point_indices_ptr(&self) -> *const List<u32> {
        view_raw_shared!(self, point_indices)
    }
    #[inline(always)]
    pub(crate) fn segments(&self) -> List<LineSegment> {
        view_read_shared!(self, segments)
    }
    #[inline(always)]
    pub(crate) fn segments_view(&self) -> &View<List<LineSegment>, M> {
        view_project!(self, segments)
    }
    #[inline(always)]
    pub(crate) fn segments_ptr(&self) -> *const List<LineSegment> {
        view_raw_shared!(self, segments)
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
impl View<LineCurve, Mut> {
    #[inline(always)]
    pub(crate) fn set_element(&self, value: Element) {
        view_write!(self, element, value)
    }
    #[inline(always)]
    pub(crate) fn element_raw(&self) -> *mut Element {
        view_raw_mut!(self, element)
    }
    #[inline(always)]
    pub(crate) fn set_color(&self, value: Vec3) {
        view_write!(self, color, value)
    }
    #[inline(always)]
    pub(crate) fn color_raw(&self) -> *mut Vec3 {
        view_raw_mut!(self, color)
    }
    #[inline(always)]
    pub(crate) fn set_control_points(&self, value: List<Vec3>) {
        view_write!(self, control_points, value)
    }
    #[inline(always)]
    pub(crate) fn control_points_raw(&self) -> *mut List<Vec3> {
        view_raw_mut!(self, control_points)
    }
    #[inline(always)]
    pub(crate) fn set_point_indices(&self, value: List<u32>) {
        view_write!(self, point_indices, value)
    }
    #[inline(always)]
    pub(crate) fn point_indices_raw(&self) -> *mut List<u32> {
        view_raw_mut!(self, point_indices)
    }
    #[inline(always)]
    pub(crate) fn set_segments(&self, value: List<LineSegment>) {
        view_write!(self, segments, value)
    }
    #[inline(always)]
    pub(crate) fn segments_raw(&self) -> *mut List<LineSegment> {
        view_raw_mut!(self, segments)
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
impl<M: Mode> View<LineSegment, M> {
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
impl View<LineSegment, Mut> {
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
impl<M: Mode> View<LodGroup, M> {
    #[inline(always)]
    pub(crate) fn element(&self) -> &View<Element, M> {
        view_project!(self, element)
    }
    #[inline(always)]
    pub(crate) fn element_ptr(&self) -> *const Element {
        view_raw_shared!(self, element)
    }
    #[inline(always)]
    pub(crate) fn relative_distances(&self) -> bool {
        view_read_shared!(self, relative_distances)
    }
    #[inline(always)]
    pub(crate) fn relative_distances_ptr(&self) -> *const bool {
        view_raw_shared!(self, relative_distances)
    }
    #[inline(always)]
    pub(crate) fn lod_levels(&self) -> List<LodLevel> {
        view_read_shared!(self, lod_levels)
    }
    #[inline(always)]
    pub(crate) fn lod_levels_view(&self) -> &View<List<LodLevel>, M> {
        view_project!(self, lod_levels)
    }
    #[inline(always)]
    pub(crate) fn lod_levels_ptr(&self) -> *const List<LodLevel> {
        view_raw_shared!(self, lod_levels)
    }
    #[inline(always)]
    pub(crate) fn ignore_parent_transform(&self) -> bool {
        view_read_shared!(self, ignore_parent_transform)
    }
    #[inline(always)]
    pub(crate) fn ignore_parent_transform_ptr(&self) -> *const bool {
        view_raw_shared!(self, ignore_parent_transform)
    }
    #[inline(always)]
    pub(crate) fn use_distance_limit(&self) -> bool {
        view_read_shared!(self, use_distance_limit)
    }
    #[inline(always)]
    pub(crate) fn use_distance_limit_ptr(&self) -> *const bool {
        view_raw_shared!(self, use_distance_limit)
    }
    #[inline(always)]
    pub(crate) fn distance_limit_min(&self) -> Real {
        view_read_shared!(self, distance_limit_min)
    }
    #[inline(always)]
    pub(crate) fn distance_limit_min_ptr(&self) -> *const Real {
        view_raw_shared!(self, distance_limit_min)
    }
    #[inline(always)]
    pub(crate) fn distance_limit_max(&self) -> Real {
        view_read_shared!(self, distance_limit_max)
    }
    #[inline(always)]
    pub(crate) fn distance_limit_max_ptr(&self) -> *const Real {
        view_raw_shared!(self, distance_limit_max)
    }
}

#[allow(dead_code)]
impl View<LodGroup, Mut> {
    #[inline(always)]
    pub(crate) fn set_element(&self, value: Element) {
        view_write!(self, element, value)
    }
    #[inline(always)]
    pub(crate) fn element_raw(&self) -> *mut Element {
        view_raw_mut!(self, element)
    }
    #[inline(always)]
    pub(crate) fn set_relative_distances(&self, value: bool) {
        view_write!(self, relative_distances, value)
    }
    #[inline(always)]
    pub(crate) fn relative_distances_raw(&self) -> *mut bool {
        view_raw_mut!(self, relative_distances)
    }
    #[inline(always)]
    pub(crate) fn set_lod_levels(&self, value: List<LodLevel>) {
        view_write!(self, lod_levels, value)
    }
    #[inline(always)]
    pub(crate) fn lod_levels_raw(&self) -> *mut List<LodLevel> {
        view_raw_mut!(self, lod_levels)
    }
    #[inline(always)]
    pub(crate) fn set_ignore_parent_transform(&self, value: bool) {
        view_write!(self, ignore_parent_transform, value)
    }
    #[inline(always)]
    pub(crate) fn ignore_parent_transform_raw(&self) -> *mut bool {
        view_raw_mut!(self, ignore_parent_transform)
    }
    #[inline(always)]
    pub(crate) fn set_use_distance_limit(&self, value: bool) {
        view_write!(self, use_distance_limit, value)
    }
    #[inline(always)]
    pub(crate) fn use_distance_limit_raw(&self) -> *mut bool {
        view_raw_mut!(self, use_distance_limit)
    }
    #[inline(always)]
    pub(crate) fn set_distance_limit_min(&self, value: Real) {
        view_write!(self, distance_limit_min, value)
    }
    #[inline(always)]
    pub(crate) fn distance_limit_min_raw(&self) -> *mut Real {
        view_raw_mut!(self, distance_limit_min)
    }
    #[inline(always)]
    pub(crate) fn set_distance_limit_max(&self, value: Real) {
        view_write!(self, distance_limit_max, value)
    }
    #[inline(always)]
    pub(crate) fn distance_limit_max_raw(&self) -> *mut Real {
        view_raw_mut!(self, distance_limit_max)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<LodLevel, M> {
    #[inline(always)]
    pub(crate) fn distance(&self) -> Real {
        view_read_shared!(self, distance)
    }
    #[inline(always)]
    pub(crate) fn distance_ptr(&self) -> *const Real {
        view_raw_shared!(self, distance)
    }
    #[inline(always)]
    pub(crate) fn display(&self) -> LodDisplay {
        view_read_shared!(self, display)
    }
    #[inline(always)]
    pub(crate) fn display_ptr(&self) -> *const LodDisplay {
        view_raw_shared!(self, display)
    }
}

#[allow(dead_code)]
impl View<LodLevel, Mut> {
    #[inline(always)]
    pub(crate) fn set_distance(&self, value: Real) {
        view_write!(self, distance, value)
    }
    #[inline(always)]
    pub(crate) fn distance_raw(&self) -> *mut Real {
        view_raw_mut!(self, distance)
    }
    #[inline(always)]
    pub(crate) fn set_display(&self, value: LodDisplay) {
        view_write!(self, display, value)
    }
    #[inline(always)]
    pub(crate) fn display_raw(&self) -> *mut LodDisplay {
        view_raw_mut!(self, display)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<Empty, M> {
    #[inline(always)]
    pub(crate) fn element(&self) -> &View<Element, M> {
        view_project!(self, element)
    }
    #[inline(always)]
    pub(crate) fn element_ptr(&self) -> *const Element {
        view_raw_shared!(self, element)
    }
}

#[allow(dead_code)]
impl View<Empty, Mut> {
    #[inline(always)]
    pub(crate) fn set_element(&self, value: Element) {
        view_write!(self, element, value)
    }
    #[inline(always)]
    pub(crate) fn element_raw(&self) -> *mut Element {
        view_raw_mut!(self, element)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<Marker, M> {
    #[inline(always)]
    pub(crate) fn element(&self) -> &View<Element, M> {
        view_project!(self, element)
    }
    #[inline(always)]
    pub(crate) fn element_ptr(&self) -> *const Element {
        view_raw_shared!(self, element)
    }
    #[inline(always)]
    pub(crate) fn type_(&self) -> MarkerType {
        view_read_shared!(self, type_)
    }
    #[inline(always)]
    pub(crate) fn type_ptr(&self) -> *const MarkerType {
        view_raw_shared!(self, type_)
    }
}

#[allow(dead_code)]
impl View<Marker, Mut> {
    #[inline(always)]
    pub(crate) fn set_element(&self, value: Element) {
        view_write!(self, element, value)
    }
    #[inline(always)]
    pub(crate) fn element_raw(&self) -> *mut Element {
        view_raw_mut!(self, element)
    }
    #[inline(always)]
    pub(crate) fn set_type(&self, value: MarkerType) {
        view_write!(self, type_, value)
    }
    #[inline(always)]
    pub(crate) fn type_raw(&self) -> *mut MarkerType {
        view_raw_mut!(self, type_)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<SelectionSet, M> {
    #[inline(always)]
    pub(crate) fn element(&self) -> &View<Element, M> {
        view_project!(self, element)
    }
    #[inline(always)]
    pub(crate) fn element_ptr(&self) -> *const Element {
        view_raw_shared!(self, element)
    }
    #[inline(always)]
    pub(crate) fn nodes(&self) -> RefList<SelectionNode> {
        view_read_shared!(self, nodes)
    }
    #[inline(always)]
    pub(crate) fn nodes_view(&self) -> &View<RefList<SelectionNode>, M> {
        view_project!(self, nodes)
    }
    #[inline(always)]
    pub(crate) fn nodes_ptr(&self) -> *const RefList<SelectionNode> {
        view_raw_shared!(self, nodes)
    }
}

#[allow(dead_code)]
impl View<SelectionSet, Mut> {
    #[inline(always)]
    pub(crate) fn set_element(&self, value: Element) {
        view_write!(self, element, value)
    }
    #[inline(always)]
    pub(crate) fn element_raw(&self) -> *mut Element {
        view_raw_mut!(self, element)
    }
    #[inline(always)]
    pub(crate) fn set_nodes(&self, value: RefList<SelectionNode>) {
        view_write!(self, nodes, value)
    }
    #[inline(always)]
    pub(crate) fn nodes_raw(&self) -> *mut RefList<SelectionNode> {
        view_raw_mut!(self, nodes)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<SelectionNode, M> {
    #[inline(always)]
    pub(crate) fn element(&self) -> &View<Element, M> {
        view_project!(self, element)
    }
    #[inline(always)]
    pub(crate) fn element_ptr(&self) -> *const Element {
        view_raw_shared!(self, element)
    }
    #[inline(always)]
    pub(crate) fn target_node(&self) -> Option<Ref<Node>> {
        view_read_shared!(self, target_node)
    }
    #[inline(always)]
    pub(crate) fn target_node_view(&self) -> Option<&View<Node, M>> {
        self.target_node().map(Ref::view)
    }
    #[inline(always)]
    pub(crate) fn target_node_ptr(&self) -> *const Option<Ref<Node>> {
        view_raw_shared!(self, target_node)
    }
    #[inline(always)]
    pub(crate) fn target_mesh(&self) -> Option<Ref<Mesh>> {
        view_read_shared!(self, target_mesh)
    }
    #[inline(always)]
    pub(crate) fn target_mesh_view(&self) -> Option<&View<Mesh, M>> {
        self.target_mesh().map(Ref::view)
    }
    #[inline(always)]
    pub(crate) fn target_mesh_ptr(&self) -> *const Option<Ref<Mesh>> {
        view_raw_shared!(self, target_mesh)
    }
    #[inline(always)]
    pub(crate) fn include_node(&self) -> bool {
        view_read_shared!(self, include_node)
    }
    #[inline(always)]
    pub(crate) fn include_node_ptr(&self) -> *const bool {
        view_raw_shared!(self, include_node)
    }
    #[inline(always)]
    pub(crate) fn vertices(&self) -> List<u32> {
        view_read_shared!(self, vertices)
    }
    #[inline(always)]
    pub(crate) fn vertices_view(&self) -> &View<List<u32>, M> {
        view_project!(self, vertices)
    }
    #[inline(always)]
    pub(crate) fn vertices_ptr(&self) -> *const List<u32> {
        view_raw_shared!(self, vertices)
    }
    #[inline(always)]
    pub(crate) fn edges(&self) -> List<u32> {
        view_read_shared!(self, edges)
    }
    #[inline(always)]
    pub(crate) fn edges_view(&self) -> &View<List<u32>, M> {
        view_project!(self, edges)
    }
    #[inline(always)]
    pub(crate) fn edges_ptr(&self) -> *const List<u32> {
        view_raw_shared!(self, edges)
    }
    #[inline(always)]
    pub(crate) fn faces(&self) -> List<u32> {
        view_read_shared!(self, faces)
    }
    #[inline(always)]
    pub(crate) fn faces_view(&self) -> &View<List<u32>, M> {
        view_project!(self, faces)
    }
    #[inline(always)]
    pub(crate) fn faces_ptr(&self) -> *const List<u32> {
        view_raw_shared!(self, faces)
    }
}

#[allow(dead_code)]
impl View<SelectionNode, Mut> {
    #[inline(always)]
    pub(crate) fn set_element(&self, value: Element) {
        view_write!(self, element, value)
    }
    #[inline(always)]
    pub(crate) fn element_raw(&self) -> *mut Element {
        view_raw_mut!(self, element)
    }
    #[inline(always)]
    pub(crate) fn set_target_node(&self, value: Option<Ref<Node>>) {
        view_write!(self, target_node, value)
    }
    #[inline(always)]
    pub(crate) fn target_node_raw(&self) -> *mut Option<Ref<Node>> {
        view_raw_mut!(self, target_node)
    }
    #[inline(always)]
    pub(crate) fn set_target_mesh(&self, value: Option<Ref<Mesh>>) {
        view_write!(self, target_mesh, value)
    }
    #[inline(always)]
    pub(crate) fn target_mesh_raw(&self) -> *mut Option<Ref<Mesh>> {
        view_raw_mut!(self, target_mesh)
    }
    #[inline(always)]
    pub(crate) fn set_include_node(&self, value: bool) {
        view_write!(self, include_node, value)
    }
    #[inline(always)]
    pub(crate) fn include_node_raw(&self) -> *mut bool {
        view_raw_mut!(self, include_node)
    }
    #[inline(always)]
    pub(crate) fn set_vertices(&self, value: List<u32>) {
        view_write!(self, vertices, value)
    }
    #[inline(always)]
    pub(crate) fn vertices_raw(&self) -> *mut List<u32> {
        view_raw_mut!(self, vertices)
    }
    #[inline(always)]
    pub(crate) fn set_edges(&self, value: List<u32>) {
        view_write!(self, edges, value)
    }
    #[inline(always)]
    pub(crate) fn edges_raw(&self) -> *mut List<u32> {
        view_raw_mut!(self, edges)
    }
    #[inline(always)]
    pub(crate) fn set_faces(&self, value: List<u32>) {
        view_write!(self, faces, value)
    }
    #[inline(always)]
    pub(crate) fn faces_raw(&self) -> *mut List<u32> {
        view_raw_mut!(self, faces)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<DisplayLayer, M> {
    #[inline(always)]
    pub(crate) fn element(&self) -> &View<Element, M> {
        view_project!(self, element)
    }
    #[inline(always)]
    pub(crate) fn element_ptr(&self) -> *const Element {
        view_raw_shared!(self, element)
    }
    #[inline(always)]
    pub(crate) fn nodes(&self) -> RefList<Node> {
        view_read_shared!(self, nodes)
    }
    #[inline(always)]
    pub(crate) fn nodes_view(&self) -> &View<RefList<Node>, M> {
        view_project!(self, nodes)
    }
    #[inline(always)]
    pub(crate) fn nodes_ptr(&self) -> *const RefList<Node> {
        view_raw_shared!(self, nodes)
    }
    #[inline(always)]
    pub(crate) fn visible(&self) -> bool {
        view_read_shared!(self, visible)
    }
    #[inline(always)]
    pub(crate) fn visible_ptr(&self) -> *const bool {
        view_raw_shared!(self, visible)
    }
    #[inline(always)]
    pub(crate) fn frozen(&self) -> bool {
        view_read_shared!(self, frozen)
    }
    #[inline(always)]
    pub(crate) fn frozen_ptr(&self) -> *const bool {
        view_raw_shared!(self, frozen)
    }
    #[inline(always)]
    pub(crate) fn ui_color(&self) -> Vec3 {
        view_read_shared!(self, ui_color)
    }
    #[inline(always)]
    pub(crate) fn ui_color_ptr(&self) -> *const Vec3 {
        view_raw_shared!(self, ui_color)
    }
}

#[allow(dead_code)]
impl View<DisplayLayer, Mut> {
    #[inline(always)]
    pub(crate) fn set_element(&self, value: Element) {
        view_write!(self, element, value)
    }
    #[inline(always)]
    pub(crate) fn element_raw(&self) -> *mut Element {
        view_raw_mut!(self, element)
    }
    #[inline(always)]
    pub(crate) fn set_nodes(&self, value: RefList<Node>) {
        view_write!(self, nodes, value)
    }
    #[inline(always)]
    pub(crate) fn nodes_raw(&self) -> *mut RefList<Node> {
        view_raw_mut!(self, nodes)
    }
    #[inline(always)]
    pub(crate) fn set_visible(&self, value: bool) {
        view_write!(self, visible, value)
    }
    #[inline(always)]
    pub(crate) fn visible_raw(&self) -> *mut bool {
        view_raw_mut!(self, visible)
    }
    #[inline(always)]
    pub(crate) fn set_frozen(&self, value: bool) {
        view_write!(self, frozen, value)
    }
    #[inline(always)]
    pub(crate) fn frozen_raw(&self) -> *mut bool {
        view_raw_mut!(self, frozen)
    }
    #[inline(always)]
    pub(crate) fn set_ui_color(&self, value: Vec3) {
        view_write!(self, ui_color, value)
    }
    #[inline(always)]
    pub(crate) fn ui_color_raw(&self) -> *mut Vec3 {
        view_raw_mut!(self, ui_color)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<AudioClip, M> {
    #[inline(always)]
    pub(crate) fn element(&self) -> &View<Element, M> {
        view_project!(self, element)
    }
    #[inline(always)]
    pub(crate) fn element_ptr(&self) -> *const Element {
        view_raw_shared!(self, element)
    }
    #[inline(always)]
    pub(crate) fn filename(&self) -> String {
        view_read_shared!(self, filename)
    }
    #[inline(always)]
    pub(crate) fn filename_view(&self) -> &View<String, M> {
        view_project!(self, filename)
    }
    #[inline(always)]
    pub(crate) fn filename_ptr(&self) -> *const String {
        view_raw_shared!(self, filename)
    }
    #[inline(always)]
    pub(crate) fn absolute_filename(&self) -> String {
        view_read_shared!(self, absolute_filename)
    }
    #[inline(always)]
    pub(crate) fn absolute_filename_view(&self) -> &View<String, M> {
        view_project!(self, absolute_filename)
    }
    #[inline(always)]
    pub(crate) fn absolute_filename_ptr(&self) -> *const String {
        view_raw_shared!(self, absolute_filename)
    }
    #[inline(always)]
    pub(crate) fn relative_filename(&self) -> String {
        view_read_shared!(self, relative_filename)
    }
    #[inline(always)]
    pub(crate) fn relative_filename_view(&self) -> &View<String, M> {
        view_project!(self, relative_filename)
    }
    #[inline(always)]
    pub(crate) fn relative_filename_ptr(&self) -> *const String {
        view_raw_shared!(self, relative_filename)
    }
    #[inline(always)]
    pub(crate) fn raw_filename(&self) -> Blob {
        view_read_shared!(self, raw_filename)
    }
    #[inline(always)]
    pub(crate) fn raw_filename_ptr(&self) -> *const Blob {
        view_raw_shared!(self, raw_filename)
    }
    #[inline(always)]
    pub(crate) fn raw_absolute_filename(&self) -> Blob {
        view_read_shared!(self, raw_absolute_filename)
    }
    #[inline(always)]
    pub(crate) fn raw_absolute_filename_ptr(&self) -> *const Blob {
        view_raw_shared!(self, raw_absolute_filename)
    }
    #[inline(always)]
    pub(crate) fn raw_relative_filename(&self) -> Blob {
        view_read_shared!(self, raw_relative_filename)
    }
    #[inline(always)]
    pub(crate) fn raw_relative_filename_ptr(&self) -> *const Blob {
        view_raw_shared!(self, raw_relative_filename)
    }
    #[inline(always)]
    pub(crate) fn content(&self) -> Blob {
        view_read_shared!(self, content)
    }
    #[inline(always)]
    pub(crate) fn content_ptr(&self) -> *const Blob {
        view_raw_shared!(self, content)
    }
}

#[allow(dead_code)]
impl View<AudioClip, Mut> {
    #[inline(always)]
    pub(crate) fn set_element(&self, value: Element) {
        view_write!(self, element, value)
    }
    #[inline(always)]
    pub(crate) fn element_raw(&self) -> *mut Element {
        view_raw_mut!(self, element)
    }
    #[inline(always)]
    pub(crate) fn set_filename(&self, value: String) {
        view_write!(self, filename, value)
    }
    #[inline(always)]
    pub(crate) fn filename_raw(&self) -> *mut String {
        view_raw_mut!(self, filename)
    }
    #[inline(always)]
    pub(crate) fn set_absolute_filename(&self, value: String) {
        view_write!(self, absolute_filename, value)
    }
    #[inline(always)]
    pub(crate) fn absolute_filename_raw(&self) -> *mut String {
        view_raw_mut!(self, absolute_filename)
    }
    #[inline(always)]
    pub(crate) fn set_relative_filename(&self, value: String) {
        view_write!(self, relative_filename, value)
    }
    #[inline(always)]
    pub(crate) fn relative_filename_raw(&self) -> *mut String {
        view_raw_mut!(self, relative_filename)
    }
    #[inline(always)]
    pub(crate) fn set_raw_filename(&self, value: Blob) {
        view_write!(self, raw_filename, value)
    }
    #[inline(always)]
    pub(crate) fn raw_filename_raw(&self) -> *mut Blob {
        view_raw_mut!(self, raw_filename)
    }
    #[inline(always)]
    pub(crate) fn set_raw_absolute_filename(&self, value: Blob) {
        view_write!(self, raw_absolute_filename, value)
    }
    #[inline(always)]
    pub(crate) fn raw_absolute_filename_raw(&self) -> *mut Blob {
        view_raw_mut!(self, raw_absolute_filename)
    }
    #[inline(always)]
    pub(crate) fn set_raw_relative_filename(&self, value: Blob) {
        view_write!(self, raw_relative_filename, value)
    }
    #[inline(always)]
    pub(crate) fn raw_relative_filename_raw(&self) -> *mut Blob {
        view_raw_mut!(self, raw_relative_filename)
    }
    #[inline(always)]
    pub(crate) fn set_content(&self, value: Blob) {
        view_write!(self, content, value)
    }
    #[inline(always)]
    pub(crate) fn content_raw(&self) -> *mut Blob {
        view_raw_mut!(self, content)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<AudioLayer, M> {
    #[inline(always)]
    pub(crate) fn element(&self) -> &View<Element, M> {
        view_project!(self, element)
    }
    #[inline(always)]
    pub(crate) fn element_ptr(&self) -> *const Element {
        view_raw_shared!(self, element)
    }
    #[inline(always)]
    pub(crate) fn clips(&self) -> RefList<AudioClip> {
        view_read_shared!(self, clips)
    }
    #[inline(always)]
    pub(crate) fn clips_view(&self) -> &View<RefList<AudioClip>, M> {
        view_project!(self, clips)
    }
    #[inline(always)]
    pub(crate) fn clips_ptr(&self) -> *const RefList<AudioClip> {
        view_raw_shared!(self, clips)
    }
}

#[allow(dead_code)]
impl View<AudioLayer, Mut> {
    #[inline(always)]
    pub(crate) fn set_element(&self, value: Element) {
        view_write!(self, element, value)
    }
    #[inline(always)]
    pub(crate) fn element_raw(&self) -> *mut Element {
        view_raw_mut!(self, element)
    }
    #[inline(always)]
    pub(crate) fn set_clips(&self, value: RefList<AudioClip>) {
        view_write!(self, clips, value)
    }
    #[inline(always)]
    pub(crate) fn clips_raw(&self) -> *mut RefList<AudioClip> {
        view_raw_mut!(self, clips)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<Character, M> {
    #[inline(always)]
    pub(crate) fn element(&self) -> &View<Element, M> {
        view_project!(self, element)
    }
    #[inline(always)]
    pub(crate) fn element_ptr(&self) -> *const Element {
        view_raw_shared!(self, element)
    }
}

#[allow(dead_code)]
impl View<Character, Mut> {
    #[inline(always)]
    pub(crate) fn set_element(&self, value: Element) {
        view_write!(self, element, value)
    }
    #[inline(always)]
    pub(crate) fn element_raw(&self) -> *mut Element {
        view_raw_mut!(self, element)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<StereoCamera, M> {
    #[inline(always)]
    pub(crate) fn element(&self) -> &View<Element, M> {
        view_project!(self, element)
    }
    #[inline(always)]
    pub(crate) fn element_ptr(&self) -> *const Element {
        view_raw_shared!(self, element)
    }
    #[inline(always)]
    pub(crate) fn left(&self) -> Option<Ref<Camera>> {
        view_read_shared!(self, left)
    }
    #[inline(always)]
    pub(crate) fn left_view(&self) -> Option<&View<Camera, M>> {
        self.left().map(Ref::view)
    }
    #[inline(always)]
    pub(crate) fn left_ptr(&self) -> *const Option<Ref<Camera>> {
        view_raw_shared!(self, left)
    }
    #[inline(always)]
    pub(crate) fn right(&self) -> Option<Ref<Camera>> {
        view_read_shared!(self, right)
    }
    #[inline(always)]
    pub(crate) fn right_view(&self) -> Option<&View<Camera, M>> {
        self.right().map(Ref::view)
    }
    #[inline(always)]
    pub(crate) fn right_ptr(&self) -> *const Option<Ref<Camera>> {
        view_raw_shared!(self, right)
    }
}

#[allow(dead_code)]
impl View<StereoCamera, Mut> {
    #[inline(always)]
    pub(crate) fn set_element(&self, value: Element) {
        view_write!(self, element, value)
    }
    #[inline(always)]
    pub(crate) fn element_raw(&self) -> *mut Element {
        view_raw_mut!(self, element)
    }
    #[inline(always)]
    pub(crate) fn set_left(&self, value: Option<Ref<Camera>>) {
        view_write!(self, left, value)
    }
    #[inline(always)]
    pub(crate) fn left_raw(&self) -> *mut Option<Ref<Camera>> {
        view_raw_mut!(self, left)
    }
    #[inline(always)]
    pub(crate) fn set_right(&self, value: Option<Ref<Camera>>) {
        view_write!(self, right, value)
    }
    #[inline(always)]
    pub(crate) fn right_raw(&self) -> *mut Option<Ref<Camera>> {
        view_raw_mut!(self, right)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<CameraSwitcher, M> {
    #[inline(always)]
    pub(crate) fn element(&self) -> &View<Element, M> {
        view_project!(self, element)
    }
    #[inline(always)]
    pub(crate) fn element_ptr(&self) -> *const Element {
        view_raw_shared!(self, element)
    }
}

#[allow(dead_code)]
impl View<CameraSwitcher, Mut> {
    #[inline(always)]
    pub(crate) fn set_element(&self, value: Element) {
        view_write!(self, element, value)
    }
    #[inline(always)]
    pub(crate) fn element_raw(&self) -> *mut Element {
        view_raw_mut!(self, element)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<Unknown, M> {
    #[inline(always)]
    pub(crate) fn element(&self) -> &View<Element, M> {
        view_project!(self, element)
    }
    #[inline(always)]
    pub(crate) fn element_ptr(&self) -> *const Element {
        view_raw_shared!(self, element)
    }
    #[inline(always)]
    pub(crate) fn type_(&self) -> String {
        view_read_shared!(self, type_)
    }
    #[inline(always)]
    pub(crate) fn type_view(&self) -> &View<String, M> {
        view_project!(self, type_)
    }
    #[inline(always)]
    pub(crate) fn type_ptr(&self) -> *const String {
        view_raw_shared!(self, type_)
    }
    #[inline(always)]
    pub(crate) fn super_type(&self) -> String {
        view_read_shared!(self, super_type)
    }
    #[inline(always)]
    pub(crate) fn super_type_view(&self) -> &View<String, M> {
        view_project!(self, super_type)
    }
    #[inline(always)]
    pub(crate) fn super_type_ptr(&self) -> *const String {
        view_raw_shared!(self, super_type)
    }
    #[inline(always)]
    pub(crate) fn sub_type(&self) -> String {
        view_read_shared!(self, sub_type)
    }
    #[inline(always)]
    pub(crate) fn sub_type_view(&self) -> &View<String, M> {
        view_project!(self, sub_type)
    }
    #[inline(always)]
    pub(crate) fn sub_type_ptr(&self) -> *const String {
        view_raw_shared!(self, sub_type)
    }
}

#[allow(dead_code)]
impl View<Unknown, Mut> {
    #[inline(always)]
    pub(crate) fn set_element(&self, value: Element) {
        view_write!(self, element, value)
    }
    #[inline(always)]
    pub(crate) fn element_raw(&self) -> *mut Element {
        view_raw_mut!(self, element)
    }
    #[inline(always)]
    pub(crate) fn set_type(&self, value: String) {
        view_write!(self, type_, value)
    }
    #[inline(always)]
    pub(crate) fn type_raw(&self) -> *mut String {
        view_raw_mut!(self, type_)
    }
    #[inline(always)]
    pub(crate) fn set_super_type(&self, value: String) {
        view_write!(self, super_type, value)
    }
    #[inline(always)]
    pub(crate) fn super_type_raw(&self) -> *mut String {
        view_raw_mut!(self, super_type)
    }
    #[inline(always)]
    pub(crate) fn set_sub_type(&self, value: String) {
        view_write!(self, sub_type, value)
    }
    #[inline(always)]
    pub(crate) fn sub_type_raw(&self) -> *mut String {
        view_raw_mut!(self, sub_type)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<MetadataObject, M> {
    #[inline(always)]
    pub(crate) fn element(&self) -> &View<Element, M> {
        view_project!(self, element)
    }
    #[inline(always)]
    pub(crate) fn element_ptr(&self) -> *const Element {
        view_raw_shared!(self, element)
    }
}

#[allow(dead_code)]
impl View<MetadataObject, Mut> {
    #[inline(always)]
    pub(crate) fn set_element(&self, value: Element) {
        view_write!(self, element, value)
    }
    #[inline(always)]
    pub(crate) fn element_raw(&self) -> *mut Element {
        view_raw_mut!(self, element)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<ProceduralGeometry, M> {
    #[inline(always)]
    pub(crate) fn element(&self) -> &View<Element, M> {
        view_project!(self, element)
    }
    #[inline(always)]
    pub(crate) fn element_ptr(&self) -> *const Element {
        view_raw_shared!(self, element)
    }
}

#[allow(dead_code)]
impl View<ProceduralGeometry, Mut> {
    #[inline(always)]
    pub(crate) fn set_element(&self, value: Element) {
        view_write!(self, element, value)
    }
    #[inline(always)]
    pub(crate) fn element_raw(&self) -> *mut Element {
        view_raw_mut!(self, element)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<NurbsTrimSurface, M> {
    #[inline(always)]
    pub(crate) fn element(&self) -> &View<Element, M> {
        view_project!(self, element)
    }
    #[inline(always)]
    pub(crate) fn element_ptr(&self) -> *const Element {
        view_raw_shared!(self, element)
    }
}

#[allow(dead_code)]
impl View<NurbsTrimSurface, Mut> {
    #[inline(always)]
    pub(crate) fn set_element(&self, value: Element) {
        view_write!(self, element, value)
    }
    #[inline(always)]
    pub(crate) fn element_raw(&self) -> *mut Element {
        view_raw_mut!(self, element)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<NurbsTrimBoundary, M> {
    #[inline(always)]
    pub(crate) fn element(&self) -> &View<Element, M> {
        view_project!(self, element)
    }
    #[inline(always)]
    pub(crate) fn element_ptr(&self) -> *const Element {
        view_raw_shared!(self, element)
    }
}

#[allow(dead_code)]
impl View<NurbsTrimBoundary, Mut> {
    #[inline(always)]
    pub(crate) fn set_element(&self, value: Element) {
        view_write!(self, element, value)
    }
    #[inline(always)]
    pub(crate) fn element_raw(&self) -> *mut Element {
        view_raw_mut!(self, element)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<SkinVertex, M> {
    #[inline(always)]
    pub(crate) fn weight_begin(&self) -> u32 {
        view_read_shared!(self, weight_begin)
    }
    #[inline(always)]
    pub(crate) fn weight_begin_ptr(&self) -> *const u32 {
        view_raw_shared!(self, weight_begin)
    }
    #[inline(always)]
    pub(crate) fn num_weights(&self) -> u32 {
        view_read_shared!(self, num_weights)
    }
    #[inline(always)]
    pub(crate) fn num_weights_ptr(&self) -> *const u32 {
        view_raw_shared!(self, num_weights)
    }
    #[inline(always)]
    pub(crate) fn dq_weight(&self) -> Real {
        view_read_shared!(self, dq_weight)
    }
    #[inline(always)]
    pub(crate) fn dq_weight_ptr(&self) -> *const Real {
        view_raw_shared!(self, dq_weight)
    }
}

#[allow(dead_code)]
impl View<SkinVertex, Mut> {
    #[inline(always)]
    pub(crate) fn set_weight_begin(&self, value: u32) {
        view_write!(self, weight_begin, value)
    }
    #[inline(always)]
    pub(crate) fn weight_begin_raw(&self) -> *mut u32 {
        view_raw_mut!(self, weight_begin)
    }
    #[inline(always)]
    pub(crate) fn set_num_weights(&self, value: u32) {
        view_write!(self, num_weights, value)
    }
    #[inline(always)]
    pub(crate) fn num_weights_raw(&self) -> *mut u32 {
        view_raw_mut!(self, num_weights)
    }
    #[inline(always)]
    pub(crate) fn set_dq_weight(&self, value: Real) {
        view_write!(self, dq_weight, value)
    }
    #[inline(always)]
    pub(crate) fn dq_weight_raw(&self) -> *mut Real {
        view_raw_mut!(self, dq_weight)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<SkinWeight, M> {
    #[inline(always)]
    pub(crate) fn cluster_index(&self) -> u32 {
        view_read_shared!(self, cluster_index)
    }
    #[inline(always)]
    pub(crate) fn cluster_index_ptr(&self) -> *const u32 {
        view_raw_shared!(self, cluster_index)
    }
    #[inline(always)]
    pub(crate) fn weight(&self) -> Real {
        view_read_shared!(self, weight)
    }
    #[inline(always)]
    pub(crate) fn weight_ptr(&self) -> *const Real {
        view_raw_shared!(self, weight)
    }
}

#[allow(dead_code)]
impl View<SkinWeight, Mut> {
    #[inline(always)]
    pub(crate) fn set_cluster_index(&self, value: u32) {
        view_write!(self, cluster_index, value)
    }
    #[inline(always)]
    pub(crate) fn cluster_index_raw(&self) -> *mut u32 {
        view_raw_mut!(self, cluster_index)
    }
    #[inline(always)]
    pub(crate) fn set_weight(&self, value: Real) {
        view_write!(self, weight, value)
    }
    #[inline(always)]
    pub(crate) fn weight_raw(&self) -> *mut Real {
        view_raw_mut!(self, weight)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<BakedNode, M> {
    #[inline(always)]
    pub(crate) fn typed_id(&self) -> u32 {
        view_read_shared!(self, typed_id)
    }
    #[inline(always)]
    pub(crate) fn typed_id_ptr(&self) -> *const u32 {
        view_raw_shared!(self, typed_id)
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
    pub(crate) fn constant_translation(&self) -> bool {
        view_read_shared!(self, constant_translation)
    }
    #[inline(always)]
    pub(crate) fn constant_translation_ptr(&self) -> *const bool {
        view_raw_shared!(self, constant_translation)
    }
    #[inline(always)]
    pub(crate) fn constant_rotation(&self) -> bool {
        view_read_shared!(self, constant_rotation)
    }
    #[inline(always)]
    pub(crate) fn constant_rotation_ptr(&self) -> *const bool {
        view_raw_shared!(self, constant_rotation)
    }
    #[inline(always)]
    pub(crate) fn constant_scale(&self) -> bool {
        view_read_shared!(self, constant_scale)
    }
    #[inline(always)]
    pub(crate) fn constant_scale_ptr(&self) -> *const bool {
        view_raw_shared!(self, constant_scale)
    }
    #[inline(always)]
    pub(crate) fn translation_keys(&self) -> List<BakedVec3> {
        view_read_shared!(self, translation_keys)
    }
    #[inline(always)]
    pub(crate) fn translation_keys_view(&self) -> &View<List<BakedVec3>, M> {
        view_project!(self, translation_keys)
    }
    #[inline(always)]
    pub(crate) fn translation_keys_ptr(&self) -> *const List<BakedVec3> {
        view_raw_shared!(self, translation_keys)
    }
    #[inline(always)]
    pub(crate) fn rotation_keys(&self) -> List<BakedQuat> {
        view_read_shared!(self, rotation_keys)
    }
    #[inline(always)]
    pub(crate) fn rotation_keys_view(&self) -> &View<List<BakedQuat>, M> {
        view_project!(self, rotation_keys)
    }
    #[inline(always)]
    pub(crate) fn rotation_keys_ptr(&self) -> *const List<BakedQuat> {
        view_raw_shared!(self, rotation_keys)
    }
    #[inline(always)]
    pub(crate) fn scale_keys(&self) -> List<BakedVec3> {
        view_read_shared!(self, scale_keys)
    }
    #[inline(always)]
    pub(crate) fn scale_keys_view(&self) -> &View<List<BakedVec3>, M> {
        view_project!(self, scale_keys)
    }
    #[inline(always)]
    pub(crate) fn scale_keys_ptr(&self) -> *const List<BakedVec3> {
        view_raw_shared!(self, scale_keys)
    }
}

#[allow(dead_code)]
impl View<BakedNode, Mut> {
    #[inline(always)]
    pub(crate) fn set_typed_id(&self, value: u32) {
        view_write!(self, typed_id, value)
    }
    #[inline(always)]
    pub(crate) fn typed_id_raw(&self) -> *mut u32 {
        view_raw_mut!(self, typed_id)
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
    pub(crate) fn set_constant_translation(&self, value: bool) {
        view_write!(self, constant_translation, value)
    }
    #[inline(always)]
    pub(crate) fn constant_translation_raw(&self) -> *mut bool {
        view_raw_mut!(self, constant_translation)
    }
    #[inline(always)]
    pub(crate) fn set_constant_rotation(&self, value: bool) {
        view_write!(self, constant_rotation, value)
    }
    #[inline(always)]
    pub(crate) fn constant_rotation_raw(&self) -> *mut bool {
        view_raw_mut!(self, constant_rotation)
    }
    #[inline(always)]
    pub(crate) fn set_constant_scale(&self, value: bool) {
        view_write!(self, constant_scale, value)
    }
    #[inline(always)]
    pub(crate) fn constant_scale_raw(&self) -> *mut bool {
        view_raw_mut!(self, constant_scale)
    }
    #[inline(always)]
    pub(crate) fn set_translation_keys(&self, value: List<BakedVec3>) {
        view_write!(self, translation_keys, value)
    }
    #[inline(always)]
    pub(crate) fn translation_keys_raw(&self) -> *mut List<BakedVec3> {
        view_raw_mut!(self, translation_keys)
    }
    #[inline(always)]
    pub(crate) fn set_rotation_keys(&self, value: List<BakedQuat>) {
        view_write!(self, rotation_keys, value)
    }
    #[inline(always)]
    pub(crate) fn rotation_keys_raw(&self) -> *mut List<BakedQuat> {
        view_raw_mut!(self, rotation_keys)
    }
    #[inline(always)]
    pub(crate) fn set_scale_keys(&self, value: List<BakedVec3>) {
        view_write!(self, scale_keys, value)
    }
    #[inline(always)]
    pub(crate) fn scale_keys_raw(&self) -> *mut List<BakedVec3> {
        view_raw_mut!(self, scale_keys)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<BakedElement, M> {
    #[inline(always)]
    pub(crate) fn element_id(&self) -> u32 {
        view_read_shared!(self, element_id)
    }
    #[inline(always)]
    pub(crate) fn element_id_ptr(&self) -> *const u32 {
        view_raw_shared!(self, element_id)
    }
    #[inline(always)]
    pub(crate) fn props(&self) -> List<BakedProp> {
        view_read_shared!(self, props)
    }
    #[inline(always)]
    pub(crate) fn props_view(&self) -> &View<List<BakedProp>, M> {
        view_project!(self, props)
    }
    #[inline(always)]
    pub(crate) fn props_ptr(&self) -> *const List<BakedProp> {
        view_raw_shared!(self, props)
    }
}

#[allow(dead_code)]
impl View<BakedElement, Mut> {
    #[inline(always)]
    pub(crate) fn set_element_id(&self, value: u32) {
        view_write!(self, element_id, value)
    }
    #[inline(always)]
    pub(crate) fn element_id_raw(&self) -> *mut u32 {
        view_raw_mut!(self, element_id)
    }
    #[inline(always)]
    pub(crate) fn set_props(&self, value: List<BakedProp>) {
        view_write!(self, props, value)
    }
    #[inline(always)]
    pub(crate) fn props_raw(&self) -> *mut List<BakedProp> {
        view_raw_mut!(self, props)
    }
}

#[allow(dead_code)]
impl<M: Mode> View<BakedProp, M> {
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
    pub(crate) fn constant_value(&self) -> bool {
        view_read_shared!(self, constant_value)
    }
    #[inline(always)]
    pub(crate) fn constant_value_ptr(&self) -> *const bool {
        view_raw_shared!(self, constant_value)
    }
    #[inline(always)]
    pub(crate) fn keys(&self) -> List<BakedVec3> {
        view_read_shared!(self, keys)
    }
    #[inline(always)]
    pub(crate) fn keys_view(&self) -> &View<List<BakedVec3>, M> {
        view_project!(self, keys)
    }
    #[inline(always)]
    pub(crate) fn keys_ptr(&self) -> *const List<BakedVec3> {
        view_raw_shared!(self, keys)
    }
}

#[allow(dead_code)]
impl View<BakedProp, Mut> {
    #[inline(always)]
    pub(crate) fn set_name(&self, value: String) {
        view_write!(self, name, value)
    }
    #[inline(always)]
    pub(crate) fn name_raw(&self) -> *mut String {
        view_raw_mut!(self, name)
    }
    #[inline(always)]
    pub(crate) fn set_constant_value(&self, value: bool) {
        view_write!(self, constant_value, value)
    }
    #[inline(always)]
    pub(crate) fn constant_value_raw(&self) -> *mut bool {
        view_raw_mut!(self, constant_value)
    }
    #[inline(always)]
    pub(crate) fn set_keys(&self, value: List<BakedVec3>) {
        view_write!(self, keys, value)
    }
    #[inline(always)]
    pub(crate) fn keys_raw(&self) -> *mut List<BakedVec3> {
        view_raw_mut!(self, keys)
    }
}
