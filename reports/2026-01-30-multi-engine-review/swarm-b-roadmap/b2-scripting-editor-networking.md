# Agent B2: Scripting, Visual Editor & Networking Feasibility Assessment

**Agent**: B2 - Scripting, Editor & Networking Feasibility Assessor
**Date**: 2026-01-30
**Scope**: Long-term plans for Scripting (P6), Visual Editor (P7), and Networking (P8)
**Lens**: Feasibility against current codebase + relevance to 4D boomer shooter goal

---

## 1. Scripting System Assessment

### Source Document
`/home/lemoneater/Projects/Rust4D/scratchpad/plans/engine-roadmap-2026/long-term-scripting.md`

### 1.1 Trigger Condition Status

The plan lists five trigger conditions (lines 537-543). Current status:

| Trigger Condition | Met? | Evidence |
|---|---|---|
| Core engine is stable | Partial | Physics, scene management, and rendering work. But no gameplay mechanics exist yet. |
| Phase 1-3 complete | Yes | Scene serialization, management, async loading all done. |
| API is stable | No | Entity API still uses monolithic `Entity` struct (`entity.rs` line 121-136). No ECS migration done. |
| There's a need | Not yet | No gameplay code exists that would benefit from scripting. |
| Willow approves | Unknown | N/A |

**Verdict**: Only 1 of 5 trigger conditions is fully met. The API stability condition is particularly important -- if an ECS migration happens, the entire scripting API surface would need to be redesigned.

### 1.2 Boomer Shooter Relevance

Scripting relevance for a Doom-like 4D FPS, ranked by use case:

**Enemy AI Behaviors (HIGH relevance)**
Classic boomer shooters use relatively simple AI state machines: patrol, chase, attack, retreat. Doom's enemies had very simple behavior patterns. In Rust, writing these as finite state machines is entirely tractable without scripting. The iteration speed argument weakens considerably when the AI is simple.

**Weapon Logic (MEDIUM relevance)**
Weapon behavior (fire rate, damage, projectile speed, spread patterns) is mostly data-driven. This can be handled with RON configuration files, which the engine already supports (`scene.rs` lines 47-53 show RON load/save working). Scripting adds value only if weapons have complex custom behaviors (e.g., gravity gun physics manipulation).

**Level Triggers/Scripts (MEDIUM-HIGH relevance)**
Door triggers, elevator platforms, secret areas, teleporters, key-locked doors -- these are the bread and butter of boomer shooters. Classic Doom used a line-based trigger system (not a scripting language). A declarative trigger system (defined in RON) could handle 80% of these cases without Lua.

**Modding Support (HIGH relevance, but deferrable)**
Boomer shooter culture heavily values modding (Doom WADs, Quake mods). Scripting is nearly essential for modding. But this is a post-release concern, not a development priority.

### 1.3 Lua vs Rhai vs Alternatives

The plan recommends Lua via `mlua` (line 171). For a Rust-centric project, I want to challenge this:

**Case for Rhai:**
- Rust4D is a Rust-first project. Rhai's Rust-like syntax means lower friction for the developer (Willow).
- Pure Rust (no C dependencies) -- simpler build, better cross-compilation.
- Sandboxed by default (critical for modding).
- No garbage collection pauses (relevant for 60 FPS gameplay).

**Case for Lua:**
- Modders are more likely to know Lua than Rhai.
- LuaJIT performance is significantly better for per-frame logic.
- Massively larger tooling ecosystem.

**My recommendation**: For a solo developer building a 4D boomer shooter, **Rhai is the better choice during development**. If modding becomes a priority post-release, add Lua as a secondary scripting language. The Rust-like syntax and sandboxing-by-default make Rhai far less friction for Willow's workflow.

### 1.4 Hot-Reload Value

The plan identifies hot-reload as a key benefit (lines 27-28). The current codebase already has infrastructure for hot-reload in the asset cache (`asset_cache.rs` lines 243-294, `check_hot_reload` method). This means the foundational pattern (file watching, reload detection) already exists and could be extended to scripts.

However, for a boomer shooter with simple AI and trigger systems, hot-reload of _scripts_ is less valuable than hot-reload of _levels_. The scene system already supports loading from RON files. The biggest iteration speed win would be a level editor or hot-reload of scene files, not script hot-reload.

### 1.5 Scripting Recommendation

**Priority for boomer shooter: LOW-MEDIUM (P6 is appropriate)**

**Reasoning:**
- Classic boomer shooter gameplay does NOT require complex scripting. Doom shipped without it.
- The engine API is not stable enough (no ECS, entity system may change).
- A declarative trigger system in RON would cover most level scripting needs.
- Development effort (6-8 sessions) is better spent on gameplay systems first.

