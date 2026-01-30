# Hive-Mind: Swarm C - Game Engine & Genre Feature Analysis
**Date**: 2026-01-30

## Shared Context
Rust4D is a 4D game engine. We want to build a **4D boomer shooter** - think Doom/Quake but in 4D space.

The engine currently has:
- 4D math (Vec4, Rotor4 rotations, convex shapes)
- 4D physics (rigid bodies, collision, gravity)
- 4D→3D rendering via compute shader hyperplane slicing
- Scene management, asset caching, entity hierarchy
- Basic camera controller with WASD + mouse + 4D rotation
- Config system (TOML), scene serialization (RON)

### Research Focus
Agent C1: Study Unity and Godot to identify missing engine features
Agent C2: Study the boomer shooter genre to identify required gameplay systems

Both: Think about what's UNIQUE to 4D - where do standard features need adaptation?

## Agent Discoveries

### Agent C1: Engine Feature Comparison (Complete)

**Report**: `c1-engine-features.md`

**Key Findings**:

1. **12 Critical Gaps Identified (P0)**:
   - No raycasting (cannot shoot), no audio (game is silent), no UI/HUD (no health/ammo display)
   - No particle effects (no visual feedback), no textures (vertex colors only), no sprites (no Doom-style enemies)
   - No event system (can't wire game logic), no trigger callbacks (pickups don't work)
   - No health/damage system, no weapon system, no enemy AI, no explosion/area damage

2. **What Rust4D Already Does Well**:
   - 4D math (Rotor4, SkipY camera) is sophisticated and would be hardest to add later
   - GPU slicing pipeline is the core differentiator -- neither Unity nor Godot have this
   - 4D physics with collision layers designed for shooters (PLAYER, ENEMY, PROJECTILE, PICKUP)
   - Scene management infrastructure is surprisingly mature (async loading, transitions, overlays)
   - Clean crate architecture enables parallel development

3. **5 Features Need 4D-Specific Adaptation** (cannot just port from 3D):
   - **Raycasting**: Math generalizes cleanly (ray-sphere, ray-AABB work in any dimension)
   - **Pathfinding**: Hardest problem -- walkable surface is 3D (XZW hyperplane). Recommend waypoint graphs as simplest approach for a boomer shooter
   - **Audio spatialization**: Use 4D Euclidean distance for attenuation. W-distance could apply low-pass filter ("hearing through dimensions")
   - **Level design**: Needs tooling (4D level editor). Data structures exist but designing 4D levels is conceptually hard
   - **AI navigation**: 4D raycast handles LOS naturally. Enemies need W-awareness for dimensional gameplay

4. **Recommended Phase Order**:
   - Phase 1: Combat Foundation (raycasting, events, health/damage) -- 2-4 sessions
   - Phase 2: Weapons & Feedback (audio, weapons, HUD, particles) -- 3-5 sessions
   - Phase 3: Enemies & AI (sprites, textures, AI, pathfinding) -- 3-4 sessions
   - Phase 4: Level Design & Polish (lights, shadows, doors, pickups) -- 3-5 sessions
   - Phase 5: Advanced (post-processing, minimap, gamepad, debug tools) -- 4-8 sessions

5. **Total effort to reach playable demo: ~12-18 sessions** (Phases 1-3)

### Agent C2: Boomer Shooter Genre Analysis (Complete)

**Report**: `c2-boomer-shooter.md`

**Key Findings**:

1. **6 Core Design Pillars of Boomer Shooters**: Speed/movement, weapon variety/feel, enemy design/AI, non-linear level design with secrets, non-regenerating health with pickups, and "game feel" (responsiveness + feedback). ALL translate to 4D but each gains new depth from the W-axis.

2. **The W-Axis Transforms Every System**:
   - **Movement**: W-strafing is the 4D equivalent of circle-strafing. Dodging projectiles by stepping in W. New speed-tech possibilities (W-hopping).
   - **Combat**: Projectiles need "W-thickness" to avoid frustrating misses. Explosions are hyperspheres covering W-range -- makes splash weapons MORE valuable in 4D. Hitscan requires W-alignment, creating a natural weapon hierarchy.
   - **Level design**: W-layered rooms (same XYZ, different content at different W). Keys at one W, doors at another. Secrets hidden in W-space. Arena encounters spanning multiple W-slices.
   - **Enemies**: W-flanking (enemies ambush from invisible W-slices). Phase-shifting enemies that blink between W values. Bosses requiring W-exploration to find weak points.

3. **Biggest Risk: Cognitive Overload**. 4D is hard. Miegakure's developer says UI/UX is the real challenge, not the math. The game MUST provide clear visual/audio cues for W-space information (ghosting for near-W entities, W-position HUD indicator, audio attenuation by W-distance). Pacing must include low-pressure W-exploration between combat.

4. **Biggest Opportunity: Genuine Novelty**. No 4D boomer shooter exists. The combination of fast aggressive combat with 4D spatial reasoning is unprecedented. Each boomer shooter system gains a new dimension of depth.

5. **Minimum Viable Boomer Shooter (MVBS)** requires: 4D character controller with jump, 2 weapons (hitscan shotgun + projectile rocket), 3 enemy types (melee rusher, projectile shooter, W-phaser), health/ammo pickups, basic HUD with W-indicator, and one arena level with W-depth. Estimated ~15-25 sessions.

6. **Key Design Lessons from Reference Games**:
   - Doom: Orthogonal enemy design + pain states create weapon-enemy matchups
   - Quake: New spatial axis (vertical) transformed combat -- W-axis will do the same
   - Ultrakill: Interlocking systems (blood-heal + style meter + weapon freshness) -- reward W-movement in scoring
   - Dusk: Secrets + pacing -- W-exploration needs breathing room
   - Amid Evil: Theme is flexible (fantasy staves work as well as guns) -- 4D-themed weapons can work
   - Miegakure: Cross-section visualization works but needs gradual teaching + heavy UI/UX investment

7. **Technical Alignment with C1**: Both analyses converge on the same critical gaps -- raycasting, health/damage, weapon system, enemy AI, HUD, audio, and event system. C2 adds: the need for W-proximity alerts, hyperspherical damage volumes, weapon viewmodel rendering, and a trigger/spawn system for arena encounters.
