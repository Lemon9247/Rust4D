# C1: Game Engine Feature Comparison Report
**Agent**: C1 - Game Engine Feature Researcher
**Date**: 2026-01-30
**Focus**: Unity vs Godot vs Rust4D feature comparison for building a 4D boomer shooter

---

## 1. Core Engine Features Comparison

### 1.1 Rendering

| Feature | Unity | Godot | Rust4D | Priority for Boomer Shooter |
|---------|-------|-------|--------|---------------------------|
| Forward rendering | Yes (URP) | Yes (Vulkan/Forward+) | Yes (wgpu forward pass) | P0 |
| Deferred rendering | Yes (HDRP, Deferred+) | Yes (Vulkan Clustered) | No | P2 |
| Directional lighting | Yes (realtime + baked) | Yes (realtime + baked) | Basic (single directional in shader) | P0 |
| Point/spot lights | Yes (dynamic, many) | Yes (OmniLight3D, SpotLight3D) | No | P1 |
| Shadows | Yes (cascaded shadow maps, contact shadows) | Yes (shadow mapping) | No | P1 |
| Particle systems | Yes (Shuriken + VFX Graph) | Yes (GPUParticles3D) | No | P0 (muzzle flash, explosions, blood) |
| Post-processing | Yes (Bloom, AO, Motion Blur, Color Grading) | Yes (Environment, WorldEnvironment) | No | P1 |
| Custom shaders | Yes (ShaderLab/HLSL) | Yes (Godot shading language) | Partial (WGSL compute + render shaders) | P1 |
| Transparency/alpha | Yes (sorted, alpha cutout) | Yes (sorted, alpha scissor) | Basic (alpha blending enabled) | P1 |
| Sprites / billboards | Yes (SpriteRenderer, Billboard) | Yes (Sprite3D, Billboard) | No | P0 (Doom-style enemies) |
| Texture mapping | Yes (UV mapping, PBR textures) | Yes (UV mapping, PBR) | No (vertex coloring only) | P0 |
| Normal mapping | Yes | Yes | No | P2 |
| Fog | Yes (linear, exponential, volumetric in HDRP) | Yes (FogVolume, VolumetricFog) | No | P1 |
| LOD | Yes (LODGroup) | Yes (visibility ranges) | No | P2 |
| Occlusion culling | Yes (baked, GPU-based) | Yes (OccluderInstance3D) | No | P2 |
| GPU compute shaders | Yes (Compute Shader) | Yes (RenderingDevice) | Yes (wgpu compute pipeline for 4D slicing) | Already have |
| 4D->3D cross-section slicing | No | No | Yes (GPU compute shader) | P0 (core differentiator) |
| W-depth color visualization | No | No | Yes (blue-to-red W gradient) | Already have |

### 1.2 Physics

| Feature | Unity | Godot | Rust4D | Priority for Boomer Shooter |
|---------|-------|-------|--------|---------------------------|
| Rigid body dynamics | Yes (PhysX/Havok) | Yes (GodotPhysics3D, Jolt) | Yes (custom 4D) | Already have |
| Collision detection | Yes (sphere, box, capsule, mesh) | Yes (sphere, box, capsule, convex, concave) | Partial (4D sphere, AABB, plane) | P0 |
| Raycasting | Yes (Physics.Raycast, batch async) | Yes (PhysicsRayQueryParameters3D) | No | P0 (hitscan weapons, line of sight) |
| Triggers / overlap detection | Yes (OnTriggerEnter/Exit/Stay) | Yes (Area3D) | Partial (collision layers with TRIGGER, but no event callbacks) | P0 (pickups, damage zones) |
| Character controller | Yes (CharacterController, Rigidbody) | Yes (CharacterBody3D) | Partial (kinematic player body with gravity/jump) | P0 (already basic) |
| Collision layers/masks | Yes (32 layers) | Yes (32 layers) | Yes (bitflag layers: PLAYER, ENEMY, STATIC, TRIGGER, PROJECTILE, PICKUP) | Already have |
| Physics materials | Yes (friction, bounciness) | Yes (PhysicsMaterial) | Yes (friction, restitution, presets: ICE, RUBBER, METAL, WOOD, CONCRETE) | Already have |
| Explosion/radial forces | Yes (AddExplosionForce) | Yes (manual impulse) | No | P0 (rocket launchers, grenades) |
| Projectile physics | Yes (Rigidbody + collisions) | Yes (RigidBody3D) | Partial (dynamic bodies exist, no projectile-specific logic) | P0 |
| Continuous collision detection | Yes (CCD modes) | Yes (CCD support) | No (step-based discrete) | P1 (fast projectiles) |
| Joint/constraint system | Yes (Hinge, Fixed, Spring, etc.) | Yes (Joint3D variants) | No | P3 |

