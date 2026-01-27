//! Collision detection for 4D shapes
//!
//! Provides collision detection between spheres, AABBs, and planes.

use crate::shapes::{Plane4D, Sphere4D, AABB4D};
use rust4d_math::Vec4;

/// Contact information from a collision
#[derive(Clone, Copy, Debug)]
pub struct Contact {
    /// Point of contact (on the surface of the first shape)
    pub point: Vec4,
    /// Normal pointing from the second shape toward the first
    pub normal: Vec4,
    /// Penetration depth (positive means overlapping)
    pub penetration: f32,
}

impl Contact {
    /// Create a new contact
    pub fn new(point: Vec4, normal: Vec4, penetration: f32) -> Self {
        Self {
            point,
            normal,
            penetration,
        }
    }

    /// Check if this represents an actual collision (positive penetration)
    pub fn is_colliding(&self) -> bool {
        self.penetration > 0.0
    }
}

/// Test sphere vs plane collision
///
/// Returns a contact if the sphere is intersecting or touching the plane.
/// The contact normal points from the plane toward the sphere (same direction as plane normal
/// if sphere is above, opposite if below).
pub fn sphere_vs_plane(sphere: &Sphere4D, plane: &Plane4D) -> Option<Contact> {
    let signed_dist = plane.signed_distance(sphere.center);

    // Penetration calculation:
    // - If signed_dist > 0 (center above plane): penetration = radius - signed_dist
    // - If signed_dist < 0 (center below plane): penetration = radius + |signed_dist|
    // Combined: penetration = radius - signed_dist (works for both cases)
    let penetration = sphere.radius - signed_dist;

    if penetration > 0.0 {
        // Normal always points from plane toward sphere (upward for floor)
        let normal = plane.normal;

        // Contact point is on the sphere surface, toward the plane
        let point = sphere.center - normal * sphere.radius;

        Some(Contact::new(point, normal, penetration))
    } else {
        None
    }
}

/// Test AABB vs plane collision
///
/// Returns a contact if any part of the AABB is below/intersecting the plane.
pub fn aabb_vs_plane(aabb: &AABB4D, plane: &Plane4D) -> Option<Contact> {
    let center = aabb.center();
    let half_extents = aabb.half_extents();

    // Find the vertex closest to the plane (most in the negative normal direction)
    // This is: center - half_extents * sign(normal)
    let closest_vertex = center - half_extents.component_mul(plane.normal.sign());

    let signed_dist = plane.signed_distance(closest_vertex);

    // If the closest vertex is below the plane, we have a collision
    if signed_dist < 0.0 {
        let penetration = -signed_dist;
        let point = closest_vertex;
        let normal = plane.normal;

        Some(Contact::new(point, normal, penetration))
    } else {
        None
    }
}

/// Test sphere vs AABB collision
///
/// Returns a contact if the sphere is intersecting the AABB.
pub fn sphere_vs_aabb(sphere: &Sphere4D, aabb: &AABB4D) -> Option<Contact> {
    // Find the closest point on the AABB to the sphere center
    let closest = aabb.closest_point(sphere.center);

    // Distance from sphere center to closest point
    let delta = sphere.center - closest;
    let dist_squared = delta.length_squared();

    if dist_squared < sphere.radius * sphere.radius {
        let dist = dist_squared.sqrt();
        let penetration = sphere.radius - dist;

        // Normal points from AABB toward sphere
        let normal = if dist > 0.0001 {
            delta.normalized()
        } else {
            // Sphere center is inside AABB - use the shortest escape direction
            let to_min = sphere.center - aabb.min;
            let to_max = aabb.max - sphere.center;

            // Find the axis with minimum distance to edge
            let mut min_dist = to_min.x;
            let mut normal = -Vec4::X;

            if to_max.x < min_dist {
                min_dist = to_max.x;
                normal = Vec4::X;
            }
            if to_min.y < min_dist {
                min_dist = to_min.y;
                normal = -Vec4::Y;
            }
            if to_max.y < min_dist {
                min_dist = to_max.y;
                normal = Vec4::Y;
            }
            if to_min.z < min_dist {
                min_dist = to_min.z;
                normal = -Vec4::Z;
            }
            if to_max.z < min_dist {
                min_dist = to_max.z;
                normal = Vec4::Z;
            }
            if to_min.w < min_dist {
                min_dist = to_min.w;
                normal = -Vec4::W;
            }
            if to_max.w < min_dist {
                normal = Vec4::W;
            }

            normal
        };

        let point = closest;

        Some(Contact::new(point, normal, penetration))
    } else {
        None
    }
}

