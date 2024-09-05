pub use shipyard;
use parking_lot::Mutex;

pub use shipyard::{
    error, ARef, ARefMut, AllStorages, AllStoragesView, AllStoragesViewMut, AsLabel, BulkAddEntity,
    BulkEntityIter, Component, Entities, EntitiesView, EntitiesViewMut, EntityId, GetComponent,
    GetUnique, IntoIterRef, IterComponent, Label, Storage, StorageId, TrackingTimestamp,
    TupleAddComponent, TupleDelete, TupleDeleteAny, TupleRemove, TupleTrack, Unique, UniqueView,
    UniqueViewMut, View, ViewMut, World,
};


pub(crate) type GlobalWorld = Mutex<World>;


pub trait WorldExt {
    fn view<C: Component>(&self) -> View<C>;
    fn view_mut<C: Component>(&self) -> ViewMut<C>;
    fn entities(&self) -> EntitiesView;
    fn entities_mut(&self) -> EntitiesViewMut;
    fn unique<C: Unique>(&self) -> UniqueView<C>;
    fn unique_mut<C: Unique>(&self) -> UniqueViewMut<C>;
}

impl WorldExt for World {
    fn view<C: Component>(&self) -> View<C> {
        self.borrow::<View<C>>().unwrap()
    }

    fn view_mut<C: Component>(&self) -> ViewMut<C> {
        self.borrow::<ViewMut<C>>().unwrap()
    }

    fn entities(&self) -> EntitiesView {
        self.borrow::<EntitiesView>().unwrap()
    }

    fn entities_mut(&self) -> EntitiesViewMut {
        self.borrow::<EntitiesViewMut>().unwrap()
    }

    fn unique<C: Unique>(&self) -> UniqueView<C> {
        self.borrow::<UniqueView<C>>().unwrap()
    }

    fn unique_mut<C: Unique>(&self) -> UniqueViewMut<C> {
        self.borrow::<UniqueViewMut<C>>().unwrap()
    }
}
