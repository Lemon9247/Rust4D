-- games/demo/main.lua
--
-- Rust4D Wave 5 — Lua demo game entry point.
--
-- Run with:
--   R4D_SCENE__PATH=games/demo/scenes/demo.ron cargo run -- --game games/demo
--
-- This script drives a curated scene (games/demo/scenes/demo.ron) of six 4D
-- primitives in a ring around a central trigger zone. It exercises the full
-- Wave 5 scripting surface:
--   * ECS bridge   — world.find_by_name / entity:get / entity:set("transform")
--   * Math         — Rotor4.from_plane (W-involving planes morph the slice)
--   * Input        — input.is_key_pressed (W-slide indicator)
--   * Audio        — audio.load_sound / play_oneshot_spatial (spatial cue)
--   * HUD          — hud.text / progress_bar / flash / screen_size
--   * Triggers     — 4D AABB detection in on_fixed_update
--   * Tweens       — manual sine scale-pulse (TweenManager is not Lua-bound)
--
-- See games/demo/README.md for the feature matrix and controls.

local Rotator = require("scripts.rotator")
local TriggerZone = require("scripts.trigger_zone")
local Hud = require("scripts.hud")

-- Per-primitive spin config: plane (a W-involving bivector) + speed (rad/s).
-- Distinct planes make each primitive's 3D slice morph differently.
local GALLERY_SPIN = {
    tesseract        = { plane = "XW", speed =  0.60 },
    pentachoron      = { plane = "YW", speed =  0.70 },
    hexadecachoron   = { plane = "ZW", speed =  0.50 },
    icositetrachoron = { plane = "XW", speed = -0.45 },
    spherinder       = { plane = "YW", speed =  0.55 },
    duocylinder      = { plane = "ZW", speed =  0.65 },
}

local rotators = {}
local trigger = nil
local blip = nil   -- SoundHandle for trigger-enter, or nil if audio unavailable
local tone = nil   -- SoundHandle for trigger-exit, or nil
local frame = 0
-- Observable counts (globals so the headless smoke example + tooling can read
-- them back through engine.lua()).
gallery_count = 0
trigger_enters = 0
trigger_exits = 0

-- Look up a scene entity by name, failing loudly if it is missing.
local function must_find(name)
    local e = world.find_by_name(name)
    if e == nil then
        log.error(("[demo] missing scene entity: %s"):format(name))
    end
    return e
end

function on_init()
    log.info("[demo] on_init: building gallery rotators")

    for name, cfg in pairs(GALLERY_SPIN) do
        local e = must_find(name)
        if e then
            local t = e:get("transform")
            rotators[name] = Rotator.new(e, cfg.plane, cfg.speed, t.position, t.scale)
            gallery_count = gallery_count + 1
        end
    end

    -- Trigger zone at the ring center.
    local zone = must_find("trigger_zone")
    if zone then
        trigger = TriggerZone.new(zone, 1.5, on_trigger_enter, on_trigger_exit)
    end

    -- Audio cues. load_sound returns nil when audio is unavailable (headless /
    -- no device); the demo degrades gracefully — cues are skipped, the trigger
    -- still flashes + pulses.
    blip = audio.load_sound("games/demo/assets/blip.wav")
    tone = audio.load_sound("games/demo/assets/tone.wav")
    if blip then
        log.info("[demo] audio cues loaded (blip + tone)")
    else
        log.info("[demo] audio unavailable — cues will be skipped")
    end

    log.info(("[demo] on_init done: %d rotators, entity_count=%d")
        :format(gallery_count, world.entity_count()))
end

-- Trigger enter: spatial blip + HUD flash + (pulse handled in update).
function on_trigger_enter(self)
    trigger_enters = trigger_enters + 1
    log.info("[demo] trigger ENTER")
    if blip then
        audio.play_oneshot_spatial(blip, self.base_pos, 1.0, 25.0, "sfx")
    end
    hud.flash({0.30, 1.0, 0.85, 0.25})
end

-- Trigger exit: low tone + reset (pulse reset handled in update).
function on_trigger_exit(self)
    trigger_exits = trigger_exits + 1
    log.info("[demo] trigger EXIT")
    if tone then
        audio.play_oneshot_spatial(tone, self.base_pos, 1.0, 25.0, "sfx")
    end
end

function on_update(dt)
    frame = frame + 1

    -- Drive the gallery rotation. Each set("transform") marks geometry dirty so
    -- the render cache rebuilds and the motion is visible.
    for _, r in pairs(rotators) do
        r:update(dt)
    end

    -- Trigger pulse / reset visual.
    if trigger then
        trigger:update(dt)
    end

    -- W-slide input for the HUD indicator (demonstrates the input binding).
    local w_sliding = input.is_key_pressed("Q") or input.is_key_pressed("E")

    -- HUD readout. Player position comes from the audio listener, which the
    -- engine updates to the camera/player position each frame.
    local player_pos = audio.get_listener_position()
    Hud.draw({
        player_pos = player_pos,
        entity_count = world.entity_count(),
        trigger_inside = trigger and trigger.inside or false,
        w_sliding = w_sliding,
    })
end

function on_fixed_update(dt)
    -- Trigger-zone AABB check runs on the fixed timestep for stable detection.
    if trigger then
        local player_pos = audio.get_listener_position()
        trigger:tick(player_pos)
    end
end

function on_shutdown()
    log.info(("[demo] on_shutdown after %d frames"):format(frame))
end