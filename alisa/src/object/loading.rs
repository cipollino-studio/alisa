
use std::{cell::RefCell, collections::HashSet};

use crate::Project;

use super::{ObjPtr, Object};

enum LoadingContextKind<'a> {
    Local {
        file: &'a mut verter::File
    },
    Collab
}

pub struct LoadingContext<'a, P: Project> {
    kind: LoadingContextKind<'a>,
    objects: &'a mut P::Objects,
    /// The keys of the objects already loaded
    loaded: HashSet<u64>,
}

impl<'a, P: Project> LoadingContext<'a, P> {

    pub(crate) fn local(objects: &'a mut P::Objects, file: &'a mut verter::File) -> Self {
        Self {
            kind: LoadingContextKind::Local {
                file
            },
            objects,
            loaded: HashSet::new(),
        }
    }

    pub(crate) fn collab(objects: &'a mut P::Objects) -> Self {
        Self {
            kind: LoadingContextKind::Collab,
            objects,
            loaded: HashSet::new()
        }
    }

}

pub struct StoringContext<'a, P: Project> {
    objects: &'a P::Objects,
    file: RefCell<&'a mut verter::File>,
    /// The keys of the objects already stored
    stored: RefCell<HashSet<u64>>,
    /// Should we do a deep encoding(going through `ObjBox`s)?
    deep: bool
}

impl<'a, P: Project> StoringContext<'a, P> {

    pub(crate) fn shallow(objects: &'a P::Objects, file: &'a mut verter::File) -> Self {
        Self {
            objects,
            file: RefCell::new(file),
            stored: RefCell::new(HashSet::new()),
            deep: false,
        }
    }

    pub(crate) fn deep(objects: &'a P::Objects, file: &'a mut verter::File) -> Self {
        Self {
            objects,
            file: RefCell::new(file),
            stored: RefCell::new(HashSet::new()),
            deep: true,
        }
    }

}

pub trait Loadable<P: Project>: Sized {

    fn load(data: &rmpv::Value, context: &mut LoadingContext<P>) -> Option<Self>;
    fn store(&self, context: &StoringContext<P>) -> rmpv::Value; 

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

    fn load_from_key_and_data(key: u64, object_data: &rmpv::Value, context: &mut LoadingContext<O::Project>) -> Option<Self> {
        let obj_ptr = ObjPtr::from_key(key);
        if !matches!(object_data.as_ext(), Some((ALREADY_LOADED_MSGPACK_EXT_CODE, _))) {
            let object = O::load(&object_data, context)?;
            O::list_mut(context.objects).insert(obj_ptr, object);
        }
        Some(ObjBox { ptr: obj_ptr })
    } 

}

impl<O: Object> Loadable<O::Project> for ObjBox<O> {

    fn load(data: &rmpv::Value, context: &mut LoadingContext<O::Project>) -> Option<Self> {
        match &mut context.kind {
            LoadingContextKind::Local { file } => {
                let data = data.as_array()?;
                let key = data.get(0)?.as_u64()?;
                let ptr = data.get(1)?.as_u64()?;
                let obj_ptr = ObjPtr::from_key(key);

                // If the object is already loaded, skip loading it
                if O::list(context.objects).get(obj_ptr).is_some() || context.loaded.contains(&key) {
                    return Some(Self {
                        ptr: obj_ptr
                    });
                }
                context.loaded.insert(key);

                let object_data = file.read(ptr).ok()?; 
                let object_data = rmpv::decode::read_value(&mut object_data.as_slice()).ok()?;

                // Remember the file pointer
                O::list_mut(context.objects).file_ptrs.borrow_mut().insert(obj_ptr, ptr);

                Self::load_from_key_and_data(key, &object_data, context)
            },
            LoadingContextKind::Collab => {
                let data = data.as_array()?;
                let key = data.get(0)?.as_u64()?;
                let obj_ptr = ObjPtr::from_key(key);

                // If the object is already loaded, skip loading it
                if O::list(context.objects).get(obj_ptr).is_some() || context.loaded.contains(&key) {
                    return Some(Self {
                        ptr: obj_ptr
                    });
                }
                context.loaded.insert(key);

                let object_data = data.get(1)?;
                Self::load_from_key_and_data(key, object_data, context)
            },
        }
    }

    fn store(&self, context: &StoringContext<O::Project>) -> rmpv::Value {
        // If we already encoded this object somewhere in the given MessagePack value, return
        if context.stored.borrow().contains(&self.ptr.key) {
            return rmpv::Value::Ext(ALREADY_LOADED_MSGPACK_EXT_CODE, vec![]);
        } 
        context.stored.borrow_mut().insert(self.ptr.key);

        if context.deep {
            let data = O::list(context.objects).get(self.ptr).map(|obj| obj.store(context)).unwrap_or(rmpv::Value::Nil);
            rmpv::Value::Array(vec![self.ptr.key.into(), data])
        } else {
            let Some(ptr) = O::list(context.objects).get_file_ptr(self.ptr, &mut context.file.borrow_mut()) else { return rmpv::Value::Nil };
            rmpv::Value::Array(vec![self.ptr.key.into(), ptr.into()])
        }
    }

}

// pub struct ObjLoader<O: Object> {
//     ptr: ObjPtr<O>
// }

macro_rules! number_loadable_impl {
    ($T: ty, $N: ty) => {
        paste::paste! {
            impl<P: Project> Loadable<P> for $T {

                fn load(data: &rmpv::Value, _context: &mut LoadingContext<P>) -> Option<Self> {
                    data.[< as_ $N >]()?.try_into().ok()
                }

                fn store(&self, _context: &StoringContext<P>) -> rmpv::Value {
                    (*self as $N).into()
                }

            } 
        }
    };
}

number_loadable_impl!(i8,  i64);
number_loadable_impl!(i16, i64);
number_loadable_impl!(i32, i64);
number_loadable_impl!(i64, i64);
number_loadable_impl!(u8,  u64);
number_loadable_impl!(u16, u64);
number_loadable_impl!(u32, u64);
number_loadable_impl!(u64, u64);

impl<P: Project> Loadable<P> for f32 {

    fn load(data: &rmpv::Value, _context: &mut LoadingContext<P>) -> Option<Self> {
        Some(data.as_f64()? as f32)
    }

    fn store(&self, _context: &StoringContext<P>) -> rmpv::Value {
        (*self as f64).into()
    }

}

number_loadable_impl!(f64, f64);

impl<P: Project> Loadable<P> for String {

    fn load(data: &rmpv::Value, _context: &mut LoadingContext<P>) -> Option<Self> {
        Some(data.as_str()?.to_owned())
    }

    fn store(&self, _context: &StoringContext<P>) -> rmpv::Value {
        self.as_str().into()
    }

}

impl<P: Project, T: Loadable<P>> Loadable<P> for Vec<T> {

    fn load(data: &rmpv::Value, context: &mut LoadingContext<P>) -> Option<Self> {
        let Some(arr) = data.as_array() else { return Some(Vec::new()); };
        Some(arr.iter().filter_map(|element| T::load(element, context)).collect())
    }

    fn store(&self, context: &StoringContext<P>) -> rmpv::Value {
        rmpv::Value::Array(self.iter().map(|val| val.store(context)).collect())
    }

}

impl<O: Object> Loadable<O::Project> for ObjPtr<O> {

    fn load(data: &rmpv::Value, _context: &mut LoadingContext<O::Project>) -> Option<Self> {
        data.as_u64().map(Self::from_key)
    }

    fn store(&self, _context: &StoringContext<O::Project>) -> rmpv::Value {
        self.key.into()
    }

}
