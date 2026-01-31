# Rust4D-Shooter: Lua Game Roadmap

**Date**: 2026-01-31
**Type**: Lua-based game implementation plan
**Companion to**: Engine roadmap (phases P1-P5), `post-split-phase-scripting.md`, `engine-game-split.md`
**Replaces**: `game-roadmap-summary.md` (Rust-based game repo approach)

---

## 1. Overview

Rust4D-Shooter is a 4D boomer shooter -- the first game built on the Rust4D engine. Under the Lua migration, all game logic is written in Lua scripts loaded by the engine's scripting runtime, rather than compiled Rust code in a separate game repository.

### The Shift

- **Before (Rust approach)**: Game repo is a Rust binary (`Cargo.toml` + `src/`) depending on engine crates via git URL. All game logic (weapons, enemies, AI, health, HUD) is compiled Rust.
- **After (Lua approach)**: Engine provides a binary/launcher that loads a game directory containing Lua scripts, RON scene files, TOML config, and assets (textures, sounds, sprites). All game logic is Lua. No compilation step for game changes.

### Key Properties

- Written entirely in Lua scripts + data files (RON scenes, TOML config, assets)
- Hot-reloadable during development: edit a Lua script, save, see changes immediately in-game
- No separate game binary or Cargo.toml -- the engine IS the binary
- Distributed as engine binary + game data directory (or bundled into a single package)
- Modding support comes nearly free -- players can modify the same Lua scripts
- The engine's `rust4d_scripting` crate exposes comprehensive Lua bindings via mlua to all engine APIs

### What Stays in the Engine (Rust)

Everything performance-critical or low-level remains in compiled Rust:
- Physics simulation (`PhysicsWorld::step()`, collision detection, raycasting)
- GPU rendering pipeline (4D slicing compute shader, 3D forward renderer, sprites, particles)
- Audio engine (kira backend, spatial audio processing)
- ECS storage and queries (hecs)
- Window management, input capture (winit)
- Asset loading and caching

### What Moves to Lua (Game Logic)

