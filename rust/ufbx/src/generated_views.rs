// GENERATED FILE — do not edit by hand. Produced by rust/regen.sh from
// ufbx.h via bindgen/ufbx_ir.py + rust/ufbx/bindgen/generate_rust.py.
// Fixes belong in the GENERATOR (see PORTING.md); hand edits are
// silently overwritten on the next regeneration and CI diffs this file.
//
// Crate-internal `View<T, M>` field accessors over the generated public
// structs (`view_accessor_structs` in generate_rust.py): a by-value read
// per leaf field, an in-place `&View` projection per aggregate field, and
// `Mut`-only setters / raw field pointers. Soundness model (mint vouch,
// `Mut`/`Const` provenance): src/native/view.rs.

use crate::generated::*;
use crate::native::view::{Mode, Mut, View};
use crate::prelude::*;
use std::ptr;

#[allow(dead_code)]
impl<M: Mode> View<Element, M> {
    #[inline(always)]
    pub(crate) fn name(&self) -> String {
        // SAFETY: by-value read of the `name` field; the viewed `Element`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).name) }
    }
    #[inline(always)]
    pub(crate) fn props(&self) -> &View<Props, M> {
        // SAFETY: in-place projection of the `props` field; liveness and
        // `M`-adequate provenance carry over from this view's own mint.
        unsafe { View::mint((&raw const (*self.as_ptr()).props).cast_mut()) }
    }
    #[inline(always)]
    pub(crate) fn element_id(&self) -> u32 {
        // SAFETY: by-value read of the `element_id` field; the viewed `Element`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).element_id) }
    }
    #[inline(always)]
    pub(crate) fn typed_id(&self) -> u32 {
        // SAFETY: by-value read of the `typed_id` field; the viewed `Element`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).typed_id) }
    }
    #[inline(always)]
    pub(crate) fn instances(&self) -> RefList<Node> {
        // SAFETY: by-value read of the `instances` field; the viewed `Element`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).instances) }
    }
    #[inline(always)]
    pub(crate) fn type_(&self) -> ElementType {
        // SAFETY: by-value read of the `type_` field; the viewed `Element`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).type_) }
    }
    #[inline(always)]
    pub(crate) fn connections_src(&self) -> List<Connection> {
        // SAFETY: by-value read of the `connections_src` field; the viewed `Element`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).connections_src) }
    }
    #[inline(always)]
    pub(crate) fn connections_dst(&self) -> List<Connection> {
        // SAFETY: by-value read of the `connections_dst` field; the viewed `Element`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).connections_dst) }
    }
    #[inline(always)]
    pub(crate) fn dom_node(&self) -> Option<Ref<DomNode>> {
        // SAFETY: by-value read of the `dom_node` field; the viewed `Element`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).dom_node) }
    }
    #[inline(always)]
    pub(crate) fn scene(&self) -> Ref<Scene> {
        // SAFETY: by-value read of the `scene` field; the viewed `Element`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).scene) }
    }
}

#[allow(dead_code)]
impl View<Element, Mut> {
    #[inline(always)]
    pub(crate) fn set_name(&self, value: String) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).name = value }
    }
    #[inline(always)]
    pub(crate) fn name_raw(&self) -> *mut String {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).name }
    }
    #[inline(always)]
    pub(crate) fn set_props(&self, value: Props) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).props = value }
    }
    #[inline(always)]
    pub(crate) fn props_raw(&self) -> *mut Props {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).props }
    }
    #[inline(always)]
    pub(crate) fn set_element_id(&self, value: u32) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).element_id = value }
    }
    #[inline(always)]
    pub(crate) fn element_id_raw(&self) -> *mut u32 {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).element_id }
    }
    #[inline(always)]
    pub(crate) fn set_typed_id(&self, value: u32) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).typed_id = value }
    }
    #[inline(always)]
    pub(crate) fn typed_id_raw(&self) -> *mut u32 {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).typed_id }
    }
    #[inline(always)]
    pub(crate) fn set_instances(&self, value: RefList<Node>) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).instances = value }
    }
    #[inline(always)]
    pub(crate) fn instances_raw(&self) -> *mut RefList<Node> {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).instances }
    }
    #[inline(always)]
    pub(crate) fn set_type(&self, value: ElementType) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).type_ = value }
    }
    #[inline(always)]
    pub(crate) fn type_raw(&self) -> *mut ElementType {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).type_ }
    }
    #[inline(always)]
    pub(crate) fn set_connections_src(&self, value: List<Connection>) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).connections_src = value }
    }
    #[inline(always)]
    pub(crate) fn connections_src_raw(&self) -> *mut List<Connection> {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).connections_src }
    }
    #[inline(always)]
    pub(crate) fn set_connections_dst(&self, value: List<Connection>) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).connections_dst = value }
    }
    #[inline(always)]
    pub(crate) fn connections_dst_raw(&self) -> *mut List<Connection> {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).connections_dst }
    }
    #[inline(always)]
    pub(crate) fn set_dom_node(&self, value: Option<Ref<DomNode>>) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).dom_node = value }
    }
    #[inline(always)]
    pub(crate) fn dom_node_raw(&self) -> *mut Option<Ref<DomNode>> {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).dom_node }
    }
    #[inline(always)]
    pub(crate) fn set_scene(&self, value: Ref<Scene>) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).scene = value }
    }
    #[inline(always)]
    pub(crate) fn scene_raw(&self) -> *mut Ref<Scene> {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).scene }
    }
}

#[allow(dead_code)]
impl<M: Mode> View<Props, M> {
    #[inline(always)]
    pub(crate) fn props(&self) -> List<Prop> {
        // SAFETY: by-value read of the `props` field; the viewed `Props`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).props) }
    }
    #[inline(always)]
    pub(crate) fn num_animated(&self) -> usize {
        // SAFETY: by-value read of the `num_animated` field; the viewed `Props`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).num_animated) }
    }
}

#[allow(dead_code)]
impl View<Props, Mut> {
    #[inline(always)]
    pub(crate) fn set_props(&self, value: List<Prop>) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).props = value }
    }
    #[inline(always)]
    pub(crate) fn props_raw(&self) -> *mut List<Prop> {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).props }
    }
    #[inline(always)]
    pub(crate) fn set_num_animated(&self, value: usize) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).num_animated = value }
    }
    #[inline(always)]
    pub(crate) fn num_animated_raw(&self) -> *mut usize {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).num_animated }
    }
    #[inline(always)]
    pub(crate) fn set_defaults(&self, value: Option<Ref<Props>>) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).defaults = value }
    }
    #[inline(always)]
    pub(crate) fn defaults_raw(&self) -> *mut Option<Ref<Props>> {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).defaults }
    }
}

#[allow(dead_code)]
impl<M: Mode> View<Prop, M> {
    #[inline(always)]
    pub(crate) fn name(&self) -> String {
        // SAFETY: by-value read of the `name` field; the viewed `Prop`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).name) }
    }
    #[inline(always)]
    pub(crate) fn _internal_key(&self) -> u32 {
        // SAFETY: by-value read of the `_internal_key` field; the viewed `Prop`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr())._internal_key) }
    }
    #[inline(always)]
    pub(crate) fn type_(&self) -> PropType {
        // SAFETY: by-value read of the `type_` field; the viewed `Prop`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).type_) }
    }
    #[inline(always)]
    pub(crate) fn flags(&self) -> PropFlags {
        // SAFETY: by-value read of the `flags` field; the viewed `Prop`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).flags) }
    }
    #[inline(always)]
    pub(crate) fn value_str(&self) -> String {
        // SAFETY: by-value read of the `value_str` field; the viewed `Prop`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).value_str) }
    }
    #[inline(always)]
    pub(crate) fn value_blob(&self) -> Blob {
        // SAFETY: by-value read of the `value_blob` field; the viewed `Prop`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).value_blob) }
    }
    #[inline(always)]
    pub(crate) fn value_int(&self) -> i64 {
        // SAFETY: by-value read of the `value_int` field; the viewed `Prop`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).value_int) }
    }
    #[inline(always)]
    pub(crate) fn value_vec4(&self) -> Vec4 {
        // SAFETY: by-value read of the `value_vec4` field; the viewed `Prop`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).value_vec4) }
    }
}