### 1.3 Audio

| Feature | Unity | Godot | Rust4D | Priority for Boomer Shooter |
|---------|-------|-------|--------|---------------------------|
| Audio playback | Yes (AudioSource) | Yes (AudioStreamPlayer) | No | P0 |
| 3D spatial audio | Yes (AudioSource + listener) | Yes (AudioStreamPlayer3D) | No | P0 (directional gunfire, enemy sounds) |
| Audio mixer / buses | Yes (AudioMixer) | Yes (AudioBus, AudioServer) | No | P1 |
| Sound effect triggering | Yes (PlayOneShot) | Yes (play()) | No | P0 |
| Music / ambient loops | Yes (AudioSource looping) | Yes (AudioStreamPlayer looping) | No | P1 |
| Audio reverb zones | Yes (AudioReverbZone) | Yes (AudioEffectReverb) | No | P2 |
| Audio occlusion | Yes (custom/plugin) | Yes (custom/plugin) | No | P2 |
| Pitch/volume variation | Yes (Random.Range on pitch) | Yes (RandomPitch, RandomVolumeOffsetDb) | No | P1 |

### 1.4 Input

| Feature | Unity | Godot | Rust4D | Priority for Boomer Shooter |
|---------|-------|-------|--------|---------------------------|
| Keyboard/mouse input | Yes (Input system, legacy Input) | Yes (InputEvent system) | Yes (winit keyboard + mouse) | Already have |
| Gamepad support | Yes (Input System) | Yes (InputEventJoypadButton/Motion) | No | P1 |
| Input action mapping | Yes (Input Actions) | Yes (InputMap) | Partial (InputMapper for special keys, hardcoded WASD) | P1 (rebindable keys) |
| Input rebinding | Yes (runtime rebinding) | Yes (runtime rebinding) | No | P1 |
| Mouse capture/release | Yes (Cursor.lockState) | Yes (Input.mouse_mode) | Yes (cursor capture/release toggle) | Already have |
| Scroll wheel | Yes | Yes | Yes (slice offset adjustment) | Already have |
| Dead zones / sensitivity | Yes (Input System) | Yes (InputEvent deadzone) | Partial (sensitivity config, no dead zones) | P1 |
| Input smoothing | Yes (custom) | Yes (custom) | Yes (exponential smoothing, configurable half-life) | Already have |

### 1.5 UI / HUD

| Feature | Unity | Godot | Rust4D | Priority for Boomer Shooter |
|---------|-------|-------|--------|---------------------------|
| UI framework | Yes (uGUI, UI Toolkit) | Yes (Control nodes) | No | P0 |
| Text rendering | Yes (TextMeshPro) | Yes (Label, RichTextLabel) | No | P0 (ammo counter, health) |
| Health bar | Yes (Slider/Image) | Yes (ProgressBar/TextureProgressBar) | No | P0 |
| Ammo counter | Yes (Text) | Yes (Label) | No | P0 |
| Crosshair | Yes (Image overlay) | Yes (TextureRect) | No | P0 |
| Minimap | Yes (RenderTexture + camera) | Yes (SubViewport + camera) | No | P2 |
| Menu system | Yes (Canvas + Button) | Yes (Control + Button) | No | P1 (pause menu, options) |
| Debug console | Yes (custom/asset) | Yes (custom) | No (logging only) | P2 |
| Title screen | Yes | Yes | No | P1 |
| Damage indicators | Yes (custom UI) | Yes (custom UI) | No | P1 |
| Kill feed | Yes (custom) | Yes (custom) | No | P2 |

