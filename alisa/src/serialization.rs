
use crate::{ObjPtr, Object};

pub trait Serializable: Sized + Default {

    fn serialize(&self) -> rmpv::Value;
    fn deserialize(data: &rmpv::Value) -> Option<Self>;

}

impl Serializable for String {

    fn serialize(&self) -> rmpv::Value {
        self.as_str().into()
    }

    fn deserialize(data: &rmpv::Value) -> Option<Self> {
        data.as_str().map(|s| s.to_owned())
    }

}

impl<Obj: Object> Serializable for ObjPtr<Obj> {

    fn serialize(&self) -> rmpv::Value {
        self.key.into()
    }

    fn deserialize(data: &rmpv::Value) -> Option<Self> {
        data.as_u64().map(|key| Self::from_key(key))
    }

}

trait SignedInt: Into<rmpv::Value> + TryFrom<i64> + Copy + Default {}
impl SignedInt for i8 {}
impl SignedInt for i16 {}
impl SignedInt for i32 {}
impl SignedInt for i64 {}

impl<T: SignedInt> Serializable for T {
    fn serialize(&self) -> rmpv::Value {
        (*self).into()
    }

    fn deserialize(data: &rmpv::Value) -> Option<Self> {
        data.as_i64().map(|val| val.try_into().ok()).flatten()
    }
}

impl<T: Serializable> Serializable for Vec<T> {

    fn serialize(&self) -> rmpv::Value {
        rmpv::Value::Array(self.iter().map(|val| val.serialize()).collect())
    }

    fn deserialize(data: &rmpv::Value) -> Option<Self> {
        let data = data.as_array()?;
        Some(data.iter().filter_map(|elem_data| T::deserialize(elem_data)).collect())
    }

}
