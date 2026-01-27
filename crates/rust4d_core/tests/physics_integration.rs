//! Integration tests for physics pipeline
//!
//! These tests verify the full scene-physics-render pipeline works correctly:
//! 1. Scene loading creates correct physics bodies
//! 2. Physics simulation applies gravity and collision
//! 3. Entity transforms sync from physics bodies
//! 4. Dirty flags trigger geometry rebuild

use rust4d_core::{
    ActiveScene, Scene, World, EntityTemplate, Transform4D, Material, ShapeRef,
    ShapeTemplate,
};
use rust4d_physics::{
    PhysicsConfig, PhysicsWorld, RigidBody4D, BodyType, StaticCollider, PhysicsMaterial,
};
use rust4d_math::{Vec4, Tesseract4D};

// ==================== Scene Loading Tests ====================

/// Test that dynamic entities get physics bodies created
#[test]
fn test_scene_dynamic_entity_has_physics_body() {
    // Create a scene with a dynamic entity
    let mut scene = Scene::new("Test Scene").with_gravity(-20.0);

    scene.add_entity(
        EntityTemplate::new(
            ShapeTemplate::tesseract(2.0),
            Transform4D::from_position(Vec4::new(0.0, 0.0, 0.0, 0.0)),
            Material::WHITE,
        )
        .with_name("tesseract")
        .with_tag("dynamic")
    );

    // Instantiate the scene
    let active = ActiveScene::from_template(&scene, None, 0.5);

    // Get the entity
    let (_, entity) = active.world.get_by_name("tesseract")
        .expect("Tesseract entity should exist");

    // Verify physics body was created
    assert!(
        entity.physics_body.is_some(),
        "Dynamic entity should have a physics body"
    );

    // Verify the body exists in the physics world
    let physics = active.world.physics().expect("World should have physics");
    let body_key = entity.physics_body.unwrap();
    let body = physics.get_body(body_key).expect("Physics body should exist");

    // Verify body type is Dynamic
    assert!(
        !body.is_static(),
        "Body should not be static"
    );
    assert!(
        body.affected_by_gravity(),
        "Dynamic body should be affected by gravity"
    );
}

/// Test that static floors get colliders created
#[test]
fn test_scene_static_floor_has_collider() {
    let mut scene = Scene::new("Test Scene").with_gravity(-20.0);

    scene.add_entity(
        EntityTemplate::new(
            ShapeTemplate::hyperplane(-2.0, 10.0, 10, 5.0, 0.001),
            Transform4D::from_position(Vec4::new(0.0, -2.0, 0.0, 0.0)),
            Material::GRAY,
        )
        .with_name("floor")
        .with_tag("static")
    );

    let active = ActiveScene::from_template(&scene, None, 0.5);

    // Verify static collider was created
    let physics = active.world.physics().expect("World should have physics");
    assert!(
        !physics.static_colliders().is_empty(),
        "Static floor should create a collider"
    );
}

// ==================== Physics Simulation Tests ====================

/// Test that a dynamic body falls under gravity
#[test]
fn test_dynamic_body_falls_under_gravity() {
    let mut physics = PhysicsWorld::with_config(PhysicsConfig::new(-20.0));

    // Add a body at y=10
    let body = RigidBody4D::new_sphere(Vec4::new(0.0, 10.0, 0.0, 0.0), 0.5)
        .with_body_type(BodyType::Dynamic);
    let key = physics.add_body(body);

    // Step physics once
    physics.step(0.1);

    // Body should have fallen
    let body = physics.get_body(key).unwrap();
    assert!(
        body.position.y < 10.0,
        "Body should fall under gravity. Position: {:?}",
        body.position
    );
    assert!(
        body.velocity.y < 0.0,
        "Body should have downward velocity. Velocity: {:?}",
        body.velocity
    );
}

/// Test that a dynamic body lands on a floor and becomes grounded
#[test]
fn test_dynamic_body_lands_on_floor() {
    let mut physics = PhysicsWorld::with_config(PhysicsConfig::new(-20.0));

    // Add a floor at y=0
    physics.add_static_collider(StaticCollider::floor(0.0, PhysicsMaterial::CONCRETE));

    // Add a body slightly above the floor
    let body = RigidBody4D::new_sphere(Vec4::new(0.0, 1.0, 0.0, 0.0), 0.5)
        .with_body_type(BodyType::Dynamic);
    let key = physics.add_body(body);

    // Step physics multiple times until it settles
    for _ in 0..100 {
        physics.step(1.0 / 60.0);
    }

    let body = physics.get_body(key).unwrap();

    // Body should be near the floor (radius 0.5, floor at 0, so center at ~0.5)
    assert!(
        body.position.y < 1.0,
        "Body should have fallen. Y={}", body.position.y
    );
    assert!(
        body.position.y > -1.0,
        "Body should be above floor. Y={}", body.position.y
    );
    assert!(
        body.grounded,
        "Body should be grounded after settling"
    );
}

