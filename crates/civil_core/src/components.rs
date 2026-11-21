//! Protótipo do núcleo de componentes BIM e grafo paramétrico.
//! Não lê SKP/RFA/GSM/IFC e não possui kernel geométrico.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImportProvenance {
    pub source_application: String,
    pub source_format: String,
    pub source_identifier: Option<String>,
    pub source_version: Option<String>,
    #[serde(default)]
    pub original_attributes: BTreeMap<String, Value>,
    #[serde(default)]
    pub conversion_warnings: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Parameter {
    pub value: f64,
    pub unit: String,
    pub min: Option<f64>,
    pub max: Option<f64>,
    #[serde(default)]
    pub label: String,
}
impl Parameter {
    pub fn validate(&self, key: &str) -> Result<(), String> {
        if !self.value.is_finite() { return Err(format!("{key}: valor não finito")); }
        if let Some(min) = self.min {
            if !min.is_finite() || self.value < min { return Err(format!("{key}: abaixo do mínimo")); }
        }
        if let Some(max) = self.max {
            if !max.is_finite() || self.value > max { return Err(format!("{key}: acima do máximo")); }
        }
        if let (Some(min), Some(max)) = (self.min, self.max) {
            if min > max { return Err(format!("{key}: intervalo inválido")); }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComponentDefinition {
    pub id: String,
    pub name: String,
    pub classification: String,
    pub provenance: Option<ImportProvenance>,
    pub parameters: BTreeMap<String, Parameter>,
    #[serde(default)]
    pub property_sets: BTreeMap<String, BTreeMap<String, Value>>,
    pub graph: Option<ParametricGraph>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComponentInstance {
    pub id: String,
    pub definition_id: String,
    #[serde(default)]
    pub overrides: BTreeMap<String, f64>,
    /// Matriz de transformação 4x4 em ordem de linhas, homogênea.
    pub transform: [f64; 16],
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ParametricGraph {
    pub nodes: Vec<GraphNode>,
    pub outputs: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    #[serde(flatten)]
    pub kind: NodeKind,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum NodeKind {
    Input { parameter: String },
    Constant { value: f64 },
    Add { a: String, b: String },
    Subtract { a: String, b: String },
    Multiply { a: String, b: String },
    Divide { a: String, b: String },
    Min { a: String, b: String },
    Max { a: String, b: String },
    Clamp { value: String, min: String, max: String },
}

impl ComponentDefinition {
    /// Avalia um grafo numérico de modo determinístico, sem execução arbitrária de código.
    /// No futuro, avaliação dimensional + geração de geometria ficarão no motor Rust.
    pub fn evaluate(&self, instance: &ComponentInstance) -> Result<BTreeMap<String, f64>, String> {
        if self.id.is_empty() || instance.definition_id != self.id {
            return Err("Definição da instância não corresponde ao componente".into());
        }
        if instance.transform.iter().any(|x| !x.is_finite()) {
            return Err("Transformação contém valores inválidos".into());
        }
        let graph = self.graph.as_ref().ok_or("Componente sem grafo paramétrico")?;
        if graph.nodes.len() > 4096 { return Err("Grafo grande demais".into()); }
        let mut inputs = BTreeMap::new();
        for (key, parameter) in &self.parameters {
            parameter.validate(key)?;
            inputs.insert(key.as_str(), parameter.value);
        }
        for (key, value) in &instance.overrides {
            let Some(parameter) = self.parameters.get(key) else {
                return Err(format!("Parâmetro desconhecido: {key}"));
            };
            let candidate = Parameter { value: *value, ..parameter.clone() };
            candidate.validate(key)?;
            inputs.insert(key.as_str(), *value);
        }
        let mut map = HashMap::new();
        for node in &graph.nodes {
            if node.id.trim().is_empty() || map.insert(node.id.as_str(), &node.kind).is_some() {
                return Err("Nós vazios ou repetidos".into());
            }
        }
        let mut cache = HashMap::new();
        let mut active = HashSet::new();
        let mut outputs = BTreeMap::new();
        for (key, node_id) in &graph.outputs {
            let evaluated = eval_node(node_id, &map, &inputs, &mut cache, &mut active, 0)?;
            outputs.insert(key.clone(), evaluated);
        }
        Ok(outputs)
    }
}

fn eval_node<'a>(
    id: &'a str,
    nodes: &HashMap<&'a str, &'a NodeKind>,
    inputs: &BTreeMap<&'a str, f64>,
    cache: &mut HashMap<&'a str, f64>,
    active: &mut HashSet<&'a str>,
    depth: usize,
) -> Result<f64, String> {
    if let Some(value) = cache.get(id) { return Ok(*value); }
    if depth > 256 { return Err("Limite de profundidade de grafo atingido".into()); }
    if !active.insert(id) { return Err(format!("Ciclo detectado em {id}")); }
    let kind = nodes.get(id).ok_or_else(|| format!("Nó inexistente: {id}"))?;
    let mut read = |key: &'a str| eval_node(key, nodes, inputs, cache, active, depth + 1);
    let result = match kind {
        NodeKind::Input { parameter } => inputs.get(parameter.as_str()).copied()
            .ok_or_else(|| format!("Entrada desconhecida: {parameter}")),
        NodeKind::Constant { value } => Ok(*value),
        NodeKind::Add { a, b } => { let va = read(a)?; Ok(va + read(b)?) },
        NodeKind::Subtract { a, b } => { let va = read(a)?; Ok(va - read(b)?) },
        NodeKind::Multiply { a, b } => { let va = read(a)?; Ok(va * read(b)?) },
        NodeKind::Divide { a, b } => {
            let va = read(a)?;
            let vb = read(b)?;
            if vb.abs() < 1e-12 { Err("Divisão por zero".into()) } else { Ok(va / vb) }
        },
        NodeKind::Min { a, b } => { let va = read(a)?; Ok(va.min(read(b)?)) },
        NodeKind::Max { a, b } => { let va = read(a)?; Ok(va.max(read(b)?)) },
        NodeKind::Clamp { value, min, max } => {
            let v = read(value)?;
            let lower = read(min)?;
            let upper = read(max)?;
            if lower > upper { Err("Clamp com intervalo inválido".into()) }
            else { Ok(v.clamp(lower, upper)) }
        }
    };
    active.remove(id);
    let value = result?;
    if !value.is_finite() { return Err(format!("Resultado inválido: {id}")); }
    cache.insert(id, value);
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn component() -> (ComponentDefinition, ComponentInstance) {
        let mut parameters = BTreeMap::new();
        parameters.insert("width".into(), Parameter { value: 0.90, unit: "m".into(), min: Some(0.60), max: Some(2.0), label: "Largura".into() });
        let definition = ComponentDefinition {
            id: "PORTA-01".into(), name: "Porta dinâmica".into(), classification: "IfcDoor".into(),
            provenance: None, parameters, property_sets: BTreeMap::new(),
            graph: Some(ParametricGraph {
                nodes: vec![
                    GraphNode { id: "entrada".into(), kind: NodeKind::Input { parameter: "width".into() } },
                    GraphNode { id: "folga".into(), kind: NodeKind::Constant { value: 0.04 } },
                    GraphNode { id: "folha".into(), kind: NodeKind::Subtract { a: "entrada".into(), b: "folga".into() } },
                ],
                outputs: BTreeMap::from([("leaf_width_m".into(), "folha".into())]),
            }),
        };
        let instance = ComponentInstance { id: "I1".into(), definition_id: definition.id.clone(), overrides: BTreeMap::new(),
            transform: [1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.] };
        (definition, instance)
    }

    #[test]
    fn override_recomputes_output_without_modifying_definition() {
        let (definition, mut instance) = component();
        assert!((definition.evaluate(&instance).unwrap()["leaf_width_m"] - 0.86).abs() < 1e-9);
        instance.overrides.insert("width".into(), 1.2);
        assert!((definition.evaluate(&instance).unwrap()["leaf_width_m"] - 1.16).abs() < 1e-9);
        assert_eq!(definition.parameters["width"].value, 0.90);
    }

    #[test]
    fn rejecting_invalid_parameters_and_cycles() {
        let (mut def, mut instance) = component();
        instance.overrides.insert("width".into(), 6.0);
        assert!(def.evaluate(&instance).is_err());
        instance.overrides.clear();
        def.graph.as_mut().unwrap().nodes[1].kind = NodeKind::Add { a: "folha".into(), b: "entrada".into() };
        assert!(def.evaluate(&instance).unwrap_err().contains("Ciclo"));
    }

    #[test]
    fn serialize_roundtrip_component() {
        let (definition, _instance) = component();
        let json = serde_json::to_string(&definition).unwrap();
        let restored: ComponentDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(definition, restored);
    }
}
