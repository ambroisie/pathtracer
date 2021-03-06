use super::Shape;
use crate::{Point, Point2D, Vector};
use beevee::aabb::AABB;
use beevee::ray::Ray;
use nalgebra::Unit;
use serde::{Deserialize, Deserializer};

/// Represent a triangle inside the scene.
#[derive(Clone, Debug, PartialEq)]
pub struct Triangle {
    c0: Point,
    c0c1: Vector,
    c0c2: Vector,
}

impl Triangle {
    /// Creates a new `Triangle` from 3 [`Point`]s.
    ///
    /// [`Point`]: ../../type.Point.html
    ///
    /// # Examples
    ///
    /// ```
    /// # use pathtracer::shape::Triangle;
    /// # use pathtracer::Point;
    /// #
    /// let t = Triangle::new(
    ///     Point::new(1.0, 0.0, 0.0),
    ///     Point::new(0.0, 1.0, 0.0),
    ///     Point::new(0.0, 0.0, 1.0),
    /// );
    /// ```
    pub fn new(c0: Point, c1: Point, c2: Point) -> Self {
        Triangle {
            c0,
            c0c1: c1 - c0,
            c0c2: c2 - c0,
        }
    }

    fn barycentric(&self, point: &Point) -> Point2D {
        let c0_pos = point - self.c0;
        // P - A  =  u * (B - A) + v * (C - A)
        // (C - A) = v0 is c0c2
        // (B - A) = v1 is c0c1
        // (P - A) = v2 is c0_pos
        let dot00 = self.c0c2.dot(&self.c0c2);
        let dot01 = self.c0c2.dot(&self.c0c1);
        let dot02 = self.c0c2.dot(&c0_pos);
        let dot11 = self.c0c1.dot(&self.c0c1);
        let dot12 = self.c0c1.dot(&c0_pos);

        let inv_denom = 1. / (dot00 * dot11 - dot01 * dot01);
        let u = (dot00 * dot12 - dot01 * dot02) * inv_denom;
        let v = (dot11 * dot02 - dot01 * dot12) * inv_denom;
        Point2D::new(u, v)
    }
}

impl Shape for Triangle {
    fn intersect(&self, ray: &Ray) -> Option<f32> {
        let pvec = ray.direction.cross(&self.c0c2);
        let det = self.c0c1.dot(&pvec);

        if det.abs() < 1e-5 {
            return None;
        }

        let to_ray = ray.origin - self.c0;
        let inv_det = 1. / det;
        let u = to_ray.dot(&pvec) * inv_det;

        if u < 0. || u > 1. {
            return None;
        }

        let qvec = to_ray.cross(&self.c0c1);
        let v = ray.direction.dot(&qvec) * inv_det;

        if v < 0. || u + v > 1. {
            return None;
        }

        let t = self.c0c2.dot(&qvec) * inv_det;
        if t < 0. {
            None
        } else {
            Some(t)
        }
    }

    fn normal(&self, _: &Point) -> Unit<Vector> {
        Unit::new_normalize(self.c0c1.cross(&self.c0c2))
    }

    fn project_texel(&self, point: &Point) -> Point2D {
        self.barycentric(point)
    }

    fn aabb(&self) -> AABB {
        AABB::empty()
            .grow(&self.c0)
            .grow(&(self.c0 + self.c0c1))
            .grow(&(self.c0 + self.c0c2))
    }

    fn centroid(&self) -> Point {
        self.c0 + (self.c0c1 + self.c0c2) / 2.
    }
}

#[derive(Debug, Deserialize)]
struct SerializedTriangle {
    corners: [Point; 3],
}

impl From<SerializedTriangle> for Triangle {
    fn from(triangle: SerializedTriangle) -> Self {
        Triangle::new(
            triangle.corners[0],
            triangle.corners[1],
            triangle.corners[2],
        )
    }
}

impl<'de> Deserialize<'de> for Triangle {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let cam: SerializedTriangle = Deserialize::deserialize(deserializer)?;
        Ok(cam.into())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use nalgebra::Unit;

    fn simple_triangle() -> Triangle {
        Triangle::new(
            Point::origin(),
            Point::new(0., 1., 1.),
            Point::new(0., 1., 0.),
        )
    }

    #[test]
    fn intersect_along_normal_works() {
        let triangle = simple_triangle();
        let ans = triangle.intersect(&Ray::new(
            Point::new(-1., 0.5, 0.5),
            Unit::new_normalize(Vector::new(1., 0., 0.)),
        ));
        assert_eq!(ans, Some(1.0))
    }

    #[test]
    fn intersect_at_angle_works() {
        let triangle = simple_triangle();
        let ans = triangle.intersect(&Ray::new(
            Point::new(-1., 0.5, 0.),
            Unit::new_normalize(Vector::new(1., 0., 0.5)),
        ));
        assert!(ans.is_some());
        assert!((ans.unwrap() - f32::sqrt(1.0 + 0.25)).abs() < 1e-5)
    }

    #[test]
    fn intersect_out_of_bounds_is_none() {
        let triangle = simple_triangle();
        let ans = triangle.intersect(&Ray::new(
            Point::new(-1., 0.5, 0.),
            Unit::new_normalize(Vector::new(1., 1., 1.)),
        ));
        assert_eq!(ans, None)
    }

    #[test]
    fn normal_works() {
        let triangle = simple_triangle();
        let normal = triangle.normal(&Point::origin());
        assert_eq!(normal, Unit::new_normalize(Vector::new(-1., 0., 0.)));
    }

    #[test]
    fn project_texel_works_1() {
        let triangle = simple_triangle();
        let ans = triangle.project_texel(&Point::origin());
        assert!((ans - Point2D::origin()).magnitude() < 1e-5)
    }

    #[test]
    fn project_texel_works_2() {
        let triangle = simple_triangle();
        let ans = triangle.project_texel(&Point::new(0., 1., 1.));
        assert!((ans - Point2D::new(1., 0.)).norm() < 1e-5)
    }

    #[test]
    fn project_texel_works_3() {
        let triangle = simple_triangle();
        let ans = triangle.project_texel(&Point::new(0., 1., 0.));
        assert!((ans - Point2D::new(0., 1.)).norm() < 1e-5)
    }

    #[test]
    fn project_texel_works_4() {
        let triangle = Triangle::new(
            Point::new(0., f32::sqrt(3.) / 2., 0.),
            Point::new(-0.5, 0., 0.),
            Point::new(0.5, 0., 0.),
        );
        // The centroid is at a third of the length of the height of the triangle
        let ans = triangle.project_texel(&Point::new(0., f32::sqrt(3.) / 6., 0.));
        assert!((ans - Point2D::new(1. / 3., 1. / 3.)).norm() < 1e-5);
    }

    #[test]
    fn project_texel_works_5() {
        let triangle = Triangle::new(
            Point::new(0., f32::sqrt(3.) / 2., 0.),
            Point::new(-0.5, 0., 0.),
            Point::new(0.5, 0., 0.),
        );
        // The centroid is at a third of the length of the height of the triangle
        let ans = triangle.project_texel(&Point::origin());
        assert!((ans - Point2D::new(0.5, 0.5)).norm() < 1e-5);
    }

    #[test]
    fn deserialization_works() {
        let yaml = r#"
            corners:
              - [0.0, 0.0, 0.0]
              - [0.0, 1.0, 1.0]
              - [0.0, 1.0, 0.0]
        "#;
        let triangle: Triangle = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            triangle,
            Triangle::new(
                Point::origin(),
                Point::new(0., 1., 1.),
                Point::new(0., 1., 0.)
            )
        )
    }
}
