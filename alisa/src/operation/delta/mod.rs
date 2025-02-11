
mod common;
pub use common::*;

use crate::{ObjList, Object, Project};

pub struct ProjectContext<'a, P: Project> {
    pub(crate) project: &'a mut P,
    pub(crate) objects: &'a mut P::Objects,
    pub(crate) context: &'a mut P::Context,

    /// Was the project object itself modified?
    pub(crate) project_modified: &'a mut bool
}

impl<P: Project> ProjectContext<'_, P> {

    pub fn project(&self) -> &P {
        self.project
    }
    
    pub fn project_mut(&mut self) -> &mut P {
        *self.project_modified = true;
        self.project
    }

    pub fn obj_list<O: Object<Project = P>>(&self) -> &ObjList<O> {
        O::list(self.objects)
    }

    pub fn obj_list_mut<O: Object<Project = P>>(&mut self) -> &mut ObjList<O> {
        O::list_mut(self.objects)
    }

    pub fn context(&self) -> &P::Context {
        &self.context
    } 

    pub fn context_mut(&mut self) -> &mut P::Context {
        self.context
    }

}

/// A tiny change to the project. Used for moving backwards in time for the collaboration conflict resolution system. 
pub trait Delta {
    type Project: Project;

    fn perform(&self, context: &mut ProjectContext<'_, Self::Project>);
}

pub struct Recorder<'a, P: Project> {
    pub(crate) context: ProjectContext<'a, P>,
    /// The reversed changes recorded while the operation was being executed 
    pub(crate) deltas: Vec<Box<dyn Delta<Project = P>>>,
}

impl<'a, P: Project> Recorder<'a, P> {

    pub(crate) fn new(context: ProjectContext<'a, P>) -> Self {
        Self {
            context,
            deltas: Vec::new(),
        }
    }

    pub fn push_delta<D: Delta<Project = P> + 'static>(&mut self, delta: D) {
        self.deltas.push(Box::new(delta));
    }

    pub fn context<'b>(&'b mut self) -> &'b mut ProjectContext<'a, P> {
        &mut self.context
    }

    pub fn project(&self) -> &P {
        self.context.project()
    }
    
    pub fn project_mut(&mut self) -> &mut P {
        self.context.project_mut()
    }

    pub fn obj_list<O: Object<Project = P>>(&self) -> &ObjList<O> {
        self.context.obj_list()
    }

    pub fn obj_list_mut<O: Object<Project = P>>(&mut self) -> &mut ObjList<O> {
        self.context.obj_list_mut()
    }

}

