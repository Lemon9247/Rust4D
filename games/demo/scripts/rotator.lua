-- scripts/rotator.lua
--
-- Per-entity 4D spin logic for the demo gallery. Each Rotator advances an
-- angle and writes a fresh Transform4D to its entity every frame, rotating in
-- a single bivector plane. Rotating in a W-involving plane (XW, YW, ZW) makes
-- the 3D slice morph over time as the slice hyperplane cuts different
-- orientations of the 4D shape — the signature visual of this demo.
--
-- Usage (from main.lua):
--   local Rotator = require("scripts.rotator")
--   local r = Rotator.new(entity, "XW", 0.6, base_pos, base_scale)
--   r:update(dt)
--
-- The transform table form {x=,y=,z=,w=,scale=,rotation=} is accepted by the
-- ECS bridge's transform setter; rotation must be a Rotor4 userdata.

local Rotator = {}
Rotator.__index = Rotator

-- Create a rotator bound to an entity.
--   entity     — LuaEntity from world.find_by_name
--   plane      — one of "XY","XZ","XW","YZ","YW","ZW"
--   speed      — radians per second
--   base_pos   — Vec4 userdata (the entity's authored position)
--   base_scale — number (the entity's authored scale)
function Rotator.new(entity, plane, speed, base_pos, base_scale)
    local self = setmetatable({}, Rotator)
    self.entity = entity
    self.plane = plane
    self.speed = speed
    self.angle = 0.0
    self.base_pos = base_pos
    self.base_scale = base_scale
    return self
end

-- Advance the rotation and write the transform. Returns the new angle.
function Rotator:update(dt)
    self.angle = self.angle + self.speed * dt
    -- Keep the angle bounded to avoid float drift over long runs.
    if self.angle > math.pi * 2.0 then
        self.angle = self.angle - math.pi * 2.0
    end
    local rotor = Rotor4.from_plane(self.plane, self.angle)
    self.entity:set("transform", {
        x = self.base_pos.x,
        y = self.base_pos.y,
        z = self.base_pos.z,
        w = self.base_pos.w,
        scale = self.base_scale,
        rotation = rotor,
    })
    return self.angle
end

return Rotator