#[allow(dead_code)]
impl View<Prop, Mut> {
    #[inline(always)]
    pub(crate) fn set_name(&self, value: String) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).name = value }
    }
    #[inline(always)]
    pub(crate) fn name_raw(&self) -> *mut String {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).name }
    }
    #[inline(always)]
    pub(crate) fn set_internal_key(&self, value: u32) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get())._internal_key = value }
    }
    #[inline(always)]
    pub(crate) fn internal_key_raw(&self) -> *mut u32 {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get())._internal_key }
    }
    #[inline(always)]
    pub(crate) fn set_type(&self, value: PropType) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).type_ = value }
    }
    #[inline(always)]
    pub(crate) fn type_raw(&self) -> *mut PropType {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).type_ }
    }
    #[inline(always)]
    pub(crate) fn set_flags(&self, value: PropFlags) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).flags = value }
    }
    #[inline(always)]
    pub(crate) fn flags_raw(&self) -> *mut PropFlags {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).flags }
    }
    #[inline(always)]
    pub(crate) fn set_value_str(&self, value: String) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).value_str = value }
    }
    #[inline(always)]
    pub(crate) fn value_str_raw(&self) -> *mut String {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).value_str }
    }
    #[inline(always)]
    pub(crate) fn set_value_blob(&self, value: Blob) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).value_blob = value }
    }
    #[inline(always)]
    pub(crate) fn value_blob_raw(&self) -> *mut Blob {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).value_blob }
    }
    #[inline(always)]
    pub(crate) fn set_value_int(&self, value: i64) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).value_int = value }
    }
    #[inline(always)]
    pub(crate) fn value_int_raw(&self) -> *mut i64 {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).value_int }
    }
    #[inline(always)]
    pub(crate) fn set_value_vec4(&self, value: Vec4) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).value_vec4 = value }
    }
    #[inline(always)]
    pub(crate) fn value_vec4_raw(&self) -> *mut Vec4 {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).value_vec4 }
    }
}

#[allow(dead_code)]
impl<M: Mode> View<Face, M> {
    #[inline(always)]
    pub(crate) fn index_begin(&self) -> u32 {
        // SAFETY: by-value read of the `index_begin` field; the viewed `Face`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).index_begin) }
    }
    #[inline(always)]
    pub(crate) fn num_indices(&self) -> u32 {
        // SAFETY: by-value read of the `num_indices` field; the viewed `Face`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).num_indices) }
    }
}

#[allow(dead_code)]
impl View<Face, Mut> {
    #[inline(always)]
    pub(crate) fn set_index_begin(&self, value: u32) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).index_begin = value }
    }
    #[inline(always)]
    pub(crate) fn index_begin_raw(&self) -> *mut u32 {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).index_begin }
    }
    #[inline(always)]
    pub(crate) fn set_num_indices(&self, value: u32) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).num_indices = value }
    }
    #[inline(always)]
    pub(crate) fn num_indices_raw(&self) -> *mut u32 {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).num_indices }
    }
}

#[allow(dead_code)]
impl<M: Mode> View<Edge, M> {
    #[inline(always)]
    pub(crate) fn a(&self) -> u32 {
        // SAFETY: by-value read of the `a` field; the viewed `Edge`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).a) }
    }
    #[inline(always)]
    pub(crate) fn b(&self) -> u32 {
        // SAFETY: by-value read of the `b` field; the viewed `Edge`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).b) }
    }
}

#[allow(dead_code)]
impl View<Edge, Mut> {
    #[inline(always)]
    pub(crate) fn set_a(&self, value: u32) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).a = value }
    }
    #[inline(always)]
    pub(crate) fn a_raw(&self) -> *mut u32 {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).a }
    }
    #[inline(always)]
    pub(crate) fn set_b(&self, value: u32) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).b = value }
    }
    #[inline(always)]
    pub(crate) fn b_raw(&self) -> *mut u32 {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).b }
    }
}

#[allow(dead_code)]
impl<M: Mode> View<VertexAttrib, M> {
    #[inline(always)]
    pub(crate) fn exists(&self) -> bool {
        // SAFETY: by-value read of the `exists` field; the viewed `VertexAttrib`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).exists) }
    }
    #[inline(always)]
    pub(crate) fn values(&self) -> VoidList {
        // SAFETY: by-value read of the `values` field; the viewed `VertexAttrib`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).values) }
    }
    #[inline(always)]
    pub(crate) fn indices(&self) -> List<u32> {
        // SAFETY: by-value read of the `indices` field; the viewed `VertexAttrib`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).indices) }
    }
    #[inline(always)]
    pub(crate) fn value_reals(&self) -> usize {
        // SAFETY: by-value read of the `value_reals` field; the viewed `VertexAttrib`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).value_reals) }
    }
    #[inline(always)]
    pub(crate) fn unique_per_vertex(&self) -> bool {
        // SAFETY: by-value read of the `unique_per_vertex` field; the viewed `VertexAttrib`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).unique_per_vertex) }
    }
    #[inline(always)]
    pub(crate) fn values_w(&self) -> List<Real> {
        // SAFETY: by-value read of the `values_w` field; the viewed `VertexAttrib`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).values_w) }
    }
}

#[allow(dead_code)]
impl View<VertexAttrib, Mut> {
    #[inline(always)]
    pub(crate) fn set_exists(&self, value: bool) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).exists = value }
    }
    #[inline(always)]
    pub(crate) fn exists_raw(&self) -> *mut bool {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).exists }
    }
    #[inline(always)]
    pub(crate) fn set_values(&self, value: VoidList) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).values = value }
    }
    #[inline(always)]
    pub(crate) fn values_raw(&self) -> *mut VoidList {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).values }
    }
    #[inline(always)]
    pub(crate) fn set_indices(&self, value: List<u32>) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).indices = value }
    }
    #[inline(always)]
    pub(crate) fn indices_raw(&self) -> *mut List<u32> {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).indices }
    }
    #[inline(always)]
    pub(crate) fn set_value_reals(&self, value: usize) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).value_reals = value }
    }
    #[inline(always)]
    pub(crate) fn value_reals_raw(&self) -> *mut usize {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).value_reals }
    }
    #[inline(always)]
    pub(crate) fn set_unique_per_vertex(&self, value: bool) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).unique_per_vertex = value }
    }
    #[inline(always)]
    pub(crate) fn unique_per_vertex_raw(&self) -> *mut bool {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).unique_per_vertex }
    }
    #[inline(always)]
    pub(crate) fn set_values_w(&self, value: List<Real>) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).values_w = value }
    }
    #[inline(always)]
    pub(crate) fn values_w_raw(&self) -> *mut List<Real> {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).values_w }
    }
}

#[allow(dead_code)]
impl<M: Mode> View<VertexReal, M> {
    #[inline(always)]
    pub(crate) fn exists(&self) -> bool {
        // SAFETY: by-value read of the `exists` field; the viewed `VertexReal`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).exists) }
    }
    #[inline(always)]
    pub(crate) fn values(&self) -> List<Real> {
        // SAFETY: by-value read of the `values` field; the viewed `VertexReal`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).values) }
    }
    #[inline(always)]
    pub(crate) fn indices(&self) -> List<u32> {
        // SAFETY: by-value read of the `indices` field; the viewed `VertexReal`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).indices) }
    }
    #[inline(always)]
    pub(crate) fn value_reals(&self) -> usize {
        // SAFETY: by-value read of the `value_reals` field; the viewed `VertexReal`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).value_reals) }
    }
    #[inline(always)]
    pub(crate) fn unique_per_vertex(&self) -> bool {
        // SAFETY: by-value read of the `unique_per_vertex` field; the viewed `VertexReal`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).unique_per_vertex) }
    }
    #[inline(always)]
    pub(crate) fn values_w(&self) -> List<Real> {
        // SAFETY: by-value read of the `values_w` field; the viewed `VertexReal`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).values_w) }
    }
}