Everything gameplay-specific is scripted:
- Health, damage, death handling
- Weapon definitions, firing logic, ammo tracking
- Enemy AI state machines, spawning, wave management
- HUD rendering (via engine's egui overlay bindings)
- Game state machine (menu, gameplay, pause)
- Pickup collection, inventory, key/door systems
- Level scripting, trigger responses
- Music and sound effect triggering

---

## 2. Game Directory Structure

```
rust4d-shooter/
├── main.lua                    # Entry point -- registers systems, sets up game
├── config.toml                 # Game configuration (window, audio, controls)
├── scripts/
│   ├── systems/                # Game systems (called per-frame by the engine)
│   │   ├── combat.lua          # Damage processing, hit detection
│   │   ├── weapons.lua         # Weapon state machine, firing, reloading
│   │   ├── enemies.lua         # Enemy AI updates, spawning, death
│   │   ├── movement.lua        # Player movement, jumping, W-strafing
│   │   ├── pickups.lua         # Pickup collection, inventory
│   │   ├── hud.lua             # HUD rendering (health, ammo, crosshair, W-indicator)
│   │   └── triggers.lua        # Trigger response handlers (doors, portals, events)
│   ├── entities/               # Entity definitions / prefabs
│   │   ├── player.lua          # Player entity setup (components, initial state)
│   │   ├── enemies/            # Enemy type definitions
│   │   │   ├── rusher.lua      # Melee rusher AI + stats
│   │   │   ├── gunner.lua      # Projectile gunner AI + stats
│   │   │   └── phaser.lua      # W-phaser AI + stats
│   │   ├── weapons/            # Weapon definitions
│   │   │   ├── shotgun.lua     # Shotgun stats, spread pattern, hitscan
│   │   │   ├── rocket.lua      # Rocket launcher, projectile, splash damage
│   │   │   └── pistol.lua      # Starting weapon, hitscan, reliable
│   │   └── pickups/            # Pickup definitions
│   │       ├── health.lua      # Health pickup
│   │       ├── ammo.lua        # Ammo pickup
│   │       └── key.lua         # Key item pickup
│   ├── states/                 # Game state machine
│   │   ├── menu.lua            # Main menu state
│   │   ├── gameplay.lua        # Active gameplay state
│   │   ├── pause.lua           # Pause menu state
│   │   └── death.lua           # Death / game over state
│   └── lib/                    # Shared utility modules
│       ├── utils.lua           # Math helpers, table utilities
│       ├── constants.lua       # Game-wide constants (speeds, damage values)
│       └── events.lua          # Event helper functions
├── scenes/                     # RON scene files (level geometry, entity placement)
│   ├── e1m1.ron                # Episode 1, Map 1 (tutorial)
│   ├── e1m2.ron                # Episode 1, Map 2
│   └── test_arena.ron          # Development test arena
├── assets/
│   ├── sprites/                # Enemy/item sprite sheets (PNG)
│   │   ├── rusher.png
│   │   ├── gunner.png
│   │   └── phaser.png
│   ├── sounds/                 # Sound effects (WAV/OGG)
│   │   ├── weapons/
│   │   │   ├── shotgun_fire.wav
│   │   │   ├── rocket_fire.wav
│   │   │   └── rocket_explode.wav
│   │   ├── enemies/
│   │   │   ├── rusher_pain.wav
│   │   │   └── enemy_death.wav
│   │   ├── pickups/
│   │   │   ├── health.wav
│   │   │   └── ammo.wav
│   │   └── ambient/
│   │       └── w_phaser_shimmer.wav
│   ├── textures/               # Wall/floor/ceiling textures (PNG)
│   │   ├── brick_wall.png
│   │   ├── metal_floor.png
│   │   └── tech_ceiling.png
│   └── music/                  # Background music (OGG)
│       ├── menu_theme.ogg
│       ├── e1m1_ambient.ogg
│       └── combat_intensity.ogg
└── README.md
```

### Directory Conventions

- `main.lua` is always the entry point. The engine looks for it at the root of the game directory.
- `scripts/systems/` contains Lua modules that export `update(dt)` functions. The engine calls these each frame in registration order.
- `scripts/entities/` contains Lua modules that export factory functions returning component tables.
- `scripts/states/` contains Lua modules implementing game states (enter, update, exit callbacks).
- `scenes/` contains RON files loaded by the engine's scene system. These are data, not scripts.
- `assets/` contains binary assets loaded via the engine's asset system.

---

## 3. Development Workflow

### The Edit-Save-Play Loop

1. Launch the engine pointed at the game directory: `rust4d ./rust4d-shooter/`
2. Engine loads `main.lua`, which registers all game systems and the initial game state
3. Game runs normally
4. Developer edits a Lua script (e.g., `scripts/systems/weapons.lua`) in their text editor
5. Developer saves the file
6. Engine detects the file change and hot-reloads the modified script
7. Changes are visible immediately -- no compilation, no restart

### Hot-Reload Behavior

- **Per-file reload**: Only the changed script is re-executed, not the entire game
- **State preservation**: The engine calls `on_reload()` if the script defines one, allowing scripts to re-initialize cleanly while the game continues running
- **Error resilience**: If a reloaded script has a syntax error or runtime error, the engine displays the error in an overlay and keeps running with the last working version of that script. The game never crashes due to a script error.
- **Scene reload**: RON scene files can also be hot-reloaded (the engine already has file-watching via `AssetCache`)

### Launching

```bash
# Development: run engine with game directory
rust4d ./rust4d-shooter/

# Development: run with debug overlay enabled
rust4d ./rust4d-shooter/ --debug

# Development: run with Lua console enabled
rust4d ./rust4d-shooter/ --console

# Run a specific scene directly (for level testing)
rust4d ./rust4d-shooter/ --scene e1m1.ron
```

### Debugging Tools

- **Error overlay**: When a Lua script errors, a semi-transparent overlay displays the error message, file, line number, and stack trace. The overlay stays until the script is fixed and reloaded.
- **Lua console**: An in-game console (toggled with tilde key) for executing arbitrary Lua expressions, inspecting game state, spawning entities, modifying values live.
- **Debug HUD**: Frame time, entity count, physics body count, active script count. Toggled with F3.
- **Print output**: `print()` in Lua scripts writes to both the terminal and an in-game log panel.

### Distribution

For release, the game directory is bundled with the engine binary:

```
rust4d-shooter-release/
├── rust4d-shooter(.exe)        # Engine binary (renamed for branding)
├── game/                       # Game data directory
│   ├── main.lua
│   ├── scripts/
│   ├── scenes/
│   ├── assets/
│   └── config.toml
└── README.txt
```

The engine binary detects a `game/` subdirectory next to itself and loads it automatically. For Steam, this is the standard layout.

---

## 4. Game Phases

### Phase 0: Project Setup + Core Loop

**Sessions**: 1
**Engine prerequisite**: Scripting runtime functional (can load Lua, call update functions, expose basic ECS/input/physics APIs)

This phase creates the game directory and proves the Lua scripting workflow works end-to-end.

#### Tasks

- [ ] Create the `rust4d-shooter/` directory with the full directory structure
- [ ] Write `main.lua` entry point:
  - Register game systems (movement, combat, weapons, enemies, hud, pickups, triggers)
  - Set up the game state machine (menu -> gameplay -> pause -> death)
  - Load game configuration from `config.toml`
- [ ] Write `scripts/systems/movement.lua`:
  - Get player entity's `CharacterController4D` via Lua bindings
  - Read input actions (move_forward, move_right, move_w, jump) via `engine.input`
  - Apply movement via `controller:move(direction, dt)`
  - Handle jumping via `controller:jump()`
  - Handle W-axis movement (the 4th dimension strafe)
- [ ] Write `scripts/entities/player.lua`:
  - Define player entity factory: spawns entity with `CharacterController4D`, `Health`, `WeaponInventory` components
  - Set initial position from scene's `player_spawn` field
- [ ] Write `scripts/states/gameplay.lua`:
  - On enter: load scene, spawn player, start music
  - On update: run all game systems
  - On exit: cleanup entities
- [ ] Write `scripts/states/menu.lua`:
  - Simple "Press Enter to Start" using `engine.ui` (egui bindings)
- [ ] Write `config.toml` with basic settings (window size, mouse sensitivity, key bindings)
- [ ] Create `scenes/test_arena.ron` -- a flat floor with some walls for testing
- [ ] Verify hot-reload: change movement speed in Lua, save, see change immediately

#### Verification

- `rust4d ./rust4d-shooter/` launches and shows the test arena
- Player can move in all 4 dimensions (WASD + Q/E for W-axis)
- Camera mouse-look works
- Jumping works
- Hot-reload works (edit movement.lua, save, speed changes immediately)
- Menu state transitions to gameplay state

---

### Phase 1: Combat Core

**Sessions**: 2-3
**Engine prerequisite**: Engine Phase 1 complete (raycasting, collision events, trigger detection exposed to Lua)

The core gameplay loop: the player can shoot things and things can take damage.

#### 1.1 Health/Damage System

- [ ] Define `Health` as a Lua component table: `{ current = 100, max = 100 }`
- [ ] Write `scripts/systems/combat.lua`:
  - Each frame: poll `engine.physics:drain_collision_events()`
  - For collision events: check if either entity has a `DealsDamage` component and the other has `Health`
  - Apply damage: `health.current = math.max(0, health.current - damage.amount)`
  - Fire game events on damage: `engine.events:fire("damage", { target = entity, amount = amount })`
  - On death (`health.current <= 0`): fire `"death"` event, begin death sequence

#### 1.2 Weapon System

- [ ] Write `scripts/systems/weapons.lua`:
  - Track active weapon, fire cooldown, ammo
  - On fire input: check cooldown and ammo, then execute weapon's fire function
  - Weapon switching via number keys or scroll wheel
  - Ammo depletion and tracking
- [ ] Write `scripts/entities/weapons/shotgun.lua`:
  - Hitscan weapon: cast 8 rays with spread pattern via `engine.physics:raycast()`
  - Each ray that hits an entity with `Health`: apply damage with distance falloff
  - Effective range ~15 units
- [ ] Write `scripts/entities/weapons/rocket.lua`:
  - Projectile weapon: spawn a physics body with velocity
  - On collision event: query `engine.physics:query_area_effect()` for hyperspherical explosion
  - Apply splash damage with linear falloff to all entities in radius
  - Explosion radius 5 units (4D hypersphere)
- [ ] Write `scripts/entities/weapons/pistol.lua`:
  - Starting weapon, infinite ammo, single hitscan ray
  - Lower damage than shotgun, higher rate of fire

#### 1.3 Death Handling

- [ ] Player death: transition to death state, show "You Died" screen, offer restart
- [ ] Enemy death: fire death event, play death animation, schedule entity despawn
- [ ] Projectile cleanup: despawn projectiles on collision or after timeout

#### Verification

- Shotgun fires and deals damage to entities
- Rocket fires projectile, explodes on impact, deals splash damage
- Weapon switching works
- Ammo depletes and prevents firing when empty
- Player death triggers game-over state
- Enemy death removes entity after animation

---

### Phase 2: HUD + Audio + Feedback

**Sessions**: 2-3
**Engine prerequisite**: Engine Phase 2 complete (audio system, egui overlay, particle system, screen effects exposed to Lua)

This phase makes combat feel good through visual and audio feedback.

#### 2.1 HUD System

- [ ] Write `scripts/systems/hud.lua`:
  - Health bar (bottom-left, red bar showing current/max HP)
  - Ammo counter (bottom-right, current ammo / max ammo for active weapon)
  - Crosshair (center screen, simple + shape)
  - W-position indicator (top-right, shows current W-slice -- CRITICAL for 4D gameplay)
  - Active weapon name display
  - Damage direction indicator (flash on screen edge showing hit direction)
  - All drawn using `engine.ui` (egui Lua bindings)

#### 2.2 Sound Effects

- [ ] Load all game sounds at startup via `engine.audio:load_sound(path)`
- [ ] Wire weapon fire events to spatial sound playback
- [ ] Wire damage/death events to enemy pain/death sounds
- [ ] Wire pickup events to pickup sounds
- [ ] Update listener position each frame via `engine.audio:update_listener(camera_pos)`

#### 2.3 Music System

- [ ] Background music per level (looping OGG tracks)
- [ ] Combat music transition: when enemies are alerted, crossfade to combat track
- [ ] Victory music on level complete

#### 2.4 Screen Effects

- [ ] Screen shake on weapon fire (trauma amount per weapon type)
- [ ] Screen shake on damage taken (proportional to damage)
- [ ] Damage flash (red overlay fading over 0.3s) using egui overlay
- [ ] Pickup flash (green/blue flash on item collection)
- [ ] Muzzle flash via particle system

#### 2.5 Particle Effect Presets

- [ ] Define particle configs as Lua tables:
  - Muzzle flash: 20 particles, 0.1s lifetime, bright yellow-white, additive blend
  - Blood splatter: 30 particles, 0.5s, dark red, gravity, alpha blend
  - Explosion: 100 particles, 1.0s, orange-to-gray, full sphere, drag
  - Impact sparks: 15 particles, 0.2s, white-yellow, narrow spread
- [ ] Wire particle spawning to combat events

#### Verification

- HUD displays health, ammo, crosshair, and W-position correctly
- W-indicator updates as player moves in W dimension
- Weapon fire produces spatial audio at correct position
- Screen shakes on fire and damage
- Damage flash appears when player takes damage
- Particle effects spawn at correct locations

---

### Phase 3: Enemies

**Sessions**: 3-4
**Engine prerequisite**: Engine Phase 3 complete (sprites/billboards, FSM, spatial queries, particles exposed to Lua)

#### 3.1 Enemy Type Definitions

Each enemy type is a Lua module exporting a definition table and AI behavior functions.

- [ ] Write `scripts/entities/enemies/rusher.lua`:
  - Stats: HP 50, speed 15, damage 20, range 1.5, pain_chance 0.5
  - Behavior: Charges straight at player (all 4 dimensions), dangerous in groups
  - W-behavior: Standard (chases in full 4D)
  - Sprite: rusher.png with animation frame ranges
- [ ] Write `scripts/entities/enemies/gunner.lua`:
  - Stats: HP 80, speed 6, damage 15, range 25, pain_chance 0.3
  - Behavior: Keeps distance, fires projectiles at player
  - W-behavior: Standard
  - Sprite: gunner.png
- [ ] Write `scripts/entities/enemies/phaser.lua`:
  - Stats: HP 60, speed 10, damage 25, range 2.0, pain_chance 0.2
  - Behavior: Phases between W-slices to ambush player
  - W-behavior: WPhaser (shifts W-position during combat, fading in/out via sprite W-distance)
  - Sprite: phaser.png

#### 3.2 AI State Machines

- [ ] Write AI as Lua coroutines or state tables:
  - **Idle**: Check for player within sight_range using `engine.physics:query_sphere()`. If found and LOS passes via `engine.physics:line_of_sight()`, transition to Chase.
  - **Chase**: Move toward player (`velocity = direction * move_speed`). If within attack_range, transition to Attack. If LOS lost for >3s, return to Idle.
  - **Attack**: Execute attack (melee hit or spawn projectile). After animation, cooldown, return to Chase.
  - **Pain**: On receiving damage (random check vs pain_chance), briefly stagger. Return to Chase.
  - **Dead**: Play death animation. Despawn after animation completes.

#### 3.3 Enemy Spawning

- [ ] Write `scripts/systems/enemies.lua`:
  - Read spawn point entities from scene (tagged "enemy_spawn" with metadata)
  - Instantiate enemy entities with all components (Health, AI, SpriteAnimation, PhysicsBody)
  - Wave management: trigger-based spawning (player enters area -> spawn wave)
  - Enemy count tracking for level completion

#### 3.4 W-Phasing Behavior

- [ ] Implement W-phaser AI extension:
  - Phase-out: shift W-coordinate away from player's W-slice (sprite fades via engine's W-distance system)
  - Approach: move in XYZ while in adjacent W-slice
  - Phase-in: snap to player's W-slice for melee attack
  - Audio cue when phaser is nearby in adjacent W-slice
