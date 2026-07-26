//! ECS bindings for Lua — real `hecs::World` bridge
//!
//! Provides Lua access to the live `rust4d_core::World` (a thin wrapper around
//! `hecs::World` with name/tag/dirty side-tables) via a per-call [`WorldRef`]
//! registered into `app_data` by `ScriptSystem`.
//!
//! # Component registry
//!
//! Components are referred to by string name. The engine's component set is
//! small and fixed, so the bridge dispatches on an explicit `match` (compile-time
//! checked, verbose by design):
//!
//! | name             | Rust component   | get | set | spawn |
//! |------------------|------------------|:---:|:---:|:-----:|
//! | `"name"`         | `Name`           | ✅  | ✅  | ✅    |
//! | `"tags"`         | `Tags`           | ✅  | —   | ✅    |
//! | `"transform"`    | `Transform4D`    | ✅  | ✅  | ✅    |
//! | `"material"`     | `Material`       | ✅  | ✅  | ✅    |
//! | `"dirty"`        | `DirtyFlags`     | ✅  | ✅  | ✅    |
//! | `"shape"`        | `ShapeRef`       | ✅  | —   | —     |
//! | `"physics_body"` | `PhysicsBody`    | —   | —   | —     |
//! | `"parent"`       | `Parent`         | ✅  | —   | —     |
//! | `"children"`     | `Children`       | ✅  | —   | —     |
//!
//! Shapes and physics bodies cannot be created from Lua (they require GPU
//! geometry / physics-world registration); scripts instead rotate or recolour
//! scene-placed entities, or spawn marker entities (name/transform/material)
//! for game logic.

use std::collections::HashSet;
use std::rc::Rc;

use mlua::prelude::*;
use rust4d_core::hecs::{Entity, EntityBuilder};
use rust4d_core::{
    Children, DirtyFlags, Material, Name, Parent, PhysicsBody, ShapeRef, Tags, Transform4D, World,
};
use rust4d_math::Rotor4;

use super::math::{LuaRotor4, LuaTransform4D};
use crate::context::{ScriptMutations, WorldRef};

/// Canonical component names accepted by `world.spawn` / `entity:get` / `entity:set`.
pub const COMPONENT_NAMES: &[&str] = &[
    "name",
    "tags",
    "transform",
    "material",
    "dirty",
    "shape",
    "physics_body",
    "parent",
    "children",
];

/// Lua-side entity handle wrapping a real `hecs::Entity`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LuaEntity(pub Entity);

impl LuaEntity {
    /// Wrap a real `hecs::Entity` for Lua.
    pub fn from_entity(entity: Entity) -> Self {
        Self(entity)
    }

    /// Reconstruct a Lua entity handle from its stable hecs bit pattern.
    ///
    /// Returns `None` for stale handles (e.g. after the entity was despawned).
    pub fn from_bits(bits: u64) -> Option<Self> {
        Entity::from_bits(bits).map(Self)
    }
}

impl FromLua for LuaEntity {
    fn from_lua(value: LuaValue, _lua: &Lua) -> LuaResult<Self> {
        match value {
            LuaValue::UserData(ud) => ud.borrow::<LuaEntity>().map(|e| *e),
            _ => Err(LuaError::FromLuaConversionError {
                from: value.type_name(),
                to: "Entity".to_string(),
                message: Some("expected Entity userdata".to_string()),
            }),
        }
    }
}

impl LuaUserData for LuaEntity {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        // entity:id() -> u64
        methods.add_method("id", |_, this, ()| Ok(this.0.id() as u64));

        // entity:to_bits() -> u64
        methods.add_method("to_bits", |_, this, ()| Ok(this.0.to_bits().get()));

        // entity:equals(other) -> bool
        methods.add_method("equals", |_, this, other: LuaAnyUserData| {
            let other = other.borrow::<LuaEntity>()?;
            Ok(this.0 == other.0)
        });

        // entity:get(component_name) -> value or nil
        methods.add_method("get", |lua, this, component: String| {
            with_world(lua, |world| component_get(lua, world, this.0, &component))
        });

        // entity:set(component_name, value)
        methods.add_method(
            "set",
            |lua, this, (component, value): (String, LuaValue)| {
                with_world(lua, |world| component_set(world, this.0, &component, value))?;
                mark_dirty(lua);
                Ok(())
            },
        );

