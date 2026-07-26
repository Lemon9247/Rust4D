-- Trivial game: Lua lifecycle smoke test for the Rust4D scripting integration.
--
-- Run with: cargo run -- --game games/trivial
--
-- This script does nothing visible; it only proves the engine boots a scripted
-- game directory, fires on_init / on_update / on_fixed_update / on_shutdown,
-- and that the `log` and `world` bindings are available from Lua.

local frame_count = 0

function on_init()
    log.info("[trivial] on_init fired")
    -- world is available even in stub mode (P1) / against the live ECS (P2)
    if world then
        log.info(("[trivial] world entity_count = %d"):format(world.entity_count()))
    end
end

function on_update(dt)
    frame_count = frame_count + 1
    -- Log once every ~60 frames so the smoke test is visible but not noisy.
    if frame_count % 60 == 0 then
        log.info(("[trivial] on_update dt=%.4f frame=%d"):format(dt, frame_count))
    end
end

function on_fixed_update(dt)
    -- Intentionally empty; proves the fixed-step callback dispatches.
    if dt <= 0 then
        log.warn("[trivial] unexpected non-positive fixed dt")
    end
end

function on_shutdown()
    log.info(("[trivial] on_shutdown fired after %d frames"):format(frame_count))
end