### 1.6 Animation

| Feature | Unity | Godot | Rust4D | Priority for Boomer Shooter |
|---------|-------|-------|--------|---------------------------|
| Skeletal animation | Yes (Animator, Mecanim) | Yes (AnimationPlayer, Skeleton3D) | No | P2 (not needed for sprite enemies) |
| Sprite animation | Yes (Animator + SpriteRenderer) | Yes (AnimatedSprite3D) | No | P0 (Doom-style enemy sprites) |
| Procedural animation | Yes (IK, scripts) | Yes (IK, SkeletonModification3D) | No | P2 |
| Property animation | Yes (Animation Clip) | Yes (AnimationPlayer: any property) | No | P1 (door opening, platform moving) |
| Animation state machine | Yes (Animator Controller) | Yes (AnimationTree) | No | P1 |
| Weapon bobbing | Yes (script) | Yes (script) | No | P0 (view model bob) |

### 1.7 AI

| Feature | Unity | Godot | Rust4D | Priority for Boomer Shooter |
|---------|-------|-------|--------|---------------------------|
| Navigation mesh | Yes (NavMesh, NavMeshAgent) | Yes (NavigationServer3D) | No | P0 (enemy pathfinding) |
| Pathfinding (A*) | Yes (NavMesh.CalculatePath) | Yes (NavigationServer3D) | No | P0 |
| State machines | Yes (Animator or custom FSM) | Yes (AnimationTree / custom) | No | P0 (enemy behavior) |
| Behavior trees | Yes (asset store / custom) | Yes (plugin / custom) | No | P2 |
| Line of sight | Yes (Physics.Raycast) | Yes (RayCast3D) | No | P0 (enemy sight detection) |
| Steering behaviors | Yes (custom / ML-Agents) | Yes (custom) | No | P1 |
| Spawn system | Yes (Instantiate) | Yes (instantiate()) | Partial (scene templates exist) | P1 |

### 1.8 Level Design / World

| Feature | Unity | Godot | Rust4D | Priority for Boomer Shooter |
|---------|-------|-------|--------|---------------------------|
| Scene / level system | Yes (SceneManager) | Yes (SceneTree) | Yes (SceneManager with stack, overlays, transitions) | Already have |
| Scene serialization | Yes (Unity YAML) | Yes (tscn/tres) | Yes (RON format) | Already have |
| Entity/component system | Yes (GameObject + Components) | Yes (Node + Scripts) | Partial (Entity with tags, transform, material) | P1 (need more components) |
| BSP / portal rendering | No (legacy) | No | No | P2 |
| Level streaming | Yes (Additive scenes) | Yes (background loading) | Partial (async scene loading exists) | P2 |
| Trigger zones | Yes (Collider isTrigger) | Yes (Area3D) | Partial (collision layer for TRIGGER, no callbacks) | P0 |
| Doors / elevators | Yes (Animator + triggers) | Yes (AnimationPlayer + Area3D) | No | P1 |
| Destructible geometry | Yes (custom) | Yes (custom) | No | P3 |

### 1.9 Scripting / Logic

| Feature | Unity | Godot | Rust4D | Priority for Boomer Shooter |
|---------|-------|-------|--------|---------------------------|
| Scripting language | C# | GDScript, C#, C++ | Rust (native) | N/A (different paradigm) |
| Event system | Yes (UnityEvent, delegates) | Yes (signals) | No | P0 (game events: damage, pickup, death) |
| Hot reload | Yes (domain reload) | Yes (script hot reload) | No (recompile required) | P2 |
| Coroutines / async | Yes (Coroutines, async/await) | Yes (await, signals) | No (Rust async possible but not integrated) | P1 |
| Timer system | Yes (Invoke, Coroutines) | Yes (Timer node) | No | P1 |
| Console commands | Yes (custom) | Yes (custom) | No | P2 |