        // entity:is_alive() -> bool
        methods.add_method("is_alive", |lua, this, ()| {
            with_world(lua, |world| Ok(world.contains(this.0)))
        });
    }
}

// === Per-call World access helper ===

/// Run `f` against the live `World` registered for this callback.
fn with_world<F, R>(lua: &Lua, f: F) -> LuaResult<R>
where
    F: FnOnce(&mut World) -> LuaResult<R>,
{
    let ptr = lua
        .app_data_ref::<WorldRef>()
        .ok_or_else(|| LuaError::RuntimeError("ECS world not available".into()))?
        .0;
    // SAFETY: `ScriptSystem` registers the WorldRef immediately before
    // dispatching the callback and clears it afterwards, and does not touch the
    // World while the callback runs. The pointer is valid for the call.
    let world = unsafe { &mut *ptr };
    f(world)
}

/// Mark that scripts mutated the world so `ScriptSystem` rebuilds geometry.
fn mark_dirty(lua: &Lua) {
    if let Some(m) = lua.app_data_ref::<ScriptMutations>() {
        m.mark_dirty();
    }
}

fn no_component(_: rust4d_core::hecs::ComponentError) -> LuaError {
    LuaError::RuntimeError("entity does not have that component".into())
}

// === Component registry: spawn ===

/// Build a component bundle from a Lua table of `{ name = value, ... }` and
/// spawn it into the world. Returns the real `hecs::Entity`.
fn spawn_from_table(world: &mut World, components: &LuaTable) -> LuaResult<Entity> {
    let mut builder = EntityBuilder::new();
    let mut had_transform = false;
    let mut had_material = false;
    let mut had_dirty = false;

    for pair in components.pairs::<String, LuaValue>() {
        let (name, value) = pair?;
        match name.as_str() {
            "name" => {
                builder.add(Name(parse_name(&value)?));
            }
            "tags" => {
                builder.add(parse_tags(&value)?);
            }
            "transform" => {
                builder.add(parse_transform(&value)?);
                had_transform = true;
            }
            "material" => {
                builder.add(parse_material(&value)?);
                had_material = true;
            }
            "dirty" | "dirty_flags" => {
                builder.add(parse_dirty(&value)?);
                had_dirty = true;
            }
            "shape" | "physics_body" | "parent" | "children" => {
                log::warn!(
                    "[ecs] world.spawn: component '{}' cannot be created from Lua; ignoring",
                    name
                );
            }
            other => {
                log::warn!("[ecs] world.spawn: unknown component '{}'; ignoring", other);
            }
        }
    }

    // Default a newly-spawned renderable entity to dirty so the geometry cache
    // rebuilds. Marker-only entities (no transform/material) default to NONE.
    if !had_dirty {
        builder.add(if had_transform || had_material {
            DirtyFlags::ALL
        } else {
            DirtyFlags::NONE
        });
    }

    Ok(world.spawn(builder.build()))
}

// === Component registry: get ===

