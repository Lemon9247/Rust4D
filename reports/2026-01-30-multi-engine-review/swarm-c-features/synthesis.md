# Swarm C Synthesis: Game Engine & Genre Feature Analysis
**Date**: 2026-01-30
**Agents**: C1 (Unity/Godot Feature Comparison), C2 (Boomer Shooter Genre Analysis)

---

## Executive Summary

Rust4D has a strong 4D-specific foundation that neither Unity nor Godot can match (4D math, GPU hyperplane slicing, 4D physics). However, it is missing **12 critical (P0) features** that every game engine needs: raycasting, audio, UI/HUD, particles, textures, sprites, events, triggers, health/damage, weapons, enemy AI, and explosion/area damage. The boomer shooter genre transforms every one of these systems through the lens of the W-axis, creating genuine novelty but also a significant cognitive overload risk. A Minimum Viable Boomer Shooter (MVBS) is achievable in an estimated **15-25 sessions**.

---

## What Rust4D Does Well (Competitive Advantages)

These are features that **neither Unity nor Godot provide** and would be hardest to add later:

1. **4D Math (Rotor4)**: Full geometric algebra rotors with correct sandwich product for all 6 rotation planes. The mathematically hardest part of 4D.
2. **GPU Hyperplane Slicing**: Compute shader that slices 4D tetrahedra into 3D triangles in real-time with indirect draw. The core technical innovation.
3. **4D Physics**: Collision detection with sphere/AABB/plane primitives in 4D. Edge-falling detection. Physics materials.
4. **FPS-Ready Collision Layers**: 7 named layers (PLAYER, ENEMY, STATIC, TRIGGER, PROJECTILE, PICKUP) designed for a shooter.
5. **Camera4D Architecture**: Engine4D-style pitch/rotation separation with SkipY preserving the Y-axis during 4D rotation.
6. **Scene Infrastructure**: Async loading, transitions, overlays, validation, hot reload -- mature for this stage.

---

## 12 Critical Gaps (P0)

| # | Gap | Impact | Effort | Notes |
|---|-----|--------|--------|-------|
| 1 | Raycasting | Cannot shoot | 1 session | Math generalizes cleanly to 4D |
| 2 | Audio system | Game is silent | 1-2 sessions | Integrate rodio/kira; 4D distance attenuation |
| 3 | UI/HUD | No health/ammo display | 1 session | egui overlay or custom wgpu_text |
| 4 | Particle effects | No visual feedback | 1-2 sessions | Muzzle flash, blood, explosions |
| 5 | Textures | Vertex colors only | 1 session | UV mapping for walls/floors |
| 6 | Sprites/billboards | No Doom-style enemies | 1 session | Camera-facing quads in 3D slice space |
| 7 | Event system | Can't wire game logic | 1 session | Simple event bus or callback mechanism |
| 8 | Trigger callbacks | Pickups don't work | 0.5 session | Extend collision system to report events |
| 9 | Health/damage | No combat | 1 session | HP component, damage application, death |
| 10 | Weapon system | No weapons | 2 sessions | Hitscan + projectile, ammo, switching |
| 11 | Enemy AI | Nothing to fight | 1 session | FSM: idle/chase/attack/pain/dead |
| 12 | Explosion/area damage | Rockets don't work | 0.5 session | Hyperspherical damage volumes |

**Total P0 effort: ~12-14 sessions**

---

## 5 Features Requiring 4D Adaptation

Standard 3D features that cannot simply be ported:

### 1. Raycasting (Difficulty: Medium)
Math generalizes cleanly. Ray-sphere and ray-AABB work identically in any dimension using the slab method. Ray4D = origin + direction in XYZW space.

### 2. Pathfinding (Difficulty: Hard)
The walkable surface in 4D is 3-dimensional (XZW hyperplane). Traditional NavMesh becomes a 3D tetrahedra mesh. **Recommend waypoint graphs** as the simplest approach for a boomer shooter -- classic Doom/Quake used waypoints effectively.

### 3. Audio Spatialization (Difficulty: Medium)
Use 4D Euclidean distance for attenuation. W-distance could apply low-pass filter for a "hearing through dimensions" effect. Practical simplification: only play sounds within a W-range of the player.

### 4. Level Design (Difficulty: Very Hard for tooling)
4D levels need visual editing tools. The conceptual approach for a boomer shooter: design 3D content per W-slice, connect slices via W-portals. This simplifies the problem from "edit in 4D" to "edit in 3D with W-layer navigation."

### 5. AI Navigation (Difficulty: Medium)
4D raycast handles LOS naturally. Enemies need W-awareness for dimensional gameplay. W-flanking (enemies ambush from invisible W-slices) creates a unique combat dynamic.

---

## The W-Axis Transforms Everything

The genre analysis revealed that the W-axis doesn't just add a dimension -- it fundamentally changes each gameplay system:

