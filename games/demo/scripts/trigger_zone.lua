-- scripts/trigger_zone.lua
--
-- Trigger-zone logic for the demo. Detects the player entering/exiting a 4D
-- axis-aligned bounding box centered on the zone entity, and on enter fires an
-- audio cue + HUD flash + a scale pulse on the zone entity itself; on exit it
-- resets the scale and plays a low tone.
--
-- Player position source: `audio.get_listener_position()`. The engine updates
-- the audio listener to the camera/player position each frame (see
-- ScriptSystem::update), so this is the live player position — one frame stale,
-- which is negligible for a trigger. When audio is unavailable (headless / no
-- device) the listener position reads as the zero vector; see the report's
-- "Engine limitations" section for why that is an acceptable degradation.
--
-- The scale pulse is a manual sine tween (`scale = base * (1 + 0.3*sin(t))`).
-- The engine's TweenManager is not exposed to Lua (no binding), so the pulse is
-- implemented by hand here.

local TriggerZone = {}
TriggerZone.__index = TriggerZone

-- Create a trigger zone.
--   entity     — LuaEntity for the zone (find_by_name("trigger_zone"))
--   half       — half-extent of the 4D AABB around the zone center
--   on_enter   — optional callback(self) fired on a fresh enter
--   on_exit    — optional callback(self) fired on a fresh exit
function TriggerZone.new(entity, half, on_enter, on_exit)
    local self = setmetatable({}, TriggerZone)
    self.entity = entity
    self.half = half
    self.on_enter = on_enter
    self.on_exit = on_exit
    self.inside = false
    self.t = 0.0 -- pulse accumulator (seconds since enter)

    -- Cache the authored transform so we can pulse scale without drift.
    local base = entity:get("transform")
    self.base_pos = base.position      -- Vec4 userdata
    self.base_scale = base.scale       -- number
    return self
end

-- AABB containment test against a player position (Vec4 userdata).
-- Returns true if the player is within the 4D box.
function TriggerZone:contains(player_pos)
    local h = self.half
    local p = self.base_pos
    return math.abs(player_pos.x - p.x) <= h
        and math.abs(player_pos.y - p.y) <= h
        and math.abs(player_pos.z - p.z) <= h
        and math.abs(player_pos.w - p.w) <= h
end

-- Run the enter/exit state machine. Call from on_fixed_update.
-- Returns "enter", "exit", or nil for this tick.
function TriggerZone:tick(player_pos)
    local within = self:contains(player_pos)
    if within and not self.inside then
        self.inside = true
        self.t = 0.0
        if self.on_enter then self.on_enter(self) end
        return "enter"
    elseif not within and self.inside then
        self.inside = false
        if self.on_exit then self.on_exit(self) end
        return "exit"
    end
    return nil
end

-- Per-frame visual update. Pulses scale while the player is inside; otherwise
-- restores the base scale. Call from on_update.
function TriggerZone:update(dt)
    if self.inside then
        self.t = self.t + dt
        local s = self.base_scale * (1.0 + 0.3 * math.sin(self.t * 6.0))
        self:_write_scale(s)
    else
        self:_write_scale(self.base_scale)
    end
end

-- Write a transform that preserves the authored position and rotation but
-- swaps in a new scale.
function TriggerZone:_write_scale(scale)
    self.entity:set("transform", {
        x = self.base_pos.x,
        y = self.base_pos.y,
        z = self.base_pos.z,
        w = self.base_pos.w,
        scale = scale,
        rotation = Rotor4.identity(),
    })
end

return TriggerZone