- [ ] W-flanker approach logic for future enemies

#### 3.5 Enemy Particles

- [ ] Blood splatter on enemy hit (burst at hit point)
- [ ] Enemy death explosion particles
- [ ] W-phaser "shimmer" particles (glow at projected position when partially visible)
- [ ] Projectile trail particles for enemy projectiles

#### Verification

- Each enemy type spawns with correct stats and sprite
- AI transitions through states correctly (Idle -> Chase -> Attack -> Pain -> Dead)
- Enemies pathfind toward player in 4D
- W-phaser fades in/out when shifting W-slices
- Enemy death spawns particles and drops items
- Spawn triggers activate when player enters trigger zone

---

### Phase 4: Level Design

**Sessions**: 2-3
**Engine prerequisite**: Engine Phase 4 complete (shapes, triggers, tweens exposed to Lua)

#### 4.1 Level Scenes

- [ ] Design Episode 1, Map 1 (tutorial/intro) as `scenes/e1m1.ron`:
  - Use `Hyperprism4D` for walls, floors, platforms
  - W-layered rooms at different W-coordinates
  - Simple layout teaching 3D combat first (no W-movement required)
- [ ] Design Episode 1, Map 2 as `scenes/e1m2.ron`:
  - Introduce W-corridors and W-portals
  - First W-phaser enemy encounter
- [ ] Define spawn points, enemy placements, pickup locations in RON

#### 4.2 Door/Elevator Mechanics

- [ ] Write `scripts/systems/triggers.lua`:
  - Handle `TriggerEnter` events from `engine.physics:drain_collision_events()`
  - Door system: on trigger, check key requirement, tween door position open
  - Elevator system: cycle through waypoints using engine's tween API
  - W-portal system: tween player's W-coordinate to target W-layer

#### 4.3 Key/Door System

- [ ] Implement key inventory tracking in Lua (player table)
- [ ] Key colors: Red, Blue, Yellow
- [ ] Pickup key -> add to inventory -> matching door unlocks
- [ ] Visual feedback on door when player has/lacks the key

#### 4.4 Pickup System

- [ ] Write `scripts/systems/pickups.lua`:
  - Health pickup: restore HP (small = 25, large = 50)
  - Ammo pickup: restore ammo for specific weapon type
  - Weapon pickup: add weapon to inventory if not already owned
  - Key pickup: add key color to inventory
  - Pickup bob animation (sine wave on Y position)
  - Audio and particle feedback on collection

#### 4.5 W-Portal Mechanics

- [ ] Define W-portal trigger zones in RON scenes
- [ ] On player enters W-portal: smoothly tween player's W-coordinate to target
- [ ] Visual cues at portal locations (particle effects, color shift)
- [ ] Audio cue during W-transition

#### 4.6 Level Scripting

- [ ] Secret doors: interaction trigger reveals hidden passage
- [ ] Trap triggers: ceiling crush, floor drop
- [ ] Enemy spawn triggers: player enters area -> spawn wave
- [ ] Completion triggers: all enemies dead -> open exit door

#### Verification

- Player can navigate through a multi-room level
- Doors open with correct key, reject without key
- Elevators cycle through waypoints smoothly
- Pickups restore health/ammo/weapons correctly
- W-portals smoothly transition between W-layers
- Level completion triggers work

---

### Phase 5: Polish + Distribution

**Sessions**: 2-4
**Engine prerequisite**: Engine Phase 5 complete (editor, textures, input rebinding, lights exposed to Lua)

#### 5.1 Menu System

- [ ] Write `scripts/states/menu.lua`:
  - Main menu: Start Game, Level Select, Settings, Quit
  - Background scene rendering behind menu
  - Music on menu screen
- [ ] Write level selection screen:
  - Show available levels with names and episode grouping
  - Track completion status
- [ ] Settings submenus:
  - Controls (key rebinding via `engine.input:rebind()`)
  - Audio (master/SFX/music volume sliders)
  - Video (resolution, fullscreen, V-sync)

#### 5.2 Save/Load System

- [ ] Serialize game state to JSON or Lua table:
  - Player position, health, ammo, weapons, keys
  - Current level, enemy states (alive/dead)
  - Door/elevator states
- [ ] Save to file via `engine.fs:write()`
- [ ] Load on game start or from menu
- [ ] Auto-save on level transitions

#### 5.3 Difficulty Settings

- [ ] Define difficulty as a config table modifying:
  - Enemy HP multiplier
  - Enemy damage multiplier
  - Ammo availability
  - Enemy count per spawn wave

#### 5.4 Textures and Materials

- [ ] Apply wall, floor, ceiling textures to level geometry in RON scenes
- [ ] Create distinct visual themes for different areas / W-layers
- [ ] Ensure triplanar mapping looks good on level geometry

#### 5.5 Distribution

- [ ] Bundle engine binary + game directory
- [ ] Rename engine binary for branding (`rust4d-shooter`)
- [ ] Create launcher script / Steam integration
- [ ] Test on clean machine (no dev environment)

#### Verification

- Full game loop: menu -> level select -> play level -> complete -> next level
- Save/load preserves game state correctly
- Difficulty settings affect gameplay
- Release build runs on a clean system

---

## 5. W-Axis Gameplay Design

The W-axis is not a gimmick -- it fundamentally transforms every boomer shooter system. These notes apply regardless of whether the game is written in Rust or Lua; the gameplay design is the same.

### W-Strafing as Dodge Technique

Players can dodge attacks by shifting along the W-axis. Hitscan attacks require W-alignment (attacker and target at same W-coordinate), making W-strafing an effective dodge. This creates a natural skill curve:
- **Novice players** fight in 3D, ignoring W entirely
- **Intermediate players** learn W-strafing to dodge hitscan attacks
- **Expert players** use W-positioning offensively (flank enemies from different W-slices)

**Lua implementation**: The movement system reads the W-strafe input action and applies it via `CharacterController4D`. The HUD's W-position indicator is the player's primary tool for understanding their W-position.

### Hyperspherical Explosions (R^4 Scaling)