#[allow(dead_code)]
impl View<VertexReal, Mut> {
    #[inline(always)]
    pub(crate) fn set_exists(&self, value: bool) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).exists = value }
    }
    #[inline(always)]
    pub(crate) fn exists_raw(&self) -> *mut bool {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).exists }
    }
    #[inline(always)]
    pub(crate) fn set_values(&self, value: List<Real>) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).values = value }
    }
    #[inline(always)]
    pub(crate) fn values_raw(&self) -> *mut List<Real> {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).values }
    }
    #[inline(always)]
    pub(crate) fn set_indices(&self, value: List<u32>) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).indices = value }
    }
    #[inline(always)]
    pub(crate) fn indices_raw(&self) -> *mut List<u32> {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).indices }
    }
    #[inline(always)]
    pub(crate) fn set_value_reals(&self, value: usize) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).value_reals = value }
    }
    #[inline(always)]
    pub(crate) fn value_reals_raw(&self) -> *mut usize {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).value_reals }
    }
    #[inline(always)]
    pub(crate) fn set_unique_per_vertex(&self, value: bool) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).unique_per_vertex = value }
    }
    #[inline(always)]
    pub(crate) fn unique_per_vertex_raw(&self) -> *mut bool {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).unique_per_vertex }
    }
    #[inline(always)]
    pub(crate) fn set_values_w(&self, value: List<Real>) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).values_w = value }
    }
    #[inline(always)]
    pub(crate) fn values_w_raw(&self) -> *mut List<Real> {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).values_w }
    }
}

#[allow(dead_code)]
impl<M: Mode> View<VertexVec2, M> {
    #[inline(always)]
    pub(crate) fn exists(&self) -> bool {
        // SAFETY: by-value read of the `exists` field; the viewed `VertexVec2`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).exists) }
    }
    #[inline(always)]
    pub(crate) fn values(&self) -> List<Vec2> {
        // SAFETY: by-value read of the `values` field; the viewed `VertexVec2`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).values) }
    }
    #[inline(always)]
    pub(crate) fn indices(&self) -> List<u32> {
        // SAFETY: by-value read of the `indices` field; the viewed `VertexVec2`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).indices) }
    }
    #[inline(always)]
    pub(crate) fn value_reals(&self) -> usize {
        // SAFETY: by-value read of the `value_reals` field; the viewed `VertexVec2`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).value_reals) }
    }
    #[inline(always)]
    pub(crate) fn unique_per_vertex(&self) -> bool {
        // SAFETY: by-value read of the `unique_per_vertex` field; the viewed `VertexVec2`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).unique_per_vertex) }
    }
    #[inline(always)]
    pub(crate) fn values_w(&self) -> List<Real> {
        // SAFETY: by-value read of the `values_w` field; the viewed `VertexVec2`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).values_w) }
    }
}

#[allow(dead_code)]
impl View<VertexVec2, Mut> {
    #[inline(always)]
    pub(crate) fn set_exists(&self, value: bool) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).exists = value }
    }
    #[inline(always)]
    pub(crate) fn exists_raw(&self) -> *mut bool {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).exists }
    }
    #[inline(always)]
    pub(crate) fn set_values(&self, value: List<Vec2>) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).values = value }
    }
    #[inline(always)]
    pub(crate) fn values_raw(&self) -> *mut List<Vec2> {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).values }
    }
    #[inline(always)]
    pub(crate) fn set_indices(&self, value: List<u32>) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).indices = value }
    }
    #[inline(always)]
    pub(crate) fn indices_raw(&self) -> *mut List<u32> {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).indices }
    }
    #[inline(always)]
    pub(crate) fn set_value_reals(&self, value: usize) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).value_reals = value }
    }
    #[inline(always)]
    pub(crate) fn value_reals_raw(&self) -> *mut usize {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).value_reals }
    }
    #[inline(always)]
    pub(crate) fn set_unique_per_vertex(&self, value: bool) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).unique_per_vertex = value }
    }
    #[inline(always)]
    pub(crate) fn unique_per_vertex_raw(&self) -> *mut bool {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).unique_per_vertex }
    }
    #[inline(always)]
    pub(crate) fn set_values_w(&self, value: List<Real>) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).values_w = value }
    }
    #[inline(always)]
    pub(crate) fn values_w_raw(&self) -> *mut List<Real> {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).values_w }
    }
}

#[allow(dead_code)]
impl<M: Mode> View<VertexVec3, M> {
    #[inline(always)]
    pub(crate) fn exists(&self) -> bool {
        // SAFETY: by-value read of the `exists` field; the viewed `VertexVec3`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).exists) }
    }
    #[inline(always)]
    pub(crate) fn values(&self) -> List<Vec3> {
        // SAFETY: by-value read of the `values` field; the viewed `VertexVec3`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).values) }
    }
    #[inline(always)]
    pub(crate) fn indices(&self) -> List<u32> {
        // SAFETY: by-value read of the `indices` field; the viewed `VertexVec3`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).indices) }
    }
    #[inline(always)]
    pub(crate) fn value_reals(&self) -> usize {
        // SAFETY: by-value read of the `value_reals` field; the viewed `VertexVec3`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).value_reals) }
    }
    #[inline(always)]
    pub(crate) fn unique_per_vertex(&self) -> bool {
        // SAFETY: by-value read of the `unique_per_vertex` field; the viewed `VertexVec3`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).unique_per_vertex) }
    }
    #[inline(always)]
    pub(crate) fn values_w(&self) -> List<Real> {
        // SAFETY: by-value read of the `values_w` field; the viewed `VertexVec3`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).values_w) }
    }
}

#[allow(dead_code)]
impl View<VertexVec3, Mut> {
    #[inline(always)]
    pub(crate) fn set_exists(&self, value: bool) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).exists = value }
    }
    #[inline(always)]
    pub(crate) fn exists_raw(&self) -> *mut bool {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).exists }
    }
    #[inline(always)]
    pub(crate) fn set_values(&self, value: List<Vec3>) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).values = value }
    }
    #[inline(always)]
    pub(crate) fn values_raw(&self) -> *mut List<Vec3> {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).values }
    }
    #[inline(always)]
    pub(crate) fn set_indices(&self, value: List<u32>) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).indices = value }
    }
    #[inline(always)]
    pub(crate) fn indices_raw(&self) -> *mut List<u32> {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).indices }
    }
    #[inline(always)]
    pub(crate) fn set_value_reals(&self, value: usize) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).value_reals = value }
    }
    #[inline(always)]
    pub(crate) fn value_reals_raw(&self) -> *mut usize {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).value_reals }
    }
    #[inline(always)]
    pub(crate) fn set_unique_per_vertex(&self, value: bool) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).unique_per_vertex = value }
    }
    #[inline(always)]
    pub(crate) fn unique_per_vertex_raw(&self) -> *mut bool {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).unique_per_vertex }
    }
    #[inline(always)]
    pub(crate) fn set_values_w(&self, value: List<Real>) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).values_w = value }
    }
    #[inline(always)]
    pub(crate) fn values_w_raw(&self) -> *mut List<Real> {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).values_w }
    }
}

