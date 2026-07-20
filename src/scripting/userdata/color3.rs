use bevy::prelude::*;
use mlua::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color3(pub Color);

impl LuaUserData for Color3 {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(LuaMetaMethod::ToString, |_, this, _: ()| {
            let srgba = this.0.to_srgba();
            Ok(format!("{}, {}, {}", srgba.red, srgba.green, srgba.blue))
        });
        methods.add_meta_method(LuaMetaMethod::Eq, |_, this, other: LuaAnyUserData| {
            if let Ok(other_col) = other.borrow::<Color3>() {
                Ok(this.0 == other_col.0)
            } else {
                Ok(false)
            }
        });

        methods.add_meta_method(LuaMetaMethod::Index, |_, this, key: String| {
            let srgba = this.0.to_srgba();
            match key.as_str() {
                "R" | "r" => Ok(Some(srgba.red)),
                "G" | "g" => Ok(Some(srgba.green)),
                "B" | "b" => Ok(Some(srgba.blue)),
                _ => Ok(None),
            }
        });
    }
}