### 1.10 Asset Pipeline

| Feature | Unity | Godot | Rust4D | Priority for Boomer Shooter |
|---------|-------|-------|--------|---------------------------|
| Model import (glTF/OBJ/FBX) | Yes (all formats) | Yes (glTF, OBJ, FBX via plugin) | No | P1 |
| Texture loading | Yes (PNG, JPG, EXR, etc.) | Yes (all common formats) | No (vertex colors only) | P0 |
| Asset cache | Yes (AssetDatabase) | Yes (ResourceLoader cache) | Yes (AssetCache with generational IDs) | Already have |
| Asset bundling | Yes (Addressables, AssetBundles) | Yes (PCK files) | No | P2 |
| Scene validation | Yes (custom) | Yes (custom) | Yes (SceneValidator) | Already have |
| Audio import | Yes (WAV, MP3, OGG) | Yes (WAV, OGG, MP3) | No | P0 |

### 1.11 Debug Tools

| Feature | Unity | Godot | Rust4D | Priority for Boomer Shooter |
|---------|-------|-------|--------|---------------------------|
| Debug logging | Yes (Debug.Log) | Yes (print, push_error) | Yes (env_logger, log crate) | Already have |
| In-game console | Yes (custom) | Yes (custom) | No | P2 |
| Frame profiler | Yes (Profiler window) | Yes (built-in profiler) | No | P2 |
| Physics visualization | Yes (Gizmos) | Yes (Debug draw) | No (config flag exists but unimplemented) | P1 |
| Frame debugger | Yes (Frame Debugger) | Yes (RenderDoc integration) | No | P3 |
| Window title debug | N/A | N/A | Yes (camera position + slice W) | Already have |
| Collision shape display | Yes (Gizmos) | Yes (Debug collision shapes) | No | P1 |

---

## 2. Must-Have Features for a 4D Boomer Shooter

### P0 - Cannot Ship Without

These features are absolutely required to have a playable 4D boomer shooter:

1. **Raycasting in 4D** - Hitscan weapons (shotgun, pistol) need to cast rays and detect hits. Without this, there is no shooting.
2. **Health / damage system** - Player and enemy HP, damage application, death handling.
3. **Weapon system** - At minimum: one hitscan weapon, one projectile weapon. Weapon switching, ammo tracking.
4. **Enemy AI (basic)** - Enemies that can detect the player (line of sight), move toward player (pathfinding), and attack. Does not need to be sophisticated for a boomer shooter -- Doom's enemy AI is quite simple.
5. **Particle/effect system** - Muzzle flash, bullet impacts, explosions, blood/damage feedback. The game will feel lifeless without visual feedback.
6. **Basic HUD** - Health display, ammo counter, crosshair. Text rendering at minimum.
7. **Audio system** - Gunfire sounds, enemy sounds, item pickup sounds. Even a basic audio system transforms the experience.
8. **Texture mapping** - Currently only vertex colors. Need at minimum basic texture support for walls, floors, items.
9. **Sprite billboards** - Doom-style enemies rendered as 2D sprites facing the camera. Classic boomer shooter aesthetic.
10. **Pickup / item system** - Health packs, ammo, weapons on the ground. Requires trigger collision + event system.
11. **Trigger zones with callbacks** - The collision layer system exists, but there's no way to get notified when collisions happen (for pickups, damage zones, level triggers).
12. **Explosion / area damage** - Radial force and damage application for rocket launchers and grenades.

### P1 - Expected Features

Players of boomer shooters expect these features:

