//! Scene serialization
//!
//! Provides Scene struct for loading/saving scenes from RON files.
//! Scenes contain entity templates, physics settings, and player spawn info.

use serde::{Serialize, Deserialize};
use std::path::Path;
use std::fs;
use std::io;

use crate::entity::EntityTemplate;

/// A serializable scene containing entity templates
///
/// Scenes are loaded from RON files and contain all the data needed
/// to populate a game world: entities, physics settings, and spawn points.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    /// Scene name (for display/debugging)
    pub name: String,
    /// Entity templates in this scene
    pub entities: Vec<EntityTemplate>,
    /// Gravity for physics (negative = downward)
    #[serde(default)]
    pub gravity: Option<f32>,
    /// Player spawn position [x, y, z, w]
    #[serde(default)]
    pub player_spawn: Option<[f32; 4]>,
}

impl Scene {
    /// Create a new empty scene
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            entities: Vec::new(),
            gravity: None,
            player_spawn: None,
        }
    }

    /// Load a scene from a RON file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, SceneLoadError> {
        let contents = fs::read_to_string(path)?;
        let scene = ron::from_str(&contents)?;
        Ok(scene)
    }

    /// Save a scene to a RON file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), SceneSaveError> {
        let pretty = ron::ser::PrettyConfig::new()
            .struct_names(true)
            .enumerate_arrays(false);
        let contents = ron::ser::to_string_pretty(self, pretty)?;
        fs::write(path, contents)?;
        Ok(())
    }

    /// Add an entity template to this scene
    pub fn add_entity(&mut self, entity: EntityTemplate) {
        self.entities.push(entity);
    }

    /// Set the gravity for this scene
    pub fn with_gravity(mut self, gravity: f32) -> Self {
        self.gravity = Some(gravity);
        self
    }

    /// Set the player spawn position
    pub fn with_player_spawn(mut self, x: f32, y: f32, z: f32, w: f32) -> Self {
        self.player_spawn = Some([x, y, z, w]);
        self
    }
}

/// Error loading a scene
#[derive(Debug)]
pub enum SceneLoadError {
    /// IO error (file not found, permission denied, etc.)
    Io(io::Error),
    /// Parse error (invalid RON syntax)
    Parse(ron::error::SpannedError),
}

impl From<io::Error> for SceneLoadError {
    fn from(e: io::Error) -> Self {
        SceneLoadError::Io(e)
    }
}

impl From<ron::error::SpannedError> for SceneLoadError {
    fn from(e: ron::error::SpannedError) -> Self {
        SceneLoadError::Parse(e)
    }
}

impl std::fmt::Display for SceneLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SceneLoadError::Io(e) => write!(f, "IO error: {}", e),
            SceneLoadError::Parse(e) => write!(f, "Parse error: {}", e),
        }
    }
}

impl std::error::Error for SceneLoadError {}

/// Error saving a scene
#[derive(Debug)]
pub enum SceneSaveError {
    /// IO error (permission denied, disk full, etc.)
    Io(io::Error),
    /// Serialization error
    Serialize(ron::Error),
}

impl From<io::Error> for SceneSaveError {
    fn from(e: io::Error) -> Self {
        SceneSaveError::Io(e)
    }
}

impl From<ron::Error> for SceneSaveError {
    fn from(e: ron::Error) -> Self {
        SceneSaveError::Serialize(e)
    }
}

impl std::fmt::Display for SceneSaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SceneSaveError::Io(e) => write!(f, "IO error: {}", e),
            SceneSaveError::Serialize(e) => write!(f, "Serialize error: {}", e),
        }
    }
}