In 4D, explosive blast volume scales as R^4 (compared to R^3 in 3D). Explosions catch enemies in adjacent W-slices, making explosive weapons disproportionately powerful in 4D. This is a deliberate gameplay advantage:
- Rockets counter W-phasing enemies (even if an enemy shifts W, a large enough explosion catches them)
- Creates a natural weapon hierarchy based on W-coverage

**Lua implementation**: `engine.physics:query_area_effect(center, radius, layer_filter, require_los)` uses 4D Euclidean distance naturally. The combat system applies falloff-scaled damage to all hits.

### Hitscan W-Alignment

Hitscan weapons require precise W-alignment between shooter and target. A target at a different W-coordinate is invisible and unhittable with hitscan. This creates a weapon hierarchy:

| Weapon Class | DPS | W-Coverage | Role |
|---|---|---|---|
| Hitscan (shotgun, pistol) | High | Requires W-alignment | Precision, high reward |
| Projectile (tracking) | Medium | Some W-tolerance | Versatile |
| Explosive (rocket) | Lower single-target | Excellent W-coverage | Area denial, anti-phaser |

**Lua implementation**: `engine.physics:raycast()` operates in full 4D. A ray at W=0 never hits an entity at W=2.

### W-Layered Architecture for Levels

Levels exist across multiple W-layers. A single physical location (X,Y,Z) can have different rooms at different W values. W-portals transition the player between layers.

**Lua implementation**: RON scene files place geometry at different W-coordinates. W-portals are trigger zones handled in `scripts/systems/triggers.lua` that tween the player's W-coordinate. The engine's rendering naturally shows/hides geometry based on W-distance.

### W-Flanking Enemies

The W-Phaser enemy approaches from adjacent W-slices, appearing as a ghostly shimmer before phasing in for an attack. Players hear audio cues and see faint sprites, creating tension about an impending attack from "the 4th dimension."

**Lua implementation**: The phaser AI script controls the enemy's W-coordinate. The engine's sprite W-distance fade handles visuals automatically. The audio system's 4D spatial positioning makes W-distant sounds muffled/filtered.

### Cognitive Overload Mitigation

4D is hard for humans to reason about. The game teaches it incrementally:

1. **Level 1 (e1m1)**: Standard 3D gameplay. No W-movement required. Teach basic combat.
2. **Level 2 (e1m2)**: Introduce W-indicator on HUD. Simple W-corridors (move forward in W to progress).
3. **Level 3**: Introduce W-strafing as dodge. Enemies telegraph attacks, giving time to W-dodge.
4. **Level 4**: Introduce W-phasing enemies. Players learn to watch W-indicator for incoming threats.
5. **Level 5+**: Full 4D gameplay. W-layered levels, multiple enemy types, all weapon types.

**Critical HUD element**: The W-position indicator must be prominent, intuitive, and always visible. It is the player's primary tool for understanding the 4th dimension. In `scripts/systems/hud.lua`, this is drawn as a vertical bar or number display in the top-right corner, updating every frame.

---

## 6. Example Lua Scripts

These are realistic, substantial examples showing what the actual game code looks like when using the engine's Lua API.

### 6.1 `main.lua` -- Entry Point

```lua
-- main.lua: Rust4D-Shooter entry point
-- The engine calls this file when the game directory is loaded.

local constants = require("scripts.lib.constants")
local events = require("scripts.lib.events")

-- Game state
local game = {
    state = "menu",
    current_level = nil,
    player = nil,
    score = 0,
}

-- System modules (loaded lazily, hot-reloadable)
local systems = {}

local function load_systems()
    systems.movement = require("scripts.systems.movement")
    systems.combat = require("scripts.systems.combat")
    systems.weapons = require("scripts.systems.weapons")
    systems.enemies = require("scripts.systems.enemies")
    systems.hud = require("scripts.systems.hud")
    systems.pickups = require("scripts.systems.pickups")
    systems.triggers = require("scripts.systems.triggers")
end

-- State modules
local states = {}

local function load_states()
    states.menu = require("scripts.states.menu")
    states.gameplay = require("scripts.states.gameplay")
    states.pause = require("scripts.states.pause")
    states.death = require("scripts.states.death")
end

-- Called once when the game first loads
function on_init()
    load_systems()
    load_states()

    -- Load game configuration
    local config = engine.config:load("config.toml")
    engine.audio:set_master_volume(config.audio.master_volume or 1.0)
    engine.audio:set_bus_volume("music", config.audio.music_volume or 0.7)
    engine.audio:set_bus_volume("sfx", config.audio.sfx_volume or 1.0)

    -- Pre-load commonly used sounds
    game.sounds = {
        shotgun_fire = engine.audio:load_sound("assets/sounds/weapons/shotgun_fire.wav"),
        rocket_fire = engine.audio:load_sound("assets/sounds/weapons/rocket_fire.wav"),
        rocket_explode = engine.audio:load_sound("assets/sounds/weapons/rocket_explode.wav"),
        pickup_health = engine.audio:load_sound("assets/sounds/pickups/health.wav"),
        pickup_ammo = engine.audio:load_sound("assets/sounds/pickups/ammo.wav"),
        menu_music = engine.audio:load_sound("assets/music/menu_theme.ogg"),
    }

    -- Start in menu state
    transition_state("menu")
end

-- Called every frame
function on_update(dt)
    local current_state = states[game.state]
    if current_state and current_state.update then
        current_state.update(dt, game)
    end
end

-- State transition
function transition_state(new_state)
    local old = states[game.state]
    if old and old.exit then
        old.exit(game)
    end

    game.state = new_state

    local new = states[game.state]
    if new and new.enter then
        new.enter(game)
    end
end

-- Called when any script file is hot-reloaded
function on_reload(changed_file)
    -- Re-require the changed module so changes take effect
    local module_path = changed_file:gsub("/", "."):gsub("%.lua$", "")
    package.loaded[module_path] = nil

    -- Reload system/state tables
    load_systems()
    load_states()

    engine.log:info("Hot-reloaded: " .. changed_file)
end

-- Expose game state for other scripts
return game
```

### 6.2 `scripts/systems/combat.lua` -- Damage Processing

```lua
-- combat.lua: Processes collision events and applies damage
local constants = require("scripts.lib.constants")

local combat = {}

-- Damage flash state
local damage_flash_timer = 0
local DAMAGE_FLASH_DURATION = 0.3

function combat.update(dt, game)
    if game.state ~= "gameplay" then return end

    -- Poll collision events from the physics engine
    local events = engine.physics:drain_collision_events()

    for _, event in ipairs(events) do
        if event.kind == "body_vs_body" then
            combat.handle_body_collision(event.body_a, event.body_b, game)
        elseif event.kind == "body_vs_static" then
            combat.handle_static_collision(event.body, event.static_index, game)
        elseif event.kind == "trigger_enter" then
            combat.handle_trigger_enter(event.body, event.trigger_index, game)
        end
    end

    -- Update damage flash
    if damage_flash_timer > 0 then
        damage_flash_timer = damage_flash_timer - dt
    end
end

function combat.handle_body_collision(entity_a, entity_b, game)
    -- Check if one entity deals damage and the other has health
    local damage_a = engine.ecs:get(entity_a, "DealsDamage")
    local damage_b = engine.ecs:get(entity_b, "DealsDamage")
    local health_a = engine.ecs:get(entity_a, "Health")
    local health_b = engine.ecs:get(entity_b, "Health")

    if damage_a and health_b then
        combat.apply_damage(entity_b, health_b, damage_a.amount, entity_a, game)
    end
    if damage_b and health_a then
        combat.apply_damage(entity_a, health_a, damage_b.amount, entity_b, game)
    end
end

function combat.apply_damage(target, health, amount, source, game)
    health.current = math.max(0, health.current - amount)
    engine.ecs:set(target, "Health", health)

    -- Fire damage event for other systems (audio, particles, AI)
    engine.events:fire("damage", {
        target = target,
        source = source,
        amount = amount,
        position = engine.ecs:get(target, "Transform").position,
    })

    -- If this is the player, trigger damage flash and screen shake
    if target == game.player then
        damage_flash_timer = DAMAGE_FLASH_DURATION
        engine.screen_shake:add_trauma(math.min(amount / 50.0, 0.8))
    end

    -- Check for death
    if health.current <= 0 then
        combat.handle_death(target, game)
    end
end

function combat.handle_death(entity, game)
    local transform = engine.ecs:get(entity, "Transform")
    local pos = transform.position

    -- Spawn death particles
    engine.particles:spawn_burst(pos, constants.PARTICLE_DEATH_EXPLOSION)

    -- Fire death event
    engine.events:fire("death", { entity = entity, position = pos })

    if entity == game.player then
        -- Player died: transition to death state
        transition_state("death")
    else
        -- Enemy died: play death sound, remove after delay
        engine.audio:play_oneshot_spatial(
            game.sounds.enemy_death,
            pos, "sfx", { max_distance = 30.0 }
        )
        -- Schedule despawn after death animation plays
        engine.ecs:set(entity, "DespawnTimer", { remaining = 1.0 })
    end
end

function combat.draw_damage_flash(game)
    if damage_flash_timer > 0 then
        local intensity = damage_flash_timer / DAMAGE_FLASH_DURATION
        local alpha = math.floor(intensity * 128)
        engine.ui:fill_screen(255, 0, 0, alpha)
    end
end

return combat
```

