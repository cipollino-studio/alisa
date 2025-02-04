
use std::{cell::RefCell, collections::HashSet};

use crate::{File, Project};

use super::{ObjPtr, Object};

mod serialization_impls;

enum DeserializationContextKind<'a, P: Project> {
    Local {
        file: &'a mut File,
        objects: &'a mut P::Objects,
    },
    Collab {
        objects: &'a mut P::Objects,
    },
    Data
}

pub struct DeserializationContext<'a, P: Project> {
    kind: DeserializationContextKind<'a, P>,
    /// The keys of the objects already loaded
    loaded: HashSet<u64>,
}

impl<'a, P: Project> DeserializationContext<'a, P> {

    pub(crate) fn local(objects: &'a mut P::Objects, file: &'a mut File) -> Self {
        Self {
            kind: DeserializationContextKind::Local {
                file,
                objects
            },
            loaded: HashSet::new(),
        }
    }

    pub(crate) fn collab(objects: &'a mut P::Objects) -> Self {
        Self {
            kind: DeserializationContextKind::Collab {
                objects,
            },
            loaded: HashSet::new()
        }
    }

    pub(crate) fn data() -> Self {
        Self {
            kind: DeserializationContextKind::Data,
            loaded: HashSet::new(),
        }
    }

}

enum SerializationContextKind<'a, P: Project> {
    Shallow,
    Deep {
        objects: &'a P::Objects,
    },
}

pub struct SerializationContext<'a, P: Project> {
    kind: SerializationContextKind<'a, P>,
    /// The keys of the objects already stored
    stored: RefCell<HashSet<u64>>,
}

impl<'a, P: Project> SerializationContext<'a, P> {

    pub(crate) fn shallow() -> Self {
        Self {
            kind: SerializationContextKind::Shallow,
            stored: RefCell::new(HashSet::new()),
        }
    }

    pub(crate) fn deep(objects: &'a P::Objects) -> Self {
        Self {
            kind: SerializationContextKind::Deep {
                objects
            },
            stored: RefCell::new(HashSet::new()),
        }
    }

}

pub trait Serializable<P: Project>: Sized {

    fn serialize(&self, context: &SerializationContext<P>) -> rmpv::Value; 
    fn deserialize(data: &rmpv::Value, context: &mut DeserializationContext<P>) -> Option<Self>;

}

const ALREADY_LOADED_MSGPACK_EXT_CODE: i8 = 123;

#[derive(Clone)]
pub struct ObjBox<O: Object> {
    ptr: ObjPtr<O>
}

impl<O: Object> ObjBox<O> {

    pub fn new(ptr: ObjPtr<O>) -> Self {
        Self {
            ptr
        }
    }

    pub fn ptr(&self) -> ObjPtr<O> {
        self.ptr
    }

    fn load_from_key_and_data(key: u64, object_data: &rmpv::Value, context: &mut DeserializationContext<O::Project>) -> Option<Self> {
        let obj_ptr = ObjPtr::from_key(key);
        if !matches!(object_data.as_ext(), Some((ALREADY_LOADED_MSGPACK_EXT_CODE, _))) {
            let object = O::deserialize(&object_data, context)?;
            match &mut context.kind {
                DeserializationContextKind::Local { file: _, objects } | 
                DeserializationContextKind::Collab { objects } => {
                    O::list_mut(objects).insert(obj_ptr, object);
                },
                DeserializationContextKind::Data => unreachable!(),
            }
        }
        Some(ObjBox { ptr: obj_ptr })
    } 

}

impl<O: Object> Serializable<O::Project> for ObjBox<O> {

    fn deserialize(data: &rmpv::Value, context: &mut DeserializationContext<O::Project>) -> Option<Self> {
        match &mut context.kind {
            DeserializationContextKind::Local { file, objects } => {
                let key = data.as_u64()?;
                let obj_ptr = ObjPtr::from_key(key);

                // If the object is already loaded, skip loading it
                if O::list(objects).get(obj_ptr).is_some() || context.loaded.contains(&key) {
                    return Some(Self {
                        ptr: obj_ptr
                    });
                }
                context.loaded.insert(key);

                let file_ptr = file.get_ptr(key)?;
                let object_data = file.read(file_ptr)?; 

                Self::load_from_key_and_data(key, &object_data, context)
            },
            DeserializationContextKind::Collab { objects } => {
                let data = data.as_array()?;
                let key = data.get(0)?.as_u64()?;
                let obj_ptr = ObjPtr::from_key(key);

                // If the object is already loaded, skip loading it
                if O::list(objects).get(obj_ptr).is_some() || context.loaded.contains(&key) {
                    return Some(Self {
                        ptr: obj_ptr
                    });
                }
                context.loaded.insert(key);

                let object_data = data.get(1)?;
                Self::load_from_key_and_data(key, object_data, context)
            },
            DeserializationContextKind::Data => {
                todo!()
            }
        }
    }

    fn serialize(&self, context: &SerializationContext<O::Project>) -> rmpv::Value {
        // If we already encoded this object somewhere in the given MessagePack value, return
        if context.stored.borrow().contains(&self.ptr.key) {
            return rmpv::Value::Ext(ALREADY_LOADED_MSGPACK_EXT_CODE, vec![]);
        } 
        context.stored.borrow_mut().insert(self.ptr.key);

        match &context.kind {
            SerializationContextKind::Shallow => {
                self.ptr.key.into()
            },
            SerializationContextKind::Deep { objects } => {
                let obj_data = O::list(objects).get(self.ptr).map(|obj| obj.serialize(context)).unwrap_or(rmpv::Value::Nil);
                rmpv::Value::Array(vec![
                    self.ptr.key.into(),
                    obj_data
                ])
            }
        }
    }

}
