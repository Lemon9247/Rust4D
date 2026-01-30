# Agent C2: Boomer Shooter Genre Analysis for a 4D Engine

**Date**: 2026-01-30
**Agent**: C2 - Boomer Shooter Genre Analyst
**Scope**: What makes a great boomer shooter, and what does a 4D boomer shooter need?

---

## 1. Defining the Boomer Shooter

The term "boomer shooter" describes a specific subset of first-person shooters that intentionally emulate and evolve the design principles of classic 1990s FPS games -- Doom, Quake, Duke Nukem 3D, Blood, and their contemporaries. Despite the retrograde label, the genre is one of the most vibrant in modern gaming, with titles like Ultrakill, Dusk, Amid Evil, and Prodeus proving these design philosophies can produce fresh, critically acclaimed experiences.

### Core Design Pillars

**1. Speed and Movement**
The player moves FAST. Classic Doom's movement speed is roughly 57 mph (if scaled to reality). There is no walking -- only running. Movement is the primary defensive tool. Circle-strafing around enemies, weaving between projectiles, and constantly repositioning are the core survival loop. There is no aim-down-sights (ADS), no movement penalty for shooting, no stamina bar. The player is always at full combat readiness. Modern boomer shooters like Ultrakill and Dusk add dashes, slides, and wall jumps to expand the movement vocabulary while keeping the same philosophy: movement IS skill.

**2. Weapon Variety and Feel**
A boomer shooter arsenal typically contains 7-10 weapons, each with a distinct tactical niche. This is what Deus Ex designer Harvey Smith called "orthogonal unit differentiation" -- the weapons are not linear upgrades but laterally different tools. The shotgun stuns at close range; the rocket launcher controls space but has self-damage; the plasma gun excels against tough single targets. Weapon switching is instant (or near-instant), and players are expected to rapidly switch weapons mid-combat to match the situation. The "gunfeel" -- screen shake, muzzle flash, impact particles, sound design, enemy stagger -- is absolutely paramount. A weapon that does not feel satisfying to fire will ruin the entire game.