1. **Multiple weapon types** - Shotgun, rocket launcher, plasma gun, melee. At least 4-5 weapons.
2. **Point lights** - Dynamic lights for muzzle flash, explosions, lava, torches.
3. **Shadow mapping** - At least basic shadows for directional light.
4. **Post-processing** - Bloom (for weapon effects), basic color grading.
5. **Fog** - Distance fog for atmosphere and to hide draw distance.
6. **Sound spatialization** - 3D positioned audio so you can hear enemies approaching.
7. **Pause menu / options** - Basic menu system for settings.
8. **Input rebinding** - Let players remap keys.
9. **Gamepad support** - Controller input.
10. **Door/elevator mechanics** - Interactive level elements.
11. **Timer / coroutine system** - For delayed events, weapon cooldowns, spawning waves.
12. **Event / signal system** - Decouple game systems (damage dealt -> update HUD, enemy killed -> update score).
13. **Animation system** - At least property animation for doors, platforms, sprite frame cycling.
14. **Damage indicators** - Visual feedback showing where damage came from.
15. **Enemy variety** - Different enemy types with different behaviors (melee rusher, ranged attacker, heavy tank).
16. **Physics visualization** - Debug rendering of collision shapes.

### P2 - Nice to Have

1. **Deferred rendering** - Better handling of many lights.
2. **Normal mapping** - Better surface detail.
3. **LOD system** - Performance at distance.
4. **Occlusion culling** - Don't render what's behind walls.
5. **Audio reverb zones** - Different reverb in different rooms.
6. **Behavior trees** - More sophisticated AI.
7. **Minimap** - 4D minimap would be a fascinating UI challenge.
8. **Debug console** - In-game command entry.
9. **Frame profiler** - Performance analysis.
10. **Skeletal animation** - For more complex enemy models (if moving beyond sprites).
11. **Hot reload** - Faster iteration.
12. **Model import** - glTF/OBJ for level geometry and props.
13. **BSP / portal rendering** - Classic boomer shooter level architecture.
14. **Kill feed / score display** - Player feedback.
15. **Destructible geometry** - 4D destructibles would be amazing.

### P3 - Stretch / Differentiator

1. **4D-native level editor** - Edit levels in 4D space.
2. **4D minimap** - Show a slice of the level in a different W-plane.
3. **Multiplayer / netcode** - Deathmatch in 4D.
4. **Procedural 4D level generation** - Algorithmically generated 4D mazes.
5. **Joint/constraint system** - Physics-based doors, chains.
6. **Visual scripting** - For modding support.
7. **Mod support** - Level loading, custom assets.

---

## 3. Features That Need 4D Adaptation

Standard engine features cannot simply be ported from 3D engines. Many need fundamental redesign for 4D space.

### 3.1 Raycasting in 4D

**Standard 3D**: Cast a ray from origin in a direction, check intersection with triangles/colliders.

**4D Challenge**: Rays are still 1-dimensional (origin + direction in 4D), but they travel through 4D space. Intersection tests must work against 4D collider primitives (4D spheres, 4D AABBs, 4D hyperplanes).

**What Rust4D needs**:
- `ray_vs_sphere_4d(ray, sphere) -> Option<HitInfo4D>` - Ray-sphere intersection in 4D
- `ray_vs_aabb_4d(ray, aabb) -> Option<HitInfo4D>` - Ray-AABB intersection in 4D (slab method generalizes to 4D)
- `ray_vs_plane_4d(ray, plane) -> Option<HitInfo4D>` - Ray-hyperplane intersection
- `physics_world.raycast(origin, direction, max_distance, layer_mask) -> Vec<HitInfo4D>` - World-level raycast with filtering

**Difficulty**: Medium. The math generalizes cleanly from 3D. The slab method for ray-AABB works in any dimension. Ray-sphere is dimension-agnostic.

### 3.2 Pathfinding in 4D

**Standard 3D**: NavMesh on a 2D surface (walkable floor), A* pathfinding on the mesh.

**4D Challenge**: The "walkable surface" in 4D is 3-dimensional (a hyperplane slice through 4D space). A traditional NavMesh would be a 3D mesh (tetrahedra) on a 3-dimensional surface. This is dramatically more complex and memory-intensive.