fn component_get(lua: &Lua, world: &mut World, entity: Entity, name: &str) -> LuaResult<LuaValue> {
    if !world.contains(entity) {
        return Ok(LuaValue::Nil);
    }
    let ecs = world.ecs();
    match name {
        "name" => {
            let n = ecs.get::<&Name>(entity).map_err(no_component)?;
            Ok(LuaValue::String(lua.create_string(n.0.as_bytes())?))
        }
        "tags" => {
            let tags = ecs.get::<&Tags>(entity).map_err(no_component)?;
            let table = lua.create_table()?;
            for (i, tag) in tags.0.iter().enumerate() {
                table.set(i + 1, lua.create_string(tag.as_bytes())?)?;
            }
            drop(tags);
            Ok(LuaValue::Table(table))
        }
        "transform" => {
            let t = *ecs.get::<&Transform4D>(entity).map_err(no_component)?;
            LuaTransform4D {
                position: t.position,
                rotation: t.rotation,
                scale: t.scale,
            }
            .into_lua(lua)
        }
        "material" => {
            let m = ecs.get::<&Material>(entity).map_err(no_component)?;
            let table = lua.create_table()?;
            table.set(1, m.base_color[0])?;
            table.set(2, m.base_color[1])?;
            table.set(3, m.base_color[2])?;
            table.set(4, m.base_color[3])?;
            drop(m);
            Ok(LuaValue::Table(table))
        }
        "dirty" | "dirty_flags" => {
            let d = ecs.get::<&DirtyFlags>(entity).map_err(no_component)?;
            let table = lua.create_table()?;
            table.set("transform", d.contains(DirtyFlags::TRANSFORM))?;
            table.set("mesh", d.contains(DirtyFlags::MESH))?;
            table.set("material", d.contains(DirtyFlags::MATERIAL))?;
            drop(d);
            Ok(LuaValue::Table(table))
        }
        "shape" => {
            let s = ecs.get::<&ShapeRef>(entity).map_err(no_component)?;
            let table = lua.create_table()?;
            table.set("vertex_count", s.as_shape().vertex_count() as i64)?;
            drop(s);
            Ok(LuaValue::Table(table))
        }
        "parent" => {
            let p = ecs.get::<&Parent>(entity).map_err(no_component)?;
            let entity = LuaEntity::from_entity(p.0);
            drop(p);
            entity.into_lua(lua)
        }
        "children" => {
            let c = ecs.get::<&Children>(entity).map_err(no_component)?;
            let table = lua.create_table()?;
            for (i, child) in c.0.iter().enumerate() {
                table.set(i + 1, LuaEntity::from_entity(*child))?;
            }
            drop(c);
            Ok(LuaValue::Table(table))
        }
        "physics_body" => Ok(LuaValue::Nil),
        other => Err(LuaError::RuntimeError(format!(
            "unknown component '{}'. Valid: {:?}",
            other, COMPONENT_NAMES
        ))),
    }
}

// === Component registry: set ===

fn component_set(world: &mut World, entity: Entity, name: &str, value: LuaValue) -> LuaResult<()> {
    if !world.contains(entity) {
        return Err(LuaError::RuntimeError("entity is not alive".into()));
    }
    match name {
        "name" => {
            let new_name = match value {
                LuaValue::String(s) => s.to_str()?.to_string(),
                other => {
                    return Err(LuaError::RuntimeError(format!(
                        "name must be a string, got {}",
                        other.type_name()
                    )))
                }
            };
            if world.rename_entity(entity, new_name).is_none() {
                return Err(LuaError::RuntimeError(
                    "entity has no Name component to rename".into(),
                ));
            }
            Ok(())
        }
        "transform" => {
            let t = parse_transform(&value)?;
            let ecs = world.ecs_mut_unchecked();
            let mut tx = ecs.get::<&mut Transform4D>(entity).map_err(no_component)?;
            *tx = t;
            drop(tx);
            mark_transform_dirty(world, entity);
            Ok(())
        }
        "material" => {
            let m = parse_material(&value)?;
            let ecs = world.ecs_mut_unchecked();
            let mut mat = ecs.get::<&mut Material>(entity).map_err(no_component)?;
            *mat = m;
            drop(mat);
            mark_dirty_flag(world, entity, DirtyFlags::MATERIAL);
            Ok(())
        }
        "dirty" | "dirty_flags" => {
            let d = parse_dirty(&value)?;
            let ecs = world.ecs_mut_unchecked();
            let mut flags = ecs.get::<&mut DirtyFlags>(entity).map_err(no_component)?;
            *flags = d;
            Ok(())
        }
        "tags" => Err(LuaError::RuntimeError(
            "setting tags after spawn is not supported; spawn with tags instead".into(),
        )),
        "shape" | "physics_body" | "parent" | "children" => Err(LuaError::RuntimeError(format!(
            "component '{}' cannot be set from Lua",
            name
        ))),
        other => Err(LuaError::RuntimeError(format!(
            "unknown component '{}'. Valid: {:?}",
            other, COMPONENT_NAMES
        ))),
    }
}

/// Set the `TRANSFORM` dirty flag on an entity (so a geometry rebuild picks it
/// up) without requiring the caller to hold a hecs borrow.
fn mark_transform_dirty(world: &mut World, entity: Entity) {
    mark_dirty_flag(world, entity, DirtyFlags::TRANSFORM);
}

