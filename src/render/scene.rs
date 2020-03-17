use super::{light_aggregate::LightAggregate, object::Object};
use crate::core::Camera;
use crate::core::LinearColor;
use crate::{Point, Vector};
use bvh::ray::Ray;
use image::RgbImage;
use rand::prelude::thread_rng;
use rand::Rng;

/// Represent the scene being rendered.
pub struct Scene<'a> {
    camera: Camera,
    lights: LightAggregate,
    objects: Vec<Object<'a>>,
    aliasing_limit: u32,
}

impl<'a> Scene<'a> {
    pub fn new(
        camera: Camera,
        lights: LightAggregate,
        objects: Vec<Object<'a>>,
        aliasing_limit: u32,
        reflection_limit: u32,
    ) -> Self {
        Scene {
            camera,
            lights,
            objects,
            aliasing_limit,
            reflection_limit,
        }
    }

    /// Render the scene into an image.
    pub fn render(&self) -> RgbImage {
        let mut image = RgbImage::new(self.camera.film().width(), self.camera.film().height());
        let pixel_func = if self.aliasing_limit > 0 {
            Self::anti_alias_pixel
        } else {
            Self::pixel
        };
        for (x, y, pixel) in image.enumerate_pixels_mut() {
            *pixel = pixel_func(&self, x as f32, y as f32).into()
        }
        image
    }

    /// Get pixel color for (x, y) a pixel **coordinate**
    fn pixel(&self, x: f32, y: f32) -> LinearColor {
        let (x, y) = self.camera.film().pixel_ratio(x, y);
        let pixel = self.camera.film().pixel_at_ratio(x, y);
        let direction = (pixel - self.camera.origin()).normalize();
        self.cast_ray(Ray::new(pixel, direction))
            .map_or_else(LinearColor::black, |(t, obj)| {
                self.color_at(pixel + direction * t, obj, direction, self.reflection_limit)
            })
    }

    /// Get pixel color with anti-aliasing
    fn anti_alias_pixel(&self, x: f32, y: f32) -> LinearColor {
        let range = 0..self.aliasing_limit;
        let mut rng = thread_rng();
        let acc: LinearColor = range
            .map(|_| {
                let random_x: f32 = rng.gen();
                let random_y: f32 = rng.gen();
                self.pixel(x + random_x, y + random_y)
            })
            .sum();
        acc / self.aliasing_limit as f32
    }

    fn cast_ray(&self, ray: Ray) -> Option<(f32, &Object)> {
        // NOTE(Bruno): should be written using iterators
        let mut shot_obj: Option<&Object> = None;
        let mut t = std::f32::INFINITY;
        for object in self.objects.iter() {
            match object.shape.intersect(&ray) {
                Some(dist) if dist < t => {
                    t = dist;
                    shot_obj = Some(&object);
                }
                _ => {}
            }
        }
        shot_obj.map(|obj| (t, obj))
    }

    fn color_at(&self, point: Point, object: &Object, incident_ray: Vector) -> LinearColor {
        self.illuminate(point, object, incident_ray)
        // FIXME: add reflection
    }

    fn illuminate(&self, point: Point, object: &Object, incident_ray: Vector) -> LinearColor {
        let texel = object.shape.project_texel(&point);
        let normal = object.shape.normal(&point);
        let reflected = reflected(incident_ray, normal);

        self.illuminate_ambient(object.texture.texel_color(texel))
            + self.illuminate_spatial(point.clone(), object, normal, reflected)
    }

    fn illuminate_ambient(&self, color: LinearColor) -> LinearColor {
        self.lights
            .ambient_lights_iter()
            .map(|light| color.clone() * light.illumination(&Point::origin()))
            .sum()
    }

    fn illuminate_spatial(
        &self,
        point: Point,
        object: &Object,
        normal: Vector,
        reflected: Vector,
    ) -> LinearColor {
        let texel = object.shape.project_texel(&point);
        let k_d = object.material.diffuse(texel);
        let k_s = object.material.specular(texel);
        self.lights
            .spatial_lights_iter()
            .map(|light| {
                let (direction, t) = light.to_source(&point);
                let light_ray = Ray::new(point + 0.001 * direction, direction);
                match self.cast_ray(light_ray) {
                    // Take shadows into account
                    Some((obstacle_t, _)) if obstacle_t < t => return LinearColor::black(),
                    _ => {}
                }
                let lum = light.illumination(&point);
                let diffused = k_d.clone() * normal.dot(&direction);
                let specular = k_s.clone() * reflected.dot(&direction);
                lum * (diffused + specular)
            })
            .sum()
    }
}

fn reflected(incident: Vector, normal: Vector) -> Vector {
    let proj = incident.dot(&normal);
    let delt = normal * (proj * 2.);
    incident - delt
}
