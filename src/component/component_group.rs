use crate::data::arena::{Arena, ArenaIndex, ArenaIterMut};
use crate::{ComponentConfig, ComponentType, Dimension, DynamicComponent, Vector};
use rustc_hash::FxHashMap;
use std::any::TypeId;

/// Helper to create a [ComponentGroup](crate::ComponentGroup).
pub struct ComponentGroupDescriptor {
    /// Id of the grou
    pub id: u32,
    /// Position of the group
    pub position: Vector<f32>,
    /// Size where the group operates
    pub size: Dimension<f32>,
    /// Describes if the group is enabled from the start
    pub enabled: bool,
}

/// Id of the default [ComponentGroup](crate::ComponentGroup). Components within this group are
/// always getting rendered and updated in every cycle.
pub const DEFAULT_GROUP_ID: u32 = u32::MAX / 2;

/// Every group has a name and a fixed position where it operates. When the camera intersects with 
/// the position and size of the group the group is marked as `active`.It can be used like a chunk 
/// system to make huge 2D worlds possible or to just order your components. The Engine has a 
/// default [ComponentGroup](crate::ComponentGroup) for components that are used all the
/// time. After every update, before rendering, the set of active component groups gets
/// computed. A group can be accessed with [group](crate::Context::group) or 
/// [group_mut](crate::Context::group_mut). The components of the group can be accessed with
/// [components](crate::Context::components) or [components_mut](crate::Context::components_mut)
/// from the [context](crate::Context).
pub struct ComponentGroup {
    type_map: FxHashMap<TypeId, ArenaIndex>,
    types: Arena<ComponentType>,
    id: u32,
    position: Vector<f32>,
    size: Dimension<f32>,
    enabled: bool,
    active: bool,
}

impl ComponentGroup {
    pub(crate) fn new(id: u32, descriptor: &ComponentGroupDescriptor) -> Self {
        Self {
            id,
            position: descriptor.position,
            size: descriptor.size,
            enabled: descriptor.enabled,
            type_map: Default::default(),
            types: Default::default(),
            active: false,
        }
    }

    pub(crate) fn intersects_camera(
        &self,
        cam_bottom_left: Vector<f32>,
        cam_top_right: Vector<f32>,
    ) -> bool {
        let pos = self.position;
        let size = self.size;
        let self_bl = Vector::new(pos.x - size.width, pos.y - size.height);
        let self_tr = Vector::new(pos.x + size.width, pos.y + size.height);
        return (cam_bottom_left.x < self_tr.x)
            && (self_bl.x < cam_top_right.x)
            && (cam_bottom_left.y < self_tr.y)
            && (self_bl.y < cam_top_right.y);
    }

    // Setters

    #[inline]
    pub(crate) fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    #[inline]
    /// Disable or enable a group
    /// 
    /// # Warning (TODO)
    /// Currently [PhysicsComponents](crate::physics::PhysicsComponent) collisions do not get disabled.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    #[inline]
    /// Set the operation size of this group.
    pub fn set_size(&mut self, size: Dimension<f32>) {
        self.size = size;
    }

    #[inline]
    /// Set the position of this group.
    pub fn set_position(&mut self, position: Vector<f32>) {
        self.position = position;
    }

    // Getters

    #[inline]
    pub(crate) fn type_index(&self, type_id: &TypeId) -> Option<&ArenaIndex> {
        self.type_map.get(type_id)
    }

    #[inline]
    pub(crate) fn type_ref(&self, index: ArenaIndex) -> Option<&ComponentType> {
        self.types.get(index)
    }

    #[inline]
    pub(crate) fn type_mut(&mut self, index: ArenaIndex) -> Option<&mut ComponentType> {
        self.types.get_mut(index)
    }

    // #[inline]
    // pub(crate) fn remove_type(&mut self, type_index: ArenaIndex) {
    //     self.types.remove(type_index);
    // }

    #[inline]
    pub(crate) fn types(&mut self) -> ArenaIterMut<ComponentType> {
        self.types.iter_mut()
    }

    #[inline]
    pub(crate) fn add_component_type(
        &mut self,
        type_id: TypeId,
        config: &'static ComponentConfig,
        component: DynamicComponent,
    ) -> (ArenaIndex, ArenaIndex) {
        let type_index = self.types.insert(ComponentType::new(type_id, config));
        let component_type = self.types.get_mut(type_index).unwrap();
        self.type_map.insert(type_id, type_index);
        let component_index = component_type.add(component);
        return (type_index, component_index);
    }

    #[inline]
    /// Get if the group is enabled
    pub const fn enabled(&self) -> bool {
        self.enabled
    }

    #[inline]
    /// Get the id of the group.
    pub const fn id(&self) -> u32 {
        self.id
    }

    #[inline]
    /// Get the position of the group.
    pub const fn pos(&self) -> &Vector<f32> {
        &self.position
    }

    #[inline]
    /// Get the operation dimension of the group.
    pub const fn size(&self) -> &Dimension<f32> {
        &self.size
    }

    #[inline]
    /// See if this group is active in the current cycle.
    pub const fn active(&self) -> bool {
        self.active
    }
}

impl Default for ComponentGroup {
    fn default() -> Self {
        Self {
            id: DEFAULT_GROUP_ID,
            position: Vector::new(0.0, 0.0),
            size: Dimension::new(f32::INFINITY, f32::INFINITY),
            enabled: true,
            type_map: Default::default(),
            types: Default::default(),
            active: true,
        }
    }
}