**Practical approaches for Rust4D**:
1. **Grid-based A* in 4D** - Discretize the XZW hyperplane (the walkable space minus Y) into a 3D grid. Run A* on the grid. Simple, works for boomer shooter levels which tend to have simple geometry.
2. **Waypoint graph** - Place waypoint nodes in the level, connect visible ones. Enemies navigate between waypoints. This is what many classic shooters used and it works well.
3. **Slice-based navigation** - Since the player sees a 3D slice, enemies could pathfind on a 2D NavMesh within the current W-slice, but be aware of W-dimension movement opportunities.
4. **Hierarchical approach** - Use a coarse 4D grid for long-range pathfinding, fine 3D grid for local navigation.

**Difficulty**: Hard. This is a genuinely novel problem. Recommend starting with a waypoint graph system as it is the simplest and most proven for boomer shooters.

### 3.3 Audio Spatialization in 4D

**Standard 3D**: Audio is positioned in 3D space. Pan left/right based on angle to listener. Attenuate by distance. Apply HRTF for height.

**4D Challenge**: Sound sources exist in 4D space. A sound at W=5 when the player is at W=0 is "far away" in the 4th dimension -- should it be quiet? Inaudible? What direction does it come from?

**Practical approaches for Rust4D**:
1. **W-distance attenuation** - Treat W-distance as additional distance for volume falloff. A sound 3 units away in XYZ but 10 units away in W should be quieter than one 3 units away in XYZ and 0 units in W. Use 4D Euclidean distance for attenuation.
2. **W-filtering** - Sounds far in W could be low-pass filtered, like sounds through a wall. This creates the sensation of "hearing through dimensions."
3. **W as pitch/reverb** - Subtle pitch shift or reverb change based on W-separation to give an eerie "dimensional distance" feel.
4. **Practical simplification** - For a boomer shooter, only play sounds that are within a certain W-range of the player. Enemies in a different W-slice are silent. This is pragmatic and gameplay-useful (you only hear threats you can actually see/interact with).

**Difficulty**: Medium. The 3D spatialization part can use standard libraries (rodio, kira). The 4D aspect is mainly an additional distance/filter calculation.

### 3.4 Level Design in 4D

**Standard 3D**: Levels are composed of rooms, corridors, open areas. BSP trees, portals, or just meshes.

**4D Challenge**: 4D levels have rooms that can be adjacent in 4 directions. A room might be "next to" another room in the W dimension but not in XYZ. Players can look "into" the 4th dimension and see different rooms appearing/disappearing as they rotate.

**Practical approaches for Rust4D**:
1. **Layered design** - Design levels as stacked 3D layers at different W values. Like a building with floors, but the "floors" are in the W dimension.
2. **4D maze/labyrinth** - Corridors that connect in W as well as XYZ. The player must navigate through W to find the path.
3. **Scene format** - The existing RON scene format supports entity positioning in 4D (Vec4). Need to add more shape types beyond Tesseract and Hyperplane.
4. **Level editor** - The biggest challenge. Without a visual editor, designing 4D levels is extremely difficult. An in-game editor that shows the 3D slice in real-time while placing objects would be essential.

**Difficulty**: Very Hard (for tooling), Medium (for data structures).

### 3.5 AI Navigation in 4D

**Standard 3D**: Enemies have line of sight checks, pathfind on NavMesh, strafe, take cover.

**4D Challenge**: Line of sight in 4D means checking if there are 4D obstacles between enemy and player. An enemy could be "behind" a wall in the W dimension even if they're visible in XYZ. Cover works in 4 dimensions.

**Practical approaches for Rust4D**:
1. **4D raycast for LOS** - Use 4D raycasting to check line of sight. This naturally handles 4D occlusion.
2. **W-awareness for enemies** - Enemies should be aware of their W-position relative to the player's W-position. If they're in a different W-slice, they might not see the player.
3. **Simple state machine** - For a boomer shooter, enemies need only: IDLE, CHASE, ATTACK, PAIN, DEAD states. This is gameplay logic, not engine features.
4. **Spawn logic** - Enemies should spawn in the same W-slice as the player, or near enough to become visible when the player rotates in 4D.

