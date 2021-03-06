use super::Intersected;
use crate::aabb::AABB;
use crate::ray::Ray;
use crate::Axis;

/// An enum representing either an internal or a leaf node of the [`BVH`]
///
/// [`BVH`]: struct.BVH.html
#[derive(Clone, Debug, PartialEq)]
enum NodeEnum {
    Internal { left: Box<Node>, right: Box<Node> },
    Leaf,
}

/// A node representing either an internal or a leaf node of the [`BVH`]
///
#[derive(Clone, Debug, PartialEq)]
struct Node {
    bounds: AABB,
    begin: usize,
    end: usize,
    kind: NodeEnum,
}

/// The BVH containing all the objects of type O.
/// This type must implement [`Intersected`].
///
/// [`Intersected`]: trait.Intersected.html
#[derive(Clone, Debug, PartialEq)]
pub struct BVH {
    tree: Node,
}

impl BVH {
    /// Build a [`BVH`] for the given slice of objects.
    /// Each leaf node will be built in a way to try and contain less than 32 objects.
    ///
    /// # Examples
    /// ```
    /// use beevee::{Point, Vector};
    /// use beevee::aabb::{AABB, Bounded};
    /// use beevee::bvh::{BVH, Intersected};
    /// use beevee::ray::Ray;
    ///
    /// #[derive(Clone, Debug, PartialEq)]
    /// struct Sphere {
    ///     center: Point,
    ///     radius: f32,
    /// }
    ///
    /// impl Bounded for Sphere {
    ///     fn aabb(&self) -> AABB {
    ///         let delt = Vector::new(self.radius, self.radius, self.radius);
    ///         AABB::with_bounds(self.center - delt, self.center + delt)
    ///     }
    ///     fn centroid(&self) -> Point {
    ///         self.center
    ///     }
    /// }
    ///
    /// impl Intersected for Sphere {
    ///     fn intersect(&self, ray: &Ray) -> Option<f32> {
    ///         use std::mem;
    ///
    ///         let delt = self.center - ray.origin;
    ///         let tca = ray.direction.dot(&delt);
    ///         let d2 = delt.norm_squared() - tca * tca;
    ///         let r_2 = self.radius * self.radius;
    ///
    ///         if d2 > r_2 {
    ///             return None;
    ///         }
    ///
    ///         let thc = (r_2 - d2).sqrt();
    ///         let mut t_0 = tca - thc;
    ///         let mut t_1 = tca + thc;
    ///
    ///         if t_0 > t_1 {
    ///             mem::swap(&mut t_0, &mut t_1)
    ///         }
    ///         if t_0 < 0. {
    ///             t_0 = t_1
    ///         }
    ///         if t_0 < 0. {
    ///             None
    ///         } else {
    ///             Some(t_0)
    ///         }
    ///     }
    /// }
    ///
    /// let spheres: &mut [Sphere] = &mut [Sphere{ center: Point::origin(), radius: 2.5 }];
    /// let bvh = BVH::build(spheres);
    /// ```
    pub fn build<O: Intersected>(objects: &mut [O]) -> Self {
        Self::with_max_capacity(objects, 32)
    }

    /// Build a [`BVH`] for the given slice of objects, with an indicated max capacity per
    /// leaf-node. The max capacity is not respected when the SAH heuristic indicate that it would
    /// be better to iterate over all objects instead of splitting.
    ///
    /// # Examples
    /// ```
    /// use beevee::{Point, Vector};
    /// use beevee::aabb::{AABB, Bounded};
    /// use beevee::bvh::{BVH, Intersected};
    /// use beevee::ray::Ray;
    ///
    /// #[derive(Clone, Debug, PartialEq)]
    /// struct Sphere {
    ///     center: Point,
    ///     radius: f32,
    /// }
    ///
    /// impl Bounded for Sphere {
    ///     fn aabb(&self) -> AABB {
    ///         let delt = Vector::new(self.radius, self.radius, self.radius);
    ///         AABB::with_bounds(self.center - delt, self.center + delt)
    ///     }
    ///     fn centroid(&self) -> Point {
    ///         self.center
    ///     }
    /// }
    ///
    /// impl Intersected for Sphere {
    ///     fn intersect(&self, ray: &Ray) -> Option<f32> {
    ///         use std::mem;
    ///
    ///         let delt = self.center - ray.origin;
    ///         let tca = ray.direction.dot(&delt);
    ///         let d2 = delt.norm_squared() - tca * tca;
    ///         let r_2 = self.radius * self.radius;
    ///
    ///         if d2 > r_2 {
    ///             return None;
    ///         }
    ///
    ///         let thc = (r_2 - d2).sqrt();
    ///         let mut t_0 = tca - thc;
    ///         let mut t_1 = tca + thc;
    ///
    ///         if t_0 > t_1 {
    ///             mem::swap(&mut t_0, &mut t_1)
    ///         }
    ///         if t_0 < 0. {
    ///             t_0 = t_1
    ///         }
    ///         if t_0 < 0. {
    ///             None
    ///         } else {
    ///             Some(t_0)
    ///         }
    ///     }
    /// }
    ///
    /// let spheres: &mut [Sphere] = &mut [Sphere{ center: Point::origin(), radius: 2.5 }];
    /// let bvh = BVH::with_max_capacity(spheres, 32);
    /// ```
    pub fn with_max_capacity<O: Intersected>(objects: &mut [O], max_cap: usize) -> Self {
        let tree = build_node(objects, 0, objects.len(), max_cap);
        Self { tree }
    }