#[allow(dead_code)]
impl<M: Mode> View<VertexVec4, M> {
    #[inline(always)]
    pub(crate) fn exists(&self) -> bool {
        // SAFETY: by-value read of the `exists` field; the viewed `VertexVec4`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).exists) }
    }
    #[inline(always)]
    pub(crate) fn values(&self) -> List<Vec4> {
        // SAFETY: by-value read of the `values` field; the viewed `VertexVec4`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).values) }
    }
    #[inline(always)]
    pub(crate) fn indices(&self) -> List<u32> {
        // SAFETY: by-value read of the `indices` field; the viewed `VertexVec4`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).indices) }
    }
    #[inline(always)]
    pub(crate) fn value_reals(&self) -> usize {
        // SAFETY: by-value read of the `value_reals` field; the viewed `VertexVec4`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).value_reals) }
    }
    #[inline(always)]
    pub(crate) fn unique_per_vertex(&self) -> bool {
        // SAFETY: by-value read of the `unique_per_vertex` field; the viewed `VertexVec4`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).unique_per_vertex) }
    }
    #[inline(always)]
    pub(crate) fn values_w(&self) -> List<Real> {
        // SAFETY: by-value read of the `values_w` field; the viewed `VertexVec4`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).values_w) }
    }
}

#[allow(dead_code)]
impl View<VertexVec4, Mut> {
    #[inline(always)]
    pub(crate) fn set_exists(&self, value: bool) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).exists = value }
    }
    #[inline(always)]
    pub(crate) fn exists_raw(&self) -> *mut bool {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).exists }
    }
    #[inline(always)]
    pub(crate) fn set_values(&self, value: List<Vec4>) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).values = value }
    }
    #[inline(always)]
    pub(crate) fn values_raw(&self) -> *mut List<Vec4> {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).values }
    }
    #[inline(always)]
    pub(crate) fn set_indices(&self, value: List<u32>) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).indices = value }
    }
    #[inline(always)]
    pub(crate) fn indices_raw(&self) -> *mut List<u32> {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).indices }
    }
    #[inline(always)]
    pub(crate) fn set_value_reals(&self, value: usize) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).value_reals = value }
    }
    #[inline(always)]
    pub(crate) fn value_reals_raw(&self) -> *mut usize {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).value_reals }
    }
    #[inline(always)]
    pub(crate) fn set_unique_per_vertex(&self, value: bool) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).unique_per_vertex = value }
    }
    #[inline(always)]
    pub(crate) fn unique_per_vertex_raw(&self) -> *mut bool {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).unique_per_vertex }
    }
    #[inline(always)]
    pub(crate) fn set_values_w(&self, value: List<Real>) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).values_w = value }
    }
    #[inline(always)]
    pub(crate) fn values_w_raw(&self) -> *mut List<Real> {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).values_w }
    }
}

#[allow(dead_code)]
impl<M: Mode> View<UvSet, M> {
    #[inline(always)]
    pub(crate) fn name(&self) -> String {
        // SAFETY: by-value read of the `name` field; the viewed `UvSet`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).name) }
    }
    #[inline(always)]
    pub(crate) fn index(&self) -> u32 {
        // SAFETY: by-value read of the `index` field; the viewed `UvSet`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).index) }
    }
    #[inline(always)]
    pub(crate) fn vertex_uv(&self) -> &View<VertexVec2, M> {
        // SAFETY: in-place projection of the `vertex_uv` field; liveness and
        // `M`-adequate provenance carry over from this view's own mint.
        unsafe { View::mint((&raw const (*self.as_ptr()).vertex_uv).cast_mut()) }
    }
    #[inline(always)]
    pub(crate) fn vertex_tangent(&self) -> &View<VertexVec3, M> {
        // SAFETY: in-place projection of the `vertex_tangent` field; liveness and
        // `M`-adequate provenance carry over from this view's own mint.
        unsafe { View::mint((&raw const (*self.as_ptr()).vertex_tangent).cast_mut()) }
    }
    #[inline(always)]
    pub(crate) fn vertex_bitangent(&self) -> &View<VertexVec3, M> {
        // SAFETY: in-place projection of the `vertex_bitangent` field; liveness and
        // `M`-adequate provenance carry over from this view's own mint.
        unsafe { View::mint((&raw const (*self.as_ptr()).vertex_bitangent).cast_mut()) }
    }
}

#[allow(dead_code)]
impl View<UvSet, Mut> {
    #[inline(always)]
    pub(crate) fn set_name(&self, value: String) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).name = value }
    }
    #[inline(always)]
    pub(crate) fn name_raw(&self) -> *mut String {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).name }
    }
    #[inline(always)]
    pub(crate) fn set_index(&self, value: u32) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).index = value }
    }
    #[inline(always)]
    pub(crate) fn index_raw(&self) -> *mut u32 {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).index }
    }
    #[inline(always)]
    pub(crate) fn set_vertex_uv(&self, value: VertexVec2) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).vertex_uv = value }
    }
    #[inline(always)]
    pub(crate) fn vertex_uv_raw(&self) -> *mut VertexVec2 {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).vertex_uv }
    }
    #[inline(always)]
    pub(crate) fn set_vertex_tangent(&self, value: VertexVec3) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).vertex_tangent = value }
    }
    #[inline(always)]
    pub(crate) fn vertex_tangent_raw(&self) -> *mut VertexVec3 {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).vertex_tangent }
    }
    #[inline(always)]
    pub(crate) fn set_vertex_bitangent(&self, value: VertexVec3) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).vertex_bitangent = value }
    }
    #[inline(always)]
    pub(crate) fn vertex_bitangent_raw(&self) -> *mut VertexVec3 {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).vertex_bitangent }
    }
}

#[allow(dead_code)]
impl<M: Mode> View<ColorSet, M> {
    #[inline(always)]
    pub(crate) fn name(&self) -> String {
        // SAFETY: by-value read of the `name` field; the viewed `ColorSet`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).name) }
    }
    #[inline(always)]
    pub(crate) fn index(&self) -> u32 {
        // SAFETY: by-value read of the `index` field; the viewed `ColorSet`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).index) }
    }
    #[inline(always)]
    pub(crate) fn vertex_color(&self) -> &View<VertexVec4, M> {
        // SAFETY: in-place projection of the `vertex_color` field; liveness and
        // `M`-adequate provenance carry over from this view's own mint.
        unsafe { View::mint((&raw const (*self.as_ptr()).vertex_color).cast_mut()) }
    }
}

#[allow(dead_code)]
impl View<ColorSet, Mut> {
    #[inline(always)]
    pub(crate) fn set_name(&self, value: String) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).name = value }
    }
    #[inline(always)]
    pub(crate) fn name_raw(&self) -> *mut String {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).name }
    }
    #[inline(always)]
    pub(crate) fn set_index(&self, value: u32) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).index = value }
    }
    #[inline(always)]
    pub(crate) fn index_raw(&self) -> *mut u32 {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).index }
    }
    #[inline(always)]
    pub(crate) fn set_vertex_color(&self, value: VertexVec4) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).vertex_color = value }
    }
    #[inline(always)]
    pub(crate) fn vertex_color_raw(&self) -> *mut VertexVec4 {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).vertex_color }
    }
}

#[allow(dead_code)]
impl<M: Mode> View<MeshPart, M> {
    #[inline(always)]
    pub(crate) fn index(&self) -> u32 {
        // SAFETY: by-value read of the `index` field; the viewed `MeshPart`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).index) }
    }
    #[inline(always)]
    pub(crate) fn num_faces(&self) -> usize {
        // SAFETY: by-value read of the `num_faces` field; the viewed `MeshPart`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).num_faces) }
    }
    #[inline(always)]
    pub(crate) fn num_triangles(&self) -> usize {
        // SAFETY: by-value read of the `num_triangles` field; the viewed `MeshPart`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).num_triangles) }
    }
    #[inline(always)]
    pub(crate) fn num_empty_faces(&self) -> usize {
        // SAFETY: by-value read of the `num_empty_faces` field; the viewed `MeshPart`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).num_empty_faces) }
    }
    #[inline(always)]
    pub(crate) fn num_point_faces(&self) -> usize {
        // SAFETY: by-value read of the `num_point_faces` field; the viewed `MeshPart`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).num_point_faces) }
    }
    #[inline(always)]
    pub(crate) fn num_line_faces(&self) -> usize {
        // SAFETY: by-value read of the `num_line_faces` field; the viewed `MeshPart`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).num_line_faces) }
    }
    #[inline(always)]
    pub(crate) fn face_indices(&self) -> List<u32> {
        // SAFETY: by-value read of the `face_indices` field; the viewed `MeshPart`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).face_indices) }
    }
}

