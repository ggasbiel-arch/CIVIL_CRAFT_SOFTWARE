use crate::{Element, Project, Wall};
use serde::{Deserialize, Serialize};

type Point = [f64; 2];
const EPS_M: f64 = 1e-9;

#[derive(Clone, Copy, Default, Debug, PartialEq, Serialize, Deserialize)]
pub enum ReferenceAxis {
    #[default]
    Center,
    LeftFace,
    RightFace,
}
#[derive(Clone, Copy, Default, Debug, PartialEq, Serialize, Deserialize)]
pub enum LayerRole {
    #[default]
    Structure,
    Finish,
    Insulation,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WallLayer {
    pub material: String,
    pub role: LayerRole,
    pub thickness_m: f64,
    pub priority: i32,
}
impl WallLayer {
    pub fn base(thickness_m: f64) -> Self {
        Self {
            material: "Genérico".into(),
            role: LayerRole::Structure,
            thickness_m,
            priority: 0,
        }
    }
}
impl Wall {
    pub fn effective_layers(&self) -> Vec<WallLayer> {
        if self.layers.is_empty() {
            vec![WallLayer::base(self.thickness_m)]
        } else {
            self.layers.clone()
        }
    }
    pub(crate) fn validate_layers(&self) -> Result<(), String> {
        if !self.layers.is_empty()
            && (self.layers.iter().any(|l| {
                l.material.trim().is_empty() || !l.thickness_m.is_finite() || l.thickness_m <= 0.
            }) || (self.layers.iter().map(|l| l.thickness_m).sum::<f64>() - self.thickness_m)
                .abs()
                > EPS_M)
        {
            return Err(format!("Composição de camadas inválida em {}", self.id));
        }
        Ok(())
    }
    pub fn strip(&self, left: f64, right: f64, from: f64, to: f64) -> [Point; 4] {
        let length = self.length_m();
        let dx = (self.end[0] - self.start[0]) / length;
        let dy = (self.end[1] - self.start[1]) / length;
        let origin = self.offset_m
            + match self.axis {
                ReferenceAxis::Center => self.thickness_m / 2.,
                ReferenceAxis::LeftFace => 0.,
                ReferenceAxis::RightFace => self.thickness_m,
            };
        let point = |s, t| {
            [
                self.start[0] + dx * s - dy * (origin - t),
                self.start[1] + dy * s + dx * (origin - t),
            ]
        };
        [
            point(from, left),
            point(from, right),
            point(to, right),
            point(to, left),
        ]
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct WallFace {
    pub wall_id: String,
    pub layer: WallLayer,
    pub polygon: Vec<Point>,
}

#[derive(Default)]
pub struct WallPlanCache {
    strips: std::collections::BTreeMap<(String, usize), CachedStrip>,
}
struct CachedStrip {
    footprint: [Point; 4],
    cutters: Vec<[Point; 4]>,
    pieces: Vec<Vec<Point>>,
}
impl Project {
    pub fn set_level_elevation(&mut self, id: &str, elevation_m: f64) -> Result<bool, String> {
        let index = self
            .levels
            .iter()
            .position(|l| l.id == id)
            .ok_or("Pavimento inexistente")?;
        let old = self.levels[index].elevation_m;
        if elevation_m == old {
            return Ok(false);
        }
        self.levels[index].elevation_m = elevation_m;
        if let Err(error) = self.validate() {
            self.levels[index].elevation_m = old;
            return Err(error);
        }
        Ok(true)
    }

    pub fn wall_height(&self, wall: &Wall) -> Result<f64, String> {
        let Some(id) = &wall.top_level_id else {
            return Ok(wall.height_m);
        };
        let top = self
            .levels
            .iter()
            .find(|level| &level.id == id)
            .ok_or("Pavimento superior inexistente")?;
        let height = top.elevation_m - self.level_elevation(&wall.level_id);
        if !height.is_finite() || height <= 0. {
            return Err("O topo deve ficar acima da base".into());
        }
        Ok(height)
    }
    pub fn update_wall(&mut self, mut proposed: Wall) -> Result<bool, String> {
        if !proposed.layers.is_empty() {
            proposed.thickness_m = proposed.layers.iter().map(|l| l.thickness_m).sum();
        }
        crate::validate_wall(&proposed)?;
        self.wall_height(&proposed)?;
        let Some(Element::Wall(current)) = self.elements.iter_mut().find(|e| e.id() == proposed.id)
        else {
            return Err("Parede inexistente".into());
        };
        if current.level_id != proposed.level_id {
            return Err("O pavimento base deve ser preservado".into());
        }
        if *current == proposed {
            return Ok(false);
        }
        let previous = std::mem::replace(current, proposed);
        for placed in &self.component_instances {
            if placed
                .host
                .as_ref()
                .is_some_and(|h| h.wall_id == previous.id)
                && let Err(err) = self.door_geometry(placed)
            {
                let index = self
                    .elements
                    .iter()
                    .position(|e| e.id() == previous.id)
                    .unwrap();
                self.elements[index] = Element::Wall(previous);
                return Err(err);
            }
        }
        Ok(true)
    }
    pub fn wall_plan(&self, level: &str) -> Vec<WallFace> {
        WallPlanCache::default().update(self, level)
    }
}
impl WallPlanCache {
    pub fn update(&mut self, project: &Project, level: &str) -> Vec<WallFace> {
        let mut strips = Vec::new();
        for element in &project.elements {
            let Element::Wall(wall) = element else {
                continue;
            };
            if wall.level_id != level {
                continue;
            }
            let openings = project.openings(wall);
            let mut offset = 0.;
            let mut ordinal = 0;
            for layer in wall.effective_layers() {
                let next = offset + layer.thickness_m;
                let mut from = 0.;
                for to in openings
                    .iter()
                    .map(|o| (o.0, o.1))
                    .chain(std::iter::once((wall.length_m(), wall.length_m())))
                {
                    if to.0 - from > EPS_M {
                        strips.push((
                            wall.id.as_str(),
                            layer.clone(),
                            wall.strip(offset, next, from, to.0),
                            ordinal,
                        ));
                        ordinal += 1;
                    }
                    from = to.1;
                }
                offset = next;
            }
        }
        strips.sort_by(|a, b| b.1.priority.cmp(&a.1.priority).then(a.0.cmp(b.0)));
        let bounds: Vec<_> = strips
            .iter()
            .map(|(_, _, quad, _)| {
                let mut min = quad[0];
                let mut max = quad[0];
                for point in quad {
                    for axis in 0..2 {
                        min[axis] = min[axis].min(point[axis]);
                        max[axis] = max[axis].max(point[axis]);
                    }
                }
                (min, max)
            })
            .collect();
        let mut faces = Vec::new();
        let mut previous = std::mem::take(&mut self.strips);
        for (index, (id, layer, quad, ordinal)) in strips.iter().enumerate() {
            let cutters: Vec<_> = strips[..index]
                .iter()
                .enumerate()
                .filter(|(j, (other, _, _, _))| {
                    other != id
                        && (0..2).all(|axis| {
                            bounds[*j].0[axis] < bounds[index].1[axis]
                                && bounds[index].0[axis] < bounds[*j].1[axis]
                        })
                })
                .map(|(_, strip)| strip.2)
                .collect();
            let key = (id.to_string(), *ordinal);
            let cached = previous
                .remove(&key)
                .filter(|cached| cached.footprint == *quad && cached.cutters == cutters);
            let entry = self.strips.entry(key).or_insert_with(|| {
                cached.unwrap_or_else(|| {
                    let mut pieces = vec![quad.to_vec()];
                    for clip in &cutters {
                        pieces = pieces
                            .into_iter()
                            .flat_map(|piece| subtract_quad(piece, clip))
                            .collect();
                    }
                    CachedStrip {
                        footprint: *quad,
                        cutters,
                        pieces,
                    }
                })
            });
            faces.extend(entry.pieces.iter().cloned().map(|polygon| WallFace {
                wall_id: (*id).into(),
                layer: layer.clone(),
                polygon,
            }));
        }
        faces
    }
}

// The clip is a counterclockwise convex strip. Emit the outside of each
// half-plane once, carrying only the still-covered portion to the next edge.
fn subtract_quad(polygon: Vec<Point>, clip: &[Point; 4]) -> Vec<Vec<Point>> {
    let mut inside = polygon.clone();
    let mut pieces = Vec::new();
    for i in 0..4 {
        let a = clip[i];
        let b = clip[(i + 1) % 4];
        let length = (b[0] - a[0]).hypot(b[1] - a[1]);
        let side = |p: Point| {
            let d = ((b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])) / length;
            if d.abs() < EPS_M { 0. } else { d }
        };
        let mut keep = Vec::new();
        let mut outside = Vec::new();
        for j in 0..inside.len() {
            let p = inside[j];
            let q = inside[(j + 1) % inside.len()];
            let u = side(p);
            let v = side(q);
            if u >= 0. {
                keep.push(p);
            }
            if u <= 0. {
                outside.push(p);
            }
            if (u < 0. && v > 0.) || (u > 0. && v < 0.) {
                let t = u / (u - v);
                let cut = [p[0] + t * (q[0] - p[0]), p[1] + t * (q[1] - p[1])];
                keep.push(cut);
                outside.push(cut);
            }
        }
        if keep.len() < 3 || inside.iter().all(|&p| side(p) <= 0.) {
            return vec![polygon];
        }
        if outside.len() >= 3 && inside.iter().any(|&p| side(p) < 0.) {
            pieces.push(outside);
        }
        inside = keep;
    }
    pieces
}
