
use crate::{Delta, ObjPtr, Object, ProjectContext};

pub(crate) struct DeleteObjectDelta<O: Object> {
    pub(crate) ptr: ObjPtr<O>
} 

impl<O: Object> Delta for DeleteObjectDelta<O> {
    type Project = O::Project;

    fn perform(&self, context: &mut ProjectContext<O::Project>) {
        context.obj_list_mut().delete(self.ptr);
    }
}

pub(crate) struct RecreateObjectDelta<O: Object> {
    pub(crate) ptr: ObjPtr<O>,
    pub(crate) obj: O
}

impl<O: Object> Delta for RecreateObjectDelta<O> {
    type Project = O::Project;

    fn perform(&self, context: &mut ProjectContext<O::Project>) {
        context.obj_list_mut().insert(self.ptr, self.obj.clone());
    }
} 
