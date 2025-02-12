
use crate::{Delta, Object, Project, ProjectContext, Ptr, Recorder, Serializable};

mod child_list;
pub use child_list::*;

/// A list of references to the children of a tree object
pub trait Children<O: Object> {

    fn n_children(&self) -> usize;
    fn insert(&mut self, idx: usize, child: Ptr<O>);
    fn remove(&mut self, child: Ptr<O>) -> Option<usize>;
    fn index_of(&self, child: Ptr<O>) -> Option<usize>;

}

pub struct RemoveChildDelta<O: TreeObj> {
    pub parent: O::ParentPtr,
    pub ptr: Ptr<O>
}

impl<O: TreeObj> Delta for RemoveChildDelta<O> {
    type Project = O::Project;

    fn perform(&self, context: &mut crate::ProjectContext<'_, Self::Project>) {
        if let Some(list) = O::child_list_mut(self.parent.clone(), context) {
            list.remove(self.ptr);
        }
    }

}

pub struct InsertChildDelta<O: TreeObj> {
    pub parent: O::ParentPtr,
    pub ptr: Ptr<O>,
    pub idx: usize
}

impl<O: TreeObj> Delta for InsertChildDelta<O> {
    type Project = O::Project;

    fn perform(&self, context: &mut ProjectContext<'_, Self::Project>) {
        if let Some(list) = O::child_list_mut(self.parent.clone(), context) {
            list.insert(self.idx, self.ptr);
        }
    }
}

pub struct SetParentDelta<O: TreeObj> {
    pub ptr: Ptr<O>,
    pub new_parent: O::ParentPtr
}

impl<O: TreeObj> Delta for SetParentDelta<O> {
    type Project = O::Project;

    fn perform(&self, context: &mut ProjectContext<'_, Self::Project>) {
        if let Some(obj) = context.obj_list_mut().get_mut(self.ptr) {
            *obj.parent_mut() = self.new_parent.clone();
        }
    }
}

/// An object that is part of a tree of objects. 
pub trait TreeObj: Object {

    /// The type of pointer that points to the parent object 
    type ParentPtr: Default + Clone;
    /// The list of children the parent has that points to this tree object
    type ChildList: Children<Self>;
    /// The information needed to recreate this object and all of its children in the tree
    type TreeData: Serializable<Self::Project>;

    /// Get the list of children that points to this object given the parent pointer
    fn child_list<'a>(parent: Self::ParentPtr, project: &'a Self::Project, objects: &'a <Self::Project as Project>::Objects) -> Option<&'a Self::ChildList>;
    /// Get a mutable reference to the list of children that points to this object given the parent pointer
    fn child_list_mut<'a>(parent: Self::ParentPtr, context: &'a mut ProjectContext<Self::Project>) -> Option<&'a mut Self::ChildList>;
    /// Get the parent
    fn parent(&self) -> Self::ParentPtr;
    /// Get a mutable reference to the parent
    fn parent_mut(&mut self) -> &mut Self::ParentPtr;

    /// Create this object and all its children from the tree data
    fn instance(data: &Self::TreeData, ptr: Ptr<Self>, parent: Self::ParentPtr, recorder: &mut Recorder<Self::Project>); 
    /// Delete this object and all its children
    fn destroy(&self, recorder: &mut Recorder<Self::Project>);
    /// Get the tree data for this object and its children
    fn collect_data(&self, objects: &<Self::Project as Project>::Objects) -> Self::TreeData;

}

mod creation;
mod transfer;

#[macro_export]
macro_rules! tree_object_operations {
    ($object: ty) => {
        paste::paste! {
            ::alisa::tree_object_creation_operations!($object);
            ::alisa::tree_object_transfer_operation!($object);
        } 
    };
}
