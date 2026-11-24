//! Camera, navigator, saved view and publishing data for the independent CAD/BIM desktop.
//! Projection and selection are pure Rust and do not depend on the UI renderer.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

use crate::Level;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionMode { Orthographic, Perspective, TwoPoint }
impl ProjectionMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Orthographic => "Ortogonal",
            Self::Perspective => "Perspectiva",
            Self::TwoPoint => "Perspectiva de dois pontos",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Camera {
    pub projection: ProjectionMode,
    pub target: [f64; 3],
    pub yaw_deg: f64,
    pub pitch_deg: f64,
    pub distance_m: f64,
    pub ortho_height_m: f64,
    pub fov_y_deg: f64,
}
impl Default for Camera {
    fn default() -> Self {
        Self {
            projection: ProjectionMode::Perspective,
            target: [0.0, 0.0, 1.0],
            yaw_deg: 35.0,
            pitch_deg: -28.0,
            distance_m: 18.0,
            ortho_height_m: 16.0,
            fov_y_deg: 55.0,
        }
    }
}
type Vec3 = [f64; 3];
fn add(a: Vec3, b: Vec3) -> Vec3 { [a[0]+b[0],a[1]+b[1],a[2]+b[2]] }
fn sub(a: Vec3, b: Vec3) -> Vec3 { [a[0]-b[0],a[1]-b[1],a[2]-b[2]] }
fn mul(a: Vec3, k: f64) -> Vec3 { [a[0]*k,a[1]*k,a[2]*k] }
fn dot(a: Vec3, b: Vec3) -> f64 { a[0]*b[0]+a[1]*b[1]+a[2]*b[2] }