### 6.3 `scripts/entities/weapons/shotgun.lua` -- Weapon Definition

```lua
-- shotgun.lua: Hitscan shotgun weapon definition
local constants = require("scripts.lib.constants")

local shotgun = {}

shotgun.definition = {
    name = "Shotgun",
    weapon_type = "shotgun",
    ammo_max = 50,
    ammo_per_shot = 1,
    fire_rate = 1.2,                -- Shots per second
    damage_per_pellet = 12,
    pellet_count = 8,
    spread_angle = 0.08,            -- Radians of cone spread
    effective_range = 20.0,         -- Distance for full damage
    max_range = 40.0,               -- Beyond this, no damage
    damage_falloff_start = 15.0,    -- Distance where falloff begins
    screen_shake_trauma = 0.3,
    -- Particle config for muzzle flash
    muzzle_flash = {
        max_particles = 20,
        burst_count = 15,
        lifetime = 0.1,
        initial_color = { 1.0, 0.9, 0.5, 1.0 },
        end_color = { 1.0, 0.3, 0.0, 0.0 },
        initial_size = 0.3,
        end_size = 0.05,
        velocity_randomness = 0.8,
        gravity = 0.0,
        drag = 5.0,
        blend_mode = "additive",
    },
}

-- Fire the shotgun
function shotgun.fire(player_entity, game)
    local def = shotgun.definition
    local inventory = engine.ecs:get(player_entity, "WeaponInventory")

    -- Check ammo
    if inventory.ammo.shotgun < def.ammo_per_shot then
        return false -- No ammo
    end

    -- Deduct ammo
    inventory.ammo.shotgun = inventory.ammo.shotgun - def.ammo_per_shot
    engine.ecs:set(player_entity, "WeaponInventory", inventory)

    -- Get camera position and direction for ray origin
    local camera = engine.camera:get_position()
    local forward = engine.camera:get_forward()
    local right = engine.camera:get_right()
    local up = engine.camera:get_up()

    -- Cast pellet rays with spread
    local total_damage_dealt = 0
    for i = 1, def.pellet_count do
        -- Random spread within cone
        local spread_x = (math.random() - 0.5) * 2.0 * def.spread_angle
        local spread_y = (math.random() - 0.5) * 2.0 * def.spread_angle
        local direction = engine.vec4.normalize(
            forward + right * spread_x + up * spread_y
        )

        -- Raycast through the physics world
        local hit = engine.physics:raycast_nearest(
            camera, direction, def.max_range,
            constants.LAYER_ENEMY + constants.LAYER_STATIC
        )

        if hit then
            if hit.target_type == "body" then
                local health = engine.ecs:get(hit.entity, "Health")
                if health then
                    -- Calculate damage with distance falloff
                    local damage = def.damage_per_pellet
                    if hit.distance > def.damage_falloff_start then
                        local falloff = 1.0 - (hit.distance - def.damage_falloff_start)
                            / (def.max_range - def.damage_falloff_start)
                        damage = damage * math.max(0, falloff)
                    end

                    -- Apply damage
                    health.current = math.max(0, health.current - damage)
                    engine.ecs:set(hit.entity, "Health", health)
                    total_damage_dealt = total_damage_dealt + damage

                    -- Blood particles at hit point
                    engine.particles:spawn_burst(
                        hit.point,
                        constants.PARTICLE_BLOOD_SPLATTER
                    )

                    -- Check death
                    if health.current <= 0 then
                        engine.events:fire("death", {
                            entity = hit.entity,
                            position = hit.point,
                        })
                    end
                end
            end

            -- Impact sparks on any surface
            engine.particles:spawn_burst(hit.point, constants.PARTICLE_IMPACT_SPARKS)
        end
    end

    -- Muzzle flash particles
    local muzzle_pos = camera + forward * 0.5
    engine.particles:spawn_burst(muzzle_pos, def.muzzle_flash)

    -- Sound
    engine.audio:play_oneshot_spatial(
        game.sounds.shotgun_fire,
        camera, "sfx", { max_distance = 40.0 }
    )

    -- Screen shake
    engine.screen_shake:add_trauma(def.screen_shake_trauma)

    return true -- Fired successfully
end

return shotgun
```

### 6.4 `scripts/entities/enemies/rusher.lua` -- Melee Enemy AI

