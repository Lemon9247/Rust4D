-- scripts/hud.lua
--
-- Per-frame HUD readout for the demo. Calls the engine's `hud.*` bindings to
-- draw the player's 4D position, entity count, trigger state, and controls.
--
-- NOTE: as of Wave 5 the `hud.*` bindings are API-complete but render to the
-- log only (they are not yet wired into the render HudContext). The calls are
-- valid and exercise the full HUD API surface; a future wave that plugs a
-- HudContext into ScriptSystem::update will make them visible on screen with
-- no script changes. See the demo-game report's "Engine limitations" section.

local Hud = {}

-- Draw the HUD for one frame.
--   state = {
--     player_pos    = Vec4,        -- from audio.get_listener_position()
--     entity_count  = number,
--     trigger_inside = bool,
--     w_sliding      = bool,       -- whether Q/E is held this frame
--   }
function Hud.draw(state)
    local sw, sh = hud.screen_size()

    -- Title
    hud.text(16, 16, "Rust4D — Lua Demo Gallery", 22, {1.0, 1.0, 1.0, 1.0})

    -- Player 4D position
    local p = state.player_pos
    hud.text(
        16, 48,
        string.format("player  pos  x=%.2f  y=%.2f  z=%.2f  w=%.2f",
            p.x, p.y, p.z, p.w),
        16, {0.6, 0.9, 1.0, 1.0})

    -- W-slide indicator (demonstrates the input binding)
    local w_txt = state.w_sliding and "W-slide: active" or "W-slide: idle"
    local w_col = state.w_sliding and {0.4, 1.0, 0.7, 1.0} or {0.5, 0.5, 0.55, 1.0}
    hud.text(16, 70, w_txt, 16, w_col)

    -- Entity count
    hud.text(
        16, 92,
        string.format("entities: %d", state.entity_count),
        16, {0.75, 0.75, 0.78, 1.0})

    -- Trigger state
    local trig_txt = state.trigger_inside and "TRIGGER: ACTIVE" or "trigger: idle"
    local trig_col = state.trigger_inside
        and {1.0, 0.85, 0.20, 1.0}
        or  {0.55, 0.55, 0.55, 1.0}
    hud.text(16, 114, trig_txt, 16, trig_col)

    -- Trigger status bar (demonstrates hud.progress_bar)
    local prog = state.trigger_inside and 1.0 or 0.0
    hud.progress_bar(16, 138, 220, 14, prog,
        {0.18, 0.18, 0.20, 0.9}, {1.0, 0.85, 0.20, 1.0})

    -- Controls hint along the bottom
    hud.text(
        16, sh - 28,
        "WASD move · Q/E slide W · scroll = slice offset · walk to the center to trigger",
        14, {0.5, 0.5, 0.55, 1.0})
end

return Hud