**Difficulty**: Medium. The 4D raycast handles the hardest part. Enemy behavior logic is mostly game-level, not engine-level.

---

## 4. Recommended Feature Roadmap

Based on the analysis, here is the recommended implementation order for missing features:

### Phase 1: Combat Foundation (2-4 sessions)
*Without these, you cannot shoot anything.*

1. **4D Raycasting** (1 session) - Ray-sphere, ray-AABB, ray-plane intersection in 4D. World-level raycast with layer filtering.
2. **Event / Signal System** (1 session) - A simple event bus or callback mechanism. When a collision happens with a TRIGGER layer, fire an event. When damage is dealt, notify the HUD.
3. **Health / Damage System** (1 session) - HP component on entities. Damage application. Death state.
4. **Trigger Zone Callbacks** (0.5 session) - Extend the existing collision system to report trigger overlaps as events.

### Phase 2: Weapons & Feedback (3-5 sessions)
*Without these, combat is invisible and silent.*

5. **Basic Audio System** (1-2 sessions) - Integrate rodio or kira. Sound effect playback. Basic 3D spatialization with 4D distance attenuation.
6. **Weapon System** (2 sessions) - Weapon state machine (idle, firing, reloading). Hitscan weapon using 4D raycast. Projectile weapon using physics bodies. Ammo tracking.
7. **Basic HUD** (1 session) - Integrate a text/UI rendering solution (e.g., wgpu_text or egui). Health bar, ammo counter, crosshair overlay.
8. **Particle/Effect System** (1-2 sessions) - GPU particle system for muzzle flash, bullet impacts, explosions. Could be 3D particles in the sliced space.

### Phase 3: Enemies & AI (3-4 sessions)
*Without these, there's nothing to shoot at.*

9. **Sprite Billboard Rendering** (1 session) - Render 2D sprites that always face the camera. Essential for Doom-style enemies.
10. **Texture Support** (1 session) - Load and apply textures to surfaces. UV mapping for at least quads/planes.
11. **Basic AI State Machine** (1 session) - IDLE/CHASE/ATTACK/PAIN/DEAD state machine. Uses 4D raycast for LOS.
12. **Waypoint Pathfinding** (1 session) - Place waypoints in 4D space. A* on waypoint graph. Enemies navigate between waypoints.
13. **Explosion / Area Damage** (0.5 session) - Radial damage query (all entities within 4D sphere). Apply damage falloff by distance.

### Phase 4: Level Design & Polish (3-5 sessions)
*Makes the game feel complete.*

14. **Point Lights** (1 session) - Dynamic point lights with falloff. Needed for muzzle flash lighting, lava glow, etc.
15. **Shadow Mapping** (1-2 sessions) - At least directional shadow maps for the main light.
16. **Fog** (0.5 session) - Distance fog in the fragment shader.
17. **Door / Elevator Mechanics** (1 session) - Animated world objects triggered by player proximity or interaction.
18. **Pickup System** (0.5 session) - Health, ammo, weapon pickups using trigger zones + events.
19. **Input Rebinding** (0.5 session) - Configurable key bindings via config file or UI.
20. **Pause Menu** (0.5 session) - Basic menu overlay using the scene stack system.

### Phase 5: Advanced Features (4-8 sessions)
*Differentiators and quality-of-life.*

21. **Post-processing Pipeline** (1-2 sessions) - Bloom, color grading. Render-to-texture then post-process.
22. **4D Minimap** (1-2 sessions) - Show a different W-slice or a top-down view.
23. **Gamepad Support** (0.5 session) - gilrs crate for controller input.
24. **Debug Console** (1 session) - In-game command console for development.
25. **Physics Visualization** (1 session) - Render collision shapes as wireframes.
26. **Audio Mixer** (1 session) - Volume buses (SFX, music, ambient).
27. **More Shape Types** (1-2 sessions) - Hypersphere, 4D cylinder, 4D cone for more varied geometry.

