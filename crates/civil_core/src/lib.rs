//! Projeto 2 — núcleo paramétrico inicial (não contém um kernel B-Rep).
pub mod components;
mod doors;
mod walls;
use components::ComponentDefinition;
pub use doors::{DoorGeometry, PlacedComponent};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::collections::HashSet;
pub use walls::{LayerRole, ReferenceAxis, WallFace, WallLayer, WallPlanCache};

pub const SCHEMA_VERSION: u32 = 2;

fn migrate_version<'de, D: serde::Deserializer<'de>>(reader: D) -> Result<u32, D::Error> {
    let version = u32::deserialize(reader)?;
    Ok(if version == 1 {
        SCHEMA_VERSION
    } else {
        version
    })
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Project {
    #[serde(deserialize_with = "migrate_version")]
    pub schema_version: u32,
    pub name: String,
    pub levels: Vec<Level>,
    pub elements: Vec<Element>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub component_definitions: BTreeMap<String, ComponentDefinition>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub component_instances: Vec<PlacedComponent>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Level {
    pub id: String,
    pub name: String,
    pub elevation_m: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Element {
    Wall(Wall),
    Slab(Slab),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Wall {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub layers: Vec<WallLayer>,
    #[serde(default)]
    pub axis: ReferenceAxis,
    #[serde(default)]
    pub offset_m: f64,
    #[serde(default)]
    pub top_level_id: Option<String>,
    pub id: String,
    pub level_id: String,
    pub start: [f64; 2],
    pub end: [f64; 2],
    pub thickness_m: f64,
    pub height_m: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Slab {
    pub id: String,
    pub level_id: String,
    pub polygon: Vec<[f64; 2]>,
    pub thickness_m: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
pub struct Quantities {
    pub hosted_opening_area_m2: f64,
    pub wall_length_m: f64,
    pub wall_gross_area_m2: f64,
    pub wall_gross_volume_m3: f64,
    pub slab_area_m2: f64,
    pub slab_volume_m3: f64,
}

impl Element {
    pub fn id(&self) -> &str {
        match self {
            Self::Wall(w) => &w.id,
            Self::Slab(s) => &s.id,
        }
    }
    pub fn level_id(&self) -> &str {
        match self {
            Self::Wall(w) => &w.level_id,
            Self::Slab(s) => &s.level_id,
        }
    }
}

impl Wall {
    pub fn length_m(&self) -> f64 {
        (self.end[0] - self.start[0]).hypot(self.end[1] - self.start[1])
    }
    pub fn footprint(&self) -> [[f64; 2]; 4] {
        self.strip(0., self.thickness_m, 0., self.length_m())
    }
}

impl Slab {
    pub fn area_m2(&self) -> f64 {
        let n = self.polygon.len();
        if n < 3 {
            return 0.0;
        }
        let mut twice_area = 0.0;
        for i in 0..n {
            let a = self.polygon[i];
            let b = self.polygon[(i + 1) % n];
            twice_area += a[0] * b[1] - b[0] * a[1];
        }
        twice_area.abs() * 0.5
    }
}

impl Default for Project {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            name: "Projeto sem título".into(),
            levels: vec![Level {
                id: "P0".into(),
                name: "Térreo".into(),
                elevation_m: 0.0,
            }],
            elements: vec![],
            component_definitions: BTreeMap::new(),
            component_instances: Vec::new(),
        }
    }
}

impl Project {
    pub fn example() -> Self {
        let mut project = Self {
            name: "Casa piloto — Projeto 2".into(),
            ..Self::default()
        };
        for (a, b) in [
            ([-4.0, -3.0], [4.0, -3.0]),
            ([4.0, -3.0], [4.0, 3.0]),
            ([4.0, 3.0], [-4.0, 3.0]),
            ([-4.0, 3.0], [-4.0, -3.0]),
        ] {
            project
                .add_wall("P0", a, b, 0.2, 2.8)
                .expect("valid example");
        }
        project
            .add_slab(
                "P0",
                vec![[-4.1, -3.1], [4.1, -3.1], [4.1, 3.1], [-4.1, 3.1]],
                0.15,
            )
            .expect("valid example");
        project
    }

    pub fn next_id(&self, prefix: &str) -> String {
        let mut n = 1_u64;
        loop {
            let id = format!("{prefix}{n}");
            if !self.elements.iter().any(|element| element.id() == id)
                && !self
                    .component_instances
                    .iter()
                    .any(|placed| placed.instance.id == id)
            {
                return id;
            }
            n += 1;
        }
    }

    pub fn add_wall(
        &mut self,
        level_id: &str,
        start: [f64; 2],
        end: [f64; 2],
        thickness_m: f64,
        height_m: f64,
    ) -> Result<String, String> {
        if !self.levels.iter().any(|level| level.id == level_id) {
            return Err("Pavimento inexistente".into());
        }
        let wall = Wall {
            id: self.next_id("W"),
            level_id: level_id.to_owned(),
            layers: Vec::new(),
            axis: ReferenceAxis::Center,
            offset_m: 0.,
            top_level_id: None,
            start,
            end,
            thickness_m,
            height_m,
        };
        validate_wall(&wall)?;
        let id = wall.id.clone();
        self.elements.push(Element::Wall(wall));
        Ok(id)
    }

    pub fn add_slab(
        &mut self,
        level_id: &str,
        polygon: Vec<[f64; 2]>,
        thickness_m: f64,
    ) -> Result<String, String> {
        if !self.levels.iter().any(|level| level.id == level_id) {
            return Err("Pavimento inexistente".into());
        }
        let slab = Slab {
            id: self.next_id("S"),
            level_id: level_id.to_owned(),
            polygon,
            thickness_m,
        };
        validate_slab(&slab)?;
        let id = slab.id.clone();
        self.elements.push(Element::Slab(slab));
        Ok(id)
    }

    pub fn level_elevation(&self, level_id: &str) -> f64 {
        self.levels
            .iter()
            .find(|level| level.id == level_id)
            .map_or(0.0, |level| level.elevation_m)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != SCHEMA_VERSION {
            return Err("Versão de projeto incompatível".into());
        }
        if self.name.trim().is_empty() {
            return Err("Nome do projeto não pode ser vazio".into());
        }
        let mut ids = HashSet::new();
        for level in &self.levels {
            if level.id.trim().is_empty()
                || level.name.trim().is_empty()
                || !level.elevation_m.is_finite()
            {
                return Err("Pavimento inválido".into());
            }
            if !ids.insert(level.id.as_str()) {
                return Err("ID de pavimento repetido".into());
            }
        }
        if self.levels.is_empty() {
            return Err("Projeto sem pavimentos".into());
        }
        ids.clear();
        for element in &self.elements {
            if !ids.insert(element.id()) {
                return Err(format!("ID repetido: {}", element.id()));
            }
            if !self
                .levels
                .iter()
                .any(|level| level.id == element.level_id())
            {
                return Err(format!("Pavimento não encontrado: {}", element.level_id()));
            }
            match element {
                Element::Wall(w) => {
                    validate_wall(w)?;
                    self.wall_height(w)?;
                }
                Element::Slab(s) => validate_slab(s)?,
            }
        }
        for (id, definition) in &self.component_definitions {
            if id.trim().is_empty() || id != &definition.id {
                return Err(format!("ID de definição inválido: {id}"));
            }
            for (key, parameter) in &definition.parameters {
                parameter.validate(key)?;
            }
        }
        for placed in &self.component_instances {
            let id = &placed.instance.id;
            if id.trim().is_empty() || !ids.insert(id) {
                return Err(format!("ID de instância inválido ou repetido: {id}"));
            }
            let definition = self.definition_for(placed)?;
            if placed.host.is_some()
                || matches!(definition.classification.as_str(), "IfcDoor" | "IfcWindow")
            {
                self.door_geometry(placed)?;
            } else if definition.graph.is_some() {
                definition.evaluate(&placed.instance)?;
            } else {
                definition.validate_instance(&placed.instance)?;
            }
        }
        Ok(())
    }

    pub fn quantities(&self) -> Quantities {
        let mut result = Quantities::default();
        for element in &self.elements {
            match element {
                Element::Wall(wall) => {
                    let length = wall.length_m();
                    result.wall_length_m += length;
                    result.wall_gross_area_m2 +=
                        length * self.wall_height(wall).unwrap_or(wall.height_m);
                    result.wall_gross_volume_m3 +=
                        length * self.wall_height(wall).unwrap_or(wall.height_m) * wall.thickness_m;
                }
                Element::Slab(slab) => {
                    let area = slab.area_m2();
                    result.slab_area_m2 += area;
                    result.slab_volume_m3 += area * slab.thickness_m;
                }
            }
        }
        for placed in &self.component_instances {
            if placed.host.is_some()
                && let Ok(g) = self.door_geometry(placed)
            {
                result.hosted_opening_area_m2 += g.width_m * (g.leaf[2][2] - g.leaf[0][2]);
            }
        }
        result
    }
}

fn valid_xy(point: [f64; 2]) -> bool {
    point.iter().all(|coord| coord.is_finite())
}
fn validate_wall(wall: &Wall) -> Result<(), String> {
    wall.validate_layers()?;
    if !wall.length_m().is_finite()
        || !wall.offset_m.is_finite()
        || wall.id.trim().is_empty()
        || !valid_xy(wall.start)
        || !valid_xy(wall.end)
        || !wall.thickness_m.is_finite()
        || wall.thickness_m <= 0.0
        || !wall.height_m.is_finite()
        || wall.height_m <= 0.0
        || wall.length_m() < 0.01
    {
        return Err(format!("Parede inválida: {}", wall.id));
    }
    Ok(())
}
fn validate_slab(slab: &Slab) -> Result<(), String> {
    if slab.id.trim().is_empty()
        || slab.polygon.len() < 3
        || slab.polygon.iter().any(|p| !valid_xy(*p))
        || !slab.thickness_m.is_finite()
        || slab.thickness_m <= 0.0
        || slab.area_m2() < 0.0001
    {
        return Err(format!("Laje inválida: {}", slab.id));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn example_is_valid_and_has_quantities() {
        let project = Project::example();
        project.validate().unwrap();
        let q = project.quantities();
        assert!((q.wall_length_m - 28.0).abs() < 1e-8);
        assert!((q.wall_gross_volume_m3 - 15.68).abs() < 1e-8);
        assert!(q.slab_area_m2 > 50.0);
    }
    #[test]
    fn zero_length_wall_is_rejected() {
        let mut project = Project::default();
        assert!(
            project
                .add_wall("P0", [0.0, 0.0], [0.0, 0.0], 0.15, 2.8)
                .is_err()
        );
    }
    #[test]
    fn serde_round_trip() {
        let project = Project::example();
        let json = serde_json::to_string(&project).unwrap();
        let restored: Project = serde_json::from_str(&json).unwrap();
        assert_eq!(project, restored);
    }
    #[test]
    fn duplicated_ids_are_invalid() {
        let mut project = Project::example();
        project.elements.push(project.elements[0].clone());
        assert!(project.validate().is_err());
    }
}