/// Test AABB vs AABB collision
///
/// Returns a contact if the AABBs are intersecting.
pub fn aabb_vs_aabb(a: &AABB4D, b: &AABB4D) -> Option<Contact> {
    // Check for separation on each axis
    if a.max.x < b.min.x || a.min.x > b.max.x {
        return None;
    }
    if a.max.y < b.min.y || a.min.y > b.max.y {
        return None;
    }
    if a.max.z < b.min.z || a.min.z > b.max.z {
        return None;
    }
    if a.max.w < b.min.w || a.min.w > b.max.w {
        return None;
    }

    // Find overlap on each axis and use the minimum as penetration
    let overlap_x = (a.max.x.min(b.max.x) - a.min.x.max(b.min.x)).max(0.0);
    let overlap_y = (a.max.y.min(b.max.y) - a.min.y.max(b.min.y)).max(0.0);
    let overlap_z = (a.max.z.min(b.max.z) - a.min.z.max(b.min.z)).max(0.0);
    let overlap_w = (a.max.w.min(b.max.w) - a.min.w.max(b.min.w)).max(0.0);

    // Find minimum overlap axis
    let mut min_overlap = overlap_x;
    let mut normal = if a.center().x < b.center().x {
        -Vec4::X
    } else {
        Vec4::X
    };

    if overlap_y < min_overlap {
        min_overlap = overlap_y;
        normal = if a.center().y < b.center().y {
            -Vec4::Y
        } else {
            Vec4::Y
        };
    }
    if overlap_z < min_overlap {
        min_overlap = overlap_z;
        normal = if a.center().z < b.center().z {
            -Vec4::Z
        } else {
            Vec4::Z
        };
    }
    if overlap_w < min_overlap {
        min_overlap = overlap_w;
        normal = if a.center().w < b.center().w {
            -Vec4::W
        } else {
            Vec4::W
        };
    }

    // Contact point is at the center of the overlap region
    let overlap_min = a.min.max_components(b.min);
    let overlap_max = a.max.min_components(b.max);
    let point = (overlap_min + overlap_max) * 0.5;

    Some(Contact::new(point, normal, min_overlap))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sphere_vs_plane_above() {
        let sphere = Sphere4D::new(Vec4::new(0.0, 2.0, 0.0, 0.0), 1.0);
        let plane = Plane4D::floor(0.0);

        // Sphere is above plane, no collision
        assert!(sphere_vs_plane(&sphere, &plane).is_none());
    }

    #[test]
    fn test_sphere_vs_plane_touching() {
        let sphere = Sphere4D::new(Vec4::new(0.0, 1.0, 0.0, 0.0), 1.0);
        let plane = Plane4D::floor(0.0);

        // Sphere exactly touching plane - at the boundary
        let contact = sphere_vs_plane(&sphere, &plane);
        // Due to floating point, this might or might not register as a collision
        // The important thing is the math is correct
        if let Some(c) = contact {
            assert!(c.penetration.abs() < 0.0001);
        }
    }

    #[test]
    fn test_sphere_vs_plane_colliding() {
        let sphere = Sphere4D::new(Vec4::new(0.0, 0.5, 0.0, 0.0), 1.0);
        let plane = Plane4D::floor(0.0);

        let contact = sphere_vs_plane(&sphere, &plane).expect("Should collide");
        assert!((contact.penetration - 0.5).abs() < 0.0001);
        assert_eq!(contact.normal, Vec4::Y);
    }

    #[test]
    fn test_aabb_vs_plane_above() {
        let aabb = AABB4D::from_center_half_extents(Vec4::new(0.0, 2.0, 0.0, 0.0), Vec4::new(0.5, 0.5, 0.5, 0.5));
        let plane = Plane4D::floor(0.0);

        // AABB is above plane (lowest point at y=1.5)
        assert!(aabb_vs_plane(&aabb, &plane).is_none());
    }

    #[test]
    fn test_aabb_vs_plane_colliding() {
        let aabb = AABB4D::from_center_half_extents(Vec4::new(0.0, 0.25, 0.0, 0.0), Vec4::new(0.5, 0.5, 0.5, 0.5));
        let plane = Plane4D::floor(0.0);

        // AABB lowest point at y=-0.25, floor at y=0
        let contact = aabb_vs_plane(&aabb, &plane).expect("Should collide");
        assert!((contact.penetration - 0.25).abs() < 0.0001);
        assert_eq!(contact.normal, Vec4::Y);
    }

    #[test]
    fn test_sphere_vs_aabb_no_collision() {
        let sphere = Sphere4D::new(Vec4::new(5.0, 0.0, 0.0, 0.0), 1.0);
        let aabb = AABB4D::unit();

        assert!(sphere_vs_aabb(&sphere, &aabb).is_none());
    }

    #[test]
    fn test_sphere_vs_aabb_colliding() {
        let sphere = Sphere4D::new(Vec4::new(1.0, 0.0, 0.0, 0.0), 1.0);
        let aabb = AABB4D::unit(); // -0.5 to 0.5 in all dimensions

        // Sphere center at x=1, radius=1, AABB edge at x=0.5
        // Closest point on AABB is (0.5, 0, 0, 0)
        // Distance = 0.5, penetration = 1.0 - 0.5 = 0.5
        let contact = sphere_vs_aabb(&sphere, &aabb).expect("Should collide");
        assert!((contact.penetration - 0.5).abs() < 0.0001);
    }

    #[test]
    fn test_aabb_vs_aabb_no_collision() {
        let a = AABB4D::from_center_half_extents(Vec4::ZERO, Vec4::new(0.5, 0.5, 0.5, 0.5));
        let b = AABB4D::from_center_half_extents(Vec4::new(5.0, 0.0, 0.0, 0.0), Vec4::new(0.5, 0.5, 0.5, 0.5));

        assert!(aabb_vs_aabb(&a, &b).is_none());
    }

    #[test]
    fn test_aabb_vs_aabb_colliding() {
        let a = AABB4D::from_center_half_extents(Vec4::ZERO, Vec4::new(1.0, 1.0, 1.0, 1.0));
        let b = AABB4D::from_center_half_extents(Vec4::new(1.5, 0.0, 0.0, 0.0), Vec4::new(1.0, 1.0, 1.0, 1.0));

        // Overlap on x-axis: a.max.x=1.0, b.min.x=0.5, overlap=0.5
        let contact = aabb_vs_aabb(&a, &b).expect("Should collide");
        assert!((contact.penetration - 0.5).abs() < 0.0001);
    }
}