fn mark_dirty_flag(world: &mut World, entity: Entity, flag: DirtyFlags) {
    if let Ok(mut d) = world.ecs_mut_unchecked().get::<&mut DirtyFlags>(entity) {
        *d |= flag;
    }
}

// === Component registry: query ===

/// Collect all entities that have the named component.
fn collect_query(world: &mut World, component: &str) -> LuaResult<Vec<Entity>> {
    let ecs = world.ecs();
    let entities: Vec<Entity> = match component {
        "name" => ecs.query::<&Name>().iter().map(|(e, _)| e).collect(),
        "tags" => ecs.query::<&Tags>().iter().map(|(e, _)| e).collect(),
        "transform" => ecs.query::<&Transform4D>().iter().map(|(e, _)| e).collect(),
        "material" => ecs.query::<&Material>().iter().map(|(e, _)| e).collect(),
        "dirty" | "dirty_flags" => ecs.query::<&DirtyFlags>().iter().map(|(e, _)| e).collect(),
        "shape" => ecs.query::<&ShapeRef>().iter().map(|(e, _)| e).collect(),
        "physics_body" => ecs.query::<&PhysicsBody>().iter().map(|(e, _)| e).collect(),
        "parent" => ecs.query::<&Parent>().iter().map(|(e, _)| e).collect(),
        "children" => ecs.query::<&Children>().iter().map(|(e, _)| e).collect(),
        other => {
            return Err(LuaError::RuntimeError(format!(
                "unknown component '{}'. Valid: {:?}",
                other, COMPONENT_NAMES
            )))
        }
    };
    Ok(entities)
}

// === Parsers (Lua value -> Rust component) ===

fn parse_name(value: &LuaValue) -> LuaResult<String> {
    match value {
        LuaValue::String(s) => Ok(s.to_str()?.to_string()),
        other => Err(LuaError::RuntimeError(format!(
            "name must be a string, got {}",
            other.type_name()
        ))),
    }
}

fn parse_tags(value: &LuaValue) -> LuaResult<Tags> {
    match value {
        LuaValue::Table(t) => {
            let mut set = HashSet::new();
            for entry in t.sequence_values::<LuaString>() {
                let s = entry?;
                set.insert(s.to_str()?.to_string());
            }
            Ok(Tags(set))
        }
        LuaValue::Nil => Ok(Tags::new()),
        other => Err(LuaError::RuntimeError(format!(
            "tags must be a table of strings, got {}",
            other.type_name()
        ))),
    }
}

fn parse_transform(value: &LuaValue) -> LuaResult<Transform4D> {
    match value {
        LuaValue::UserData(ud) => {
            let lt = ud.borrow::<LuaTransform4D>()?;
            Ok(Transform4D {
                position: lt.position,
                rotation: lt.rotation,
                scale: lt.scale,
            })
        }
        LuaValue::Table(t) => {
            // Accept {x=,y=,z=,w=} for position, optional scale= and rotation=.
            if t.contains_key("x")? || t.contains_key("y")? {
                let x: f32 = t.get("x").unwrap_or(0.0);
                let y: f32 = t.get("y").unwrap_or(0.0);
                let z: f32 = t.get("z").unwrap_or(0.0);
                let w: f32 = t.get("w").unwrap_or(0.0);
                let scale: f32 = t.get("scale").unwrap_or(1.0);
                let rotation = match t.get::<LuaValue>("rotation") {
                    Ok(LuaValue::UserData(ud)) => ud.borrow::<LuaRotor4>()?.0,
                    _ => Rotor4::IDENTITY,
                };
                Ok(Transform4D {
                    position: rust4d_math::Vec4::new(x, y, z, w),
                    rotation,
                    scale,
                })
            } else if t.contains_key("position")? {
                let pos = parse_vec4_table(&t.get::<LuaTable>("position")?)?;
                let scale: f32 = t.get("scale").unwrap_or(1.0);
                let rotation = match t.get::<LuaValue>("rotation") {
                    Ok(LuaValue::UserData(ud)) => ud.borrow::<LuaRotor4>()?.0,
                    _ => Rotor4::IDENTITY,
                };
                Ok(Transform4D {
                    position: pos,
                    rotation,
                    scale,
                })
            } else {
                // Bare array {x, y, z, w}
                let x: f32 = t.get(1).unwrap_or(0.0);
                let y: f32 = t.get(2).unwrap_or(0.0);
                let z: f32 = t.get(3).unwrap_or(0.0);
                let w: f32 = t.get(4).unwrap_or(0.0);
                Ok(Transform4D::from_position(rust4d_math::Vec4::new(
                    x, y, z, w,
                )))
            }
        }
        other => Err(LuaError::RuntimeError(format!(
            "transform must be a Transform4D or table, got {}",
            other.type_name()
        ))),
    }
}