impl Camera {
    pub fn validate(&self) -> Result<(), String> {
        if !self.target.iter().all(|n| n.is_finite())
            || ![self.yaw_deg,self.pitch_deg,self.distance_m,self.ortho_height_m,self.fov_y_deg]
                .iter().all(|n| n.is_finite())
            || self.distance_m <= 0.01
            || self.ortho_height_m <= 0.01
            || !(10.0..=140.0).contains(&self.fov_y_deg)
            || self.pitch_deg.abs() >= 89.9 {
            return Err("Configuração de câmera inválida".into());
        }
        Ok(())
    }
    pub fn orbit(&mut self, dx_px: f64, dy_px: f64) {
        self.yaw_deg = (self.yaw_deg + dx_px * 0.35).rem_euclid(360.0);
        if self.projection != ProjectionMode::TwoPoint {
            self.pitch_deg = (self.pitch_deg + dy_px * 0.30).clamp(-85.0, 85.0);
        }
    }
    pub fn zoom(&mut self, scroll: f64) {
        let factor = (-scroll * 0.10).clamp(-5.0, 5.0).exp();
        if self.projection == ProjectionMode::Orthographic {
            self.ortho_height_m = (self.ortho_height_m * factor).clamp(0.05, 100_000.0);
        } else {
            self.distance_m = (self.distance_m * factor).clamp(0.05, 100_000.0);
        }
    }
    pub fn axes(&self) -> (Vec3, Vec3, Vec3, Vec3) {
        let yaw = self.yaw_deg.to_radians();
        // Two-point perspective keeps verticals parallel: optical axis horizontal.
        let pitch = if self.projection == ProjectionMode::TwoPoint {
            0.0
        } else { self.pitch_deg.to_radians() };
        let forward = [yaw.sin()*pitch.cos(), yaw.cos()*pitch.cos(), pitch.sin()];
        let right = [yaw.cos(), -yaw.sin(), 0.0];
        let up = [-yaw.sin()*pitch.sin(), -yaw.cos()*pitch.sin(), pitch.cos()];
        let eye = sub(self.target, mul(forward, self.distance_m));
        (eye, forward, right, up)
    }
    /// Returns screen coordinates in pixels, origin at top left. Points behind camera are hidden.
    pub fn project(&self, point: Vec3, width: f64, height: f64) -> Option<[f64; 2]> {
        if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0
            || !point.iter().all(|v| v.is_finite()) {
            return None;
        }
        let (eye, forward, right, up) = self.axes();
        let relative = sub(point, eye);
        let x = dot(relative, right);
        let y = dot(relative, up);
        let (sx, sy) = if self.projection == ProjectionMode::Orthographic {
            let scale = height / self.ortho_height_m;
            (x * scale, y * scale)
        } else {
            let depth = dot(relative, forward);
            if depth < 0.01 { return None; }
            let focal = height / (2.0 * (self.fov_y_deg.to_radians() * 0.5).tan());
            (focal * x / depth, focal * y / depth)
        };
        Some([width * 0.5 + sx, height * 0.5 - sy])
    }
    pub fn pan(&mut self, dx_px: f64, dy_px: f64, viewport_height: f64) {
        if viewport_height <= 0.0 { return; }
        let (_, _, right, up) = self.axes();
        let units_per_px = if self.projection == ProjectionMode::Orthographic {
            self.ortho_height_m / viewport_height
        } else {
            2.0 * self.distance_m * (self.fov_y_deg.to_radians() * 0.5).tan()
                / viewport_height
        };
        self.target = add(self.target, add(
            mul(right, -dx_px * units_per_px),
            mul(up, dy_px * units_per_px),
        ));
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionMode { Window, Crossing }
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScreenBounds { pub min: [f64; 2], pub max: [f64; 2] }
impl ScreenBounds {
    pub fn from_points(a: [f64; 2], b: [f64; 2]) -> Self {
        Self {
            min: [a[0].min(b[0]), a[1].min(b[1])],
            max: [a[0].max(b[0]), a[1].max(b[1])],
        }
    }
    pub fn contains(self, b: Self) -> bool {
        b.min[0] >= self.min[0] && b.max[0] <= self.max[0]
        && b.min[1] >= self.min[1] && b.max[1] <= self.max[1]
    }
    pub fn intersects(self, b: Self) -> bool {
        self.min[0] <= b.max[0] && self.max[0] >= b.min[0]
        && self.min[1] <= b.max[1] && self.max[1] >= b.min[1]
    }
}
pub fn selection_mode(start_x: f64, end_x: f64) -> SelectionMode {
    if end_x >= start_x { SelectionMode::Window } else { SelectionMode::Crossing }
}
pub fn select_projected<'a>(
    start: [f64; 2],
    end: [f64; 2],
    candidates: impl IntoIterator<Item = (&'a str, ScreenBounds)>,
) -> Vec<String> {
    let rect = ScreenBounds::from_points(start, end);
    let mode = selection_mode(start[0], end[0]);
    candidates.into_iter().filter_map(|(id, bounds)| {
        let matches = match mode {
            SelectionMode::Window => rect.contains(bounds),
            SelectionMode::Crossing => rect.intersects(bounds),
        };
        matches.then(|| id.to_owned())
    }).collect()
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProjectInformation {
    pub project_code: String,
    pub address: String,
    pub client: String,
    pub architect: String,
    pub engineering_company: String,
    pub phase: String,
    pub description: String,
    pub revision: String,
    pub custom: BTreeMap<String, String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViewpointKind { ThreeD, FloorPlan, Section, Elevation, Detail }
impl ViewpointKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::ThreeD=>"3D", Self::FloorPlan=>"Planta",
            Self::Section=>"Corte", Self::Elevation=>"Elevação", Self::Detail=>"Detalhe",
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Viewpoint {
    pub id: String,
    pub title: String,
    pub kind: ViewpointKind,
    pub level_id: Option<String>,
    pub camera: Camera,
    /// Location of 2D marker in world coordinates, used for navigator selection.
    pub marker: [f64; 3],
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SavedView {
    pub id: String,
    pub title: String,
    pub viewpoint_id: String,
    pub camera: Camera,
    pub scale: String,
    #[serde(default)]
    pub layer_preset: String,
    #[serde(default)]
    pub notes: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleKind { Levels, Elements, Views }
impl ScheduleKind {
    pub fn label(self)-> &'static str {
        match self { Self::Levels=>"Pavimentos",Self::Elements=>"Elementos",Self::Views=>"Vistas" }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Schedule {
    pub id: String,
    pub title: String,
    pub kind: ScheduleKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PublishFormat { Json, Csv }
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PublishSource { SavedView, Schedule }
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PublishItem {
    pub id: String,
    pub source: PublishSource,
    pub source_id: String,
    pub format: PublishFormat,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PublisherSet {
    pub id: String,
    pub title: String,
    pub items: Vec<PublishItem>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Workspace {
    pub camera: Camera,
    pub viewpoints: Vec<Viewpoint>,
    pub saved_views: Vec<SavedView>,
    pub schedules: Vec<Schedule>,
    pub publisher_sets: Vec<PublisherSet>,
    pub next_id: u64,
}
impl Default for Workspace {
    fn default() -> Self {
        Self { camera: Camera::default(), viewpoints: vec![], saved_views: vec![],
            schedules: vec![], publisher_sets: vec![], next_id: 1 }
    }
}
impl Workspace {
    pub fn new_project(levels: &[Level]) -> Self {
        let mut w = Self::default();
        w.new_viewpoint("Vista 3D".into(), ViewpointKind::ThreeD, None);
        for level in levels {
            w.new_viewpoint(format!("Planta — {}", level.name), ViewpointKind::FloorPlan, Some(level.id.clone()));
        }
        w
    }
    pub fn allocate_id(&mut self, prefix: &str) -> String {
        let id = format!("{prefix}-{}", self.next_id);
        self.next_id += 1;
        id
    }
    pub fn new_viewpoint(&mut self, title: String, kind: ViewpointKind, level_id: Option<String>) -> String {
        let id = self.allocate_id("vp");
        let mut camera = self.camera.clone();
        if kind == ViewpointKind::FloorPlan {
            camera.projection = ProjectionMode::Orthographic;
            camera.yaw_deg = 0.0;
            camera.pitch_deg = -85.0; // approximately top-down; exact plan renderer follows later
        }
        self.viewpoints.push(Viewpoint {
            id: id.clone(), title, kind, level_id,
            camera, marker: [0.0, 0.0, 0.0],
        });
        id
    }
    pub fn save_view(&mut self, viewpoint_id: &str, title: String, camera: Camera) -> Result<String, String> {
        if !self.viewpoints.iter().any(|v| v.id == viewpoint_id) {
            return Err("Ponto de vista inexistente".into());
        }
        let id = self.allocate_id("view");
        self.saved_views.push(SavedView {
            id: id.clone(), title, viewpoint_id: viewpoint_id.into(), camera,
            scale: "1:100".into(), layer_preset: "Todas as camadas".into(), notes: String::new(),
        });
        Ok(id)
    }
    pub fn validate(&self, levels: &[Level]) -> Result<(), String> {
        self.camera.validate()?;
        let mut ids = HashSet::new();
        for v in &self.viewpoints {
            v.camera.validate()?;
            if v.id.trim().is_empty() || v.title.trim().is_empty() || !ids.insert(v.id.as_str())
                || !v.marker.iter().all(|x| x.is_finite()) {
                return Err("Ponto de vista inválido ou repetido".into());
            }
            if let Some(level_id) = &v.level_id {
                if !levels.iter().any(|level| &level.id == level_id) {
                    return Err(format!("Pavimento de vista inexistente: {level_id}"));
                }
            }
        }
        ids.clear();
        for v in &self.saved_views {
            v.camera.validate()?;
            if v.id.trim().is_empty() || v.title.trim().is_empty() || !ids.insert(v.id.as_str())
                || !self.viewpoints.iter().any(|p| p.id == v.viewpoint_id) {
                return Err("Vista salva inválida ou sem origem".into());
            }
        }
        ids.clear();
        for schedule in &self.schedules {
            if schedule.id.trim().is_empty() || schedule.title.trim().is_empty()
                || !ids.insert(schedule.id.as_str()) { return Err("Tabela inválida".into()); }
        }
        ids.clear();
        for set in &self.publisher_sets {
            if set.id.trim().is_empty() || set.title.trim().is_empty()
                || !ids.insert(set.id.as_str()) { return Err("Conjunto publicador inválido".into()); }
            let mut item_ids = HashSet::new();
            for item in &set.items {
                if item.id.is_empty() || !item_ids.insert(item.id.as_str()) {
                    return Err("Item do publicador inválido".into());
                }
                let found = match item.source {
                    PublishSource::SavedView => self.saved_views.iter().any(|v| v.id == item.source_id),
                    PublishSource::Schedule => self.schedules.iter().any(|s| s.id == item.source_id),
                };
                if !found { return Err("Fonte do publicador não encontrada".into()); }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projection_modes_have_finite_positions() {
        for mode in [ProjectionMode::Orthographic,ProjectionMode::Perspective,ProjectionMode::TwoPoint] {
            let c = Camera { projection: mode, ..Camera::default() };
            c.validate().unwrap();
            assert!(c.project([0.0,0.0,0.0],800.0,600.0).is_some());
        }
    }

    #[test]
    fn two_point_verticals_remain_parallel() {
        let c = Camera { projection: ProjectionMode::TwoPoint, pitch_deg: -40.0, ..Camera::default() };
        let p = c.project([0.0,0.0,0.0],800.0,600.0).unwrap();
        let p2 = c.project([0.0,0.0,4.0],800.0,600.0).unwrap();
        assert!((p[0]-p2[0]).abs() < 1e-9);
    }

    #[test]
    fn different_drag_directions_select_differently() {
        let objects = vec![
            ("inside", ScreenBounds::from_points([20.0,20.0],[40.0,40.0])),
            ("crossing", ScreenBounds::from_points([90.0,20.0],[110.0,40.0]))
        ];
        let selected = select_projected([10.0,10.0],[100.0,100.0],objects.iter().map(|(id,b)|(*id,*b)));
        assert_eq!(selected,vec!["inside"]);
        let selected = select_projected([100.0,100.0],[10.0,10.0],objects.iter().map(|(id,b)|(*id,*b)));
        assert_eq!(selected,vec!["inside","crossing"]);
    }

    #[test]
    fn saved_view_restores_camera_after_serialization() {
        let levels = vec![Level{id:"P0".into(), name:"Térreo".into(),elevation_m:0.0}];
        let mut w = Workspace::new_project(&levels);
        let source = w.viewpoints[0].id.clone();
        let mut cam = w.camera.clone();
        cam.projection = ProjectionMode::TwoPoint;
        let id = w.save_view(&source,"Fachada".into(),cam.clone()).unwrap();
        let loaded: Workspace = serde_json::from_str(&serde_json::to_string(&w).unwrap()).unwrap();
        assert_eq!(loaded.saved_views.iter().find(|v|v.id==id).unwrap().camera,cam);
        loaded.validate(&levels).unwrap();
    }

    #[test]
    fn broken_publisher_reference_is_rejected() {
        let levels = vec![Level{id:"P0".into(), name:"Térreo".into(),elevation_m:0.0}];
        let mut w = Workspace::new_project(&levels);
        w.publisher_sets.push(PublisherSet {
            id:"pub-1".into(),title:"Emissão".into(),
            items:vec![PublishItem{id:"i1".into(),source:PublishSource::SavedView,
            source_id:"missing".into(),format:PublishFormat::Json}],
        });
        assert!(w.validate(&levels).is_err());
    }
}