---

## 5. What Rust4D Does Well

It is important to recognize where Rust4D already has strong foundations:

### 5.1 4D Math Foundation
- **Vec4** with full operations (dot, cross-like, normalize, component-wise ops)
- **Rotor4** using geometric algebra for 4D rotations -- this is the correct and gimbal-lock-free approach
- **RotationPlane** enum covering all 6 planes of 4D rotation (XY, XZ, XW, YZ, YW, ZW)
- **Mat4** with `skip_y` for Engine4D-style camera architecture -- this is a sophisticated solution to the "Y-axis preservation" problem
- This math foundation is solid and would be the hardest part to bolt on after the fact

### 5.2 GPU-Based 4D Slicing
- **Compute shader pipeline** that slices 4D tetrahedra into 3D triangles in real-time
- **Indirect drawing** for efficient variable-count triangle rendering
- **W-depth visualization** with configurable color gradient
- This is the core technical innovation and works well. Neither Unity nor Godot have this.

### 5.3 4D Physics
- **Full 4D collision detection** with sphere, AABB, and plane primitives
- **Collision layer system** already designed for a shooter (PLAYER, ENEMY, PROJECTILE, PICKUP layers)
- **Player character physics** with gravity, jumping, grounded detection
- **Physics materials** with friction and restitution presets
- **Edge-falling detection** for bounded floors -- this is a 4D-specific challenge that has been solved
- The physics is ahead of where many custom engines would be at this stage

### 5.4 Camera Architecture
- **Engine4D-style camera** with separated pitch and 4D rotation
- **SkipY** matrix transform ensuring Y-axis preservation during 4D rotation
- **Smooth input** with configurable exponential smoothing
- This is a well-thought-out solution to a hard problem (intuitive 4D camera control)

### 5.5 Scene & Asset Infrastructure
- **Scene serialization** in RON format with entity templates
- **Scene manager** with stack for overlays, transitions, and async loading
- **Asset cache** with generational IDs preventing stale references
- **Configuration system** with TOML files and environment variable overrides
- **Scene validator** for catching errors early
- This infrastructure is mature and ready for a full game

### 5.6 Clean Architecture
- **Modular crate structure** (math, physics, render, input, core) enables parallel development
- **Generational keys** (slotmap) for safe entity/body references
- **Builder pattern** throughout for ergonomic API design
- **Comprehensive test coverage** with hundreds of tests across all crates
- The code quality is high and the architecture will scale

---

## Summary

Rust4D has a remarkably strong foundation for its stage of development. The 4D-specific features (math, slicing, physics, camera) are sophisticated and well-implemented. The biggest gaps are in **standard game engine features** that are well-understood but require implementation effort:

| Gap Category | Severity | Effort |
|-------------|----------|--------|
| No raycasting | Critical | Low (math generalizes to 4D) |
| No audio system | Critical | Medium (integrate existing Rust crate) |
| No UI/HUD rendering | Critical | Medium (integrate egui or similar) |
| No particle effects | Critical | Medium-High (GPU particles) |
| No texture support | Critical | Medium (texture loading + UV mapping) |
| No event/signal system | Critical | Low |
| No enemy AI | Critical | Medium (state machine + pathfinding) |
| No sprite rendering | High | Low-Medium |
| No dynamic lights | High | Medium |
| No shadows | High | Medium-High |
| No gamepad | Moderate | Low (gilrs crate) |
| No post-processing | Moderate | Medium |

The recommended approach is to focus on **Phase 1 (Combat Foundation)** first since shooting is the core of a boomer shooter. With raycasting, events, and health systems, the engine can support the most basic gameplay loop. Then **Phase 2 (Weapons & Feedback)** makes it feel like a game. **Phase 3 (Enemies & AI)** gives you something to fight. Everything after that is polish.