/// Test bounded floor collision (the specific bug scenario)
#[test]
fn test_aabb_body_lands_on_bounded_floor() {
    let mut physics = PhysicsWorld::with_config(PhysicsConfig::new(-20.0));

    // Add a bounded floor at y=-2 (matching default.ron)
    physics.add_static_collider(StaticCollider::floor_bounded(
        -2.0,   // y (surface level)
        10.0,   // half_size_xz
        5.0,    // half_size_w
        5.0,    // thickness (minimum)
        PhysicsMaterial::CONCRETE,
    ));

    // Add an AABB body at y=0 with half_extent=1 (matching tesseract in default.ron)
    let body = RigidBody4D::new_aabb(
        Vec4::new(0.0, 0.0, 0.0, 0.0),  // position
        Vec4::new(1.0, 1.0, 1.0, 1.0),  // half_extents
    )
    .with_body_type(BodyType::Dynamic)
    .with_mass(10.0)
    .with_material(PhysicsMaterial::WOOD);

    let key = physics.add_body(body);

    // Record initial position
    let initial_y = physics.get_body(key).unwrap().position.y;

    // Step physics for 2 seconds
    for _ in 0..120 {
        physics.step(1.0 / 60.0);
    }

    let body = physics.get_body(key).unwrap();

    // Body should have fallen from y=0
    assert!(
        body.position.y < initial_y,
        "Body should have fallen. Initial: {}, Final: {}", initial_y, body.position.y
    );

    // Body center should be at approximately y=-1 (bottom at y=-2, floor surface at y=-2)
    // With half_extent.y=1, center at y=-1 means bottom is at y=-2 (floor surface)
    assert!(
        body.position.y > -2.0,
        "Body should be above floor. Y={}", body.position.y
    );
    assert!(
        body.position.y < 0.0,
        "Body should be below starting position. Y={}", body.position.y
    );

    // Body should be grounded
    assert!(
        body.grounded,
        "Body should be grounded after landing. Position: {:?}, Grounded: {}",
        body.position, body.grounded
    );
}

// ==================== Entity-Physics Sync Tests ====================

/// Test that entity transform syncs from physics body
#[test]
fn test_entity_transform_syncs_from_physics() {
    let mut world = World::new().with_physics(PhysicsConfig::new(-20.0));

    // Add a physics body with velocity
    let body = RigidBody4D::new_sphere(Vec4::new(0.0, 10.0, 0.0, 0.0), 0.5)
        .with_body_type(BodyType::Dynamic);
    let body_key = world.physics_mut().unwrap().add_body(body);

    // Create an entity linked to the physics body
    let tesseract = Tesseract4D::new(2.0);
    let entity = rust4d_core::Entity::new(ShapeRef::shared(tesseract))
        .with_name("test")
        .with_physics_body(body_key);
    world.add_entity(entity);

    // Clear dirty flags
    world.clear_all_dirty();

    // Step physics
    world.update(0.1);

    // Entity should have new position
    let (_, entity) = world.get_by_name("test").unwrap();
    assert!(
        entity.transform.position.y < 10.0,
        "Entity should have moved. Y={}", entity.transform.position.y
    );

    // Entity should be marked dirty
    assert!(
        entity.is_dirty(),
        "Entity should be marked dirty after position sync"
    );
}

// ==================== Full Pipeline Test ====================