    /// Return the true if the [`BVH`] has been built soundly:
    /// * Each child node is contained inside the parent's bounding box.
    /// * Each object in a leaf node is inside the node's bounding box.
    /// * There is no missing object indices.
    ///
    /// # Examples
    /// ```
    /// # use beevee::{Point, Vector};
    /// # use beevee::aabb::{AABB, Bounded};
    /// # use beevee::bvh::{BVH, Intersected};
    /// # use beevee::ray::Ray;
    /// #
    /// # #[derive(Clone, Debug, PartialEq)]
    /// # struct Sphere {
    /// #     center: Point,
    /// #     radius: f32,
    /// # }
    /// #
    /// # impl Bounded for Sphere {
    /// #     fn aabb(&self) -> AABB {
    /// #         let delt = Vector::new(self.radius, self.radius, self.radius);
    /// #         AABB::with_bounds(self.center - delt, self.center + delt)
    /// #     }
    /// #     fn centroid(&self) -> Point {
    /// #         self.center
    /// #     }
    /// # }
    /// #
    /// # impl Intersected for Sphere {
    /// #     fn intersect(&self, ray: &Ray) -> Option<f32> {
    /// #         use std::mem;
    /// #
    /// #         let delt = self.center - ray.origin;
    /// #         let tca = ray.direction.dot(&delt);
    /// #         let d2 = delt.norm_squared() - tca * tca;
    /// #         let r_2 = self.radius * self.radius;
    /// #
    /// #         if d2 > r_2 {
    /// #             return None;
    /// #         }
    /// #
    /// #         let thc = (r_2 - d2).sqrt();
    /// #         let mut t_0 = tca - thc;
    /// #         let mut t_1 = tca + thc;
    /// #
    /// #         if t_0 > t_1 {
    /// #             mem::swap(&mut t_0, &mut t_1)
    /// #         }
    /// #         if t_0 < 0. {
    /// #             t_0 = t_1
    /// #         }
    /// #         if t_0 < 0. {
    /// #             None
    /// #         } else {
    /// #             Some(t_0)
    /// #         }
    /// #     }
    /// # }
    /// #
    /// // Using the same sphere definition than build
    /// let spheres: &mut [Sphere] = &mut [Sphere{ center: Point::origin(), radius: 2.5 }];
    /// let bvh = BVH::with_max_capacity(spheres, 32);
    /// assert!(bvh.is_sound(spheres));
    /// ```
    pub fn is_sound<O: Intersected>(&self, objects: &[O]) -> bool {
        fn check_node<O: Intersected>(objects: &[O], node: &Node) -> bool {
            if node.begin > node.end {
                return false;
            }
            match node.kind {
                NodeEnum::Leaf => objects[node.begin..node.end]
                    .iter()
                    .all(|o| node.bounds.union(&o.aabb()) == node.bounds),
                NodeEnum::Internal {
                    ref left,
                    ref right,
                } => {
                    check_node(objects, left.as_ref())
                        && check_node(objects, right.as_ref())
                        && node.bounds.union(&left.bounds) == node.bounds
                        && node.bounds.union(&right.bounds) == node.bounds
                }
            }
        };
        check_node(objects, &self.tree)
    }