**When to start:** After the first playable prototype exists (player movement, shooting, at least one enemy type). Only if iteration pain is genuinely felt. The plan's estimate of 6-8 sessions still seems accurate.

**Alternative approach:** Before full scripting, implement a **declarative trigger system** in RON:
```ron
Trigger(
    condition: PlayerEntersZone(zone: "door_area"),
    action: OpenDoor(entity: "secret_door", speed: 2.0),
)
```
This could be done in 1-2 sessions and would cover the majority of boomer shooter level scripting needs.

---

## 2. Visual Editor Assessment

### Source Document
`/home/lemoneater/Projects/Rust4D/scratchpad/plans/engine-roadmap-2026/long-term-visual-editor.md`

### 2.1 Trigger Condition Status

The plan lists five prerequisites (lines 96-133). Current status:

| Trigger Condition | Met? | Evidence |
|---|---|---|
| Scene system is mature (Phase 2) | Yes | RON serialization works (`scene.rs` lines 47-63), templates exist, async loading works. |
| Engine runtime is stable (Phase 5) | Partial | Rendering and physics work. Input handling is robust. But "no major architectural changes planned" is uncertain with ECS migration pending. |
| 4D rendering is production-ready | Partial | Cross-section rendering works. W-axis navigation exists. But still early. |
| Documentation exists (Phase 3) | Partial | Code is well-documented, but no user-facing docs. |
| There's actual demand | Emerging | Only simple test scenes exist currently. |

**Verdict**: 1-2 of 5 conditions are met. The "demand" condition is the most interesting -- boomer shooters need levels, and levels need level design tools.

### 2.2 Boomer Shooter Relevance

**Level editing is CRITICAL for a boomer shooter.** Doom lives and dies by its level design. Every boomer shooter from Quake to DUSK to Ultrakill succeeds or fails based on level quality.

However, the question is not "do we need level editing?" but "do we need to BUILD a custom editor?"

### 2.3 Build vs. Integrate: The Key Decision