| System | 3D Boomer Shooter | 4D Boomer Shooter |
|--------|-------------------|-------------------|
| Movement | Circle-strafe to dodge | **W-strafe** to dodge through dimensions |
| Projectiles | Dodge by strafing | Dodge by **stepping in W** |
| Explosions | Sphere of damage | **Hypersphere** covering W-range (more valuable in 4D) |
| Hitscan | Point and shoot | Requires **W-alignment** (natural weapon hierarchy) |
| Level design | Rooms and corridors | **W-layered rooms** (same XYZ, different content per W) |
| Secrets | Hidden walls | Hidden in **W-space** (invisible without W-exploration) |
| Enemies | Flank from sides | **W-flank** from invisible W-slices |
| Bosses | Find weak point in 3D | **W-explore** to find the vulnerable slice |

This creates a natural weapon hierarchy:
- **Hitscan**: Precise but requires W-alignment (high skill ceiling)
- **Projectiles with W-thickness**: More forgiving, good general purpose
- **Explosions (hypersphere)**: Cover W-range, best for uncertain W-positions

---

## Minimum Viable Boomer Shooter (MVBS)

The absolute minimum for a playable 4D boomer shooter:

1. 4D character controller with WASD + jump + W-movement
2. 2 weapons: hitscan shotgun + projectile rocket launcher
3. 3 enemy types: melee rusher, projectile shooter, W-phaser
4. Health and ammo pickups (no regeneration)
5. Basic HUD: health, ammo, W-position indicator, crosshair
6. One arena level with W-depth (W=-2 to W=+2)
7. Damage, death, and win condition

**Estimated effort: 15-25 sessions** (conservative, includes iteration)

---

## Recommended Development Phases

### Phase 1: Combat Foundation (2-4 sessions)
- 4D raycasting (ray-sphere, ray-AABB, ray-plane, world raycast with filtering)
- Event/signal system
- Health/damage system
- Trigger zone callbacks

### Phase 2: Weapons & Feedback (3-5 sessions)
- Audio system (rodio/kira integration, basic spatial with 4D distance)
- Weapon system (hitscan + projectile, ammo, switching)
- Basic HUD (health, ammo, crosshair, W-indicator)
- Particle/effect system (muzzle flash, impacts, explosions)

### Phase 3: Enemies & AI (3-4 sessions)
- Sprite billboard rendering
- Texture support (UV mapping for surfaces)
- Basic AI state machine (idle/chase/attack/pain/dead)
- Waypoint pathfinding in 4D
- Explosion/area damage (hyperspherical volumes)

### Phase 4: Level Design & Polish (3-5 sessions)
- Point lights (muzzle flash, explosion glow)
- Shadow mapping (optional, helps spatial awareness)
- Door/elevator mechanics
- Pickup system
- Input rebinding, pause menu

### Phase 5: Advanced (4-8 sessions)
- Post-processing (bloom, color grading)
- 4D minimap
- Gamepad support
- Debug tools (console, physics visualization)
- More shape types (hypersphere, 4D cylinder)

**Phases 1-3 = playable demo (~12-18 sessions)**

---

## Biggest Risk: Cognitive Overload

Both agents independently identified cognitive overload as the primary risk. 4D is genuinely hard to think about. Miegakure's developer says UI/UX is the real challenge, not the math. The game MUST provide:

- **Visual cues**: Ghosting/transparency for near-W entities
- **Audio cues**: W-distance affects volume and filtering
- **HUD indicator**: Always-visible W-position display
- **Pacing**: Low-pressure W-exploration periods between combat encounters
- **Gradual teaching**: Start with pure 3D combat, then introduce W-movement

## Biggest Opportunity: Genuine Novelty

No 4D boomer shooter exists. The combination of fast aggressive combat with 4D spatial reasoning is unprecedented. Every boomer shooter system gains new depth from the W-axis. The 6 core design pillars (speed, weapons, enemies, levels, non-regen health, game feel) all translate to 4D and all gain new dimensions of possibility.

---

## Key Lessons from Reference Games

| Game | Key Lesson for 4D |
|------|-------------------|
| Doom | Orthogonal enemy design + pain states create weapon-enemy matchups |
| Quake | New spatial axis (vertical) transformed combat -- W-axis will do the same |
| Ultrakill | Interlocking systems -- reward W-movement in scoring |
| Dusk | Secrets + pacing -- W-exploration needs breathing room |
| Amid Evil | Theme is flexible (staves work as well as guns) -- 4D-themed weapons will work |
| Miegakure | Cross-section visualization works but needs gradual teaching + heavy UI/UX investment |

---

## Source Reports
- [C1: Engine Feature Comparison](./c1-engine-features.md)
- [C2: Boomer Shooter Genre Analysis](./c2-boomer-shooter.md)
