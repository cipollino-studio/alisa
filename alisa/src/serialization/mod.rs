
use std::{cell::RefCell, collections::HashSet};

use crate::Project;

use super::{ObjPtr, Object};

mod serialization_impls;

enum DeserializationContextKind<'a, P: Project> {
    Local {
        file: &'a mut verter::File,
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

    pub(crate) fn local(objects: &'a mut P::Objects, file: &'a mut verter::File) -> Self {
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
    Shallow {
        file: RefCell<&'a mut verter::File>,
        objects: &'a P::Objects,
    },
    Deep {
        #[allow(unused)]
        file: RefCell<&'a mut verter::File>,
        objects: &'a P::Objects,
    },
    Data
}

pub struct SerializationContext<'a, P: Project> {
    kind: SerializationContextKind<'a, P>,
    /// The keys of the objects already stored
    stored: RefCell<HashSet<u64>>,
}

impl<'a, P: Project> SerializationContext<'a, P> {

    pub(crate) fn shallow(objects: &'a P::Objects, file: &'a mut verter::File) -> Self {
        Self {
            kind: SerializationContextKind::Shallow {
                file: RefCell::new(file),
                objects
            },
            stored: RefCell::new(HashSet::new()),
        }
    }

    pub(crate) fn deep(objects: &'a P::Objects, file: &'a mut verter::File) -> Self {
        Self {
            kind: SerializationContextKind::Deep {
                file: RefCell::new(file),
                objects
            },
            stored: RefCell::new(HashSet::new()),
        }
    }

    pub(crate) fn data() -> Self {
        Self {
            kind: SerializationContextKind::Data,
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
                let data = data.as_array()?;
                let key = data.get(0)?.as_u64()?;
                let ptr = data.get(1)?.as_u64()?;
                let obj_ptr = ObjPtr::from_key(key);

                // If the object is already loaded, skip loading it
                if O::list(objects).get(obj_ptr).is_some() || context.loaded.contains(&key) {
                    return Some(Self {
                        ptr: obj_ptr
                    });
                }
                context.loaded.insert(key);

                let object_data = file.read(ptr).ok()?; 
                let object_data = rmpv::decode::read_value(&mut object_data.as_slice()).ok()?;

                // Remember the file pointer
                O::list_mut(objects).file_ptrs.borrow_mut().insert(obj_ptr, ptr);

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
            SerializationContextKind::Shallow { file, objects } => {
                let Some(ptr) = O::list(objects).get_file_ptr(self.ptr, &mut file.borrow_mut()) else { return rmpv::Value::Nil };
                rmpv::Value::Array(vec![self.ptr.key.into(), ptr.into()])
            },
            SerializationContextKind::Deep { file: _, objects } => {
                let data = O::list(objects).get(self.ptr).map(|obj| obj.serialize(context)).unwrap_or(rmpv::Value::Nil);
                rmpv::Value::Array(vec![self.ptr.key.into(), data])
            },
            SerializationContextKind::Data => todo!(),
        }
    }

}