**Option A: Custom egui Editor (the plan's approach)**
- Estimated 10-15 sessions (plan line 5)
- Full control over 4D-specific UX
- Integrated with engine runtime
- Massive development investment

**Option B: External Level Editor + Export Pipeline**
- Use TrenchBroom (Quake-style BSP editor) with custom export
- Use LDTK or similar for 2D/3D layout with W-axis added via config
- Much less development effort (2-3 sessions for importer)
- Proven UX patterns from decades of FPS level editing

**Option C: RON-first with preview tool**
- Edit RON files directly (already supported)
- Build a lightweight preview tool (not full editor)
- Hot-reload scenes via the existing `AssetCache` infrastructure
- Estimated 3-4 sessions

**My analysis of each option:**

**TrenchBroom integration** is compelling but has a fundamental problem: it outputs 3D geometry (Quake BSP format), and Rust4D needs 4D geometry. Converting 3D Quake maps to 4D would require defining how to "extrude" into the W dimension. This could work for levels that are fundamentally 3D with W as a gameplay mechanic (like Miegakure), but not for truly 4D level geometry.

**RON-first with preview** is the most pragmatic near-term option. The scene system (`scene.rs`) already supports load, save, and serialization. The `SceneLoader` (`scene_loader.rs`) provides async loading. A preview window that hot-reloads RON files would give fast iteration for level design at minimal cost.

**Custom editor** is the long-term correct answer because no external tool handles 4D level editing. The plan acknowledges this (line 473: "There is no industry standard for 4D editing").

### 2.4 egui Viability

egui remains a solid choice. The plan's rationale (lines 233-241) is still valid. The Rust ecosystem hasn't produced a clearly superior alternative since the plan was written. Key confirmations:

- egui continues active development
- `egui_dock` for panel layouts is mature
- wgpu integration is well-supported (Rust4D uses wgpu)

### 2.5 4D Editing UX: The Hard Problem

The plan correctly identifies this as the hardest challenge (lines 442-483). For a boomer shooter specifically:

**Key insight**: Boomer shooter levels are primarily experienced as 3D spaces. The 4D element adds a "dimension switch" mechanic (move to a different W-slice to access different areas). This means:

1. Most level editing can be done in 3D (designing individual W-slices)
2. The 4D aspect is primarily about connecting W-slices (doorways/transitions between W-coordinates)
3. A "multi-slice view" (plan Phase 5, line 447) is more important than a true 4D gizmo

This simplifies the editor significantly. Instead of a full 4D spatial editor, you need:
- A 3D editor for each W-slice
- A W-slice navigation bar (already proposed, plan line 322)
- A way to place "W-portals" or transition zones between slices

### 2.6 Visual Editor Recommendation

**Priority for boomer shooter: HIGH (should be elevated from P7)**

**Near-term (Phase A, 2-3 sessions):** Build a RON scene preview tool with hot-reload. Use the existing `AssetCache` hot-reload infrastructure. This gives fast level iteration without the full editor investment.

**Medium-term (Phase B, 6-8 sessions):** Build a minimal egui editor with:
- Entity list panel
- Property inspector
- W-slice navigation
- 3D viewport (reuse existing renderer)
- Save/load

This is the plan's Phases 1-2 (lines 310-370). Skip Phases 3-5 initially (gizmos, prefabs, 4D tools).

**Long-term:** Add spatial manipulation and 4D-specific tools as needed.

**Revised effort estimate:** 8-10 sessions for a usable boomer shooter level editor (down from 10-15, by scoping to 3D-per-slice editing).

---

## 3. Networking Assessment

### Source Document
`/home/lemoneater/Projects/Rust4D/scratchpad/plans/engine-roadmap-2026/long-term-networking.md`

### 3.1 Boomer Shooter Relevance

Multiplayer is part of the boomer shooter DNA. Doom shipped with deathmatch and co-op. Every major boomer shooter has multiplayer. **However**, many successful modern boomer shooters shipped single-player first (DUSK, Ultrakill, Prodeus) and added multiplayer later.

**Verdict:** Important for the genre's identity, but firmly a post-single-player-completion feature.

### 3.2 Current Architecture: Networking Readiness

I assessed every aspect the networking plan depends on:

**Entity Serialization: PARTIAL**
- `Vec4` has `Serialize`/`Deserialize` (`vec4.rs` line 9)
- `Transform4D` has `Serialize`/`Deserialize` with custom Rotor4 serde (`transform.rs` lines 33-42)
- `EntityTemplate` has `Serialize`/`Deserialize` (`entity.rs` line 264)
- `PhysicsConfig` has `Serialize`/`Deserialize` (`world.rs` line 12)
- **`Rotor4` does NOT have Serialize/Deserialize** (`rotor4.rs` -- no serde derives). It has a manual workaround via `rotor4_serde` module in `transform.rs`.
- **`RigidBody4D` does NOT have Serialize/Deserialize** (`body.rs` line 33 -- no serde derives)
- **`PhysicsWorld` does NOT have Serialize/Deserialize** (no state snapshot capability)
- **`World` does NOT have Serialize/Deserialize** (`world.rs` line 50-61, uses SlotMap which needs custom serde)

**State Snapshot Capability: NOT PRESENT**
The networking plan's Phase 3 requires snapshot generation (line 569). Currently, there is no way to serialize the runtime `World` or `PhysicsWorld` state. The `Scene` template is serializable, but the runtime `ActiveScene` is not. This is a significant gap.

**Deterministic Physics: NOT GUARANTEED**
The plan identifies this as a HIGH RISK (lines 693-704). I found no evidence of deterministic physics in the codebase:
- No fixed timestep (no `deterministic` or `fixed_step` references in physics code)
- The `PhysicsWorld::step()` takes a variable `dt` parameter
- No evidence of sorted entity processing (order-dependent collision resolution)
- Standard f32 floating point (not fixed-point)

**Dirty Tracking: PRESENT**
The plan notes that dirty tracking helps with networking (line 921). The codebase has `DirtyFlags` (`entity.rs`) and `clear_all_dirty()` in `World`. This is a good foundation for delta-based replication.

**Entity IDs: PROBLEMATIC**
The plan notes the need for stable network IDs (line 916). Currently, entities use `EntityKey` from SlotMap (`world.rs` line 19), which is generational and local. These keys are not stable across network boundaries. A separate `NetworkId` (UUID or incremental u64) would be needed.

### 3.3 4D Networking Bandwidth

The plan estimates 70% more bandwidth for 4D vs 3D entities (line 259):
- 4D position: 16 bytes (Vec4) vs 12 bytes (Vec3)
- 4D rotation: 32 bytes (Rotor4 has 8 f32s) vs 16 bytes (quaternion has 4 f32s)
- 4D velocity: 16 bytes vs 12 bytes

**Total per dynamic entity: ~88 bytes vs ~52 bytes = 69% increase**

This is manageable for a boomer shooter with 10-50 dynamic entities (players, projectiles, enemies). At 60 Hz with 50 entities: 50 * 88 * 60 = ~264 KB/s uncompressed. With delta compression, this drops to <50 KB/s in typical gameplay. This is well within modern network capabilities.

### 3.4 Groundwork That Can Be Laid Now

Without starting full networking, several foundational steps would help:

1. **Add `Serialize`/`Deserialize` to `Rotor4`** -- Currently missing, needed for any state serialization.
2. **Add `Serialize`/`Deserialize` to `RigidBody4D`** -- Needed for physics state replication.
3. **Implement fixed timestep** -- Critical for deterministic physics and prediction.
4. **Add stable entity IDs** -- UUID or network ID field on Entity for cross-boundary identification.
5. **Add state snapshot to `World`** -- Ability to serialize/deserialize runtime world state.

Items 1-3 are useful even without networking (better save/load, reproducible physics).

### 3.5 Networking Recommendation

**Priority for boomer shooter: LOW (P8 is appropriate)**

**Reasoning:**
- Single-player must come first. No successful boomer shooter launched multiplayer-first.
- The codebase has significant gaps for networking (no deterministic physics, no state serialization of runtime data, no stable entity IDs).
- The 15-20 session estimate seems accurate given the current state. Possibly even an underestimate given the 4D complexity.
- Co-op would be the most valuable multiplayer mode for initial implementation (simpler than competitive deathmatch with lag compensation).

**When to start:** After a complete single-player experience exists (3-5 levels, multiple enemy types, multiple weapons, boss fights). This is likely 20+ sessions away.

**Groundwork to lay now (0.5-1 session, high value):**
- Add `Serialize`/`Deserialize` to `Rotor4`
- Implement fixed timestep in physics
- These benefit the engine regardless of networking plans

---

## 4. Priority Ranking for Boomer Shooter

### Rank Order

| Rank | Feature | Priority | When to Start | Why |
|---|---|---|---|---|
| **1** | **Visual Editor** | HIGH (elevate to P5-P6) | After basic gameplay exists (5-10 sessions from now) | Level design is the #1 differentiator for boomer shooters. Editing 4D levels in RON text files will become unbearable fast. |
| **2** | **Scripting** | MEDIUM (P6 is fine) | After first playable prototype + editor exists (15-20 sessions from now) | Useful for enemy AI and level triggers, but simple patterns work in Rust first. A declarative trigger system covers 80% of needs. |
| **3** | **Networking** | LOW (P8 is fine) | After complete single-player game (30+ sessions from now) | Essential for genre identity long-term, but the codebase has significant gaps and single-player must come first. |

### Optimal Development Sequence

```
Phase: Build core gameplay (NOW)
  - Player movement, shooting, basic enemies
  - Work in Rust, use RON for level data
  - 5-10 sessions

Phase: Level editing tools (NEXT)
  - RON preview tool with hot-reload (2-3 sessions)
  - Minimal egui editor (6-8 sessions)
  - This unlocks rapid level iteration

Phase: Gameplay scripting (LATER)
  - Declarative trigger system first (1-2 sessions)
  - Full scripting only if triggers prove insufficient
  - 6-8 sessions for full scripting

Phase: Networking (MUCH LATER)
  - Lay groundwork now (Rotor4 serde, fixed timestep)
  - Full implementation after single-player is done
  - 15-20 sessions
```

### Critical Insight

The single most impactful decision is **elevating the visual editor priority**. For a boomer shooter, level design IS the game. Having a basic level editor 10 sessions from now would unlock vastly more gameplay iteration than having scripting or networking. The 4D nature of the engine makes this even more critical -- it is genuinely difficult to design 4D levels in a text editor, and impossible to get good spatial intuition without visual feedback.

---

## 5. Cross-Cutting Observations

### Serialization Gap
The biggest technical gap across all three plans is **runtime state serialization**. The engine can serialize _templates_ (Scene, EntityTemplate) but cannot serialize _runtime state_ (World, ActiveScene, PhysicsWorld, RigidBody4D). This blocks:
- Editor save (modified runtime state back to file)
- Networking snapshots
- Save game systems
- Replay systems

**Recommendation**: Before any of these three features, invest 1-2 sessions in making runtime state serializable. This is foundational for all three plans and for the game itself (save/load is expected in any game).

### ECS Migration Dependency
All three plans implicitly assume a stable entity API. If an ECS migration happens (the separate ECS roadmap plan), it would invalidate:
- Scripting API design (entirely different entity access patterns)
- Editor property inspector (different component structure)
- Networking replication strategy (component-based vs monolithic entity)

**Recommendation**: Decide whether to do ECS migration BEFORE starting any of these three features. The ECS decision is the most impactful architectural choice remaining.

### Asset Cache as Foundation
The existing `AssetCache` (`asset_cache.rs`) with its hot-reload support, dependency tracking, and garbage collection is a surprisingly strong foundation. It could be extended to support:
- Script asset hot-reload (scripting plan)
- Prefab management in editor (editor plan)
- Network asset distribution (networking plan, for map data)

The 341-line asset cache is well-tested (35 tests) and architecturally sound. This is an underappreciated asset of the current codebase.