#[allow(dead_code)]
impl View<MeshPart, Mut> {
    #[inline(always)]
    pub(crate) fn set_index(&self, value: u32) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).index = value }
    }
    #[inline(always)]
    pub(crate) fn index_raw(&self) -> *mut u32 {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).index }
    }
    #[inline(always)]
    pub(crate) fn set_num_faces(&self, value: usize) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).num_faces = value }
    }
    #[inline(always)]
    pub(crate) fn num_faces_raw(&self) -> *mut usize {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).num_faces }
    }
    #[inline(always)]
    pub(crate) fn set_num_triangles(&self, value: usize) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).num_triangles = value }
    }
    #[inline(always)]
    pub(crate) fn num_triangles_raw(&self) -> *mut usize {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).num_triangles }
    }
    #[inline(always)]
    pub(crate) fn set_num_empty_faces(&self, value: usize) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).num_empty_faces = value }
    }
    #[inline(always)]
    pub(crate) fn num_empty_faces_raw(&self) -> *mut usize {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).num_empty_faces }
    }
    #[inline(always)]
    pub(crate) fn set_num_point_faces(&self, value: usize) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).num_point_faces = value }
    }
    #[inline(always)]
    pub(crate) fn num_point_faces_raw(&self) -> *mut usize {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).num_point_faces }
    }
    #[inline(always)]
    pub(crate) fn set_num_line_faces(&self, value: usize) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).num_line_faces = value }
    }
    #[inline(always)]
    pub(crate) fn num_line_faces_raw(&self) -> *mut usize {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).num_line_faces }
    }
    #[inline(always)]
    pub(crate) fn set_face_indices(&self, value: List<u32>) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).face_indices = value }
    }
    #[inline(always)]
    pub(crate) fn face_indices_raw(&self) -> *mut List<u32> {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).face_indices }
    }
}

#[allow(dead_code)]
impl<M: Mode> View<FaceGroup, M> {
    #[inline(always)]
    pub(crate) fn id(&self) -> i32 {
        // SAFETY: by-value read of the `id` field; the viewed `FaceGroup`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).id) }
    }
    #[inline(always)]
    pub(crate) fn name(&self) -> String {
        // SAFETY: by-value read of the `name` field; the viewed `FaceGroup`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).name) }
    }
}

#[allow(dead_code)]
impl View<FaceGroup, Mut> {
    #[inline(always)]
    pub(crate) fn set_id(&self, value: i32) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).id = value }
    }
    #[inline(always)]
    pub(crate) fn id_raw(&self) -> *mut i32 {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).id }
    }
    #[inline(always)]
    pub(crate) fn set_name(&self, value: String) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).name = value }
    }
    #[inline(always)]
    pub(crate) fn name_raw(&self) -> *mut String {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).name }
    }
}