    /// Iterate recursively over the [`BVH`] to find an intersection point with the given [`Ray`].
    /// This algorithm tries to only iterate over Nodes that are abolutely necessary, and skip
    /// visiting nodes that are too far away.
    ///
    /// [`BVH`]: struct.BVH.html
    /// [`Ray`]: ../ray/struct.Ray.html
    /// # Examples
    /// ```
    /// # use beevee::{Point, Vector};
    /// # use beevee::aabb::{AABB, Bounded};
    /// # use beevee::bvh::{BVH, Intersected};
    /// # use beevee::ray::Ray;
    /// #
    /// # #[derive(Clone, Debug, PartialEq)]
    /// # struct Sphere {
    /// #     center: Point,
    /// #     radius: f32,
    /// # }
    /// #
    /// # impl Bounded for Sphere {
    /// #     fn aabb(&self) -> AABB {
    /// #         let delt = Vector::new(self.radius, self.radius, self.radius);
    /// #         AABB::with_bounds(self.center - delt, self.center + delt)
    /// #     }
    /// #     fn centroid(&self) -> Point {
    /// #         self.center
    /// #     }
    /// # }
    /// #
    /// # impl Intersected for Sphere {
    /// #     fn intersect(&self, ray: &Ray) -> Option<f32> {
    /// #         use std::mem;
    /// #
    /// #         let delt = self.center - ray.origin;
    /// #         let tca = ray.direction.dot(&delt);
    /// #         let d2 = delt.norm_squared() - tca * tca;
    /// #         let r_2 = self.radius * self.radius;
    /// #
    /// #         if d2 > r_2 {
    /// #             return None;
    /// #         }
    /// #
    /// #         let thc = (r_2 - d2).sqrt();
    /// #         let mut t_0 = tca - thc;
    /// #         let mut t_1 = tca + thc;
    /// #
    /// #         if t_0 > t_1 {
    /// #             mem::swap(&mut t_0, &mut t_1)
    /// #         }
    /// #         if t_0 < 0. {
    /// #             t_0 = t_1
    /// #         }
    /// #         if t_0 < 0. {
    /// #             None
    /// #         } else {
    /// #             Some(t_0)
    /// #         }
    /// #     }
    /// # }
    /// #
    /// // Using the same sphere definition than build
    /// let spheres: &mut [Sphere] = &mut [Sphere{ center: Point::origin(), radius: 0.5 }];
    /// let bvh = BVH::with_max_capacity(spheres, 32);
    ///
    /// // This ray is directly looking at the spheres
    /// let ray = Ray::new(Point::new(-1., 0., 0.), Vector::x_axis());
    /// let res = bvh.walk(&ray, spheres);
    ///
    /// assert!(res.is_some());
    /// let (dist, obj) = res.unwrap();
    /// assert_eq!(dist, 0.5);
    /// assert_eq!(obj, &spheres[0]);
    /// ```
    pub fn walk<'o, O: Intersected>(&self, ray: &Ray, objects: &'o [O]) -> Option<(f32, &'o O)> {
        walk_rec_helper(ray, objects, &self.tree, std::f32::INFINITY)
    }
}

fn walk_rec_helper<'o, O: Intersected>(
    ray: &Ray,
    objects: &'o [O],
    node: &Node,
    min: f32,
) -> Option<(f32, &'o O)> {
    use std::cmp::Ordering;

    match &node.kind {
        // Return the smallest intersection distance on leaf nodes
        NodeEnum::Leaf => objects[node.begin..node.end]
            .iter()
            // This turns the Option<f32> of an intersection into an Option<(f32, &O)>
            .filter_map(|o| o.intersect(ray).map(|d| (d, o)))
            // Discard values that are too far away
            .filter(|(dist, _)| dist < &min)
            // Only keep the minimum value, if there is one
            .min_by(|(lhs, _), (rhs, _)| lhs.partial_cmp(rhs).unwrap_or(Ordering::Equal)),

        // Recursively find the best node otherwise
        NodeEnum::Internal { left, right } => {
            let left_dist = left.bounds.distance_to_point(ray.origin);
            let right_dist = right.bounds.distance_to_point(ray.origin);
            // Pick the short and far nodes
            let (near, far, short_dist, far_dist) = if left_dist < right_dist {
                (left, right, left_dist, right_dist)
            } else {
                (right, left, right_dist, left_dist)
            };
            // Don't recurse if we know we cannot possibly find a short-enough intersection
            if short_dist > min {
                return None;
            }
            // Recurse to the nearest Node first
            let nearest_res = walk_rec_helper(ray, objects, near.as_ref(), min);
            // Return immediately if there is no point going to the right at all
            if far_dist > min {
                return nearest_res;
            }
            match nearest_res {
                // Short-circuit if we know it is shorter than any point in the far node
                Some((t, obj)) if t <= far_dist => Some((t, obj)),
                // We have short_dist <= far_dist <= min in this scenario
                // With the eventual val.0 in the [short_dist, min) window
                val => {
                    // Compute the new minimal distance encountered
                    let min = val.map_or(min, |(t, _)| min.min(t));
                    // Recursing with this new minimum can only return None or a better intersecion
                    walk_rec_helper(ray, objects, far.as_ref(), min).or(val)
                }
            }
        }
    }
}

