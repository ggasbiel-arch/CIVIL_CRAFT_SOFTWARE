use crate::{
    Project,
    components::{ComponentDefinition, ComponentInstance},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const WIDTH: &str = "largura_m";
const HEIGHT: &str = "altura_m";
const LEAF_WIDTH: &str = "largura_folha_m";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlacedComponent {
    pub level_id: String,
    #[serde(flatten)]
    pub instance: ComponentInstance,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DoorGeometry {
    pub width_m: f64,
    pub opening: [[f64; 3]; 2],
    pub leaf: [[f64; 3]; 4],
}

impl Project {
    pub(crate) fn definition_for(
        &self,
        placed: &PlacedComponent,
    ) -> Result<&ComponentDefinition, String> {
        if !self.levels.iter().any(|level| level.id == placed.level_id) {
            return Err(format!("Pavimento inexistente: {}", placed.level_id));
        }
        self.component_definitions
            .get(&placed.instance.definition_id)
            .ok_or_else(|| format!("Definição inexistente: {}", placed.instance.definition_id))
    }

    pub fn add_door(&mut self, level_id: &str, [x, y]: [f64; 2]) -> Result<String, String> {
        #[derive(Deserialize)]
        struct CatalogEntry {
            definition: ComponentDefinition,
        }
        let entry: CatalogEntry = serde_json::from_str(include_str!(
            "../../../samples/porta_parametrica.component.json"
        ))
        .map_err(|error| format!("Catálogo de portas inválido: {error}"))?;
        let definition = self
            .component_definitions
            .get(&entry.definition.id)
            .unwrap_or(&entry.definition);
        let id = self.next_id("D");
        let placed = PlacedComponent {
            level_id: level_id.into(),
            instance: ComponentInstance {
                id: id.clone(),
                definition_id: definition.id.clone(),
                overrides: BTreeMap::new(),
                transform: [1., 0., 0., x, 0., 1., 0., y, 0., 0., 1., 0., 0., 0., 0., 1.],
            },
        };
        let elevation = self
            .levels
            .iter()
            .find(|level| level.id == level_id)
            .ok_or("Pavimento inexistente")?
            .elevation_m;
        door_geometry(definition, &placed.instance, elevation)?;
        self.component_definitions
            .entry(entry.definition.id.clone())
            .or_insert(entry.definition);
        self.component_instances.push(placed);
        Ok(id)
    }

    pub fn door_geometry(&self, placed: &PlacedComponent) -> Result<DoorGeometry, String> {
        door_geometry(
            self.definition_for(placed)?,
            &placed.instance,
            self.level_elevation(&placed.level_id),
        )
    }

    pub fn set_door_width(&mut self, id: &str, width_m: f64) -> Result<bool, String> {
        let index = self
            .component_instances
            .iter()
            .position(|placed| placed.instance.id == id)
            .ok_or_else(|| format!("Instância inexistente: {id}"))?;
        let current = &self.component_instances[index];
        if self.door_geometry(current)?.width_m == width_m {
            return Ok(false);
        }
        let mut proposed = current.clone();
        proposed.instance.overrides.insert(WIDTH.into(), width_m);
        self.door_geometry(&proposed)?;
        self.component_instances[index] = proposed;
        Ok(true)
    }
}

fn door_geometry(
    definition: &ComponentDefinition,
    instance: &ComponentInstance,
    elevation: f64,
) -> Result<DoorGeometry, String> {
    if definition.classification != "IfcDoor" {
        return Err("O componente não é uma porta".into());
    }
    let outputs = definition.evaluate(instance)?;
    let length = |key| {
        let parameter = definition
            .parameters
            .get(key)
            .filter(|parameter| parameter.unit == "m")
            .ok_or_else(|| format!("Parâmetro de comprimento ausente: {key}"))?;
        Ok::<_, String>(
            instance
                .overrides
                .get(key)
                .copied()
                .unwrap_or(parameter.value),
        )
    };
    let width = length(WIDTH)?;
    let height = length(HEIGHT)?;
    let leaf_width = *outputs
        .get(LEAF_WIDTH)
        .ok_or("Saída de largura da folha ausente")?;
    if leaf_width <= 0.0 || leaf_width > width || height <= 0.0 {
        return Err("Dimensões da porta inválidas".into());
    }
    let gap = (width - leaf_width) / 2.0;
    let t = instance.transform;
    let point = |x: f64, z: f64| {
        [
            t[0] * x + t[2] * z + t[3],
            t[4] * x + t[6] * z + t[7],
            t[8] * x + t[10] * z + t[11] + elevation,
        ]
    };
    let opening = [point(0.0, 0.0), point(width, 0.0)];
    let leaf = [
        point(gap, 0.0),
        point(gap + leaf_width, 0.0),
        point(gap + leaf_width, height),
        point(gap, height),
    ];
    if opening
        .iter()
        .chain(&leaf)
        .flatten()
        .any(|coord| !coord.is_finite())
    {
        return Err("Coordenadas da porta inválidas".into());
    }
    Ok(DoorGeometry {
        width_m: width,
        opening,
        leaf,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: f64, expected: f64) {
        assert!((actual - expected).abs() < 1e-9);
    }

    #[test]
    fn doors_share_definition_and_regenerate_independently() {
        let mut project = Project::default();
        project.levels[0].elevation_m = 3.0;
        let first = project.add_door("P0", [2.0, 1.0]).unwrap();
        project.add_door("P0", [4.0, 1.0]).unwrap();
        let definitions = project.component_definitions.clone();

        assert!(project.set_door_width(&first, 1.2).unwrap());
        assert!(!project.set_door_width(&first, 1.2).unwrap());
        assert_eq!(definitions.len(), 1);
        assert_eq!(project.component_definitions, definitions);
        let first = project
            .door_geometry(&project.component_instances[0])
            .unwrap();
        assert_eq!(first.opening, [[2.0, 1.0, 3.0], [3.2, 1.0, 3.0]]);
        assert_close(first.leaf[1][0] - first.leaf[0][0], 1.16);
        assert_close(first.leaf[2][2] - first.leaf[1][2], 2.1);
        let second = project
            .door_geometry(&project.component_instances[1])
            .unwrap();
        assert_close(second.leaf[1][0] - second.leaf[0][0], 0.86);
        project.validate().unwrap();
    }

    #[test]
    fn invalid_edits_and_references_are_rejected() {
        let mut project = Project::default();
        assert!(project.add_door("missing", [0.0, 0.0]).is_err());
        assert!(project.component_definitions.is_empty());
        let id = project.add_door("P0", [0.0, 0.0]).unwrap();
        let before = project.clone();
        for invalid in [0.5, 2.1, f64::NAN, f64::INFINITY] {
            assert!(project.set_door_width(&id, invalid).is_err());
            assert_eq!(project, before);
        }
        project
            .component_instances
            .push(project.component_instances[0].clone());
        assert!(project.validate().is_err());
        project.component_instances.pop();
        project.component_definitions.clear();
        assert!(project.validate().is_err());
    }

    #[test]
    fn legacy_projects_open_without_component_tables() {
        let project: Project =
            serde_json::from_str(include_str!("../../../samples/casa_piloto.bim.json")).unwrap();
        project.validate().unwrap();
        assert_eq!(project.elements.len(), 5);
        assert!(project.component_definitions.is_empty());
        assert!(project.component_instances.is_empty());
    }
}