#[allow(dead_code)]
impl<M: Mode> View<Mesh, M> {
    #[inline(always)]
    pub(crate) fn element(&self) -> &View<Element, M> {
        // SAFETY: in-place projection of the `element` field; liveness and
        // `M`-adequate provenance carry over from this view's own mint.
        unsafe { View::mint((&raw const (*self.as_ptr()).element).cast_mut()) }
    }
    #[inline(always)]
    pub(crate) fn num_vertices(&self) -> usize {
        // SAFETY: by-value read of the `num_vertices` field; the viewed `Mesh`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).num_vertices) }
    }
    #[inline(always)]
    pub(crate) fn num_indices(&self) -> usize {
        // SAFETY: by-value read of the `num_indices` field; the viewed `Mesh`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).num_indices) }
    }
    #[inline(always)]
    pub(crate) fn num_faces(&self) -> usize {
        // SAFETY: by-value read of the `num_faces` field; the viewed `Mesh`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).num_faces) }
    }
    #[inline(always)]
    pub(crate) fn num_triangles(&self) -> usize {
        // SAFETY: by-value read of the `num_triangles` field; the viewed `Mesh`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).num_triangles) }
    }
    #[inline(always)]
    pub(crate) fn num_edges(&self) -> usize {
        // SAFETY: by-value read of the `num_edges` field; the viewed `Mesh`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).num_edges) }
    }
    #[inline(always)]
    pub(crate) fn max_face_triangles(&self) -> usize {
        // SAFETY: by-value read of the `max_face_triangles` field; the viewed `Mesh`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).max_face_triangles) }
    }
    #[inline(always)]
    pub(crate) fn num_empty_faces(&self) -> usize {
        // SAFETY: by-value read of the `num_empty_faces` field; the viewed `Mesh`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).num_empty_faces) }
    }
    #[inline(always)]
    pub(crate) fn num_point_faces(&self) -> usize {
        // SAFETY: by-value read of the `num_point_faces` field; the viewed `Mesh`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).num_point_faces) }
    }
    #[inline(always)]
    pub(crate) fn num_line_faces(&self) -> usize {
        // SAFETY: by-value read of the `num_line_faces` field; the viewed `Mesh`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).num_line_faces) }
    }
    #[inline(always)]
    pub(crate) fn faces(&self) -> List<Face> {
        // SAFETY: by-value read of the `faces` field; the viewed `Mesh`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).faces) }
    }
    #[inline(always)]
    pub(crate) fn face_smoothing(&self) -> List<bool> {
        // SAFETY: by-value read of the `face_smoothing` field; the viewed `Mesh`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).face_smoothing) }
    }
    #[inline(always)]
    pub(crate) fn face_material(&self) -> List<u32> {
        // SAFETY: by-value read of the `face_material` field; the viewed `Mesh`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).face_material) }
    }
    #[inline(always)]
    pub(crate) fn face_group(&self) -> List<u32> {
        // SAFETY: by-value read of the `face_group` field; the viewed `Mesh`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).face_group) }
    }
    #[inline(always)]
    pub(crate) fn face_hole(&self) -> List<bool> {
        // SAFETY: by-value read of the `face_hole` field; the viewed `Mesh`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).face_hole) }
    }
    #[inline(always)]
    pub(crate) fn edges(&self) -> List<Edge> {
        // SAFETY: by-value read of the `edges` field; the viewed `Mesh`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).edges) }
    }
    #[inline(always)]
    pub(crate) fn edge_smoothing(&self) -> List<bool> {
        // SAFETY: by-value read of the `edge_smoothing` field; the viewed `Mesh`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).edge_smoothing) }
    }
    #[inline(always)]
    pub(crate) fn edge_crease(&self) -> List<Real> {
        // SAFETY: by-value read of the `edge_crease` field; the viewed `Mesh`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).edge_crease) }
    }
    #[inline(always)]
    pub(crate) fn edge_visibility(&self) -> List<bool> {
        // SAFETY: by-value read of the `edge_visibility` field; the viewed `Mesh`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).edge_visibility) }
    }
    #[inline(always)]
    pub(crate) fn vertex_indices(&self) -> List<u32> {
        // SAFETY: by-value read of the `vertex_indices` field; the viewed `Mesh`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).vertex_indices) }
    }
    #[inline(always)]
    pub(crate) fn vertices(&self) -> List<Vec3> {
        // SAFETY: by-value read of the `vertices` field; the viewed `Mesh`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).vertices) }
    }
    #[inline(always)]
    pub(crate) fn vertex_first_index(&self) -> List<u32> {
        // SAFETY: by-value read of the `vertex_first_index` field; the viewed `Mesh`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).vertex_first_index) }
    }
    #[inline(always)]
    pub(crate) fn vertex_position(&self) -> &View<VertexVec3, M> {
        // SAFETY: in-place projection of the `vertex_position` field; liveness and
        // `M`-adequate provenance carry over from this view's own mint.
        unsafe { View::mint((&raw const (*self.as_ptr()).vertex_position).cast_mut()) }
    }
    #[inline(always)]
    pub(crate) fn vertex_normal(&self) -> &View<VertexVec3, M> {
        // SAFETY: in-place projection of the `vertex_normal` field; liveness and
        // `M`-adequate provenance carry over from this view's own mint.
        unsafe { View::mint((&raw const (*self.as_ptr()).vertex_normal).cast_mut()) }
    }
    #[inline(always)]
    pub(crate) fn vertex_uv(&self) -> &View<VertexVec2, M> {
        // SAFETY: in-place projection of the `vertex_uv` field; liveness and
        // `M`-adequate provenance carry over from this view's own mint.
        unsafe { View::mint((&raw const (*self.as_ptr()).vertex_uv).cast_mut()) }
    }
    #[inline(always)]
    pub(crate) fn vertex_tangent(&self) -> &View<VertexVec3, M> {
        // SAFETY: in-place projection of the `vertex_tangent` field; liveness and
        // `M`-adequate provenance carry over from this view's own mint.
        unsafe { View::mint((&raw const (*self.as_ptr()).vertex_tangent).cast_mut()) }
    }
    #[inline(always)]
    pub(crate) fn vertex_bitangent(&self) -> &View<VertexVec3, M> {
        // SAFETY: in-place projection of the `vertex_bitangent` field; liveness and
        // `M`-adequate provenance carry over from this view's own mint.
        unsafe { View::mint((&raw const (*self.as_ptr()).vertex_bitangent).cast_mut()) }
    }
    #[inline(always)]
    pub(crate) fn vertex_color(&self) -> &View<VertexVec4, M> {
        // SAFETY: in-place projection of the `vertex_color` field; liveness and
        // `M`-adequate provenance carry over from this view's own mint.
        unsafe { View::mint((&raw const (*self.as_ptr()).vertex_color).cast_mut()) }
    }
    #[inline(always)]
    pub(crate) fn vertex_crease(&self) -> &View<VertexReal, M> {
        // SAFETY: in-place projection of the `vertex_crease` field; liveness and
        // `M`-adequate provenance carry over from this view's own mint.
        unsafe { View::mint((&raw const (*self.as_ptr()).vertex_crease).cast_mut()) }
    }
    #[inline(always)]
    pub(crate) fn uv_sets(&self) -> List<UvSet> {
        // SAFETY: by-value read of the `uv_sets` field; the viewed `Mesh`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).uv_sets) }
    }
    #[inline(always)]
    pub(crate) fn color_sets(&self) -> List<ColorSet> {
        // SAFETY: by-value read of the `color_sets` field; the viewed `Mesh`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).color_sets) }
    }
    #[inline(always)]
    pub(crate) fn materials(&self) -> RefList<Material> {
        // SAFETY: by-value read of the `materials` field; the viewed `Mesh`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).materials) }
    }
    #[inline(always)]
    pub(crate) fn face_groups(&self) -> List<FaceGroup> {
        // SAFETY: by-value read of the `face_groups` field; the viewed `Mesh`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).face_groups) }
    }
    #[inline(always)]
    pub(crate) fn material_parts(&self) -> List<MeshPart> {
        // SAFETY: by-value read of the `material_parts` field; the viewed `Mesh`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).material_parts) }
    }
    #[inline(always)]
    pub(crate) fn face_group_parts(&self) -> List<MeshPart> {
        // SAFETY: by-value read of the `face_group_parts` field; the viewed `Mesh`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).face_group_parts) }
    }
    #[inline(always)]
    pub(crate) fn material_part_usage_order(&self) -> List<u32> {
        // SAFETY: by-value read of the `material_part_usage_order` field; the viewed `Mesh`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).material_part_usage_order) }
    }
    #[inline(always)]
    pub(crate) fn skinned_is_local(&self) -> bool {
        // SAFETY: by-value read of the `skinned_is_local` field; the viewed `Mesh`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).skinned_is_local) }
    }
    #[inline(always)]
    pub(crate) fn skinned_position(&self) -> &View<VertexVec3, M> {
        // SAFETY: in-place projection of the `skinned_position` field; liveness and
        // `M`-adequate provenance carry over from this view's own mint.
        unsafe { View::mint((&raw const (*self.as_ptr()).skinned_position).cast_mut()) }
    }
    #[inline(always)]
    pub(crate) fn skinned_normal(&self) -> &View<VertexVec3, M> {
        // SAFETY: in-place projection of the `skinned_normal` field; liveness and
        // `M`-adequate provenance carry over from this view's own mint.
        unsafe { View::mint((&raw const (*self.as_ptr()).skinned_normal).cast_mut()) }
    }
    #[inline(always)]
    pub(crate) fn skin_deformers(&self) -> RefList<SkinDeformer> {
        // SAFETY: by-value read of the `skin_deformers` field; the viewed `Mesh`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).skin_deformers) }
    }
    #[inline(always)]
    pub(crate) fn blend_deformers(&self) -> RefList<BlendDeformer> {
        // SAFETY: by-value read of the `blend_deformers` field; the viewed `Mesh`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).blend_deformers) }
    }
    #[inline(always)]
    pub(crate) fn cache_deformers(&self) -> RefList<CacheDeformer> {
        // SAFETY: by-value read of the `cache_deformers` field; the viewed `Mesh`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).cache_deformers) }
    }
    #[inline(always)]
    pub(crate) fn all_deformers(&self) -> RefList<Element> {
        // SAFETY: by-value read of the `all_deformers` field; the viewed `Mesh`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).all_deformers) }
    }
    #[inline(always)]
    pub(crate) fn subdivision_preview_levels(&self) -> u32 {
        // SAFETY: by-value read of the `subdivision_preview_levels` field; the viewed `Mesh`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).subdivision_preview_levels) }
    }
    #[inline(always)]
    pub(crate) fn subdivision_render_levels(&self) -> u32 {
        // SAFETY: by-value read of the `subdivision_render_levels` field; the viewed `Mesh`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).subdivision_render_levels) }
    }
    #[inline(always)]
    pub(crate) fn subdivision_display_mode(&self) -> SubdivisionDisplayMode {
        // SAFETY: by-value read of the `subdivision_display_mode` field; the viewed `Mesh`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).subdivision_display_mode) }
    }
    #[inline(always)]
    pub(crate) fn subdivision_boundary(&self) -> SubdivisionBoundary {
        // SAFETY: by-value read of the `subdivision_boundary` field; the viewed `Mesh`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).subdivision_boundary) }
    }
    #[inline(always)]
    pub(crate) fn subdivision_uv_boundary(&self) -> SubdivisionBoundary {
        // SAFETY: by-value read of the `subdivision_uv_boundary` field; the viewed `Mesh`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).subdivision_uv_boundary) }
    }
    #[inline(always)]
    pub(crate) fn reversed_winding(&self) -> bool {
        // SAFETY: by-value read of the `reversed_winding` field; the viewed `Mesh`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).reversed_winding) }
    }
    #[inline(always)]
    pub(crate) fn generated_normals(&self) -> bool {
        // SAFETY: by-value read of the `generated_normals` field; the viewed `Mesh`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).generated_normals) }
    }
    #[inline(always)]
    pub(crate) fn subdivision_evaluated(&self) -> bool {
        // SAFETY: by-value read of the `subdivision_evaluated` field; the viewed `Mesh`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).subdivision_evaluated) }
    }
    #[inline(always)]
    pub(crate) fn subdivision_result(&self) -> Option<Ref<SubdivisionResult>> {
        // SAFETY: by-value read of the `subdivision_result` field; the viewed `Mesh`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).subdivision_result) }
    }
    #[allow(clippy::wrong_self_convention)]
    #[inline(always)]
    pub(crate) fn from_tessellated_nurbs(&self) -> bool {
        // SAFETY: by-value read of the `from_tessellated_nurbs` field; the viewed `Mesh`
        // is valid and live per this view's mint vouch.
        unsafe { ptr::read(&raw const (*self.as_ptr()).from_tessellated_nurbs) }
    }
}

