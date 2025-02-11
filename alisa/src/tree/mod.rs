
use crate::{Delta, Object, Project, ProjectContext, Ptr, Recorder, Serializable};

mod child_list;
pub use child_list::*;

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

pub trait TreeObj: Object {

    type ParentPtr: Default + Clone;
    type ChildList: Children<Self>;
    type TreeData: Serializable<Self::Project>;

    fn child_list<'a>(parent: Self::ParentPtr, project: &'a Self::Project, objects: &'a <Self::Project as Project>::Objects) -> Option<&'a Self::ChildList>;
    fn child_list_mut<'a>(parent: Self::ParentPtr, context: &'a mut ProjectContext<Self::Project>) -> Option<&'a mut Self::ChildList>;
    fn parent(&self) -> Self::ParentPtr;
    fn parent_mut(&mut self) -> &mut Self::ParentPtr;

    fn instance(data: &Self::TreeData, ptr: Ptr<Self>, parent: Self::ParentPtr, recorder: &mut Recorder<Self::Project>); 
    fn destroy(&self, recorder: &mut Recorder<Self::Project>);
    fn collect_data(&self, objects: &<Self::Project as Project>::Objects) -> Self::TreeData;

}

#[macro_export]
macro_rules! tree_object_operations {
    ($object: ty) => {
        paste::paste! {

            #[derive(::alisa::Serializable)]
            #[project(<$object as ::alisa::Object>::Project)]
            pub struct [< Create $object:camel >] {
                pub ptr: ::alisa::Ptr<$object>,
                pub parent: <$object as ::alisa::TreeObj>::ParentPtr,
                pub idx: usize,
                pub data: <$object as ::alisa::TreeObj>::TreeData
            }

            impl Default for [< Create $object:camel >] {

                fn default() -> Self {
                    Self {
                        ptr: ::alisa::Ptr::null(),
                        parent: Default::default(),
                        idx: 0,
                        data: <<$object as ::alisa::TreeObj>::TreeData as Default>::default()
                    }
                }

            }

            impl ::alisa::Operation for [< Create $object:camel >] {

                type Project = <$object as ::alisa::Object>::Project;
                type Inverse = [< Delete $object:camel >];

                const NAME: &'static str = stringify!([< Create $object:camel >]);

                fn perform(&self, recorder: &mut ::alisa::Recorder<Self::Project>) {
                    use ::alisa::Children;
                    <$object as ::alisa::TreeObj>::instance(&self.data, self.ptr, self.parent, recorder);
                    if let Some(child_list) = <$object as ::alisa::TreeObj>::child_list_mut(self.parent, recorder.context()) {
                        child_list.insert(self.idx, self.ptr);
                        recorder.push_delta(::alisa::RemoveChildDelta {
                            parent: self.parent.clone(),
                            ptr: self.ptr
                        });
                    }
                }

                fn inverse(&self, _project: &Self::Project, objects: &<Self::Project as ::alisa::Project>::Objects) -> Option<Self::Inverse> {
                    Some([<Delete $object:camel >] {
                        ptr: self.ptr
                    })
                }

            }

            #[derive(::alisa::Serializable)]
            #[project(<$object as ::alisa::Object>::Project)]
            pub struct [< Delete $object:camel >] {
                pub ptr: ::alisa::Ptr<$object>
            }

            impl Default for [< Delete $object:camel >] {

                fn default() -> Self {
                    Self {
                        ptr: ::alisa::Ptr::null()
                    }
                }

            } 

            impl ::alisa::Operation for [< Delete $object:camel >] {

                type Project = <$object as ::alisa::Object>::Project;
                type Inverse = [< Create $object:camel >];

                const NAME: &'static str = stringify!([< Delete $object:camel >]);

                fn perform(&self, recorder: &mut ::alisa::Recorder<Self::Project>) {
                    use ::alisa::Children;
                    use ::alisa::TreeObj;
                    if let Some(obj) = recorder.obj_list_mut().delete(self.ptr) {
                        obj.destroy(recorder);
                        let parent = obj.parent(); 
                        recorder.push_delta(::alisa::RecreateObjectDelta {
                            ptr: self.ptr,
                            obj
                        });
                        if let Some(child_list) = $object::child_list_mut(parent.clone(), recorder.context()) {
                            if let Some(idx) = child_list.remove(self.ptr) {
                                recorder.push_delta(::alisa::InsertChildDelta {
                                    parent,
                                    ptr: self.ptr,
                                    idx
                                });
                            }
                        }
                    }
                }

                fn inverse(&self, project: &Self::Project, objects: &<Self::Project as ::alisa::Project>::Objects) -> Option<Self::Inverse> {
                    use ::alisa::Children;
                    let object = $object::list(objects).get(self.ptr)?; 
                    let data = <$object as ::alisa::TreeObj>::collect_data(&object, objects);
                    let parent = <$object as ::alisa::TreeObj>::parent(&object);
                    let child_list = <$object as ::alisa::TreeObj>::child_list(parent, project, objects)?;
                    let idx = child_list.index_of(self.ptr)?;
                    Some([< Create $object:camel >] {
                        ptr: self.ptr,
                        idx,
                        parent,
                        data
                    })
                }

            }

            #[derive(::alisa::Serializable)]
            #[project(<$object as ::alisa::Object>::Project)]
            pub struct [< Transfer $object >] {
                pub ptr: ::alisa::Ptr<$object>,
                pub new_parent: <$object as ::alisa::TreeObj>::ParentPtr,
                pub new_idx: usize
            }

            impl Default for [< Transfer $object >] {

                fn default() -> Self {
                    Self {
                        ptr: ::alisa::Ptr::null(),
                        new_parent: Default::default(),
                        new_idx: 0
                    }
                }

            }

            impl ::alisa::Operation for [< Transfer $object >] {

                type Project = <$object as ::alisa::Object>::Project;
                type Inverse = [< Transfer $object:camel >];

                const NAME: &'static str = stringify!([< Transfer $object:camel >]);

                fn perform(&self, recorder: &mut ::alisa::Recorder<Self::Project>) {
                    use ::alisa::TreeObj;
                    use ::alisa::Children;

                    // Make sure everything we need exists
                    let Some(obj) = recorder.obj_list_mut().get_mut(self.ptr) else { return; };
                    let old_parent = obj.parent().clone();
                    if $object::child_list_mut(old_parent.clone(), recorder.context()).is_none() {
                        return;
                    }
                    if $object::child_list_mut(self.new_parent.clone(), recorder.context()).is_none() {
                        return;
                    }

                    // Set the object's parent
                    let Some(obj) = recorder.obj_list_mut().get_mut(self.ptr) else { return; };
                    *obj.parent_mut() = self.new_parent.clone();
                    recorder.push_delta(::alisa::SetParentDelta {
                        ptr: self.ptr,
                        new_parent: old_parent.clone()
                    });
                    
                    // Remove the object from the old parent's child list
                    if let Some(old_child_list) = $object::child_list_mut(old_parent.clone(), recorder.context()) {
                        if let Some(idx) = old_child_list.remove(self.ptr) {
                            recorder.push_delta(::alisa::InsertChildDelta {
                                parent: old_parent,
                                ptr: self.ptr,
                                idx
                            });
                        }
                    }

                    // Add the object to the new parent's child list
                    if let Some(new_child_list) = $object::child_list_mut(self.new_parent.clone(), recorder.context()) {
                        new_child_list.insert(self.new_idx, self.ptr);
                        recorder.push_delta(::alisa::RemoveChildDelta {
                            parent: self.new_parent.clone(),
                            ptr: self.ptr
                        });
                    }
                }

                fn inverse(&self, project: &Self::Project, objects: &<Self::Project as ::alisa::Project>::Objects) -> Option<Self::Inverse> {
                    use ::alisa::TreeObj;
                    use ::alisa::Children;
                    let object = $object::list(objects).get(self.ptr)?; 
                    let parent = object.parent();
                    let child_list = $object::child_list(parent, project, objects)?; 
                    let idx = child_list.index_of(self.ptr)?;
                    Some(Self {
                        ptr: self.ptr,
                        new_parent: parent,
                        new_idx: idx
                    })
                }

            }

        } 
    };
}
