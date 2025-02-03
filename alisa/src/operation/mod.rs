
use std::any::{Any, TypeId};

use crate::{Serializable, DeserializationContext, ObjList, Object, Project, SerializationContext};

mod common;

/// An operation performed on the project. 
/// Operations can be inverted for undo/redo. 
/// Note that when collaborating, undoing an operation and redoing might not return to the original state of the project. 
pub trait Operation: Sized + Any + Serializable<Self::Project> {

    type Project: Project;
    type Inverse: Operation<Project = Self::Project, Inverse = Self>;

    /// The name of the operation, used for collab messages. MAKE SURE THIS IS UNIQUE FOR ALL OPERATIONS!
    const NAME: &'static str;

    /// Perform the operation.
    fn perform(&self, recorder: &mut Recorder<'_, Self::Project>); 
    /// Get the inverse operation. 
    fn inverse(&self, project: &Self::Project, objects: &<Self::Project as Project>::Objects) -> Option<Self::Inverse>;

}

/// Shim trait for turning an operation into a dyn object
pub(crate) trait OperationDyn {
    type Project: Project;

    fn perform(&self, recorder: &mut Recorder<'_, Self::Project>);
    fn inverse(&self, project: &Self::Project, objects: &<Self::Project as Project>::Objects) -> Option<Box<dyn OperationDyn<Project = Self::Project>>>;
    fn name(&self) -> &'static str;
    fn serialize(&self) -> rmpv::Value;
}

impl<O: Operation + Serializable<O::Project>> OperationDyn for O {
    type Project = O::Project;

    fn perform(&self, recorder: &mut Recorder<'_, Self::Project>) {
        self.perform(recorder);
    }

    fn inverse(&self, project: &Self::Project, objects: &<Self::Project as Project>::Objects) -> Option<Box<dyn OperationDyn<Project = Self::Project>>> {
        if let Some(inverse) = <Self as Operation>::inverse(self, project, objects) {
            return Some(Box::new(inverse)); 
        }
        None
    }

    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn serialize(&self) -> rmpv::Value {
        self.serialize(&SerializationContext::data())
    }

}

/// A kind of operation, stored as a struct in `Project::OPERATIONS`.
pub struct OperationKind<P: Project> {
    pub(crate) name: &'static str,
    pub(crate) deserialize: fn(&rmpv::Value) -> Option<Box<dyn Any>>,
    pub(crate) perform: fn(Box<dyn Any>, &mut Recorder<'_, P>),

    #[cfg(debug_assertions)]
    pub(crate) type_id: fn() -> TypeId
}

impl<P: Project> OperationKind<P> {

    pub const fn from<O: Operation<Project = P>>() -> Self {
        Self {
            name: O::NAME,
            deserialize: |data| {
                Some(Box::new(O::deserialize(data, &mut DeserializationContext::data())?))
            },
            perform: |operation, recorder| {
                let Ok(operation) = operation.downcast::<O>() else { return; };
                operation.perform(recorder);
            },
            #[cfg(debug_assertions)]
            type_id: || TypeId::of::<O>()
        }
    }

}

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

/// An operation that was not yet confirmed by the server. Used for moving backwards/forwards in time for conflict resolution.  
pub(crate) struct UnconfirmedOperation<P: Project> {
    pub(crate) operation: Box<dyn OperationDyn<Project = P>>,
    pub(crate) deltas: Vec<Box<dyn Delta<Project = P>>> 
}