#[allow(dead_code)]
impl View<Mesh, Mut> {
    #[inline(always)]
    pub(crate) fn set_element(&self, value: Element) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).element = value }
    }
    #[inline(always)]
    pub(crate) fn element_raw(&self) -> *mut Element {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).element }
    }
    #[inline(always)]
    pub(crate) fn set_num_vertices(&self, value: usize) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).num_vertices = value }
    }
    #[inline(always)]
    pub(crate) fn num_vertices_raw(&self) -> *mut usize {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).num_vertices }
    }
    #[inline(always)]
    pub(crate) fn set_num_indices(&self, value: usize) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).num_indices = value }
    }
    #[inline(always)]
    pub(crate) fn num_indices_raw(&self) -> *mut usize {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).num_indices }
    }
    #[inline(always)]
    pub(crate) fn set_num_faces(&self, value: usize) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).num_faces = value }
    }
    #[inline(always)]
    pub(crate) fn num_faces_raw(&self) -> *mut usize {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).num_faces }
    }
    #[inline(always)]
    pub(crate) fn set_num_triangles(&self, value: usize) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).num_triangles = value }
    }
    #[inline(always)]
    pub(crate) fn num_triangles_raw(&self) -> *mut usize {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).num_triangles }
    }
    #[inline(always)]
    pub(crate) fn set_num_edges(&self, value: usize) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).num_edges = value }
    }
    #[inline(always)]
    pub(crate) fn num_edges_raw(&self) -> *mut usize {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).num_edges }
    }
    #[inline(always)]
    pub(crate) fn set_max_face_triangles(&self, value: usize) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).max_face_triangles = value }
    }
    #[inline(always)]
    pub(crate) fn max_face_triangles_raw(&self) -> *mut usize {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).max_face_triangles }
    }
    #[inline(always)]
    pub(crate) fn set_num_empty_faces(&self, value: usize) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).num_empty_faces = value }
    }
    #[inline(always)]
    pub(crate) fn num_empty_faces_raw(&self) -> *mut usize {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).num_empty_faces }
    }
    #[inline(always)]
    pub(crate) fn set_num_point_faces(&self, value: usize) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).num_point_faces = value }
    }
    #[inline(always)]
    pub(crate) fn num_point_faces_raw(&self) -> *mut usize {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).num_point_faces }
    }
    #[inline(always)]
    pub(crate) fn set_num_line_faces(&self, value: usize) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).num_line_faces = value }
    }
    #[inline(always)]
    pub(crate) fn num_line_faces_raw(&self) -> *mut usize {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).num_line_faces }
    }
    #[inline(always)]
    pub(crate) fn set_faces(&self, value: List<Face>) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).faces = value }
    }
    #[inline(always)]
    pub(crate) fn faces_raw(&self) -> *mut List<Face> {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).faces }
    }
    #[inline(always)]
    pub(crate) fn set_face_smoothing(&self, value: List<bool>) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).face_smoothing = value }
    }
    #[inline(always)]
    pub(crate) fn face_smoothing_raw(&self) -> *mut List<bool> {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).face_smoothing }
    }
    #[inline(always)]
    pub(crate) fn set_face_material(&self, value: List<u32>) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).face_material = value }
    }
    #[inline(always)]
    pub(crate) fn face_material_raw(&self) -> *mut List<u32> {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).face_material }
    }
    #[inline(always)]
    pub(crate) fn set_face_group(&self, value: List<u32>) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).face_group = value }
    }
    #[inline(always)]
    pub(crate) fn face_group_raw(&self) -> *mut List<u32> {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).face_group }
    }
    #[inline(always)]
    pub(crate) fn set_face_hole(&self, value: List<bool>) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).face_hole = value }
    }
    #[inline(always)]
    pub(crate) fn face_hole_raw(&self) -> *mut List<bool> {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).face_hole }
    }
    #[inline(always)]
    pub(crate) fn set_edges(&self, value: List<Edge>) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).edges = value }
    }
    #[inline(always)]
    pub(crate) fn edges_raw(&self) -> *mut List<Edge> {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).edges }
    }
    #[inline(always)]
    pub(crate) fn set_edge_smoothing(&self, value: List<bool>) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).edge_smoothing = value }
    }
    #[inline(always)]
    pub(crate) fn edge_smoothing_raw(&self) -> *mut List<bool> {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).edge_smoothing }
    }
    #[inline(always)]
    pub(crate) fn set_edge_crease(&self, value: List<Real>) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).edge_crease = value }
    }
    #[inline(always)]
    pub(crate) fn edge_crease_raw(&self) -> *mut List<Real> {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).edge_crease }
    }
    #[inline(always)]
    pub(crate) fn set_edge_visibility(&self, value: List<bool>) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).edge_visibility = value }
    }
    #[inline(always)]
    pub(crate) fn edge_visibility_raw(&self) -> *mut List<bool> {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).edge_visibility }
    }
    #[inline(always)]
    pub(crate) fn set_vertex_indices(&self, value: List<u32>) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).vertex_indices = value }
    }
    #[inline(always)]
    pub(crate) fn vertex_indices_raw(&self) -> *mut List<u32> {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).vertex_indices }
    }
    #[inline(always)]
    pub(crate) fn set_vertices(&self, value: List<Vec3>) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).vertices = value }
    }
    #[inline(always)]
    pub(crate) fn vertices_raw(&self) -> *mut List<Vec3> {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).vertices }
    }
    #[inline(always)]
    pub(crate) fn set_vertex_first_index(&self, value: List<u32>) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).vertex_first_index = value }
    }
    #[inline(always)]
    pub(crate) fn vertex_first_index_raw(&self) -> *mut List<u32> {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).vertex_first_index }
    }
    #[inline(always)]
    pub(crate) fn set_vertex_position(&self, value: VertexVec3) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).vertex_position = value }
    }
    #[inline(always)]
    pub(crate) fn vertex_position_raw(&self) -> *mut VertexVec3 {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).vertex_position }
    }
    #[inline(always)]
    pub(crate) fn set_vertex_normal(&self, value: VertexVec3) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).vertex_normal = value }
    }
    #[inline(always)]
    pub(crate) fn vertex_normal_raw(&self) -> *mut VertexVec3 {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).vertex_normal }
    }
    #[inline(always)]
    pub(crate) fn set_vertex_uv(&self, value: VertexVec2) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).vertex_uv = value }
    }
    #[inline(always)]
    pub(crate) fn vertex_uv_raw(&self) -> *mut VertexVec2 {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).vertex_uv }
    }
    #[inline(always)]
    pub(crate) fn set_vertex_tangent(&self, value: VertexVec3) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).vertex_tangent = value }
    }
    #[inline(always)]
    pub(crate) fn vertex_tangent_raw(&self) -> *mut VertexVec3 {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).vertex_tangent }
    }
    #[inline(always)]
    pub(crate) fn set_vertex_bitangent(&self, value: VertexVec3) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).vertex_bitangent = value }
    }
    #[inline(always)]
    pub(crate) fn vertex_bitangent_raw(&self) -> *mut VertexVec3 {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).vertex_bitangent }
    }
    #[inline(always)]
    pub(crate) fn set_vertex_color(&self, value: VertexVec4) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).vertex_color = value }
    }
    #[inline(always)]
    pub(crate) fn vertex_color_raw(&self) -> *mut VertexVec4 {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).vertex_color }
    }
    #[inline(always)]
    pub(crate) fn set_vertex_crease(&self, value: VertexReal) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).vertex_crease = value }
    }
    #[inline(always)]
    pub(crate) fn vertex_crease_raw(&self) -> *mut VertexReal {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).vertex_crease }
    }
    #[inline(always)]
    pub(crate) fn set_uv_sets(&self, value: List<UvSet>) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).uv_sets = value }
    }
    #[inline(always)]
    pub(crate) fn uv_sets_raw(&self) -> *mut List<UvSet> {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).uv_sets }
    }
    #[inline(always)]
    pub(crate) fn set_color_sets(&self, value: List<ColorSet>) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).color_sets = value }
    }
    #[inline(always)]
    pub(crate) fn color_sets_raw(&self) -> *mut List<ColorSet> {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).color_sets }
    }
    #[inline(always)]
    pub(crate) fn set_materials(&self, value: RefList<Material>) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).materials = value }
    }
    #[inline(always)]
    pub(crate) fn materials_raw(&self) -> *mut RefList<Material> {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).materials }
    }
    #[inline(always)]
    pub(crate) fn set_face_groups(&self, value: List<FaceGroup>) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).face_groups = value }
    }
    #[inline(always)]
    pub(crate) fn face_groups_raw(&self) -> *mut List<FaceGroup> {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).face_groups }
    }
    #[inline(always)]
    pub(crate) fn set_material_parts(&self, value: List<MeshPart>) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).material_parts = value }
    }
    #[inline(always)]
    pub(crate) fn material_parts_raw(&self) -> *mut List<MeshPart> {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).material_parts }
    }
    #[inline(always)]
    pub(crate) fn set_face_group_parts(&self, value: List<MeshPart>) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).face_group_parts = value }
    }
    #[inline(always)]
    pub(crate) fn face_group_parts_raw(&self) -> *mut List<MeshPart> {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).face_group_parts }
    }
    #[inline(always)]
    pub(crate) fn set_material_part_usage_order(&self, value: List<u32>) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).material_part_usage_order = value }
    }
    #[inline(always)]
    pub(crate) fn material_part_usage_order_raw(&self) -> *mut List<u32> {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).material_part_usage_order }
    }
    #[inline(always)]
    pub(crate) fn set_skinned_is_local(&self, value: bool) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).skinned_is_local = value }
    }
    #[inline(always)]
    pub(crate) fn skinned_is_local_raw(&self) -> *mut bool {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).skinned_is_local }
    }
    #[inline(always)]
    pub(crate) fn set_skinned_position(&self, value: VertexVec3) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).skinned_position = value }
    }
    #[inline(always)]
    pub(crate) fn skinned_position_raw(&self) -> *mut VertexVec3 {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).skinned_position }
    }
    #[inline(always)]
    pub(crate) fn set_skinned_normal(&self, value: VertexVec3) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).skinned_normal = value }
    }
    #[inline(always)]
    pub(crate) fn skinned_normal_raw(&self) -> *mut VertexVec3 {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).skinned_normal }
    }
    #[inline(always)]
    pub(crate) fn set_skin_deformers(&self, value: RefList<SkinDeformer>) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).skin_deformers = value }
    }
    #[inline(always)]
    pub(crate) fn skin_deformers_raw(&self) -> *mut RefList<SkinDeformer> {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).skin_deformers }
    }
    #[inline(always)]
    pub(crate) fn set_blend_deformers(&self, value: RefList<BlendDeformer>) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).blend_deformers = value }
    }
    #[inline(always)]
    pub(crate) fn blend_deformers_raw(&self) -> *mut RefList<BlendDeformer> {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).blend_deformers }
    }
    #[inline(always)]
    pub(crate) fn set_cache_deformers(&self, value: RefList<CacheDeformer>) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).cache_deformers = value }
    }
    #[inline(always)]
    pub(crate) fn cache_deformers_raw(&self) -> *mut RefList<CacheDeformer> {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).cache_deformers }
    }
    #[inline(always)]
    pub(crate) fn set_all_deformers(&self, value: RefList<Element>) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).all_deformers = value }
    }
    #[inline(always)]
    pub(crate) fn all_deformers_raw(&self) -> *mut RefList<Element> {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).all_deformers }
    }
    #[inline(always)]
    pub(crate) fn set_subdivision_preview_levels(&self, value: u32) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).subdivision_preview_levels = value }
    }
    #[inline(always)]
    pub(crate) fn subdivision_preview_levels_raw(&self) -> *mut u32 {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).subdivision_preview_levels }
    }
    #[inline(always)]
    pub(crate) fn set_subdivision_render_levels(&self, value: u32) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).subdivision_render_levels = value }
    }
    #[inline(always)]
    pub(crate) fn subdivision_render_levels_raw(&self) -> *mut u32 {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).subdivision_render_levels }
    }
    #[inline(always)]
    pub(crate) fn set_subdivision_display_mode(&self, value: SubdivisionDisplayMode) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).subdivision_display_mode = value }
    }
    #[inline(always)]
    pub(crate) fn subdivision_display_mode_raw(&self) -> *mut SubdivisionDisplayMode {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).subdivision_display_mode }
    }
    #[inline(always)]
    pub(crate) fn set_subdivision_boundary(&self, value: SubdivisionBoundary) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).subdivision_boundary = value }
    }
    #[inline(always)]
    pub(crate) fn subdivision_boundary_raw(&self) -> *mut SubdivisionBoundary {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).subdivision_boundary }
    }
    #[inline(always)]
    pub(crate) fn set_subdivision_uv_boundary(&self, value: SubdivisionBoundary) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).subdivision_uv_boundary = value }
    }
    #[inline(always)]
    pub(crate) fn subdivision_uv_boundary_raw(&self) -> *mut SubdivisionBoundary {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).subdivision_uv_boundary }
    }
    #[inline(always)]
    pub(crate) fn set_reversed_winding(&self, value: bool) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).reversed_winding = value }
    }
    #[inline(always)]
    pub(crate) fn reversed_winding_raw(&self) -> *mut bool {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).reversed_winding }
    }
    #[inline(always)]
    pub(crate) fn set_generated_normals(&self, value: bool) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).generated_normals = value }
    }
    #[inline(always)]
    pub(crate) fn generated_normals_raw(&self) -> *mut bool {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).generated_normals }
    }
    #[inline(always)]
    pub(crate) fn set_subdivision_evaluated(&self, value: bool) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).subdivision_evaluated = value }
    }
    #[inline(always)]
    pub(crate) fn subdivision_evaluated_raw(&self) -> *mut bool {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).subdivision_evaluated }
    }
    #[inline(always)]
    pub(crate) fn set_subdivision_result(&self, value: Option<Ref<SubdivisionResult>>) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).subdivision_result = value }
    }
    #[inline(always)]
    pub(crate) fn subdivision_result_raw(&self) -> *mut Option<Ref<SubdivisionResult>> {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).subdivision_result }
    }
    #[inline(always)]
    pub(crate) fn set_from_tessellated_nurbs(&self, value: bool) {
        // SAFETY: field write through the `Mut` view's write-capable viewed
        // memory (mint vouch); no `&`/`&mut` to the viewed bytes is live.
        unsafe { (*self.get()).from_tessellated_nurbs = value }
    }
    #[allow(clippy::wrong_self_convention)]
    #[inline(always)]
    pub(crate) fn from_tessellated_nurbs_raw(&self) -> *mut bool {
        // SAFETY: in-bounds field projection; the returned raw pointer
        // inherits the view's write-capable provenance.
        unsafe { &raw mut (*self.get()).from_tessellated_nurbs }
    }
}
