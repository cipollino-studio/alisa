
use crate::{Loadable, ObjectKind, OperationKind};

pub trait Project: Sized + Loadable<Self> + 'static {

    type Context;
    /// A struct containing an `ObjList` for every kind of object in the project
    type Objects: Default;

    fn empty() -> Self;
    fn create_default(&mut self);

    const OBJECTS: &'static [ObjectKind<Self>];
    const OPERATIONS: &'static [OperationKind<Self>];

}