fn parse_vec4_table(t: &LuaTable) -> LuaResult<rust4d_math::Vec4> {
    if t.contains_key("x")? {
        let x: f32 = t.get("x").unwrap_or(0.0);
        let y: f32 = t.get("y").unwrap_or(0.0);
        let z: f32 = t.get("z").unwrap_or(0.0);
        let w: f32 = t.get("w").unwrap_or(0.0);
        Ok(rust4d_math::Vec4::new(x, y, z, w))
    } else {
        let x: f32 = t.get(1).unwrap_or(0.0);
        let y: f32 = t.get(2).unwrap_or(0.0);
        let z: f32 = t.get(3).unwrap_or(0.0);
        let w: f32 = t.get(4).unwrap_or(0.0);
        Ok(rust4d_math::Vec4::new(x, y, z, w))
    }
}

fn parse_material(value: &LuaValue) -> LuaResult<Material> {
    match value {
        LuaValue::Table(t) => {
            let [r, g, b, a] = parse_color(t)?;
            Ok(Material::new(r, g, b, a))
        }
        other => Err(LuaError::RuntimeError(format!(
            "material must be a color table, got {}",
            other.type_name()
        ))),
    }
}

/// Parse an RGBA color from a Lua table (array `{r,g,b,a}` or named `{r=,g=,b=,a=}`).
fn parse_color(t: &LuaTable) -> LuaResult<[f32; 4]> {
    if t.contains_key("r")? {
        let r: f32 = t.get("r").unwrap_or(0.0);
        let g: f32 = t.get("g").unwrap_or(0.0);
        let b: f32 = t.get("b").unwrap_or(0.0);
        let a: f32 = t.get("a").unwrap_or(1.0);
        Ok([r, g, b, a])
    } else {
        let r: f32 = t.get(1).unwrap_or(0.0);
        let g: f32 = t.get(2).unwrap_or(0.0);
        let b: f32 = t.get(3).unwrap_or(0.0);
        let a: f32 = t.get(4).unwrap_or(1.0);
        Ok([r, g, b, a])
    }
}

fn parse_dirty(value: &LuaValue) -> LuaResult<DirtyFlags> {
    match value {
        LuaValue::Boolean(true) => Ok(DirtyFlags::ALL),
        LuaValue::Boolean(false) => Ok(DirtyFlags::NONE),
        LuaValue::Integer(n) => Ok(DirtyFlags::from_bits_truncate(*n as u8)),
        LuaValue::Number(n) => Ok(DirtyFlags::from_bits_truncate(*n as u8)),
        LuaValue::Table(t) => {
            let mut flags = DirtyFlags::NONE;
            if t.get::<bool>("transform").unwrap_or(false) {
                flags |= DirtyFlags::TRANSFORM;
            }
            if t.get::<bool>("mesh").unwrap_or(false) {
                flags |= DirtyFlags::MESH;
            }
            if t.get::<bool>("material").unwrap_or(false) {
                flags |= DirtyFlags::MATERIAL;
            }
            Ok(flags)
        }
        LuaValue::Nil => Ok(DirtyFlags::ALL),
        other => Err(LuaError::RuntimeError(format!(
            "dirty must be a bool, number, or table, got {}",
            other.type_name()
        ))),
    }
}