/// The critical test: full scene loading to physics settling
/// This tests the exact scenario from default.ron
#[test]
fn test_scene_dynamic_entity_falls_to_floor() {
    // Create a scene matching default.ron structure
    let mut scene = Scene::new("Test Scene").with_gravity(-20.0);

    // Add floor
    scene.add_entity(
        EntityTemplate::new(
            ShapeTemplate::hyperplane(-2.0, 10.0, 10, 5.0, 0.001),
            Transform4D::from_position(Vec4::new(0.0, -2.0, 0.0, 0.0)),
            Material::GRAY,
        )
        .with_name("floor")
        .with_tag("static")
    );

    // Add tesseract at y=0
    scene.add_entity(
        EntityTemplate::new(
            ShapeTemplate::tesseract(2.0),
            Transform4D::from_position(Vec4::new(0.0, 0.0, 0.0, 0.0)),
            Material::WHITE,
        )
        .with_name("tesseract")
        .with_tag("dynamic")
    );

    // Instantiate scene
    let mut active = ActiveScene::from_template(&scene, None, 0.5);

    // Get initial tesseract position
    let initial_y = active.world.get_by_name("tesseract")
        .unwrap().1.transform.position.y;

    // Simulate 2 seconds (120 frames at 60fps)
    for _ in 0..120 {
        active.update(1.0 / 60.0);
    }

    // Get final position
    let (_, entity) = active.world.get_by_name("tesseract").unwrap();
    let final_y = entity.transform.position.y;

    // Tesseract should have fallen
    assert!(
        final_y < initial_y,
        "Tesseract should have fallen. Initial: {}, Final: {}", initial_y, final_y
    );

    // Tesseract should be near the floor (center at ~-1, bottom at -2)
    // With size=2.0 (half_extent=1.0), center at y=-1 means bottom is at y=-2 (floor surface)
    assert!(
        final_y > -2.0,
        "Tesseract should be above floor surface. Y={}", final_y
    );
    assert!(
        final_y < 0.0,
        "Tesseract should be below starting position. Y={}", final_y
    );

    // Verify physics body is grounded
    let physics = active.world.physics().unwrap();
    let body = physics.get_body(entity.physics_body.unwrap()).unwrap();
    assert!(
        body.grounded,
        "Tesseract physics body should be grounded"
    );
}

/// Test with actual scene file if it exists
#[test]
fn test_load_default_scene_file() {
    // Try to load the actual default.ron scene
    let scene_result = Scene::load("../../../scenes/default.ron");

    // If the file doesn't exist, skip this test
    let scene = match scene_result {
        Ok(s) => s,
        Err(_) => {
            println!("Skipping test_load_default_scene_file: scenes/default.ron not found");
            return;
        }
    };

    // Instantiate scene
    let mut active = ActiveScene::from_template(&scene, None, 0.5);

    // Verify tesseract entity exists and has physics body
    let (_, entity) = active.world.get_by_name("tesseract")
        .expect("Tesseract entity should exist in default scene");

    assert!(
        entity.physics_body.is_some(),
        "Tesseract should have physics body"
    );

    let initial_y = entity.transform.position.y;

    // Simulate 2 seconds
    for _ in 0..120 {
        active.update(1.0 / 60.0);
    }

    // Get final position
    let (_, entity) = active.world.get_by_name("tesseract").unwrap();
    let final_y = entity.transform.position.y;

    println!("Default scene test: initial_y={}, final_y={}", initial_y, final_y);

    // Tesseract should have fallen
    assert!(
        final_y < initial_y,
        "Tesseract should have fallen from {} to near floor. Final: {}", initial_y, final_y
    );
}

// ==================== Diagnostic Tests ====================

/// Print detailed state for debugging
#[test]
fn test_physics_step_trace() {
    let mut physics = PhysicsWorld::with_config(PhysicsConfig::new(-20.0));

    // Add bounded floor at y=-2
    physics.add_static_collider(StaticCollider::floor_bounded(
        -2.0, 10.0, 5.0, 5.0, PhysicsMaterial::CONCRETE,
    ));

    // Add AABB body at y=0
    let body = RigidBody4D::new_aabb(
        Vec4::new(0.0, 0.0, 0.0, 0.0),
        Vec4::new(1.0, 1.0, 1.0, 1.0),
    )
    .with_body_type(BodyType::Dynamic)
    .with_mass(10.0);

    let key = physics.add_body(body);

    println!("=== Physics Step Trace ===");
    println!("Gravity: {}", physics.config.gravity);
    println!("Static colliders: {}", physics.static_colliders().len());

    for frame in 0..10 {
        let body = physics.get_body(key).unwrap();
        println!(
            "Frame {}: pos.y={:.4}, vel.y={:.4}, grounded={}",
            frame, body.position.y, body.velocity.y, body.grounded
        );
        physics.step(1.0 / 60.0);
    }

    let body = physics.get_body(key).unwrap();
    println!("Final: pos.y={:.4}, vel.y={:.4}, grounded={}",
        body.position.y, body.velocity.y, body.grounded);

    // Should be falling
    assert!(body.position.y < 0.0, "Body should have fallen");
}
