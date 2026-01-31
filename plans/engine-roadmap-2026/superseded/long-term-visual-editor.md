# Visual Scene Editor - Long-Term Plan

**Status:** FUTURE/DRAFT - Not for immediate implementation
**Estimated Effort:** 10-15 sessions
**Priority:** P7 (After scene system maturity)
**Created:** 2026-01-27

---

## Table of Contents

1. [Overview and Rationale](#overview-and-rationale)
2. [Prerequisites and Trigger Conditions](#prerequisites-and-trigger-conditions)
3. [GUI Framework Analysis](#gui-framework-analysis)
4. [Recommended Approach](#recommended-approach)
5. [Editor Features Roadmap](#editor-features-roadmap)
6. [Technical Architecture](#technical-architecture)
7. [Integration with Engine Runtime](#integration-with-engine-runtime)
8. [File Format Considerations](#file-format-considerations)
9. [Phased Implementation Plan](#phased-implementation-plan)
10. [Risk Assessment](#risk-assessment)
11. [Success Criteria](#success-criteria)
12. [References](#references)

---

## Overview and Rationale

### Current State

Rust4D scenes are currently created and edited through two methods:

1. **Programmatic construction** using `SceneBuilder`:
   ```rust
   let world = SceneBuilder::new()
       .with_physics(-20.0)
       .add_floor(-2.0, 10.0, PhysicsMaterial::CONCRETE)
       .add_player(Vec4::new(0.0, 0.0, 5.0, 0.0), 0.5)
       .add_tesseract(Vec4::ZERO, 2.0, "tesseract")
       .build();
   ```

2. **Manual RON file editing** (planned in Phase 2):
   ```ron
   Scene(
       entities: [
           (
               name: Some("floor"),
               transform: (position: (0.0, -2.0, 0.0, 0.0)),
               // ...
           ),
       ],
   )
   ```

### The Need for a Visual Editor

While code and text-based editing work well for programmers, a visual scene editor would:

1. **Enable non-programmers** (artists, designers, level designers) to create game content
2. **Accelerate iteration** - visual feedback is faster than compile-run-test cycles
3. **Reduce errors** - visual placement is more intuitive than typing coordinates
4. **Support complex scenes** - easier to manage 100+ entities visually than in text
5. **Provide 4D visualization** - critical for a 4D engine where spatial reasoning is difficult

### Why This Is a Large Effort

A production-quality scene editor is a substantial undertaking because it requires:

- **Dual rendering contexts** - Editor UI + 3D/4D viewport running simultaneously
- **4D visualization** - Novel UX for manipulating 4D objects (no industry standard exists)
- **Tool architecture** - Gizmos, selection, undo/redo, clipboard, drag-and-drop
- **Asset browser** - Visual management of prefabs, materials, shapes
- **Real-time preview** - See physics and game logic in the editor
- **Save/load robustness** - Must never corrupt user scenes
- **Polish and UX** - Professional tools require significant UX refinement

**Estimated effort:** 10-15 sessions of focused work, plus ongoing refinement.

### Strategic Positioning

This is a **long-term goal**, not an immediate priority. It should only be pursued after:

1. Scene serialization system is mature and stable (Phase 2)
2. RON scene format is proven and versioned
3. Prefab system is working and tested
4. Core engine features are complete (rendering, physics, input)
5. Example scenes exist that demonstrate the engine's capabilities

**Earliest realistic start date:** After Phases 1-5 complete (~20-22 sessions from now)

---

## Prerequisites and Trigger Conditions

### Do NOT start this work until:

1. **Scene system is mature** (Phase 2 complete)
   - RON serialization working flawlessly
   - Scene save/load tested extensively
   - Prefab system implemented and stable
   - No known bugs in scene format

2. **Engine runtime is stable** (Phase 5 complete)
   - Rendering pipeline complete
   - Physics system tested and optimized
   - Input handling robust
   - No major architectural changes planned

3. **4D rendering is production-ready**
   - Can visualize 4D objects in 3D slices
   - W-axis navigation is intuitive
   - Cross-section rendering works correctly

4. **Documentation exists** (Phase 3 complete)
   - User guide explains scene concepts
   - API docs cover all scene types
   - Examples demonstrate scene patterns

5. **There's actual demand**
   - Multiple scenes exist as test cases
   - Users request visual editing
   - RON editing is a proven bottleneck

### Green Light Criteria

Start visual editor work when:

- Manual RON editing becomes painful (5+ entities per scene)
- Non-programmers want to create content
- Scene iteration time is too slow
- Willow explicitly requests it
- All prerequisite phases are complete and stable

### Red Flags (Don't Start If True)

- Scene system still has bugs or design churn
- Engine architecture is unstable
- No clear use case for the editor yet
- Resource constraints (time, energy) are tight

---

## GUI Framework Analysis

Three viable Rust GUI frameworks for a scene editor:

### Option 1: egui

**Repository:** [emilk/egui](https://github.com/emilk/egui)
**Maturity:** Production-ready, actively maintained
**License:** MIT/Apache-2.0

**Pros:**
- **Immediate mode** - Simple mental model, easy to integrate
- **Excellent Rust integration** - Idiomatic Rust, no FFI
- **Fast iteration** - UI code is just Rust code, hot-reloads trivially
- **Built-in widgets** - Buttons, sliders, text boxes, trees, drag-and-drop
- **wgpu support** - Can share rendering context with Rust4D
- **Cross-platform** - Works on all major platforms
- **Active ecosystem** - Many third-party widget libraries
- **Used in production** - Rerun, vvvv, Ambient, Veloren all use egui

**Cons:**
- Immediate mode can be less efficient for complex UIs
- Less "native" look than retained-mode frameworks
- Limited accessibility features
- No built-in scene graph widget (must implement custom)

**Verdict:** **RECOMMENDED** - Best fit for Rust4D.

### Option 2: Iced

**Repository:** [iced-rs/iced](https://github.com/iced-rs/iced)
**Maturity:** Stable, actively maintained
**License:** MIT

**Pros:**
- **Retained mode** - Efficient for static UIs
- **Elm Architecture** - Clean, declarative state management
- **Native look** - Better platform integration than egui
- **Type-safe** - Strong compile-time guarantees
- **Good performance** - Optimized for minimal redraws

**Cons:**
- **Steep learning curve** - Elm architecture is unfamiliar to many
- **Less flexible** - Retained mode is harder to integrate with dynamic game state
- **Smaller ecosystem** - Fewer third-party widgets than egui
- **wgpu integration** - Possible but less mature than egui
- **Verbose** - More boilerplate than immediate mode

**Verdict:** Viable but not ideal for a game editor's dynamic nature.

### Option 3: Druid

**Repository:** [linebender/druid](https://github.com/linebender/druid)
**Maturity:** **Archived** - Development has moved to Xilem
**License:** Apache-2.0

**Pros:**
- Mature retained-mode architecture
- Good performance and native integration

**Cons:**
- **Project archived** - No longer actively maintained
- Development moved to experimental Xilem project
- Not recommended for new projects

**Verdict:** Avoid - use egui or Iced instead.

### Option 4: Tauri + Web Frontend

**Repository:** [tauri-apps/tauri](https://github.com/tauri-apps/tauri)

**Pros:**
- Use HTML/CSS/JavaScript for UI (familiar to web developers)
- Rich ecosystem of UI libraries
- Professional desktop app wrapper

**Cons:**
- Requires JavaScript/web stack knowledge
- Less integrated with Rust game engine
- Heavier runtime (webview overhead)
- Complex build process

**Verdict:** Overkill for a scene editor; better for standalone apps.

---

## Recommended Approach

### Framework: egui

**Rationale:**

1. **Rust-first design** - egui is written in pure Rust, idiomatic, no FFI
2. **wgpu integration** - Rust4D uses wgpu; egui has first-class wgpu support
3. **Immediate mode simplicity** - Easy to synchronize editor UI with game state
4. **Proven in game tools** - Multiple game engines use egui for editors
5. **Fast iteration** - UI code is just Rust; no separate asset pipeline
6. **Strong ecosystem** - egui_extras, egui_plot, egui_dock for advanced layouts

### Integration Pattern: Embedded Editor

The editor will be **embedded in the engine binary**, not a separate application.

```rust
// Main binary can run in two modes:
fn main() {
    let args = parse_args();

    if args.editor {
        // Launch scene editor
        run_editor(args.scene_path);
    } else {
        // Launch game runtime
        run_game(args.scene_path);
    }
}
```

**Benefits:**
- Single codebase for runtime and editor
- Editor sees the exact same engine behavior as runtime
- Easy to switch between editor and play mode
- Shared asset loading and rendering pipeline

**Drawbacks:**
- Binary size increases (acceptable for development builds)
- Editor code ships with game (can be feature-gated for release builds)

### Technical Stack

```
┌─────────────────────────────────────────┐
│         Scene Editor (egui)             │
│  - UI panels, menus, dialogs            │
│  - Property inspector                   │
│  - Asset browser                        │
└─────────────────────────────────────────┘
                   │
                   ▼
┌─────────────────────────────────────────┐
│       Editor Viewport (wgpu)            │
│  - 4D → 3D slice rendering              │
│  - Gizmos (translate, rotate, scale)    │
│  - Selection highlights                 │
└─────────────────────────────────────────┘
                   │
                   ▼
┌─────────────────────────────────────────┐
│      Rust4D Engine Runtime              │
│  - Scene/World/Entity system            │
│  - Physics simulation                   │
│  - Rendering pipeline                   │
└─────────────────────────────────────────┘
```

**Key libraries:**

- `egui` - Core UI framework
- `egui_wgpu` - wgpu backend for egui
- `egui_winit` - Window integration
- `egui_dock` - Dockable panel layout
- `egui_extras` - Additional widgets (file browser, color picker)

---

## Editor Features Roadmap

### Phase 1: Basic Viewport + Entity List (2-3 sessions)

**Goal:** Minimal viable editor - can view and select entities.

**Features:**
- Window with egui integration
- 3D viewport showing current W-slice of the 4D scene
- Entity hierarchy panel (list of entities by name)
- Click entity in list to select it
- Selected entity highlights in viewport
- W-slice slider to navigate 4D space

**Technical tasks:**
1. Set up egui + wgpu rendering context
2. Create EditorApp struct with panels
3. Implement entity list widget (egui::Tree or custom)
4. Add selection state management
5. Render selection highlight in viewport
6. Add W-axis slider for cross-section navigation

**Deliverable:** Can open a scene file and browse entities visually.

**Success criteria:**
- Loads existing RON scene files
- Displays all entities in a scrollable list
- Viewport shows 3D cross-section at current W-coordinate
- Clicking entity selects it (highlight in viewport)
- W-slider changes visible cross-section

### Phase 2: Property Inspector (2 sessions)

**Goal:** View and edit entity properties.

**Features:**
- Property inspector panel (right side of editor)
- Display selected entity's properties:
  - Name (text input)
  - Tags (list with add/remove)
  - Transform (X, Y, Z, W position; rotation)
  - Material (color picker)
  - Physics properties (mass, friction, restitution)
- Changes update in real-time in viewport
- Undo/redo for property changes
- Save button to write changes to RON file

**Technical tasks:**
1. Create PropertyInspector widget
2. Implement property editors for each field type
3. Add undo/redo stack (Command pattern)
4. Update entity in World when properties change
5. Synchronize physics bodies with property changes
6. Implement scene save (write modified scene to RON)

**Deliverable:** Can edit entity properties and save changes.

**Success criteria:**
- All entity properties are editable
- Changes reflect immediately in viewport
- Undo/redo works (Ctrl+Z, Ctrl+Shift+Z)
- Save writes valid RON file
- Reload scene shows saved changes

### Phase 3: Drag-and-Drop Placement (2-3 sessions)

**Goal:** Move entities in 3D/4D space with mouse.

**Features:**
- Click and drag entities in viewport to move them
- 3D gizmo for translate/rotate/scale (like Unity/Blender)
- Snap to grid (configurable grid size)
- Constrain to axis (hold X/Y/Z/W to lock axis)
- Multi-select (Shift+Click, Ctrl+Click)
- Duplicate entities (Ctrl+D)
- Delete entities (Delete key)

**Technical tasks:**
1. Implement raycasting for viewport clicks (pick entity under cursor)
2. Create 3D gizmo rendering (translate arrows, rotation circles)
3. Add mouse drag handling for gizmo manipulation
4. Implement grid snapping
5. Add axis-lock constraints
6. Multi-selection state management
7. Implement entity duplication and deletion

**Deliverable:** Can manipulate entities spatially with mouse.

**Success criteria:**
- Click entity to select, drag to move
- Gizmo appears on selected entity
- Dragging gizmo arrows moves entity along axis
- Grid snapping works (toggleable)
- Can select multiple entities (Shift+Click)
- Ctrl+D duplicates, Delete removes

**4D Challenge:** Moving in 4D is non-trivial. Initial implementation:
- Gizmo operates in 3D (current W-slice)
- Separate W-axis slider in property inspector
- Future: 4D gizmo with W-axis handle (advanced UX problem)

### Phase 4: Prefab Editing (2 sessions)

**Goal:** Create, edit, and instantiate prefabs.

**Features:**
- Asset browser panel (bottom of editor)
- Display available prefabs from `assets/prefabs/` folder
- Drag prefab from browser into viewport to instantiate
- Edit prefab properties in prefab editor mode
- Save changes to prefab file
- Prefab instances update when source prefab changes
- Prefab overrides (instance-specific property changes)

**Technical tasks:**
1. Create AssetBrowser widget (file tree view)
2. Implement prefab drag-and-drop from browser to viewport
3. Add prefab editor mode (edit template, not instance)
4. Implement prefab save/load
5. Track prefab instances and source linkage
6. Handle prefab override system
7. Auto-reload prefabs when files change

**Deliverable:** Full prefab workflow in editor.

**Success criteria:**
- Can browse prefabs in asset panel
- Drag-and-drop prefab into scene to instantiate
- Edit prefab in prefab mode, save changes
- Instances update when source changes
- Can override instance properties
- Prefab files are valid RON

### Phase 5: 4D-Specific Tools (3-4 sessions)

**Goal:** Solve the unique challenges of 4D editing.

**Features:**
- **Multi-slice view** - Show multiple W-slices side-by-side
- **4D gizmo** - Visual control for W-axis translation
- **4D object preview** - Tesseract unwrapping/wireframe view
- **Cross-section visualization** - Highlight where 3D slice cuts 4D object
- **W-axis timeline** - Scrub through W dimension like a timeline
- **4D camera controls** - Rotate 4D view (project 4D → 3D differently)
- **Dimension locking** - Lock editing to 3D subspace (ignore W)

**Technical tasks:**
1. Implement multi-viewport layout (show 4+ W-slices)
2. Design and implement 4D gizmo (research required)
3. Add 4D wireframe rendering mode
4. Implement cross-section highlight shader
5. Create W-axis timeline widget
6. Add 4D camera rotation controls (isoclinic rotations)
7. Dimension lock toggle for 3D-only editing

**Deliverable:** Robust 4D editing experience.

**Success criteria:**
- Can see multiple W-slices simultaneously
- 4D gizmo allows intuitive W-axis movement
- Tesseract structure is clear in preview
- W-timeline makes 4D navigation intuitive
- Users report 4D editing is usable (UX testing)

**Major UX Challenge:** There is no industry standard for 4D editing. This phase requires:
- UX research and experimentation
- User testing with Willow and potential users
- Iteration based on feedback
- Possibly multiple design attempts

**References for 4D visualization:**
- [4D Toys](http://4dtoys.com/) - 4D physics game (inspiration for UX)
- [Miegakure](https://miegakure.com/) - 4D puzzle game (4D navigation)
- Academic papers on 4D visualization techniques

---

## Technical Architecture

### Editor Components

```rust
/// Main editor application
pub struct SceneEditor {
    /// egui context
    egui_ctx: egui::Context,

    /// Editor state
    state: EditorState,

    /// Panels
    viewport: ViewportPanel,
    entity_list: EntityListPanel,
    properties: PropertyInspector,
    assets: AssetBrowser,

    /// Current scene
    scene: Scene,

    /// Undo/redo system
    history: CommandHistory,

    /// Tool state (select, move, rotate, scale)
    active_tool: Tool,
}

/// Editor state (selection, camera, etc.)
pub struct EditorState {
    /// Selected entities
    selection: Selection,

    /// Editor camera (independent of runtime camera)
    camera: Camera4D,

    /// Current W-slice being viewed
    w_slice: f32,

    /// Grid settings
    grid: GridSettings,

    /// Editor preferences
    preferences: EditorPreferences,
}

/// Selection state
pub enum Selection {
    None,
    Single(EntityKey),
    Multiple(Vec<EntityKey>),
}

/// Active tool
pub enum Tool {
    Select,
    Translate,
    Rotate,
    Scale,
    Prefab,  // Drag-drop prefab placement
}

/// Command pattern for undo/redo
pub trait Command {
    fn execute(&mut self, scene: &mut Scene);
    fn undo(&mut self, scene: &mut Scene);
}

/// Example: Move entity command
pub struct MoveEntityCommand {
    entity: EntityKey,
    old_position: Vec4,
    new_position: Vec4,
}
```

### Panel Architecture

Each panel is a self-contained widget:

```rust
/// Viewport panel - 3D/4D rendering
pub struct ViewportPanel {
    /// wgpu rendering context (shared with engine)
    renderer: EditorRenderer,

    /// Gizmo system
    gizmo: Gizmo3D,

    /// Raycaster for picking
    picker: EntityPicker,
}

impl ViewportPanel {
    pub fn ui(&mut self, ui: &mut egui::Ui, state: &mut EditorState, scene: &Scene) {
        // Render viewport using wgpu
        // Handle mouse input for selection and manipulation
        // Draw gizmos for selected entities
    }
}

/// Entity list panel - hierarchy view
pub struct EntityListPanel {
    /// Search/filter state
    filter: String,

    /// Scroll position (preserve between frames)
    scroll: f32,
}

impl EntityListPanel {
    pub fn ui(&mut self, ui: &mut egui::Ui, state: &mut EditorState, scene: &Scene) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (key, entity) in scene.world.entities() {
                if self.matches_filter(entity) {
                    let selected = state.selection.contains(key);
                    if ui.selectable_label(selected, entity.name()).clicked() {
                        state.selection = Selection::Single(key);
                    }
                }
            }
        });
    }
}

/// Property inspector - edit selected entity
pub struct PropertyInspector;

impl PropertyInspector {
    pub fn ui(&mut self, ui: &mut egui::Ui, state: &mut EditorState, scene: &mut Scene) {
        if let Some(entity_key) = state.selection.single() {
            let entity = scene.world.get_mut(entity_key);

            // Name field
            ui.text_edit_singleline(&mut entity.name);

            // Transform
            ui.label("Position");
            let mut pos = entity.transform.position;
            ui.horizontal(|ui| {
                ui.label("X:"); ui.drag_value(&mut pos.x);
                ui.label("Y:"); ui.drag_value(&mut pos.y);
                ui.label("Z:"); ui.drag_value(&mut pos.z);
                ui.label("W:"); ui.drag_value(&mut pos.w);
            });
            if pos != entity.transform.position {
                // Create move command and execute
                let cmd = MoveEntityCommand { /* ... */ };
                state.history.execute(cmd, scene);
            }

            // Material
            ui.label("Color");
            ui.color_edit_button_rgba_unmultiplied(&mut entity.material.color);

            // Physics (if present)
            if let Some(physics_key) = entity.physics_body {
                // Show physics properties
            }
        } else {
            ui.label("No entity selected");
        }
    }
}

/// Asset browser - prefabs, materials, etc.
pub struct AssetBrowser {
    /// Available prefabs
    prefabs: Vec<PrefabAsset>,

    /// Asset search/filter
    filter: String,
}

impl AssetBrowser {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Search:");
            ui.text_edit_singleline(&mut self.filter);
        });

        egui::ScrollArea::vertical().show(ui, |ui| {
            for prefab in &self.prefabs {
                if prefab.name.contains(&self.filter) {
                    // Draggable prefab icon
                    let response = ui.selectable_label(false, &prefab.name);
                    if response.drag_started() {
                        // Start prefab drag operation
                    }
                }
            }
        });
    }
}
```

### Undo/Redo System

```rust
pub struct CommandHistory {
    /// Stack of executed commands
    done: Vec<Box<dyn Command>>,

    /// Stack of undone commands (for redo)
    undone: Vec<Box<dyn Command>>,
}

impl CommandHistory {
    pub fn execute(&mut self, mut cmd: Box<dyn Command>, scene: &mut Scene) {
        cmd.execute(scene);
        self.done.push(cmd);
        self.undone.clear();  // Redo stack invalidated
    }

    pub fn undo(&mut self, scene: &mut Scene) {
        if let Some(mut cmd) = self.done.pop() {
            cmd.undo(scene);
            self.undone.push(cmd);
        }
    }

    pub fn redo(&mut self, scene: &mut Scene) {
        if let Some(mut cmd) = self.undone.pop() {
            cmd.execute(scene);
            self.done.push(cmd);
        }
    }
}
```

### Gizmo System

```rust
/// 3D/4D gizmo for entity manipulation
pub struct Gizmo3D {
    /// Gizmo mode
    mode: GizmoMode,

    /// Active axis (if dragging)
    active_axis: Option<Axis4D>,
}

pub enum GizmoMode {
    Translate,
    Rotate,
    Scale,
}

pub enum Axis4D {
    X, Y, Z, W,
}

impl Gizmo3D {
    /// Render gizmo for selected entity
    pub fn render(&self, entity_position: Vec4, camera: &Camera4D) {
        match self.mode {
            GizmoMode::Translate => {
                // Draw 3 arrows (X=red, Y=green, Z=blue)
                // For Phase 5: Add W-axis handle (color TBD - magenta?)
            }
            GizmoMode::Rotate => {
                // Draw 3 rotation circles (XY, YZ, XZ planes)
            }
            GizmoMode::Scale => {
                // Draw 3 scale handles
            }
        }
    }

    /// Handle mouse interaction with gizmo
    pub fn handle_input(&mut self, mouse: &MouseState, camera: &Camera4D) -> Option<GizmoAction> {
        // Raycast mouse to gizmo geometry
        // If hit, start drag operation
        // Return transform delta
    }
}

pub struct GizmoAction {
    pub axis: Axis4D,
    pub delta: f32,  // How much to move/rotate/scale
}
```

---

## Integration with Engine Runtime

### Dual Rendering

The editor runs two rendering passes per frame:

```rust
impl SceneEditor {
    pub fn render(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, surface: &wgpu::Surface) {
        // 1. Render game viewport (scene entities)
        let viewport_texture = self.viewport.render(
            device,
            queue,
            &self.scene,
            &self.state.camera,
            self.state.w_slice,
        );

        // 2. Render editor overlays (gizmos, selection highlights)
        self.viewport.render_overlays(
            device,
            queue,
            viewport_texture,
            &self.state.selection,
        );

        // 3. Render egui UI (panels, menus)
        self.egui_ctx.run(|ctx| {
            egui::SidePanel::left("entities").show(ctx, |ui| {
                self.entity_list.ui(ui, &mut self.state, &self.scene);
            });

            egui::SidePanel::right("properties").show(ctx, |ui| {
                self.properties.ui(ui, &mut self.state, &mut self.scene);
            });

            egui::TopBottomPanel::bottom("assets").show(ctx, |ui| {
                self.assets.ui(ui);
            });

            egui::CentralPanel::default().show(ctx, |ui| {
                // Embed viewport texture in egui
                ui.image(viewport_texture);
            });
        });
    }
}
```

### Play Mode

The editor can switch to "Play Mode" to test the scene:

```rust
pub enum EditorMode {
    Edit,   // Pause physics, allow editing
    Play,   // Run physics, lock editing
}

impl SceneEditor {
    pub fn toggle_play_mode(&mut self) {
        match self.mode {
            EditorMode::Edit => {
                // Save scene state (for revert on stop)
                self.saved_state = self.scene.clone();

                // Start physics simulation
                self.mode = EditorMode::Play;
            }
            EditorMode::Play => {
                // Stop physics, revert to saved state
                self.scene = self.saved_state.clone();
                self.mode = EditorMode::Edit;
            }
        }
    }

    pub fn update(&mut self, dt: f32) {
        if self.mode == EditorMode::Play {
            // Run physics and game logic
            self.scene.update(dt);
        }
    }
}
```

**Play mode features:**
- Physics simulation runs
- Player controls work (WASD movement)
- Can observe scene behavior before export
- Stop returns to edit mode (non-destructive)

### Asset Hot Reloading

The editor watches asset files and reloads on change:

```rust
pub struct AssetWatcher {
    watcher: notify::RecommendedWatcher,
    events: mpsc::Receiver<notify::Event>,
}

impl SceneEditor {
    pub fn check_asset_updates(&mut self) {
        while let Ok(event) = self.asset_watcher.poll() {
            match event.kind {
                notify::EventKind::Modify(_) => {
                    if let Some(path) = event.paths.first() {
                        if path.extension() == Some("ron") {
                            // Reload prefab
                            self.reload_prefab(path);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn reload_prefab(&mut self, path: &Path) {
        if let Ok(prefab) = Prefab::load(path) {
            // Update prefab in registry
            self.scene.prefabs.insert(prefab.name.clone(), prefab);

            // Update all instances in scene
            for (key, entity) in self.scene.world.entities_mut() {
                if entity.prefab_source == Some(prefab.name.clone()) {
                    // Re-instantiate from updated prefab
                }
            }
        }
    }
}
```

---

## File Format Considerations

### Scene Files (RON)

The editor reads and writes the same RON scene format as the runtime:

```ron
Scene(
    metadata: (
        name: "Test Level",
        version: "1.0.0",
        author: "Willow",
    ),
    physics: Some((gravity: -20.0)),
    entities: [ /* ... */ ],
)
```

**Editor responsibilities:**

1. **Format preservation** - Don't reformat entire file on save (preserve user's formatting)
2. **Validation** - Check scene validity before saving (don't corrupt files)
3. **Backup** - Create `.bak` files before overwriting scenes
4. **Version detection** - Warn if opening old format, offer migration
5. **Error reporting** - Helpful error messages for malformed files

### Prefab Files (RON)

Prefabs are saved as separate `.ron` files in `assets/prefabs/`:

```ron
Prefab(
    name: "EnemyCube",
    template: (
        tags: ["enemy", "dynamic"],
        shape: Tesseract(size: 1.0),
        material: (base_color: (1.0, 0.0, 0.0, 1.0)),
        physics: Some(Dynamic(
            mass: 5.0,
            material: Wood,
        )),
    ),
)
```

**Editor prefab workflow:**

1. Create new prefab from selected entity ("Save as Prefab")
2. Edit prefab in prefab editor mode
3. Save changes to prefab file
4. All instances auto-update (via hot reload)

### Editor Preferences (TOML)

Editor-specific settings stored in `~/.config/rust4d/editor.toml`:

```toml
[editor]
theme = "dark"
grid_size = 1.0
snap_to_grid = true
w_slice_increment = 0.1

[viewport]
fov = 60.0
near_clip = 0.1
far_clip = 1000.0

[shortcuts]
save = "Ctrl+S"
undo = "Ctrl+Z"
redo = "Ctrl+Shift+Z"
duplicate = "Ctrl+D"
delete = "Delete"
play_mode = "Space"

[recent_scenes]
files = [
    "/home/willow/scenes/test_level.ron",
    "/home/willow/scenes/physics_demo.ron",
]
```

**Preferences UI:**

- Settings menu for configuring editor
- Keyboard shortcut customization
- Theme selection (dark/light)
- Grid and snap settings

---

## Phased Implementation Plan

### Phase 1: Foundation (2-3 sessions)

**Goal:** Basic editor window with viewport and entity list.

**Tasks:**

1. **Set up egui + wgpu integration** (0.5 sessions)
   - Add egui dependencies to Cargo.toml
   - Create EditorApp with egui context
   - Integrate egui_wgpu backend
   - Render basic window with panels

2. **Implement viewport rendering** (1 session)
   - Reuse Rust4D renderer for viewport
   - Add editor camera controls (orbit, pan, zoom)
   - Render scene entities in viewport
   - Add W-slice slider widget

3. **Create entity list panel** (0.5 sessions)
   - Display all entities in scrollable list
   - Show entity names and types
   - Implement selection on click
   - Highlight selected entity in viewport

4. **Add selection system** (0.5 sessions)
   - Selection state management
   - Click-to-select in viewport (raycasting)
   - Selection highlight rendering
   - Multi-select (Shift+Click)

**Deliverables:**
- Can load and view scene files
- Entity list shows all entities
- Clicking selects entity (highlights in viewport)
- W-slider navigates 4D cross-sections

**Testing:**
- Load existing test scenes
- Verify all entities visible
- Selection works in list and viewport
- W-slider shows different cross-sections

### Phase 2: Property Editing (2 sessions)

**Goal:** View and edit entity properties.

**Tasks:**

1. **Create property inspector widget** (1 session)
   - Display selected entity's properties
   - Editable fields for name, tags, transform
   - Material color picker
   - Physics property editors

2. **Implement undo/redo** (0.5 sessions)
   - Command pattern infrastructure
   - Undo/redo stack
   - Keyboard shortcuts (Ctrl+Z, Ctrl+Shift+Z)
   - Commands: Move, Rotate, Scale, Edit Property

3. **Add save/load** (0.5 sessions)
   - Save button writes scene to RON
   - Validate scene before save
   - Create backup (.bak) files
   - Load scene from file picker

**Deliverables:**
- Property inspector shows all entity properties
- All properties are editable
- Changes reflect immediately in viewport
- Undo/redo works
- Can save modified scene

**Testing:**
- Edit various entity properties
- Verify changes appear in viewport
- Undo/redo multiple changes
- Save and reload scene

### Phase 3: Spatial Manipulation (2-3 sessions)

**Goal:** Move, rotate, scale entities with mouse.

**Tasks:**

1. **Implement 3D gizmo** (1.5 sessions)
   - Render translate arrows (X/Y/Z)
   - Render rotation circles
   - Render scale handles
   - Gizmo appears on selected entity

2. **Add gizmo interaction** (1 session)
   - Raycast to gizmo handles
   - Drag to manipulate entity
   - Visual feedback during drag
   - Snap to grid (toggleable)

3. **Entity operations** (0.5 sessions)
   - Duplicate (Ctrl+D)
   - Delete (Delete key)
   - Copy/paste (Ctrl+C, Ctrl+V)
   - Move to W-slice

**Deliverables:**
- Gizmo renders on selected entity
- Dragging gizmo moves/rotates/scales entity
- Grid snapping works
- Can duplicate and delete entities

**Testing:**
- Select entity, verify gizmo appears
- Drag gizmo handles, verify entity moves
- Test snapping with various grid sizes
- Duplicate and delete entities

### Phase 4: Prefab System (2 sessions)

**Goal:** Prefab creation, editing, instantiation.

**Tasks:**

1. **Asset browser panel** (0.5 sessions)
   - File tree view of `assets/prefabs/`
   - Thumbnail/icon for each prefab
   - Search/filter functionality

2. **Prefab drag-and-drop** (0.5 sessions)
   - Drag prefab from browser to viewport
   - Instantiate at mouse position
   - Place on current W-slice

3. **Prefab editor mode** (0.5 sessions)
   - Open prefab for editing
   - Edit template properties
   - Save changes to prefab file

4. **Prefab instance management** (0.5 sessions)
   - Track prefab instances
   - Auto-update on prefab change
   - Instance-specific overrides

**Deliverables:**
- Asset browser shows all prefabs
- Can drag prefab into scene
- Prefab editor works
- Instances update when source changes

**Testing:**
- Create new prefab from entity
- Edit prefab, verify instances update
- Drag prefab into scene multiple times
- Override instance properties

### Phase 5: 4D Tools (3-4 sessions)

**Goal:** Solve 4D editing challenges.

**Tasks:**

1. **Multi-slice viewport** (1 session)
   - Show 4 W-slices side-by-side
   - Synchronized camera controls
   - Highlight active slice

2. **4D gizmo design** (1 session)
   - Research 4D manipulation UX
   - Design W-axis handle
   - Prototype 4D gizmo rendering

3. **4D visualization tools** (1 session)
   - Tesseract wireframe view
   - Cross-section highlight
   - W-axis timeline widget

4. **UX testing and refinement** (1 session)
   - User testing with Willow
   - Iterate based on feedback
   - Polish 4D interactions

**Deliverables:**
- Multi-slice view works
- 4D gizmo allows W-axis movement
- 4D visualization is clear
- Users can edit 4D scenes comfortably

**Testing:**
- Move tesseract in 4D space
- Verify cross-sections are clear
- User testing for usability
- Iteration based on feedback

---

## Risk Assessment

### Technical Risks

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| **egui performance issues with complex scenes** | Medium | High | Profile early; optimize rendering; implement occlusion culling |
| **4D gizmo UX is unusable** | High | High | Extensive prototyping; user testing; accept iteration |
| **wgpu + egui integration bugs** | Low | Medium | Use stable versions; test on multiple platforms |
| **Undo/redo stack memory usage** | Medium | Low | Limit history size; implement compact command representation |
| **Scene file corruption** | Low | Critical | Always backup before save; extensive validation |

### UX Risks

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| **4D editing is too confusing** | High | Critical | Multi-slice view; W-timeline; extensive UX research |
| **Editor is too complex for artists** | Medium | High | Guided tutorials; tooltips; simplified default UI |
| **Viewport navigation is unintuitive** | Low | Medium | Copy Unity/Blender controls; add preference presets |

### Project Risks

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| **Scope creep (too many features)** | High | Medium | Strict phase boundaries; defer nice-to-haves |
| **Takes longer than estimated** | Medium | Low | Phased approach allows partial delivery |
| **Scene format changes break editor** | Medium | Medium | Version detection; migration tools; extensive tests |
| **Editor becomes unmaintained** | Low | High | Keep codebase simple; good documentation |

### Critical Risk: 4D UX

**The biggest risk is that 4D editing is fundamentally too difficult for humans.**

**Mitigation strategies:**

1. **Fallback to 3D + W-coordinate editing** - If 4D gizmo fails, use 3D manipulation + W slider
2. **Preset views** - "XYZ view (W=0)", "XYW view (Z=0)", etc.
3. **Constraints** - Lock dimensions (3D editing only, ignore W)
4. **Scripting escape hatch** - For complex 4D transforms, use code
5. **Accept iteration** - Plan for multiple UX redesigns in Phase 5

**Success metrics:**

- Willow can place tesseract in 4D space in <1 minute
- Non-expert users can complete basic 4D editing tasks
- Feedback is "challenging but doable" not "impossible"

---

## Success Criteria

### Minimum Viable Editor (MVP)

The editor is considered successful if Willow can:

1. **Load existing scene files** - Open RON scenes without errors
2. **View scene structure** - See all entities and their properties
3. **Edit properties** - Change positions, colors, physics settings
4. **Save changes** - Write modified scene back to RON file
5. **Place entities visually** - Drag entities in 3D space
6. **Use prefabs** - Instantiate prefab templates into scenes
7. **Navigate 4D space** - Move through W-slices to see cross-sections

### Full Success Criteria

The editor is considered production-ready if:

1. **Workflow is faster than code** - Creating a scene in the editor is faster than writing RON/Rust
2. **Non-programmers can use it** - Artists can create content without learning Rust
3. **4D editing is usable** - Users can manipulate 4D objects with reasonable effort
4. **Stable and reliable** - No crashes, no file corruption, no data loss
5. **Integrated with runtime** - Play mode works, hot reload works
6. **Polished UX** - Professional feel, good performance, helpful error messages

### Measurable Metrics

- **Scene creation time** - Simple scene (<10 entities) takes <5 minutes in editor
- **Error rate** - <5% of save operations fail validation
- **Crash rate** - <1 crash per 100 editor sessions
- **User satisfaction** - Willow rates editor ≥7/10 for usability

### Long-Term Vision

In the future, the editor could support:

- **Scripting integration** - Attach Lua/Rhai scripts to entities
- **Animation timeline** - Animate entity transforms over time
- **Particle systems** - Visual particle effect editor
- **4D terrain brushes** - Paint 4D terrain/geometry
- **Collaborative editing** - Multiple users edit same scene (very long-term)

---

## References

### Game Engine Editors

- [Unity Editor Architecture](https://docs.unity3d.com/Manual/EditorWindow.html) - Industry standard editor patterns
- [Godot Editor Overview](https://docs.godotengine.org/en/stable/contributing/development/editor/editor_style_guide.html) - Open-source editor design
- [Bevy Editor (WIP)](https://github.com/bevyengine/bevy/discussions/1734) - Rust game engine editor plans
- [Ambient Editor](https://github.com/AmbientRun/Ambient) - Rust multiplayer game engine with editor

### egui Resources

- [egui Documentation](https://docs.rs/egui/latest/egui/) - Official docs
- [egui Demo](https://www.egui.rs/) - Interactive feature showcase
- [egui Examples](https://github.com/emilk/egui/tree/master/examples) - Code examples
- [egui_dock](https://github.com/Adanos020/egui_dock) - Dockable panel layout
- [egui_extras](https://docs.rs/egui_extras/latest/egui_extras/) - Additional widgets

### 4D Visualization

- [4D Toys](http://4dtoys.com/) - 4D physics game (UX reference)
- [Miegakure](https://miegakure.com/) - 4D puzzle platformer (navigation reference)
- [4D Visualization Techniques (Paper)](https://scholar.google.com/scholar?q=4D+visualization+techniques) - Academic research
- [Hypercube Projection](https://en.wikipedia.org/wiki/Tesseract) - Mathematical background

### Gizmo Systems

- [Unity Gizmo API](https://docs.unity3d.com/ScriptReference/Gizmos.html) - Industry standard
- [Blender Transform Gizmos](https://docs.blender.org/manual/en/latest/editors/3dview/controls/gizmos.html) - Professional 3D editor reference
- [egui_gizmo](https://github.com/urholaukkarinen/egui-gizmo) - Existing egui gizmo implementation

### Undo/Redo Patterns

- [Command Pattern](https://refactoring.guru/design-patterns/command) - Design pattern for undo/redo
- [Undo/Redo in Game Engines](https://www.codingame.com/playgrounds/4956/undo-redo-for-a-game-engine) - Implementation strategies

---

## Appendix: Alternative Architectures Considered

### Standalone Electron App

**Idea:** Build editor as separate Electron app with web frontend.

**Pros:**
- Rich web UI libraries (React, Vue)
- Familiar to web developers
- Cross-platform desktop app

**Cons:**
- Requires maintaining separate codebase
- Harder to share code with engine runtime
- Electron is heavy (100+ MB)
- Communication overhead (IPC between editor and engine)

**Verdict:** Rejected - too much complexity for little benefit.

### Web-Based Editor

**Idea:** Editor runs in browser, uses WebGPU to render scenes.

**Pros:**
- Accessible from anywhere
- No installation required
- Shareable links to scenes

**Cons:**
- WebGPU is still experimental
- Requires porting engine to WASM
- Performance limitations
- Offline usage is complicated

**Verdict:** Rejected for now - consider for distant future.

### Text-Based TUI Editor

**Idea:** Terminal UI editor (using ratatui or similar).

**Pros:**
- Lightweight, fast
- Runs over SSH
- Cool hacker aesthetic

**Cons:**
- Can't render 3D/4D viewport
- Limited UX (no mouse, no graphics)
- Not suitable for spatial editing

**Verdict:** Rejected - viewport is essential for scene editor.

---

**End of Plan**
