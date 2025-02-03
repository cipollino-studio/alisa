
use std::{cell::RefCell, collections::{HashMap, HashSet}, hash::Hash, marker::PhantomData};

use crate::{rmpv_encode, DeleteObjectDelta, Project, Recorder, RecreateObjectDelta};

mod loading;
pub use loading::*;

pub trait Object: Sized + Clone + Loadable<Self::Project> + 'static {

    type Project: Project;

    const NAME: &'static str;

    fn list(objects: &<Self::Project as Project>::Objects) -> &ObjList<Self>;
    fn list_mut(objects: &mut <Self::Project as Project>::Objects) -> &mut ObjList<Self>;

    fn add(recorder: &mut Recorder<Self::Project>, ptr: ObjPtr<Self>, obj: Self) {
        recorder.obj_list_mut().insert(ptr, obj);
        recorder.push_delta(DeleteObjectDelta {
            ptr
        });
    }

    fn delete(recorder: &mut Recorder<Self::Project>, ptr: ObjPtr<Self>) {
        if let Some(obj) = recorder.obj_list_mut().delete(ptr) {
            recorder.push_delta(RecreateObjectDelta {
                ptr,
                obj,
            });
        }
    }

}

pub struct ObjPtr<Obj: Object> {
    /// The unique key of the object being pointed to
    pub(crate) key: u64,
    _marker: PhantomData<Obj>
}

impl<Obj: Object> Clone for ObjPtr<Obj> {

    fn clone(&self) -> Self {
        Self { key: self.key.clone(), _marker: self._marker.clone() }
    }

}

impl<Obj: Object> Copy for ObjPtr<Obj> {}

impl<Obj: Object> ObjPtr<Obj> {

    pub fn from_key(key: u64) -> Self {
        Self {
            key,
            _marker: PhantomData,
        }
    }

    pub fn null() -> Self {
        Self::from_key(0)
    }

}

impl<Obj: Object> PartialEq for ObjPtr<Obj> {

    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }

}

impl<Obj: Object> Eq for ObjPtr<Obj> {}

impl<Obj: Object> Hash for ObjPtr<Obj> {

    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.key.hash(state);
    }

}

impl<Obj: Object> Default for ObjPtr<Obj> {

    fn default() -> Self {
        Self::null()
    }

}

pub struct ObjList<Obj: Object> {
    objs: HashMap<ObjPtr<Obj>, Obj>,
    modified: HashSet<ObjPtr<Obj>>,
    to_delete: HashSet<ObjPtr<Obj>>,
    /// The pointers to the object data stored in the Verter file
    file_ptrs: RefCell<HashMap<ObjPtr<Obj>, u64>>
}

impl<Obj: Object> ObjList<Obj> {

    pub fn new() -> Self {
        Self {
            objs: HashMap::new(),
            modified: HashSet::new(),
            to_delete: HashSet::new(),
            file_ptrs: RefCell::new(HashMap::new())
        }
    }

    pub fn insert(&mut self, ptr: ObjPtr<Obj>, obj: Obj) {
        if self.objs.contains_key(&ptr) {
            return;
        }
        self.objs.insert(ptr, obj);
        self.modified.insert(ptr);
    }

    pub fn delete(&mut self, ptr: ObjPtr<Obj>) -> Option<Obj> {
        if self.get(ptr).is_none() {
            return None;
        }
        self.to_delete.insert(ptr);
        self.objs.remove(&ptr)
    }

    pub fn get(&self, ptr: ObjPtr<Obj>) -> Option<&Obj> {
        self.objs.get(&ptr) 
    }

    pub fn get_mut(&mut self, ptr: ObjPtr<Obj>) -> Option<&mut Obj> {
        self.modified.insert(ptr);
        self.objs.get_mut(&ptr) 
    }

    pub(crate) fn get_file_ptr(&self, ptr: ObjPtr<Obj>, file: &mut verter::File) -> Option<u64> {
        if let Some(ptr) = self.file_ptrs.borrow().get(&ptr) {
            return Some(*ptr);
        }
        let new_file_ptr = file.alloc().ok()?;
        self.file_ptrs.borrow_mut().insert(ptr, new_file_ptr);
        Some(new_file_ptr)
    }

}

impl<O: Object> Default for ObjList<O> {

    fn default() -> Self {
        Self::new()
    }

}

pub struct ObjectKind<P: Project> {
    pub(crate) save_modifications: fn(&mut verter::File, objects: &mut P::Objects)
}

impl<P: Project> ObjectKind<P> {

    pub const fn from<O: Object<Project = P>>() -> Self {
        Self {
            save_modifications: |file, objects| {
                for modified in std::mem::replace(&mut O::list_mut(objects).modified, HashSet::new()) {
                    if let Some(object) = O::list(objects).get(modified) {
                        let object_data = object.store(&StoringContext::shallow(objects, file));
                        if let Some(ptr) = O::list(objects).get_file_ptr(modified, file) {
                            if let Some(encoded_data) = rmpv_encode(&object_data) {
                                let _ = file.write(ptr, &encoded_data);
                            }
                        }
                    }
                }
                for deleted in std::mem::replace(&mut O::list_mut(objects).to_delete, HashSet::new()) {
                    if let Some(ptr) = O::list(objects).file_ptrs.borrow_mut().remove(&deleted) {
                        let _ = file.delete(ptr);
                    }
                }
            },
        }
    }

}
