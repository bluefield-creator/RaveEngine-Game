use super::vector3::Vector3;
use bevy::prelude::*;
use mlua::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CFrame {
    pub position: Vec3,
    pub rotation: Quat,
}

impl LuaUserData for CFrame {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(
            LuaMetaMethod::Mul,
            |lua, this, other: LuaValue| match other {
                LuaValue::UserData(ud) => {
                    if let Ok(other_cf) = ud.borrow::<CFrame>() {
                        let new_pos = this.position + this.rotation.mul_vec3(other_cf.position);
                        let new_rot = this.rotation * other_cf.rotation;
                        lua.create_userdata(CFrame {
                            position: new_pos,
                            rotation: new_rot,
                        })
                        .map(LuaValue::UserData)
                    } else if let Ok(other_vec) = ud.borrow::<Vector3>() {
                        let new_pos = this.position + this.rotation.mul_vec3(other_vec.0);
                        lua.create_userdata(Vector3(new_pos))
                            .map(LuaValue::UserData)
                    } else {
                        Err(mlua::Error::RuntimeError(
                            "Unsupported multiplier for CFrame".to_string(),
                        ))
                    }
                }
                _ => Err(mlua::Error::RuntimeError(
                    "Unsupported multiplier for CFrame".to_string(),
                )),
            },
        );

        methods.add_meta_method(LuaMetaMethod::ToString, |_, this, _: ()| {
            Ok(format!(
                "Position: {:?}, Rotation: {:?}",
                this.position, this.rotation
            ))
        });

        methods.add_meta_method(LuaMetaMethod::Index, |_, this, key: String| {
            match key.as_str() {
                "Position" => Ok(Some(Vector3(this.position))),
                "LookVector" => Ok(Some(Vector3(this.rotation.mul_vec3(Vec3::NEG_Z)))),
                "RightVector" => Ok(Some(Vector3(this.rotation.mul_vec3(Vec3::X)))),
                "UpVector" => Ok(Some(Vector3(this.rotation.mul_vec3(Vec3::Y)))),
                _ => Ok(None),
            }
        });
    }
}