impl std::error::Error for SceneSaveError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Transform4D, Material};
    use crate::shapes::ShapeTemplate;
    use rust4d_math::Vec4;

    #[test]
    fn test_scene_new() {
        let scene = Scene::new("Test Scene");
        assert_eq!(scene.name, "Test Scene");
        assert!(scene.entities.is_empty());
        assert!(scene.gravity.is_none());
        assert!(scene.player_spawn.is_none());
    }

    #[test]
    fn test_scene_with_gravity() {
        let scene = Scene::new("Test").with_gravity(-20.0);
        assert_eq!(scene.gravity, Some(-20.0));
    }

    #[test]
    fn test_scene_with_player_spawn() {
        let scene = Scene::new("Test").with_player_spawn(1.0, 2.0, 3.0, 4.0);
        assert_eq!(scene.player_spawn, Some([1.0, 2.0, 3.0, 4.0]));
    }

    #[test]
    fn test_scene_add_entity() {
        let mut scene = Scene::new("Test");
        let entity = EntityTemplate::new(
            ShapeTemplate::tesseract(2.0),
            Transform4D::identity(),
            Material::WHITE,
        );
        scene.add_entity(entity);
        assert_eq!(scene.entities.len(), 1);
    }

    #[test]
    fn test_scene_serialization() {
        let mut scene = Scene::new("Test Scene")
            .with_gravity(-20.0)
            .with_player_spawn(0.0, 2.0, 5.0, 0.0);

        let entity = EntityTemplate::new(
            ShapeTemplate::tesseract(2.0),
            Transform4D::from_position(Vec4::new(1.0, 0.0, 0.0, 0.0)),
            Material::RED,
        ).with_name("test_cube").with_tag("dynamic");

        scene.add_entity(entity);

        // Serialize to RON
        let pretty = ron::ser::PrettyConfig::new().struct_names(true);
        let serialized = ron::ser::to_string_pretty(&scene, pretty).unwrap();

        // Verify it contains expected content
        assert!(serialized.contains("Test Scene"));
        assert!(serialized.contains("test_cube"));
        assert!(serialized.contains("Tesseract"));

        // Deserialize back
        let deserialized: Scene = ron::from_str(&serialized).unwrap();
        assert_eq!(deserialized.name, "Test Scene");
        assert_eq!(deserialized.gravity, Some(-20.0));
        assert_eq!(deserialized.entities.len(), 1);
        assert_eq!(deserialized.entities[0].name, Some("test_cube".to_string()));
    }

    #[test]
    fn test_parse_scene_file_format() {
        // Test parsing a scene matching the actual serialization format
        let scene_ron = r#"
Scene(
    name: "Test Scene",
    entities: [
        EntityTemplate(
            name: Some("floor"),
            tags: ["static"],
            transform: Transform4D(
                position: Vec4(x: 0.0, y: -2.0, z: 0.0, w: 0.0),
                rotation: (1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
                scale: 1.0,
            ),
            shape: ShapeTemplate(
                type: "Hyperplane",
                y: -2.0,
                size: 10.0,
                subdivisions: 10,
                cell_size: 5.0,
                thickness: 0.001,
            ),
            material: Material(base_color: (0.5, 0.5, 0.5, 1.0)),
        ),
        EntityTemplate(
            name: Some("tesseract"),
            tags: ["dynamic"],
            transform: Transform4D(
                position: Vec4(x: 0.0, y: 0.0, z: 0.0, w: 0.0),
                rotation: (1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
                scale: 1.0,
            ),
            shape: ShapeTemplate(
                type: "Tesseract",
                size: 2.0,
            ),
            material: Material(base_color: (1.0, 1.0, 1.0, 1.0)),
        ),
    ],
    gravity: Some(-20.0),
    player_spawn: Some((0.0, 2.0, 5.0, 0.0)),
)
"#;
        let scene: Scene = ron::from_str(scene_ron).unwrap();
        assert_eq!(scene.name, "Test Scene");
        assert_eq!(scene.gravity, Some(-20.0));
        assert_eq!(scene.player_spawn, Some([0.0, 2.0, 5.0, 0.0]));
        assert_eq!(scene.entities.len(), 2);

        // Check floor entity
        assert_eq!(scene.entities[0].name, Some("floor".to_string()));
        assert_eq!(scene.entities[0].tags, vec!["static"]);
        match &scene.entities[0].shape {
            ShapeTemplate::Hyperplane { y, size, subdivisions, cell_size, thickness } => {
                assert_eq!(*y, -2.0);
                assert_eq!(*size, 10.0);
                assert_eq!(*subdivisions, 10);
                assert_eq!(*cell_size, 5.0);
                assert_eq!(*thickness, 0.001);
            }
            _ => panic!("Expected Hyperplane shape"),
        }

        // Check tesseract entity
        assert_eq!(scene.entities[1].name, Some("tesseract".to_string()));
        assert_eq!(scene.entities[1].tags, vec!["dynamic"]);
        match &scene.entities[1].shape {
            ShapeTemplate::Tesseract { size } => {
                assert_eq!(*size, 2.0);
            }
            _ => panic!("Expected Tesseract shape"),
        }
    }

    #[test]
    fn test_entity_template_to_entity() {
        let template = EntityTemplate::new(
            ShapeTemplate::tesseract(2.0),
            Transform4D::from_position(Vec4::new(1.0, 2.0, 3.0, 4.0)),
            Material::RED,
        ).with_name("my_cube").with_tag("dynamic");

        let entity = template.to_entity();

        assert_eq!(entity.name, Some("my_cube".to_string()));
        assert!(entity.has_tag("dynamic"));
        assert_eq!(entity.transform.position.x, 1.0);
        assert_eq!(entity.material.base_color, [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(entity.shape().vertex_count(), 16); // Tesseract has 16 vertices
    }
}