/// Register ECS bindings with the Lua VM.
pub fn register(lua: &Lua) -> LuaResult<()> {
    let world_table = lua.create_table()?;

    // world.spawn(components) -> LuaEntity
    world_table.set(
        "spawn",
        lua.create_function(|lua, components: LuaTable| {
            let entity = with_world(lua, |world| spawn_from_table(world, &components))?;
            mark_dirty(lua);
            Ok(LuaEntity::from_entity(entity))
        })?,
    )?;

    // world.query(component_name) -> iterator function yielding LuaEntity
    world_table.set(
        "query",
        lua.create_function(|lua, component: String| {
            let entities = with_world(lua, |world| collect_query(world, &component))?;
            let index = Rc::new(std::cell::Cell::new(0usize));
            let entities = Rc::new(entities);
            let iter = lua.create_function(move |_, ()| {
                let i = index.get();
                if i >= entities.len() {
                    Ok(Option::<LuaEntity>::None)
                } else {
                    index.set(i + 1);
                    Ok(Some(LuaEntity::from_entity(entities[i])))
                }
            })?;
            Ok(iter)
        })?,
    )?;

    // world.find_by_name(name) -> LuaEntity or nil
    world_table.set(
        "find_by_name",
        lua.create_function(|lua, name: String| {
            with_world(lua, |world| {
                Ok(world.get_by_name(&name).map(LuaEntity::from_entity))
            })
        })?,
    )?;

    // world.entity_from_bits(bits) -> LuaEntity or nil
    world_table.set(
        "entity_from_bits",
        lua.create_function(|_, bits: u64| Ok(LuaEntity::from_bits(bits)))?,
    )?;

    // world.despawn(entity) -> bool
    world_table.set(
        "despawn",
        lua.create_function(|lua, entity: LuaEntity| {
            let removed = with_world(lua, |world| Ok(world.despawn(entity.0)))?;
            if removed {
                mark_dirty(lua);
            }
            Ok(removed)
        })?,
    )?;

    // world.entity_count() -> u64
    world_table.set(
        "entity_count",
        lua.create_function(|lua, ()| with_world(lua, |world| Ok(world.entity_count() as u64)))?,
    )?;

    // world.contains(entity) -> bool
    world_table.set(
        "contains",
        lua.create_function(|lua, entity: LuaEntity| {
            with_world(lua, |world| Ok(world.contains(entity.0)))
        })?,
    )?;

    lua.globals().set("world", world_table)?;

    log::debug!("[ecs] ECS bindings registered (live hecs bridge)");
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::bindings::math;
    use crate::context::{ScriptMutations, WorldRef};
    use rust4d_core::World;

    /// Register ecs + math bindings on a fresh Lua.
    fn lua_with_bindings() -> Lua {
        let lua = Lua::new();
        math::register(&lua).unwrap();
        register(&lua).unwrap();
        lua
    }

    /// Register a WorldRef + fresh ScriptMutations for the duration of a Lua
    /// snippet, then clear them so the caller may inspect `world` afterwards.
    fn run_with_world(lua: &Lua, world: &mut World, code: &str) {
        lua.set_app_data(WorldRef::new(world));
        lua.set_app_data(ScriptMutations::default());
        lua.load(code).exec().unwrap();
        lua.remove_app_data::<ScriptMutations>();
        lua.remove_app_data::<WorldRef>();
    }

    #[test]
    fn test_spawn_creates_real_entity() {
        let lua = lua_with_bindings();
        let mut world = World::new();
        run_with_world(
            &lua,
            &mut world,
            r#"
            e = world.spawn({ name = "foo", transform = { x = 1, y = 2, z = 3, w = 4 } })
            assert(e ~= nil)
            assert(e:id() >= 0)
        "#,
        );
        assert_eq!(world.entity_count(), 1);
        assert!(world.get_by_name("foo").is_some());
    }

    #[test]
    fn test_find_by_name() {
        let lua = lua_with_bindings();
        let mut world = World::new();
        run_with_world(
            &lua,
            &mut world,
            r#"
            world.spawn({ name = "alpha" })
            world.spawn({ name = "beta" })
            local e = world.find_by_name("beta")
            assert(e ~= nil, "find_by_name should find beta")
            missing = world.find_by_name("nope")
        "#,
        );
        assert!(lua.load("return missing == nil").eval::<bool>().unwrap());
    }

    #[test]
    fn test_query_yields_entities() {
        let lua = lua_with_bindings();
        let mut world = World::new();
        // Seed a transform-bearing entity from Rust.
        world.spawn((Transform4D::identity(), DirtyFlags::NONE));
        run_with_world(
            &lua,
            &mut world,
            r#"
            world.spawn({ transform = { x = 1, y = 0, z = 0, w = 0 } })
            count = 0
            for _ in world.query("transform") do
                count = count + 1
            end
        "#,
        );
        let count: i64 = lua.load("return count").eval().unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_get_set_transform_round_trip() {
        let lua = lua_with_bindings();
        let mut world = World::new();
        run_with_world(
            &lua,
            &mut world,
            r#"
            e = world.spawn({ transform = { x = 0, y = 0, z = 0, w = 0 } })
            e:set("transform", { x = 5, y = 6, z = 7, w = 8 })
            t = e:get("transform")
            assert(t.position.x == 5, "x should be 5")
            assert(t.position.w == 8, "w should be 8")
        "#,
        );
        // Verify from Rust that the transform actually changed.
        let e = world
            .get_by_name("")
            .or(world.root_entities().first().copied());
        let _ = e;
        let entity = world.root_entities().pop().unwrap();
        let t = world.ecs().get::<&Transform4D>(entity).unwrap();
        assert_eq!(t.position.x, 5.0);
        assert_eq!(t.position.w, 8.0);
    }

    #[test]
    fn test_set_material_and_get() {
        let lua = lua_with_bindings();
        let mut world = World::new();
        run_with_world(
            &lua,
            &mut world,
            r#"
            e = world.spawn({ material = { 1, 0, 0, 1 } })
            m = e:get("material")
            assert(m[1] == 1.0)
            assert(m[2] == 0.0)
            e:set("material", { r = 0, g = 1, b = 0, a = 1 })
            m2 = e:get("material")
            assert(m2[2] == 1.0, "green should be 1 after set")
        "#,
        );
    }

    #[test]
    fn test_despawn_removes_entity() {
        let lua = lua_with_bindings();
        let mut world = World::new();
        run_with_world(
            &lua,
            &mut world,
            r#"
            e = world.spawn({ name = "temp" })
            removed = world.despawn(e)
            assert(removed, "despawn should return true")
            assert(world.find_by_name("temp") == nil)
        "#,
        );
        assert_eq!(world.entity_count(), 0);
    }

    #[test]
    fn test_entity_count() {
        let lua = lua_with_bindings();
        let mut world = World::new();
        run_with_world(
            &lua,
            &mut world,
            r#"
            world.spawn({ name = "a" })
            world.spawn({ name = "b" })
            n = world.entity_count()
        "#,
        );
        let n: i64 = lua.load("return n").eval().unwrap();
        assert_eq!(n, 2);
    }

    #[test]
    fn test_is_alive() {
        let lua = lua_with_bindings();
        let mut world = World::new();
        run_with_world(
            &lua,
            &mut world,
            r#"
            e = world.spawn({ name = "x" })
            alive_before = e:is_alive()
            world.despawn(e)
            alive_after = e:is_alive()
        "#,
        );
        assert!(lua.load("return alive_before").eval::<bool>().unwrap());
        assert!(!lua.load("return alive_after").eval::<bool>().unwrap());
    }

    #[test]
    fn test_entity_bits_round_trip() {
        let lua = lua_with_bindings();
        let mut world = World::new();
        run_with_world(
            &lua,
            &mut world,
            r#"
            e = world.spawn({ name = "rt" })
            bits = e:to_bits()
            e2 = world.entity_from_bits(bits)
            assert(e2 ~= nil)
            assert(e:equals(e2))
        "#,
        );
    }

    #[test]
    fn test_unknown_component_errors() {
        let lua = lua_with_bindings();
        let mut world = World::new();
        run_with_world(
            &lua,
            &mut world,
            r#"
            e = world.spawn({ name = "u" })
            ok, err = pcall(function() e:get("nope") end)
            assert(not ok, "get on unknown component should error")
        "#,
        );
        let ok: bool = lua.load("return ok").eval().unwrap();
        assert!(!ok);
    }

    #[test]
    fn test_spawn_without_world_errors() {
        let lua = lua_with_bindings();
        // No WorldRef registered.
        let result: LuaResult<()> = lua.load("world.spawn({ name = 'x' })").eval();
        assert!(result.is_err(), "spawn without a world should error");
    }
}