```lua
-- rusher.lua: Melee rusher enemy type
-- Fast, aggressive, charges straight at the player in all 4 dimensions.
local constants = require("scripts.lib.constants")

local rusher = {}

-- Enemy definition (stats)
rusher.definition = {
    name = "Rusher",
    health = 50,
    move_speed = 15.0,
    attack_damage = 20,
    attack_range = 1.5,
    sight_range = 30.0,
    pain_chance = 0.5,           -- 50% chance to flinch on hit
    pain_duration = 0.4,
    attack_cooldown = 0.8,
    w_behavior = "standard",     -- Chases in all 4 dimensions
    sprite_sheet = "assets/sprites/rusher.png",
    anim = {
        idle   = { frames = {0, 1, 2, 3},     fps = 4,  looping = true },
        walk   = { frames = {4, 5, 6, 7, 8, 9, 10, 11}, fps = 10, looping = true },
        attack = { frames = {12, 13, 14, 15},  fps = 8,  looping = false },
        pain   = { frames = {16},              fps = 1,  looping = false },
        death  = { frames = {17, 18, 19, 20, 21}, fps = 8, looping = false },
    },
}

-- Spawn a rusher at a given 4D position
function rusher.spawn(position)
    local def = rusher.definition
    local entity = engine.ecs:spawn()

    engine.ecs:set(entity, "Transform", { position = position })
    engine.ecs:set(entity, "Health", { current = def.health, max = def.health })
    engine.ecs:set(entity, "EnemyAI", {
        state = "idle",
        state_timer = 0,
        target = nil,
        attack_cooldown = 0,
        enemy_type = "rusher",
    })
    engine.ecs:set(entity, "Sprite", {
        sheet = def.sprite_sheet,
        current_anim = "idle",
        current_frame = 0,
        w_fade_range = 3.0,
    })

    -- Physics body: sphere collider, enemy layer
    local body_key = engine.physics:add_body({
        position = position,
        collider = { type = "sphere", radius = 0.5 },
        mass = 2.0,
        layer = constants.LAYER_ENEMY,
        mask = constants.LAYER_PLAYER + constants.LAYER_STATIC + constants.LAYER_PROJECTILE,
    })
    engine.ecs:set(entity, "PhysicsBody", { key = body_key })

    return entity
end

-- AI update (called per frame for each rusher entity)
function rusher.update_ai(entity, dt, game)
    local ai = engine.ecs:get(entity, "EnemyAI")
    local transform = engine.ecs:get(entity, "Transform")
    local def = rusher.definition

    ai.state_timer = ai.state_timer + dt
    ai.attack_cooldown = math.max(0, ai.attack_cooldown - dt)

    if ai.state == "idle" then
        rusher.update_idle(entity, ai, transform, dt, game)
    elseif ai.state == "chase" then
        rusher.update_chase(entity, ai, transform, dt, game)
    elseif ai.state == "attack" then
        rusher.update_attack(entity, ai, transform, dt, game)
    elseif ai.state == "pain" then
        rusher.update_pain(entity, ai, transform, dt, game)
    elseif ai.state == "dead" then
        rusher.update_dead(entity, ai, transform, dt, game)
    end

    engine.ecs:set(entity, "EnemyAI", ai)
end

function rusher.update_idle(entity, ai, transform, dt, game)
    -- Look for player within sight range
    local player_transform = engine.ecs:get(game.player, "Transform")
    if not player_transform then return end

    local distance = engine.vec4.distance(transform.position, player_transform.position)

    if distance <= rusher.definition.sight_range then
        -- Check line of sight
        local can_see = engine.physics:line_of_sight(
            transform.position,
            player_transform.position,
            constants.LAYER_STATIC
        )
        if can_see then
            rusher.transition(entity, ai, "chase")
            ai.target = game.player
        end
    end

    -- Set idle animation
    rusher.set_anim(entity, "idle")
end

function rusher.update_chase(entity, ai, transform, dt, game)
    local def = rusher.definition
    local player_transform = engine.ecs:get(game.player, "Transform")
    if not player_transform then return end

    local direction = engine.vec4.normalize(
        player_transform.position - transform.position
    )
    local distance = engine.vec4.distance(transform.position, player_transform.position)

    -- Move toward player (in full 4D)
    local body = engine.ecs:get(entity, "PhysicsBody")
    engine.physics:set_velocity(body.key, direction * def.move_speed)

    -- Check if in attack range
    if distance <= def.attack_range and ai.attack_cooldown <= 0 then
        rusher.transition(entity, ai, "attack")
    end

    -- If lost LOS for too long, return to idle
    local can_see = engine.physics:line_of_sight(
        transform.position,
        player_transform.position,
        constants.LAYER_STATIC
    )
    if not can_see and ai.state_timer > 3.0 then
        rusher.transition(entity, ai, "idle")
        ai.target = nil
    end

    rusher.set_anim(entity, "walk")
end

function rusher.update_attack(entity, ai, transform, dt, game)
    local def = rusher.definition

    -- On first frame of attack state, deal damage
    if ai.state_timer < dt * 2 then
        -- Melee hit: check if player is still in range
        local player_transform = engine.ecs:get(game.player, "Transform")
        if player_transform then
            local distance = engine.vec4.distance(
                transform.position, player_transform.position
            )
            if distance <= def.attack_range * 1.5 then
                local player_health = engine.ecs:get(game.player, "Health")
                if player_health then
                    player_health.current = math.max(0,
                        player_health.current - def.attack_damage)
                    engine.ecs:set(game.player, "Health", player_health)
                    engine.events:fire("damage", {
                        target = game.player,
                        source = entity,
                        amount = def.attack_damage,
                        position = player_transform.position,
                    })
                end
            end
        end
    end

    -- After attack animation, return to chase
    local sprite = engine.ecs:get(entity, "Sprite")
    if sprite and sprite.anim_finished then
        ai.attack_cooldown = def.attack_cooldown
        rusher.transition(entity, ai, "chase")
    end

    rusher.set_anim(entity, "attack")
end

function rusher.update_pain(entity, ai, transform, dt, game)
    if ai.state_timer >= rusher.definition.pain_duration then
        rusher.transition(entity, ai, "chase")
    end
    rusher.set_anim(entity, "pain")
end

function rusher.update_dead(entity, ai, transform, dt, game)
    -- Wait for death animation, then despawn
    local sprite = engine.ecs:get(entity, "Sprite")
    if sprite and sprite.anim_finished then
        engine.ecs:despawn(entity)
    end
    rusher.set_anim(entity, "death")
end

-- Handle damage event (called from enemy system)
function rusher.on_damage(entity, amount, game)
    local ai = engine.ecs:get(entity, "EnemyAI")
    if ai.state == "dead" then return end

    -- Pain chance check
    if math.random() < rusher.definition.pain_chance then
        rusher.transition(entity, ai, "pain")
        engine.ecs:set(entity, "EnemyAI", ai)

        -- Stop movement during pain
        local body = engine.ecs:get(entity, "PhysicsBody")
        engine.physics:set_velocity(body.key, engine.vec4.ZERO)
    end

    -- Blood particles at entity position
    local transform = engine.ecs:get(entity, "Transform")
    engine.particles:spawn_burst(transform.position, constants.PARTICLE_BLOOD_SPLATTER)

    -- Pain sound
    engine.audio:play_oneshot_spatial(
        game.sounds.enemy_pain or game.sounds.enemy_death,
        transform.position, "sfx", { max_distance = 25.0 }
    )
end

-- State transition helper
function rusher.transition(entity, ai, new_state)
    ai.state = new_state
    ai.state_timer = 0
end

-- Animation helper
function rusher.set_anim(entity, anim_name)
    local sprite = engine.ecs:get(entity, "Sprite")
    if sprite and sprite.current_anim ~= anim_name then
        sprite.current_anim = anim_name
        sprite.current_frame = 0
        sprite.anim_finished = false
        engine.ecs:set(entity, "Sprite", sprite)
    end
end

return rusher
```

### 6.5 `scripts/systems/hud.lua` -- HUD Drawing