**3. Enemy Design and AI**
Enemies in boomer shooters are designed as puzzle pieces. Each enemy type creates a distinct tactical pressure:
- **Melee rushers** (Doom's Pinkies) force the player to create space
- **Projectile enemies** (Imps, Cacodemons) create dodging patterns
- **Hitscan enemies** (Chaingunners) demand immediate prioritization
- **Flying enemies** add a vertical axis to combat
- **Pain Elementals / spawners** punish passivity by creating more threats over time
- **Boss enemies** test mastery of all movement and weapon skills

The AI itself is typically simple (finite state machines, chase/attack/pain states), but the emergent gameplay from mixing enemy types in crafted arenas creates enormous depth. Doom's "pain state" system -- where enemies have a chance to be stunned when hit -- creates weapon-enemy interplay that rewards knowledge. Sound propagation (enemies hear gunfire through connected sectors) creates organic tension.

**4. Level Design Philosophy**
Boomer shooter levels are non-linear explorable spaces, not corridors. The classic formula involves:
- **Key/door progression**: Find the red key, open the red door. Creates a navigation puzzle layered on top of combat.
- **Interconnected spaces**: Levels loop back on themselves. Shortcuts open. The player builds a mental map.
- **Secrets**: Hidden rooms, walls that open, obscure paths. Reward curiosity and mastery.
- **Arena encounters**: Rooms that lock when entered, releasing waves of enemies. The set-piece combat challenge.
- **Environmental hazards**: Lava, crushing ceilings, toxic waste. The level itself is a threat.
- **Pacing**: Combat intensity rises and falls. Dusk exemplifies this -- frantic arena fights punctuated by quiet exploration and discovery.

**5. Health, Armor, and Pickups (No Regeneration)**
Health does NOT regenerate. The player collects health kits, armor shards, and ammo from the environment and from defeated enemies. This creates a resource management layer: "Can I afford to take this fight with 30 HP?" It also rewards exploration -- finding a secret cache of health can save a run. Power-ups (quad damage, invulnerability, berserk) create explosive moments of empowerment.

**6. The "Game Feel"**
This is the intangible quality that separates a good boomer shooter from a forgettable one. It is the sum of:
- Input latency (must be imperceptible)
- Responsiveness of movement (instant acceleration, sharp turning)
- Weapon feedback (visual + audio + haptic)
- Enemy reactions (pain states, gibs, death animations)
- Player agency (the feeling that skill determines outcome)
- The "power fantasy" -- the player should feel like a force of destruction

As PC Gamer put it: "a heady blend of purity, nostalgia and above all, incredible gunfeel."

---

## 2. Core Gameplay Systems Required

### Movement System

**What it needs to do**: Provide fast, responsive, skill-expressive player movement in 4D space.

| Feature | Priority | Notes |
|---------|----------|-------|
| High base move speed | Critical | 2-3x typical modern FPS speed |
| Instant direction changes | Critical | No acceleration ramp-up |
| Jumping | Critical | Generous air time, full air control |
| Circle-strafing | Critical | Must feel natural and fluid |
| No ADS | Critical | Always hip-firing, always ready |
| Dash/dodge | High | Modern boomer shooters expect this (Ultrakill, Dusk) |
| Bunny-hopping/speed tech | Medium | Rewards skill without being required |
| Sliding | Medium | Ultrakill's slide is core to its identity |
| Wall-jumping | Low | Nice-to-have for advanced movement |

**Why**: Movement is the primary skill expression in a boomer shooter. If the player character feels sluggish or unresponsive, nothing else matters. The character controller must support high speeds with frame-perfect responsiveness.

### Weapon System

**What it needs to do**: Provide 7-10 weapons with instant switching, distinct tactical roles, and deeply satisfying feedback.

| Feature | Priority | Notes |
|---------|----------|-------|
| Multiple weapon slots | Critical | Number key switching (1-9) |
| Instant weapon switch | Critical | No switch animations that delay firing |
| Per-weapon ammo types | Critical | Often shared (e.g., bullets for pistol + chaingun) |
| Projectile weapons | Critical | Rockets, plasma, nails -- most are non-hitscan |
| Hitscan weapons | Critical | Shotgun, chaingun -- instant hit with range falloff |
| Weapon viewmodel | Critical | First-person weapon rendering with animation |
| Muzzle flash + screen shake | Critical | Core to "gunfeel" |
| Impact particles/decals | High | Visual feedback on hit |
| Alt-fire modes | Medium | Modern shooters add these (Ultrakill, Amid Evil) |
| Weapon interactions | Low | Ultrakill's coin-toss + revolver combos |

**Why**: Weapons are the player's primary interface with the game world. Each weapon creates a different relationship with the enemy set. Amid Evil proved that the weapon theme can be anything (magical staves throwing planets) as long as the tactical differentiation and feel are preserved.

### Enemy System

**What it needs to do**: Provide diverse enemy types that combine to create emergent tactical challenges.

| Feature | Priority | Notes |
|---------|----------|-------|
| Multiple enemy types (8-15) | Critical | Each with distinct behavior |
| Melee AI (chase + attack) | Critical | Pinkies, zombies, imps |
| Ranged AI (shoot + dodge) | Critical | Soldiers, mages, turrets |
| Flying enemies | High | Adds vertical combat axis |
| Spawner enemies | High | Creates time pressure |
| Boss enemies with patterns | High | Set-piece fights |
| Pain/stagger system | Critical | Weapon-enemy interplay |
| Death effects (gibs, explosions) | High | Satisfying feedback loop |
| Sound propagation (AI hearing) | Medium | Emergent tension |
| Infighting | Medium | Doom's monster infighting creates chaos |
| Spawn triggers | Critical | Enemies appear when player enters areas |

**Why**: Enemies are the puzzle pieces that make combat interesting. A room full of one enemy type is boring; a room mixing melee rushers, hitscan snipers, and flying harassers forces the player to prioritize, reposition, and switch weapons constantly. The pain system (enemies flinch when hit) is crucial -- it makes weapon choice meaningful beyond raw DPS.

### Level Design System

**What it needs to do**: Support non-linear, explorable levels with key/door progression, secrets, and arena encounters.

| Feature | Priority | Notes |
|---------|----------|-------|
| Key/door system | Critical | Color-coded keys and locked doors |
| Secret areas | Critical | Hidden walls, obscured paths |
| Trigger zones | Critical | Enter area -> spawn enemies, lock doors |
| Environmental hazards | High | Damage floors, crushing ceilings |
| Elevators/platforms | High | Moving geometry |
| Teleporters | High | Instant transport between areas |
| Arena lock-in | High | Doors lock until all enemies dead |
| Interactive objects | Medium | Switches, buttons, destructible walls |
| Level transitions | Medium | End-of-level triggers, episode structure |
| A level editor or format | High | Content creation pipeline |

**Why**: Levels are where all the other systems come together. Dusk's success is largely attributed to its level design -- non-linear, secret-rich, varied in theme and pacing. A boomer shooter without good level design is just a shooting gallery.

### Health and Pickup System

**What it needs to do**: Provide resource management through pickups scattered in the world and dropped by enemies.

| Feature | Priority | Notes |
|---------|----------|-------|
| Health pickups (small/large) | Critical | 1, 10, 25, 100 HP varieties |
| Armor pickups | Critical | Damage reduction layer |
| Ammo pickups | Critical | Per-type ammo drops |
| Weapon pickups | Critical | First-time pickup grants the weapon |
| Power-ups | High | Quad damage, invulnerability, speed |
| Enemy item drops | High | Ammo/health from killed enemies |
| Overheal/overcharge | Medium | Temporary health above max |
| Item respawn (multiplayer) | Low | For deathmatch modes |

**Why**: Non-regenerating health creates stakes and rewards exploration. Finding a hidden armor cache before a boss fight changes the experience. Resource scarcity (running low on rockets, forced to use the shotgun against a Baron) creates memorable moments.

### HUD System

**What it needs to do**: Display critical gameplay information clearly and immediately.

| Feature | Priority | Notes |
|---------|----------|-------|
| Health bar/counter | Critical | Always visible |
| Armor bar/counter | Critical | Always visible |
| Ammo counter | Critical | Current weapon ammo + reserve |
| Crosshair | Critical | Center screen, customizable |
| Weapon indicator | High | Which weapon is equipped |
| Damage direction indicator | High | Where damage is coming from |
| Key inventory | High | Which keys the player has |
| Kill messages/feed | Medium | Feedback on kills |
| Minimap/automap | Medium | Classic Doom feature |
| Boss health bar | Medium | For boss encounters |
| Style meter | Low | Ultrakill-style scoring (optional) |

**Why**: In a fast-paced game, the player must be able to assess their status at a glance. Information delay = death.

---

## 3. The 4D Twist -- What Changes?

This is where a 4D boomer shooter becomes genuinely unprecedented. No one has combined boomer shooter mechanics with true 4D space. Each system must be rethought through the lens of a fourth spatial dimension.

### 4D Movement

The player moves in XYZW space. XYZ works like a traditional FPS (forward/back, strafe left/right, up/down via jumping). W is the fourth spatial dimension.

**Key design questions:**

1. **How does the player perceive W movement?** The engine already uses hyperplane slicing -- the player sees a 3D cross-section of 4D space. Moving in W changes which slice is visible. Objects at different W coordinates appear, grow, shrink, and vanish as their cross-section passes through the player's viewing hyperplane. This is analogous to how a 2D creature would perceive a sphere passing through its plane (a circle that grows then shrinks).

2. **How is W movement controlled?** Two approaches:
   - **Continuous W movement**: Dedicated keys (Q/E or bumpers) slide the player along W. The world smoothly transitions as objects cross-section differently. This is more immersive but can be disorienting.
   - **Discrete W stepping**: Button press snaps the player to the next W-slice (like Miegakure's dimension swap). Clearer for navigation but less fluid for combat.
   - **Recommendation for a boomer shooter**: Continuous W movement for combat fluidity, but with a "snap to nearest W-grid" option for exploration. Speed matters -- W movement should feel as responsive as XYZ.

3. **W-strafing as combat technique**: Just as circle-strafing in XYZ is core to boomer shooter combat, **W-strafing** becomes a new skill. The player dodges projectiles by stepping sideways in W, making them pass through a different slice. This is the 4D equivalent of sidestepping a bullet.

4. **4D bunny-hopping**: Could there be a speed-tech that involves rhythmic W-shifting? A "W-hop" that builds momentum across dimensional boundaries?

**What the engine needs:**
- Character controller supporting 4-axis movement (XYZW)
- Configurable W movement speed (probably slightly slower than XYZ for readability)
- Visual feedback for W position (HUD indicator, ambient visual shift)
- Collision detection in 4D (already partially implemented in rust4d_physics)

### 4D Combat

Projectiles travel through 4D space. This fundamentally changes how combat works.

**Projectile behavior in 4D:**
- A rocket fired in 4D travels along a ray in XYZW space. It only hits targets that overlap in ALL four coordinates.
- If the target is at a different W than the projectile, it misses -- even if it looks like a direct hit in XYZ. This is profoundly disorienting at first but creates incredible depth.
- **Solution**: Projectiles should have a "W-thickness" (a small W-range they affect). A rocket might affect W +/- 0.5 from its center. This prevents frustrating near-misses and makes combat feel fair.
- **Explosions are 4D**: A rocket explosion is a 4D hypersphere. It damages everything within radius in all four dimensions. Splash damage naturally covers nearby W-slices, making explosives more reliable in 4D combat. This gives rockets/grenades a natural tactical role: "I can't see exactly where the enemy is in W, so I'll use splash damage."

**Line-of-sight in 4D:**
- The player sees a 3D slice. An enemy at a different W is invisible (or appears as a cross-section fragment).
- This creates a stealth/ambush dynamic unique to 4D: enemies can lurk in adjacent W-slices and attack without being fully visible.
- **Visual cues are essential**: Objects near the player's W-slice but not exactly on it should have visual indicators -- transparency/ghosting, color shift (like Adanaxis's red/blue system), particle effects, or distortion.

**Hitscan in 4D:**
- Hitscan weapons fire a 4D ray. They hit the first entity along that ray in XYZW.
- The player needs to be at (roughly) the same W as the target. This makes hitscan weapons more demanding in 4D -- you need to be W-aligned to hit.
- This creates a natural weapon hierarchy: hitscan weapons are precise but require W-alignment; projectile weapons with W-thickness are more forgiving; explosives work across W-slices.

**New weapon concepts unique to 4D:**
- **W-Splitter**: A weapon that fires projectiles spreading across W-slices (a 4D shotgun analog)
- **Dimensional Piercer**: A hitscan weapon that ignores W -- it damages everything along a 3D ray regardless of W position
- **Phase Grenade**: Explodes only in a specific W-range, letting players target enemies in other slices
- **Slice Blade**: Melee weapon that sweeps across W, hitting enemies in adjacent slices
- **Tesseract Trap**: A deployable that creates a damage zone spanning multiple W-slices

### 4D Level Design

Levels exist in 4D space, which exponentially increases design possibilities.

**W-layered architecture:**
- A building at W=0 might have a completely different interior at W=2. Same XYZ footprint, different content at different W.
- This is like having multiple "layers" of a level occupying the same space. Moving in W switches between them.
- A corridor that appears to dead-end at W=0 might continue at W=1. A wall is only a wall in certain W-slices.

**4D keys and doors:**
- The red key might be at W=3, while the red door is at W=0. Finding the key requires W-exploration.
- Doors could require specific W-alignment to open (stand at the right W to interact).
- A locked room might be bypassable by W-shifting around the walls (like how a 4D being can reach inside a 3D locked safe).

**4D secrets:**
- Secrets hidden in W-space are the ultimate evolution of the hidden-wall secret. A secret room might exist at a W-coordinate that is never hinted at unless the player explores W thoroughly.
- Sound cues from adjacent W-slices (muffled pickup sounds, distant enemy growls) could hint at secrets.
- Visual artifacts: a faint shimmer or distortion where a secret exists in a nearby W.

**4D arena encounters:**
- Enemies attack from multiple W-slices simultaneously. The player must monitor threats in 4 dimensions.
- Arena geometry could shift in W during combat (platforms at different W-slices activate/deactivate).
- "Phase gates" -- teleporters that shift the player's W coordinate, creating a combat space that spans multiple slices.

**4D environmental hazards:**
- Lava might exist at W=0 but not W=1 -- the player can avoid it by W-stepping.
- Crushing hazards could operate in W (a W-plane closing in on the player).
- "W-rifts" -- zones where W is unstable, randomly shifting the player's W position.

### 4D Enemies

Enemies that exist in 4D create entirely new combat dynamics.

**W-flanking:**
- An enemy at a different W can move toward the player in XYZ while remaining invisible. It only becomes visible when it W-shifts to the player's slice. This creates a terrifying ambush dynamic -- enemies can appear from "nowhere."
- Counterplay: visual/audio cues for nearby W-entities. A "W-proximity radar" on the HUD.

**W-specific enemy types:**
- **Phase Lurker**: Stays in adjacent W-slices, darting into the player's W to attack then retreating. Hard to hit, requires W-tracking or splash weapons.
- **W-Blink Knight**: Teleports between W-slices during combat. Can only be damaged when on the player's W.
- **Dimensional Anchor**: A tank enemy that occupies a wide W-range, always partially visible and always damageable. Absorbs splash damage.
- **4D Spawner**: Creates enemies in adjacent W-slices that migrate toward the player's W.
- **Tesseract Guardian (Boss)**: Occupies all W-slices simultaneously. Different cross-sections reveal different weak points. The player must shift W to find the vulnerable slice.

**Visibility rules:**
- Enemy fully on player's W: fully visible, normal combat.
- Enemy within W +/- threshold: partially visible (ghosted, outlined, distorted). Can be hit with splash damage.
- Enemy outside W threshold: invisible but potentially audible. Cannot be directly targeted.

---

## 4. Reference Game Analysis

### Doom (1993) -- The Foundation
**Key lesson 1: Orthogonal enemy design.** Every enemy creates a distinct tactical pressure. The interplay between enemy types (mixing melee rushers with hitscan snipers with projectile lobbers) is what makes encounters interesting. A 4D boomer shooter must achieve this same variety, PLUS add W-axis behaviors.

**Key lesson 2: Pain state system.** Enemies flinch when hit, with per-enemy probabilities. This creates weapon-enemy matchups (chaingun stun-locks high-pain-chance enemies). A 4D version should extend this: some enemies might be more susceptible to stagger when hit from a different W-slice.

### Quake (1996) -- True 3D, Speed, and Verticality
**Key lesson 1: Verticality transforms combat.** Doom was a 2.5D game. Quake added true 3D, and suddenly enemies could be above and below. For a 4D game, the W-axis is this same paradigm shift again -- adding an entire new axis of threat. Quake's rocket-jumping created emergent speed-tech; a 4D game could create "W-jumping."

**Key lesson 2: Movement skill ceiling.** Bunny-hopping and strafe-jumping emerged from Quake's physics. These were unintentional but became beloved. A 4D game should deliberately design W-movement mechanics with a skill ceiling -- simple to use, deep to master.

### Ultrakill (2020) -- Modern Innovation
**Key lesson 1: Systems that feed into each other.** Ultrakill's blood-healing mechanic forces aggression; the style meter rewards variety; weapon freshness prevents repetition; movement multipliers reward staying airborne. Every system reinforces every other system. A 4D boomer shooter should similarly make W-movement feed into combat scoring -- reward players for attacking from different W-slices.

**Key lesson 2: Weapon interactivity.** Ultrakill's weapons interact with each other (coin-toss + revolver, punch + projectile boost). In 4D, weapons could interact with the W dimension -- firing a projectile then W-shifting to redirect it, or using melee to punt enemies into a different W-slice.

### Dusk (2018) -- Level Design Mastery
**Key lesson 1: Secrets drive exploration.** Dusk's levels are packed with secrets that reward curiosity -- hidden rooms behind moveable objects, areas accessible by stacking items. In 4D, the secret design space is vastly richer: secret areas hidden in W-space, objects that only appear at certain W-coordinates, paths that require W-exploration to discover.

**Key lesson 2: Pacing.** Dusk alternates between intense combat and quiet exploration. A 4D game needs this even more, because 4D navigation is cognitively demanding. Players need breather moments to explore W-space without combat pressure.

### Amid Evil (2019) -- Fantasy Weapons and Unique Projectiles
**Key lesson 1: Theme doesn't matter; feel does.** Amid Evil replaced guns with magical staves that throw planets, ice shards, and black holes. The boomer shooter formula works with ANY theme as long as weapon differentiation and feel are preserved. A 4D game could have weapons that are explicitly 4D-themed (dimensional rifters, hypersphere launchers) without feeling gimmicky, as long as the underlying mechanics are solid.

**Key lesson 2: Mana/shared ammo simplifies.** Amid Evil uses mana instead of per-weapon ammo. This reduces resource management complexity, leaving cognitive bandwidth for 4D navigation. A 4D game should consider simplified ammo to avoid overwhelming the player.

### Miegakure -- The 4D Pioneer
**Key lesson 1: Cross-section visualization works, but requires teaching.** Miegakure's entire design is built on helping players intuit 4D space through cross-section slicing. The same visualization approach is already in Rust4D's engine. The critical lesson: players CAN learn to think in 4D, but the game must teach them gradually.

**Key lesson 2: "Design as discovery."** Marc ten Bosch's philosophy -- rich mechanics that produce surprising but logical outcomes. A 4D boomer shooter's mechanics should emerge from the physics of 4D space, not be artificially imposed. If the 4D math says explosions are hyperspheres, lean into that. If W-flanking is a natural consequence of 4D movement, make it a core mechanic rather than hiding it.

**Key lesson 3: UI/UX is the real challenge.** Miegakure's developer says what takes development time is "not the cool 4D stuff but all the UI/UX around it." A 4D boomer shooter will live or die by how clearly it communicates W-space information to the player -- W-position indicators, nearby-entity warnings, cross-section previews.

---

## 5. Minimum Viable Boomer Shooter (MVBS)

### Absolute Minimum Systems

1. **4D Character Controller**: WASD + mouse look (XYZ) + Q/E for W-movement. Jump. Fast speed. Air control. No ADS.
2. **One hitscan weapon** (shotgun): Instant hit in 4D, some W-tolerance.
3. **One projectile weapon** (rocket launcher): Travels in 4D, explodes as hypersphere with W-splash.
4. **3 enemy types**:
   - Melee rusher (chases in XYZW)
   - Projectile shooter (fires 4D projectiles)
   - W-phaser (shifts between W-slices, ambush attacks)
5. **Health pickups**: Simple healing items in the world. No regeneration.
6. **Ammo pickups**: Per-weapon ammo in the world.
7. **Basic HUD**: Health, ammo, W-position indicator, crosshair.
8. **One playable level**: Small arena with W-depth.
9. **Damage and death**: Player can die, enemies can die. Hit feedback (flash, sound).
10. **Win condition**: Kill all enemies or reach the exit.

### Single Playable Level Concept: "The W-Arena"

A square arena at W=0 with walls and cover. The arena extends from W=-2 to W=2. The player starts at W=0.

- **Wave 1**: Melee rushers attack from W=0. Standard boomer shooter combat. Teaches XYZ movement.
- **Wave 2**: Projectile enemies appear at W=0 and W=1. Player must W-shift to engage enemies on a different slice. Teaches W-movement.
- **Wave 3**: W-phasers attack from various W-slices, blinking in and out. Player must use the rocket launcher's splash damage to hit enemies across W-slices. Teaches 4D combat.
- **Health and ammo pickups scattered at different W-coordinates.** Finding them requires W-exploration.

The arena has columns that exist at all W-values (cover works in 4D) and platforms at specific W-values (reward W-exploration).

### What Can Be Faked or Simplified

- **Weapon viewmodel**: Start with a simple geometric shape (a floating cube) rather than a detailed weapon model.
- **Enemy AI**: Finite state machine with chase/attack/pain states. No pathfinding beyond "move toward player in XYZW."
- **Death effects**: Enemies flash and disappear. No gibs or ragdolls initially.
- **Level geometry**: Use tesseracts and hyperplanes (already supported by the engine) as building blocks. No detailed architecture initially.
- **Sound**: Placeholder sounds. Spatial audio can come later.
- **HUD**: Minimal text overlay. Health number, ammo number, W-position number.
- **Weapon feedback**: Screen shake and color flash. No particle systems initially.
- **Enemy variety**: Start with 3 types, differentiated by speed, HP, and whether they attack in XYZ or W.

---

## 6. Technical Requirements Summary

Based on analysis of the existing Rust4D engine code:

### What Rust4D Currently Has

The engine already provides:
- **4D math**: Vec4 (XYZW vectors), Rotor4 (4D rotations via geometric algebra), Mat4 (4x4 matrices)
- **4D shapes**: ConvexShape4D trait, Tetrahedron, Tesseract4D, Hyperplane4D
- **4D physics**: RigidBody4D, StaticCollider, Sphere4D/AABB4D/Plane4D colliders, collision detection (sphere-AABB, sphere-plane, AABB-AABB, AABB-plane), PhysicsWorld simulation, PhysicsMaterial, CollisionFilter/CollisionLayer
- **4D rendering**: Camera4D with position/rotation, compute shader hyperplane slicing (4D -> 3D cross-sections), wgpu-based 3D rendering pipeline with lighting
- **Scene system**: Entity/World/Scene, Transform4D, Material, scene serialization (RON), SceneManager with scene stack, asset caching
- **Input**: CameraController with WASD + mouse + W-axis movement, configurable speeds and sensitivity
- **App structure**: winit event loop, simulation system, window management, TOML config

### What Must Be Built

| Gameplay Need | Engine Feature Required | Rust4D Has It? | Priority |
|--------------|----------------------|----------------|----------|
| Fast movement | Character controller with 4D physics | Partial (CameraController + physics exist, but no proper FPS controller with jump/dash) | Critical |
| Jumping | Jump mechanic with ground detection | No | Critical |
| Dashing/dodging | Dash mechanic with cooldown | No | High |
| W-strafing | W-axis movement in combat | Partial (W movement exists, needs tuning for combat) | Critical |
| Weapon switching | Weapon slot system with inventory | No | Critical |
| Weapon firing | Projectile spawning + hitscan raycasting in 4D | No (physics has collision but no raycasting or projectile system) | Critical |
| Weapon viewmodel | First-person weapon rendering (separate from world) | No | Critical |
| Weapon feedback | Screen shake, flash, sound triggers | No | Critical |
| Projectile physics | 4D projectile movement + collision | Partial (RigidBody4D exists, needs projectile behavior) | Critical |
| 4D explosions | Hyperspherical damage volumes | No | High |
| Enemy entities | AI-controlled entities with health | No | Critical |
| Enemy AI | Finite state machine + chase/attack/pain | No | Critical |
| Enemy spawning | Trigger-based entity instantiation | No (SceneManager exists but no spawn triggers) | Critical |
| Pain/stagger system | Per-enemy pain chance + stun state | No | High |
| Death effects | Entity removal + visual effects | No | High |
| Health system | Player HP + damage/healing | No | Critical |
| Armor system | Damage reduction layer | No | High |
| Pickup system | Collectible items in the world | No | Critical |
| HUD rendering | 2D overlay on 3D scene | No | Critical |
| Key/door system | Inventory + door state logic | No | High |
| Trigger zones | Volume-based event triggers | No | High |
| Level format | Extended scene format with triggers/pickups | Partial (Scene exists, needs gameplay data) | High |
| W-position indicator | HUD element showing W coordinate | No | Critical |
| W-proximity alerts | Visual/audio cues for nearby W entities | No | High |
| Damage direction | Indicator showing damage source direction | No | Medium |
| Sound system | Spatial audio, weapon sounds, enemy sounds | No | High |
| Particle system | Muzzle flash, impacts, explosions | No | High |
| Entity health/damage | Damageable entities with HP | No | Critical |
| Win/lose state | Game over + level complete logic | No | High |
| Menu system | Start screen, pause menu | No | Medium |

### Architecture Recommendations

1. **ECS or component-based architecture**: The current Entity/World system could be extended with gameplay components (Health, Weapon, AIState, Pickup), but the engine may benefit from a proper ECS (like bevy_ecs or hecs) to handle the large number of gameplay systems. This is a major architectural decision.

2. **Gameplay layer separation**: Keep the 4D math/physics/render engine separate from gameplay systems. The boomer shooter is ONE game built on the engine. The engine should not be boomer-shooter-specific.

3. **4D raycasting**: Critical for hitscan weapons and line-of-sight checks. Needs a 4D ray-AABB and ray-sphere intersection implementation.

4. **Event/messaging system**: Gameplay systems need to communicate (weapon fires -> spawn projectile -> projectile hits enemy -> enemy takes damage -> enemy enters pain state -> enemy drops item). An event bus or message queue would prevent tight coupling.

5. **Time/tick system**: The SimulationSystem exists but gameplay needs fixed-timestep updates for deterministic physics and AI, separate from render frame rate.

---

## Summary of Key Findings

A 4D boomer shooter is an unprecedented game concept. The core design pillars of the boomer shooter genre -- speed, weapon variety, enemy diversity, explorable levels, non-regenerating health -- all translate to 4D, but each gains an extraordinary new dimension (literally).

The W-axis transforms every system:
- **Movement** gains W-strafing as a new dodge technique
- **Combat** gains W-thickness projectiles and hyperspherical explosions
- **Level design** gains W-layered architecture and dimensional secrets
- **Enemy AI** gains W-flanking and phase-shifting behaviors
- **The HUD** gains W-position awareness as a critical information need

The biggest risk is **cognitive overload**. 4D is hard to think about. The game must teach W-awareness gradually and provide clear visual/audio cues for W-space information. Miegakure's developer spent years on UI/UX for this reason.

The biggest opportunity is **novelty**. No one has played a 4D boomer shooter. The combination of fast, aggressive combat with 4D spatial puzzles could create a game that feels genuinely new -- not retro, not modern, but something that has never existed.

The Rust4D engine has a strong foundation (4D math, physics, rendering, scenes), but it currently has zero gameplay systems. Every system in the "What Must Be Built" table needs to be created from scratch. The MVBS (Minimum Viable Boomer Shooter) is achievable but represents a substantial engineering effort -- estimated at 15-25 sessions of focused work across movement, weapons, enemies, pickups, HUD, and one playable level.