fn bounds_from_slice<O: Intersected>(objects: &[O]) -> AABB {
    objects
        .iter()
        .map(|o| o.aabb())
        .fold(AABB::empty(), |acc, other| acc.union(&other))
}

fn build_node<O: Intersected>(objects: &mut [O], begin: usize, end: usize, max_cap: usize) -> Node {
    let aabb = bounds_from_slice(objects);
    // Don't split nodes under capacity
    if objects.len() <= max_cap {
        return Node {
            bounds: aabb,
            begin,
            end,
            kind: NodeEnum::Leaf,
        };
    }
    // Calculate the SAH heuristic for this slice
    let (split, axis, cost) = compute_sah(&mut objects[begin..end], aabb.surface(), max_cap);
    // Only split if the heuristic shows that it is worth it
    if cost >= objects.len() as f32 {
        return Node {
            bounds: aabb,
            begin,
            end,
            kind: NodeEnum::Leaf,
        };
    }
    // Avoid degenerate cases, and recenter the split inside [begin, end)
    let split = if split == 0 || split >= (end - begin - 1) {
        begin + (end - begin) / 2
    } else {
        begin + split
    };
    // Project along chosen axis
    pdqselect::select_by(objects, split, |lhs, rhs| {
        lhs.centroid()[axis]
            .partial_cmp(&rhs.centroid()[axis])
            .expect("Can't use Nans in the SAH computation")
    });
    // Construct children recurivsely on [begin, split) and [split, end)
    let left = Box::new(build_node(objects, begin, split, max_cap));
    let right = Box::new(build_node(objects, split, end, max_cap));
    // Build the node recursivelly
    Node {
        bounds: aabb,
        begin,
        end,
        kind: NodeEnum::Internal { left, right },
    }
}

/// Returns the index at which to split for SAH, the Axis along which to split, and the calculated
/// cost.
fn compute_sah<O: Intersected>(
    objects: &mut [O],
    surface: f32,
    max_cap: usize,
) -> (usize, Axis, f32) {
    // FIXME(Bruno): too imperative to my taste...
    let mut mid = objects.len() / 2;
    let mut dim = Axis::X; // Arbitrary split
    let mut min = std::f32::INFINITY;

    // Pre-allocate the vectors
    let mut left_surfaces = Vec::<f32>::with_capacity(objects.len() - 1);
    let mut right_surfaces = Vec::<f32>::with_capacity(objects.len() - 1);

    // For each axis compute the cost
    for &axis in [Axis::X, Axis::Y, Axis::Z].iter() {
        left_surfaces.clear();
        right_surfaces.clear();
        // Sort in order along the axis
        objects.sort_by(|lhs, rhs| {
            lhs.centroid()[axis]
                .partial_cmp(&rhs.centroid()[axis])
                .expect("Can't use NaNs in the SAH computation")
        });

        // Compute the surface for each possible split
        {
            let mut left_box = AABB::empty();
            let mut right_box = AABB::empty();
            for i in 0..(objects.len() - 1) {
                left_box.union_mut(&objects[i].aabb());
                left_surfaces.push(left_box.surface());

                right_box.union_mut(&objects[objects.len() - 1 - i].aabb());
                right_surfaces.push(right_box.surface());
            }
        }

        // Calculate the cost
        for left_count in 1..objects.len() {
            let right_count = objects.len() - left_count;

            let cost = 1. / max_cap as f32
                + (left_count as f32 * left_surfaces[left_count - 1]
                    + right_count as f32 * right_surfaces[right_count])
                    / surface;

            if cost < min {
                min = cost;
                dim = axis;
                mid = left_count
            }
        }
    }
    (mid, dim, min)
}