```lua
-- hud.lua: Draws the in-game HUD using engine.ui (egui bindings)
local constants = require("scripts.lib.constants")

local hud = {}

-- W-indicator pulse for proximity warnings
local w_warning_pulse = 0

function hud.update(dt, game)
    if game.state ~= "gameplay" then return end

    local player = game.player
    if not player then return end

    local health = engine.ecs:get(player, "Health")
    local inventory = engine.ecs:get(player, "WeaponInventory")
    local transform = engine.ecs:get(player, "Transform")

    if not health or not inventory or not transform then return end

    local screen_w, screen_h = engine.ui:screen_size()

    -- === Health Bar (bottom-left) ===
    local health_pct = health.current / health.max
    local health_color = hud.health_color(health_pct)

    engine.ui:begin_area("health", 20, screen_h - 80)
    engine.ui:text("HP", { size = 14, color = { 200, 200, 200 } })
    engine.ui:progress_bar(health_pct, 150, 20, health_color)
    engine.ui:text(
        string.format("%d / %d", math.ceil(health.current), health.max),
        { size = 18, color = { 255, 255, 255 }, bold = true }
    )
    engine.ui:end_area()

    -- === Ammo Counter (bottom-right) ===
    local weapon_name = inventory.current_weapon or "Pistol"
    local ammo = inventory.ammo[weapon_name:lower()] or 0
    local ammo_max = inventory.ammo_max[weapon_name:lower()] or 999

    engine.ui:begin_area("ammo", screen_w - 180, screen_h - 80)
    engine.ui:text(weapon_name:upper(), { size = 14, color = { 200, 200, 200 } })
    engine.ui:text(
        string.format("%d / %d", ammo, ammo_max),
        { size = 24, color = { 255, 255, 200 }, bold = true }
    )
    engine.ui:end_area()

    -- === Crosshair (center screen) ===
    local cx = screen_w / 2
    local cy = screen_h / 2
    local cross_size = 12
    local cross_thick = 2
    local cross_color = { 255, 255, 255, 200 }
    -- Horizontal line
    engine.ui:draw_line(cx - cross_size, cy, cx - 4, cy, cross_thick, cross_color)
    engine.ui:draw_line(cx + 4, cy, cx + cross_size, cy, cross_thick, cross_color)
    -- Vertical line
    engine.ui:draw_line(cx, cy - cross_size, cx, cy - 4, cross_thick, cross_color)
    engine.ui:draw_line(cx, cy + 4, cx, cy + cross_size, cross_thick, cross_color)

    -- === W-Position Indicator (top-right) ===
    -- This is the CRITICAL element for 4D gameplay awareness
    local player_w = transform.position.w

    -- Check for nearby W-threats (enemy in adjacent W-slice)
    local w_threats = hud.check_w_threats(transform.position, game)
    if w_threats > 0 then
        w_warning_pulse = w_warning_pulse + dt * 4.0
    else
        w_warning_pulse = math.max(0, w_warning_pulse - dt * 2.0)
    end

    local w_bg_color = { 30, 30, 50, 180 }
    if w_warning_pulse > 0 then
        local pulse = math.abs(math.sin(w_warning_pulse * 3.14))
        w_bg_color = {
            math.floor(30 + 100 * pulse),
            30,
            math.floor(50 + 50 * pulse),
            200,
        }
    end

    engine.ui:begin_area("w_indicator", screen_w - 130, 20)
    engine.ui:panel(120, 60, w_bg_color)
    engine.ui:text("W-SLICE", { size = 11, color = { 150, 150, 200 } })
    engine.ui:text(
        string.format("%.1f", player_w),
        { size = 28, color = { 100, 200, 255 }, bold = true }
    )
    if w_threats > 0 then
        engine.ui:text("!! THREAT !!", { size = 10, color = { 255, 100, 100 } })
    end
    engine.ui:end_area()

    -- === W-Slice Mini-Bar (below W indicator) ===
    -- Visual representation of player's W position on a scale
    engine.ui:begin_area("w_bar", screen_w - 130, 85)
    hud.draw_w_bar(player_w, 120, 8)
    engine.ui:end_area()
end

-- Returns a color based on health percentage (green -> yellow -> red)
function hud.health_color(pct)
    if pct > 0.6 then
        return { 50, 200, 50 }
    elseif pct > 0.3 then
        return { 200, 200, 50 }
    else
        return { 200, 50, 50 }
    end
end

-- Draw the W-position bar showing where the player is on the W-axis
function hud.draw_w_bar(w_pos, width, height)
    local w_min = -5.0
    local w_max = 10.0
    local w_range = w_max - w_min
    local normalized = (w_pos - w_min) / w_range
    normalized = math.max(0, math.min(1, normalized))

    -- Background bar
    engine.ui:draw_rect(0, 0, width, height, { 40, 40, 60, 150 })
    -- Position indicator
    local indicator_x = math.floor(normalized * (width - 4))
    engine.ui:draw_rect(indicator_x, 0, 4, height, { 100, 200, 255, 255 })
end

-- Check for enemies in adjacent W-slices (within threat range)
function hud.check_w_threats(player_pos, game)
    -- Query for enemies within 5 units but NOT in the same W-slice
    local nearby = engine.physics:query_sphere(
        player_pos, 15.0, constants.LAYER_ENEMY
    )

    local threats = 0
    for _, result in ipairs(nearby) do
        local w_distance = math.abs(result.position.w - player_pos.w)
        -- Threat: enemy is nearby in XYZ but in a different W-slice
        if w_distance > 0.5 and w_distance < 3.0 then
            threats = threats + 1
        end
    end
    return threats
end

return hud
```

---

## 7. Session Estimates

### Per-Phase Estimates

| Phase | Sessions | Engine Prerequisite | Can Start When |
|---|---|---|---|
| Phase 0: Project Setup + Core Loop | 1 | Scripting runtime functional | Scripting phase complete |
| Phase 1: Combat Core | 2-3 | Engine P1 Lua bindings (raycasting, events, triggers) | Engine P1 + scripting done |
| Phase 2: HUD + Audio + Feedback | 2-3 | Engine P2 Lua bindings (audio, egui, particles, screen effects) | Engine P2 + scripting done |
| Phase 3: Enemies | 3-4 | Engine P3 Lua bindings (sprites, FSM, spatial queries) | Engine P3 + scripting done |
| Phase 4: Level Design | 2-3 | Engine P4 Lua bindings (shapes, triggers, tweens) | Engine P4 + scripting done |
| Phase 5: Polish + Distribution | 2-4 | Engine P5 Lua bindings (editor, textures, rebinding) | Engine P5 + scripting done |
| **Total Game Work** | **12-18** | | |

### Comparison to Rust Approach

The Lua approach is slightly faster than the original Rust estimate (13-21 sessions) because:
- No compilation step means faster iteration during development
- No `Cargo.toml` / dependency management for the game repo
- Hot-reload eliminates the build-test cycle for tuning values
- Lua tables are more concise than Rust struct definitions for game data

However, some tasks take slightly longer in Lua due to:
- Lack of type checking (bugs surface at runtime, not compile time)
- Lua API bindings need to be learned (less documentation than Rust crate APIs)
- Some patterns (enums, pattern matching) are less ergonomic in Lua

### Dependencies on Engine

| Game Phase | Required Engine Work | Specific Lua APIs Needed |
|---|---|---|
| Phase 0 | Scripting runtime + basic bindings | `engine.ecs`, `engine.physics` (basic), `engine.input`, `engine.camera`, `engine.config`, `engine.audio` (basic) |
| Phase 1 | Engine P1 bindings | `engine.physics:raycast()`, `engine.physics:raycast_nearest()`, `engine.physics:drain_collision_events()`, `engine.physics:query_area_effect()`, `engine.events` |
| Phase 2 | Engine P2 bindings | `engine.audio` (full: `load_sound`, `play_spatial`, `play_oneshot_spatial`, `update_listener`, `set_bus_volume`), `engine.ui` (egui), `engine.particles`, `engine.screen_shake` |
| Phase 3 | Engine P3 bindings | `engine.sprites` (`add_sprite_4d`, `SpriteAnimation`), `engine.physics:query_sphere()`, `engine.physics:line_of_sight()`, `engine.physics:apply_impulse()` |
| Phase 4 | Engine P4 bindings | `engine.tweens` (`tween_position`, `EasingFunction`), `engine.triggers` (`TriggerRuntime`), scene loading with shapes |
| Phase 5 | Engine P5 bindings | `engine.input:rebind()`, `engine.textures`, `engine.fs` (file I/O for save/load) |

### What Can Start Immediately

As soon as the scripting runtime loads and executes Lua files with basic ECS and input bindings:
- Phase 0 (directory structure, main.lua, game state machine, basic movement)
- Basic player movement and camera control
- Menu and pause state scaffolding
- Game configuration loading

This means game development can begin in parallel with Engine Phases P1-P5, as long as the scripting runtime itself is functional.

### Critical Path

```
Engine Scripting Phase (new, ~3-4 sessions)
  -> Game Phase 0 (1 session, can start immediately after scripting runtime works)
    -> Game Phase 1 (2-3 sessions, needs P1 Lua bindings)
      -> Game Phase 2 (2-3 sessions, needs P2 Lua bindings)
        -> Game Phase 3 (3-4 sessions, needs P3 Lua bindings)
          -> Game Phase 4 (2-3 sessions, needs P4 Lua bindings)
            -> Game Phase 5 (2-4 sessions, needs P5 Lua bindings)
```

Game Phase 0 can begin as soon as the scripting runtime loads Lua and exposes basic APIs. Each subsequent game phase unlocks as the corresponding engine phase's Lua bindings are complete. The game critical path is approximately 12-18 sessions, running in parallel with engine development.

---

## 8. Advantages of the Lua Approach

### Hot-Reload = Fast Iteration

The single biggest advantage. In the Rust approach, changing a damage value requires:
1. Edit Rust source
2. `cargo build` (10-30 seconds, sometimes minutes)
3. Restart the game
4. Navigate back to the test scenario
5. Test the change

In the Lua approach:
1. Edit Lua script
2. Save
3. Change is live immediately, in the same running game session

For gameplay tuning (damage values, movement speeds, enemy behavior, weapon spread) this is transformative. A weapon that takes an hour to tune in Rust takes 15 minutes in Lua.

### No Compile Times for Game Logic

Rust compile times are a real friction point. Even with incremental compilation, the game binary rebuild cycle is 10-30 seconds. With Lua, game logic changes are instant. Only engine changes (physics, rendering, audio backends) require recompilation.

### Modding Support Comes Nearly Free

