//! Projeto 2 — núcleo paramétrico inicial (não contém um kernel B-Rep).
pub mod components;
pub mod navigation;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub schema_version: u32,
    pub name: String,
    pub levels: Vec<Level>,
    pub elements: Vec<Element>,
    #[serde(default)]
    pub information: navigation::ProjectInformation,
    #[serde(default)]
    pub workspace: navigation::Workspace,
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

impl Element {
    pub fn id(&self) -> &str {
        match self { Self::Wall(w) => &w.id, Self::Slab(s) => &s.id }
    }
    pub fn level_id(&self) -> &str {
        match self { Self::Wall(w) => &w.level_id, Self::Slab(s) => &s.level_id }
    }
}

impl Wall {
    pub fn length_m(&self) -> f64 {
        (self.end[0] - self.start[0]).hypot(self.end[1] - self.start[1])
    }

}

impl Slab {
    pub fn area_m2(&self) -> f64 {
        let n = self.polygon.len();
        if n < 3 { return 0.0; }
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
            levels: vec![Level { id: "P0".into(), name: "Térreo".into(), elevation_m: 0.0 }],
            elements: vec![],
            information: navigation::ProjectInformation::default(),
            workspace: navigation::Workspace::new_project(&[
                Level { id: "P0".into(), name: "Térreo".into(), elevation_m: 0.0 }
            ]),
        }
    }
}

impl Project {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != SCHEMA_VERSION { return Err("Versão de projeto incompatível".into()); }
        if self.name.trim().is_empty() { return Err("Nome do projeto não pode ser vazio".into()); }
        let mut ids = HashSet::new();
        for level in &self.levels {
            if level.id.trim().is_empty() || level.name.trim().is_empty() || !level.elevation_m.is_finite() { return Err("Pavimento inválido".into()); }
            if !ids.insert(level.id.as_str()) { return Err("ID de pavimento repetido".into()); }
        }
        if self.levels.is_empty() { return Err("Projeto sem pavimentos".into()); }
        ids.clear();
        for element in &self.elements {
            if !ids.insert(element.id()) { return Err(format!("ID repetido: {}", element.id())); }
            if !self.levels.iter().any(|level| level.id == element.level_id()) { return Err(format!("Pavimento não encontrado: {}", element.level_id())); }
            match element { Element::Wall(w) => validate_wall(w)?, Element::Slab(s) => validate_slab(s)? }
        }
        self.workspace.validate(&self.levels)?;
        Ok(())
    }

}

fn valid_xy(point: [f64; 2]) -> bool { point.iter().all(|coord| coord.is_finite()) }
fn validate_wall(wall: &Wall) -> Result<(), String> {
    if wall.id.trim().is_empty() || !valid_xy(wall.start) || !valid_xy(wall.end)
        || !wall.thickness_m.is_finite() || wall.thickness_m <= 0.0
        || !wall.height_m.is_finite() || wall.height_m <= 0.0 || wall.length_m() < 0.01 {
        return Err(format!("Parede inválida: {}", wall.id));
    }
    Ok(())
}
fn validate_slab(slab: &Slab) -> Result<(), String> {
    if slab.id.trim().is_empty() || slab.polygon.len() < 3 || slab.polygon.iter().any(|p| !valid_xy(*p))
        || !slab.thickness_m.is_finite() || slab.thickness_m <= 0.0 || slab.area_m2() < 0.0001 {
        return Err(format!("Laje inválida: {}", slab.id));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_project_is_valid() {
        let project = Project::default();
        project.validate().unwrap();
        assert!(project.elements.is_empty());
    }

    #[test]
    fn empty_project_round_trip() {
        let project = Project::default();
        let json = serde_json::to_string(&project).unwrap();
        let restored: Project = serde_json::from_str(&json).unwrap();
        assert_eq!(project, restored);
    }

    #[test]
    fn duplicate_level_id_is_rejected() {
        let mut project = Project::default();
        project.levels.push(project.levels[0].clone());
        assert!(project.validate().is_err());
    }

    #[test]
    fn legacy_elements_remain_readable() {
        let json = r#"{"schema_version":1,"name":"Legacy","levels":[{"id":"P0","name":"Ground","elevation_m":0.0}],"elements":[{"type":"wall","id":"W1","level_id":"P0","start":[0.0,0.0],"end":[2.0,0.0],"thickness_m":0.2,"height_m":2.8}]}"#;
        let project: Project = serde_json::from_str(json).unwrap();
        project.validate().unwrap();
    }
}