Since the game is Lua scripts + data files, players can modify the same scripts to create mods. The engine loads scripts from a directory -- point it at a modded copy and it just works. Possible mods with zero engine changes:
- New weapons (add a Lua file to `scripts/entities/weapons/`)
- New enemy types (add a Lua file to `scripts/entities/enemies/`)
- New levels (add RON scene files to `scenes/`)
- Gameplay tweaks (edit `scripts/lib/constants.lua`)
- Total conversions (replace everything in the game directory)

### Clear Engine/Game Separation

The Lua approach enforces a clean boundary between engine and game by the language barrier itself. There is no temptation to "just put this game-specific thing in the engine crate" because you physically cannot write Rust code from the game side. Every game feature must use the engine's Lua API, which means the API must be well-designed and complete.

### Lua is Well-Known

Lua is one of the most widely used scripting languages in game development (World of Warcraft, Roblox, Garry's Mod, Factorio, LOVE2D). There is a much larger pool of people who can write Lua gameplay code than people who can write Rust gameplay code. This matters for:
- Modding community
- Future collaborators
- Learning resources and documentation

### Faster Prototyping

Lua's dynamic typing and concise syntax make it faster to prototype new game systems. An enemy AI that takes 150 lines of Rust (with struct definitions, impl blocks, match statements) is 60 lines of Lua. This speeds up the experimental phase where designs are still changing.

---

## 9. Risks and Mitigations

### Performance

**Risk**: Lua is slower than compiled Rust for computation.

**Mitigation**: Game logic is not the performance bottleneck. Physics simulation, collision detection, rendering, and audio processing are all Rust. Lua handles decision-making (AI state transitions, damage calculations, input processing) which involves simple arithmetic and table lookups, not heavy computation. At 60fps with 50 enemies, the Lua overhead is negligible compared to physics and rendering.

**Monitoring**: If Lua performance becomes measurable, the most expensive systems (spatial queries, raycasting) are already in Rust. Only the game-side decision logic runs in Lua.

### Debugging

**Risk**: Lua errors are runtime-only (no compiler to catch type errors).

**Mitigation**:
- Error overlay displays file, line, and stack trace -- errors are visible immediately
- Hot-reload means the fix cycle is fast (fix typo, save, game continues)
- Lua console for live inspection of game state
- Consider `luacheck` (static analysis tool for Lua) as part of a CI pipeline
- Consider runtime type-checking helpers in dev mode (e.g., `assert(type(health) == "table", "Health must be a table")`)
- The engine should format Lua stack traces with clear, readable file paths

### Type Safety

**Risk**: Lua is dynamically typed. Misspelled field names, wrong argument types, and nil access are silent until they crash at runtime.

**Mitigation**:
- Document all engine APIs with expected types (Lua doc comments)
- Runtime type checks in development mode (assert argument types in engine bindings)
- Consistent naming conventions across all game scripts
- The `constants.lua` module centralizes magic values
- Consider generating type stubs for IDE autocompletion (Lua Language Server supports type annotations)

### State Management on Hot-Reload

**Risk**: When a script is hot-reloaded, its module-level state is lost. A weapon system that tracks cooldown timers in module-local variables will lose those timers on reload.

**Mitigation**:
- Design scripts to store mutable state on ECS entities, not in module-local variables. Entity state survives hot-reload because it lives in the engine's Rust ECS.
- The `on_reload()` callback allows scripts to re-initialize gracefully.
- Module-level state should be limited to constant definitions and configuration tables.
- Document this pattern clearly: "state on entities, logic in scripts."

### API Surface Coverage

**Risk**: The engine's Lua API might not expose everything the game needs, requiring engine changes mid-development.

**Mitigation**:
- The scripting phase plan should define bindings for ALL APIs listed in this document
- The example scripts in this document serve as a specification for the Lua API surface
- A "binding coverage" checklist maps every `engine.*` call in the examples to a specific Rust binding implementation
- If a missing binding is discovered during game development, adding it is a small task (typically <30 minutes for a simple wrapper)

### Distribution Complexity

**Risk**: Distributing "engine binary + game directory" is less standard than a single executable.

**Mitigation**:
- The engine detects a `game/` subdirectory next to itself, making distribution straightforward
- For Steam, this is the standard model (executable + data files)
- A future build step could embed Lua scripts into the binary if a single-file distribution is needed
- Other engines (LOVE2D, Defold) use this same model successfully

### Lua Version / Compatibility

**Risk**: Different Lua versions have incompatibilities (5.1 vs 5.4 vs LuaJIT).

**Mitigation**:
- mlua supports both Lua 5.4 and LuaJIT. Choose one and document it.
- Recommendation: **LuaJIT** for performance (JIT compilation, FFI). It is Lua 5.1 compatible with some 5.2 extensions.
- If LuaJIT is chosen, document the specific Lua 5.1 dialect used (no goto, no bitwise operators via `//`, use `bit` library instead).
- Lock the Lua version in the engine's `Cargo.toml` mlua feature flags.

---

## 10. Lua API Design Principles

These principles guide how the engine exposes APIs to Lua scripts:

### 1. Namespace Everything Under `engine.*`

All engine APIs are accessed via the global `engine` table:
- `engine.ecs` -- Entity Component System operations
- `engine.physics` -- Physics world queries and manipulation
- `engine.audio` -- Sound loading and playback
- `engine.input` -- Input action queries
- `engine.camera` -- Camera position and orientation
- `engine.ui` -- egui overlay drawing
- `engine.particles` -- Particle effect spawning
- `engine.screen_shake` -- Camera shake
- `engine.events` -- Game event bus
- `engine.config` -- Configuration loading
- `engine.tweens` -- Tween/interpolation system
- `engine.triggers` -- Trigger runtime
- `engine.sprites` -- Sprite batch rendering
- `engine.vec4` -- 4D vector math utilities
- `engine.fs` -- File system access (sandboxed to game directory)
- `engine.log` -- Logging (info, warn, error)
- `engine.time` -- Frame time, total time, fixed timestep

### 2. Prefer Tables Over Userdata for Game Data

Engine-owned objects (physics bodies, audio handles) are opaque userdata. Game-defined data (Health, WeaponInventory, EnemyAI) are plain Lua tables stored as ECS components. This keeps game data inspectable and serializable.

### 3. Error Messages Include Context

Every engine API call that can fail includes the Lua file and line in its error message. "attempt to call nil" should never appear -- instead: "scripts/systems/combat.lua:42: engine.physics:raycast() -- 'direction' must be a Vec4, got nil".

### 4. Consistent Return Patterns

- Functions that search for something return `nil` on not-found (not error)
- Functions that can fail return `nil, error_message` (Lua convention)
- Functions that always succeed return the result directly

### 5. Vec4 as First-Class Type

4D vectors are used constantly. `engine.vec4` provides:
- `engine.vec4.new(x, y, z, w)` -- constructor
- `engine.vec4.ZERO` -- constant
- `engine.vec4.distance(a, b)` -- 4D distance
- `engine.vec4.normalize(v)` -- normalize
- Arithmetic operators (`+`, `-`, `*` scalar) via metatables

---

## 11. Summary

The Rust4D-Shooter game is a 4D boomer shooter written entirely in Lua, running on the Rust4D engine. The game directory contains Lua scripts for all gameplay logic, RON scene files for level geometry, and asset files for textures, sounds, and sprites.

The total estimated effort is 12-18 sessions, with the first session (Phase 0) starting as soon as the engine's scripting runtime is functional. Each subsequent phase depends on the corresponding engine phase's Lua bindings being complete.

The W-axis is the game's defining feature. Every system -- weapons, enemies, levels, HUD -- is designed with 4D in mind. The engine provides the mathematical and rendering foundation in compiled Rust; Lua scripts make it fun to play. Hot-reload enables rapid iteration on the gameplay feel that makes or breaks a boomer shooter.

The key advantages over the original Rust game repo approach are: instant hot-reload for gameplay tuning, zero compile times for game changes, near-free modding support, and a forced clean separation between engine and game. The key risks (performance, debugging, type safety) are well-mitigated by keeping all heavy computation in Rust and providing good developer tooling in the engine.
