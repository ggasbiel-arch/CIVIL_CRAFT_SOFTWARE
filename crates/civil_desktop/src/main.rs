mod ifc;
mod normas;
mod nr;
mod wall_editor;
use civil_core::{Element, LayerRole, Project, ReferenceAxis, Wall, WallFace, WallLayer};
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Sense, Shape, Stroke, Vec2};
use std::{fs, io::Write, path::Path};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tool {
    Select,
    Wall,
    Door,
    Window,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum View {
    Plan,
    Isometric,
    Split,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ModelingMode {
    Parametric,
    Free,
}

const TOOLS: [(Tool, &str); 4] = [
    (Tool::Select, "Selecionar"),
    (Tool::Wall, "Parede"),
    (Tool::Door, "Porta"),
    (Tool::Window, "Janela"),
];

struct StudioApp {
    pinned_tools: [bool; 4],
    show_grid: bool,
    outlines: bool,
    nr: nr::Biblioteca,
    normas: normas::Biblioteca,
    ifc: ifc::IfcInspector,
    modeling_mode: ModelingMode,
    project: Project,
    tool: Tool,
    view: View,
    show_navigator: bool,
    show_inspector: bool,
    wall_editor: Option<Wall>,
    geometry_cache: civil_core::WallPlanCache,
    plan_cache: Option<(String, Vec<WallFace>)>,
    chain_walls: bool,
    pending_start: Option<[f64; 2]>,
    selected_id: Option<String>,
    active_level: String,
    level_draft: Option<(String, f64)>,
    opening_dims: Option<(String, f64, f64)>,
    host_draft: Option<(String, String, f64)>,
    door_width_draft: Option<(String, String)>,
    thickness_m: f64,
    height_m: f64,
    zoom: f32,
    snap: bool,
    file_path: String,
    status: String,
    undo_stack: Vec<Project>,
    redo_stack: Vec<Project>,
}

impl Default for StudioApp {
    fn default() -> Self {
        Self {
            pinned_tools: [true, false, false, false],
            show_grid: true,
            outlines: false,
            nr: Default::default(),
            normas: Default::default(),
            ifc: Default::default(),
            modeling_mode: ModelingMode::Parametric,
            project: Project::example(),
            tool: Tool::Select,
            view: View::Plan,
            show_navigator: true,
            show_inspector: true,
            wall_editor: None,
            geometry_cache: Default::default(),
            plan_cache: None,
            chain_walls: false,
            pending_start: None,
            selected_id: None,
            active_level: "P0".into(),
            level_draft: None,
            opening_dims: None,
            host_draft: None,
            door_width_draft: None,
            thickness_m: 0.20,
            height_m: 2.80,
            zoom: 29.0,
            snap: true,
            file_path: "meu_projeto.CCW".into(),
            status: "Casa piloto carregada".into(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }
}

impl StudioApp {
    fn save_snapshot(&mut self) {
        self.opening_dims = None;
        self.host_draft = None;
        self.level_draft = None;
        self.plan_cache = None;
        self.wall_editor = None;
        if self.undo_stack.len() >= 50 {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(self.project.clone());
        self.redo_stack.clear();
    }

    fn undo(&mut self) {
        if let Some(before) = self.undo_stack.pop() {
            self.redo_stack.push(self.project.clone());
            self.project = before;
            self.reset_after_history();
            self.status = "Alteração desfeita".into();
        }
    }

    fn redo(&mut self) {
        if let Some(after) = self.redo_stack.pop() {
            self.undo_stack.push(self.project.clone());
            self.project = after;
            self.reset_after_history();
            self.status = "Alteração refeita".into();
        }
    }

    fn reset_after_history(&mut self) {
        self.opening_dims = None;
        self.host_draft = None;
        self.level_draft = None;
        self.plan_cache = None;
        self.wall_editor = None;
        if !self
            .project
            .levels
            .iter()
            .any(|level| level.id == self.active_level)
        {
            self.active_level = self.project.levels[0].id.clone();
        }
        self.pending_start = None;
        self.door_width_draft = None;
        if self.selected_id.as_ref().is_some_and(|id| {
            !self
                .project
                .elements
                .iter()
                .any(|element| element.id() == id)
                && !self
                    .project
                    .component_instances
                    .iter()
                    .any(|placed| &placed.instance.id == id)
        }) {
            self.selected_id = None;
        }
    }

    fn nearest_host(&self, point: [f64; 2]) -> Option<(String, f64)> {
        self.project
            .elements
            .iter()
            .filter_map(|element| {
                let Element::Wall(w) = element else {
                    return None;
                };
                if w.level_id != self.active_level {
                    return None;
                }
                let mid = w.thickness_m / 2.;
                let a = w.strip(mid, mid, 0., w.length_m())[0];
                let dx = (w.end[0] - w.start[0]) / w.length_m();
                let dy = (w.end[1] - w.start[1]) / w.length_m();
                let along = (point[0] - a[0]) * dx + (point[1] - a[1]) * dy;
                let across = ((point[1] - a[1]) * dx - (point[0] - a[0]) * dy).abs();
                (along >= 0. && along <= w.length_m() && across <= mid + 0.10).then_some((
                    across,
                    w.id.clone(),
                    along,
                ))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)))
            .map(|(_, id, d)| (id, d))
    }
    fn insert_door(&mut self, point: [f64; 2]) {
        let before = self.project.clone();
        let mut proposed = self.project.clone();
        let result = proposed.add_door(&self.active_level, point).and_then(|id| {
            if let Some((wall, distance)) = self.nearest_host(point) {
                proposed.host_opening(&id, &wall, distance)?;
            }
            Ok::<_, String>(id)
        });
        match result {
            Ok(id) => {
                self.project = proposed;
                self.save_snapshot_from(before);
                self.selected_id = Some(id);
                self.door_width_draft = None;
                self.tool = Tool::Select;
                self.status = "Porta inserida".into();
            }
            Err(error) => self.status = error,
        }
    }

    fn apply_door_width(&mut self, id: &str, width_m: f64) {
        let before = self.project.clone();
        match self.project.set_door_width(id, width_m) {
            Ok(true) => {
                self.save_snapshot_from(before);
                self.status = "Largura do vão atualizada".into();
            }
            Ok(false) => {}
            Err(error) => self.status = error,
        }
    }

    fn door_inspector(&mut self, ui: &mut egui::Ui, id: &str) {
        let Some(placed) = self
            .project
            .component_instances
            .iter()
            .find(|placed| placed.instance.id == id)
        else {
            self.door_width_draft = None;
            return;
        };
        let Ok(geometry) = self.project.door_geometry(placed) else {
            return;
        };
        let def = &self.project.component_definitions[&placed.instance.definition_id];
        let window = def.classification == "IfcWindow";
        if self
            .opening_dims
            .as_ref()
            .is_none_or(|(owner, _, _)| owner != id)
        {
            let value = |key: &str| {
                placed
                    .instance
                    .overrides
                    .get(key)
                    .copied()
                    .or_else(|| def.parameters.get(key).map(|p| p.value))
                    .unwrap_or_default()
            };
            self.opening_dims = Some((id.into(), value("altura_m"), value("peitoril_m")));
        }
        if let Some((_, height, sill)) = &mut self.opening_dims {
            ui.add(
                egui::DragValue::new(height)
                    .prefix("Altura ")
                    .suffix(" m")
                    .speed(0.01),
            );
            let mut edit = ui
                .button("Aplicar altura")
                .clicked()
                .then_some(("altura_m", *height));
            if window {
                ui.add(
                    egui::DragValue::new(sill)
                        .prefix("Peitoril ")
                        .suffix(" m")
                        .speed(0.01),
                );
                if ui.button("Aplicar peitoril").clicked() {
                    edit = Some(("peitoril_m", *sill));
                }
            }
            if let Some((key, value)) = edit {
                let before = self.project.clone();
                match self.project.set_opening_parameter(id, key, value) {
                    Ok(true) => {
                        self.save_snapshot_from(before);
                        self.status = "Vão atualizado".into();
                    }
                    Ok(false) => {}
                    Err(error) => self.status = error,
                }
                return;
            }
        }
        let first_wall = self
            .project
            .elements
            .iter()
            .find(|e| matches!(e, Element::Wall(_)) && e.level_id() == placed.level_id);
        if let Some(first) = first_wall {
            if self
                .host_draft
                .as_ref()
                .is_none_or(|(owner, _, _)| owner != id)
            {
                let (wall, distance) = placed
                    .host
                    .as_ref()
                    .map_or((first.id(), 0.), |h| (h.wall_id.as_str(), h.distance_m));
                self.host_draft = Some((id.into(), wall.into(), distance));
            }
            if let Some((_, wall, distance)) = &mut self.host_draft {
                egui::ComboBox::from_label("Hospedeira")
                    .selected_text(wall.as_str())
                    .show_ui(ui, |ui| {
                        for element in &self.project.elements {
                            if let Element::Wall(w) = element {
                                ui.selectable_value(
                                    wall,
                                    w.id.clone(),
                                    format!("{} · {}", w.level_id, w.id),
                                );
                            }
                        }
                    });
                ui.add(
                    egui::DragValue::new(distance)
                        .speed(0.05)
                        .suffix(" m do início"),
                );
                if ui.button("Aplicar posição").clicked() {
                    let before = self.project.clone();
                    match self.project.host_opening(id, wall, *distance) {
                        Ok(()) => {
                            if self.project != before {
                                self.active_level = self
                                    .project
                                    .component_instances
                                    .iter()
                                    .find(|p| p.instance.id == id)
                                    .unwrap()
                                    .level_id
                                    .clone();
                                self.save_snapshot_from(before);
                            }
                            self.status = "Hospedagem atualizada".into();
                        }
                        Err(error) => self.status = error,
                    }
                    return;
                }
            }
        }
        let draft = &mut self.door_width_draft;
        if draft.as_ref().is_none_or(|(owner, _)| owner != id) {
            *draft = Some((id.into(), geometry.width_m.to_string()));
        }
        if let Some((_, text)) = draft {
            ui.label("Largura do vão (m)");
            ui.add(egui::TextEdit::singleline(text).desired_width(72.0));
            if ui.button("Aplicar largura").clicked() {
                match text.trim().replace(',', ".").parse() {
                    Ok(width) => self.apply_door_width(id, width),
                    Err(_) => self.status = "Informe uma largura em metros".into(),
                }
            }
        }
    }

    fn replace_project(&mut self, project: Project) {
        self.save_snapshot();
        self.active_level = project
            .levels
            .first()
            .map_or_else(|| "P0".into(), |l| l.id.clone());
        self.project = project;
        self.selected_id = None;
        self.door_width_draft = None;
        self.pending_start = None;
    }

    fn save_file(&mut self) {
        let entered = self.file_path.trim();
        let path = Path::new(entered);
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            self.status = "Informe o nome do arquivo .CCW".into();
            return;
        };
        if path.is_dir() || entered.ends_with(std::path::is_separator) {
            self.status = "Informe um arquivo, além da pasta".into();
            return;
        }
        let lower = name.to_ascii_lowercase();
        let explicit = lower.ends_with(".ccw");
        let destination = if explicit {
            path.to_owned()
        } else {
            let stem = lower
                .strip_suffix(".bim.json")
                .map_or(name, |prefix| &name[..prefix.len()]);
            path.with_file_name(format!("{stem}.CCW"))
        };
        match self
            .project
            .validate()
            .and_then(|_| serde_json::to_string_pretty(&self.project).map_err(|e| e.to_string()))
            .and_then(|json| {
                write_project(&destination, &json, explicit).map_err(|e| e.to_string())
            }) {
            Ok(()) => {
                self.file_path = destination.to_string_lossy().into_owned();
                self.status = format!("Projeto salvo: {}", self.file_path);
            }
            Err(err) => self.status = format!("Erro ao salvar: {err}"),
        }
    }

    fn open_file(&mut self) {
        let loaded = fs::read_to_string(self.file_path.trim())
            .map_err(|e| e.to_string())
            .and_then(|data| serde_json::from_str::<Project>(&data).map_err(|e| e.to_string()))
            .and_then(|project| {
                project.validate()?;
                Ok(project)
            });
        match loaded {
            Ok(project) => {
                self.replace_project(project);
                self.status = format!("Projeto aberto: {}", self.file_path);
            }
            Err(err) => self.status = format!("Erro ao abrir: {err}"),
        }
    }

    fn draw_plan(&mut self, ui: &mut egui::Ui) {
        ui.label(match self.tool {
            Tool::Wall => "Marque o início e o fim da parede.",
            Tool::Door => {
                "Clique na parede para hospedar a porta ou fora dela para uma porta independente."
            }
            Tool::Window => "Clique na parede para inserir a janela.",
            Tool::Select => "Clique em um elemento para selecioná-lo.",
        });
        let available = ui.available_size();
        let desired = Vec2::new(available.x.max(1.0), available.y.max(1.0));
        let (response, painter) = ui.allocate_painter(desired, Sense::click());
        let rect = response.rect;
        painter.rect_filled(
            rect,
            egui::CornerRadius::ZERO,
            Color32::from_rgb(25, 32, 42),
        );
        let origin = rect.center();
        let scale = self.zoom;
        let world_to_screen = |point: [f64; 2]| -> Pos2 {
            egui::pos2(
                origin.x + point[0] as f32 * scale,
                origin.y - point[1] as f32 * scale,
            )
        };
        let screen_to_world = |position: Pos2| -> [f64; 2] {
            let x = ((position.x - origin.x) / scale) as f64;
            let y = ((origin.y - position.y) / scale) as f64;
            if self.snap {
                [(x * 4.0).round() / 4.0, (y * 4.0).round() / 4.0]
            } else {
                [x, y]
            }
        };
        let xcount = ((rect.width() / scale / 2.0).ceil() as i32 + 1).min(150);
        let ycount = ((rect.height() / scale / 2.0).ceil() as i32 + 1).min(150);
        if self.show_grid {
            for x in -xcount..=xcount {
                let px = origin.x + x as f32 * scale;
                let color = if x == 0 {
                    Color32::from_rgb(86, 151, 152)
                } else {
                    Color32::from_rgb(43, 52, 63)
                };
                painter.line_segment(
                    [egui::pos2(px, rect.top()), egui::pos2(px, rect.bottom())],
                    Stroke::new(if x == 0 { 1.7 } else { 0.7 }, color),
                );
            }
            for y in -ycount..=ycount {
                let py = origin.y + y as f32 * scale;
                let color = if y == 0 {
                    Color32::from_rgb(86, 151, 152)
                } else {
                    Color32::from_rgb(43, 52, 63)
                };
                painter.line_segment(
                    [egui::pos2(rect.left(), py), egui::pos2(rect.right(), py)],
                    Stroke::new(if y == 0 { 1.7 } else { 0.7 }, color),
                );
            }
        }
        for element in &self.project.elements {
            if let Element::Slab(slab) = element
                && slab.level_id == self.active_level
            {
                painter.add(self.model_polygon(
                    slab.polygon.iter().copied().map(world_to_screen).collect(),
                    Color32::from_rgb(54, 65, 74),
                    Stroke::new(1.2, Color32::from_rgb(100, 119, 132)),
                ));
            }
        }
        if self
            .plan_cache
            .as_ref()
            .is_none_or(|(level, _)| level != &self.active_level)
        {
            self.plan_cache = Some((
                self.active_level.clone(),
                self.geometry_cache
                    .update(&self.project, &self.active_level),
            ));
        }
        let faces = &self.plan_cache.as_ref().unwrap().1;
        for face in faces {
            let fill = if self.selected_id.as_deref() == Some(&face.wall_id) {
                Color32::GOLD
            } else {
                layer_color(face.layer.role)
            };
            painter.add(self.model_polygon(
                face.polygon.iter().copied().map(world_to_screen).collect(),
                fill,
                Stroke::new(0.7, Color32::from_rgb(25, 39, 44)),
            ));
        }
        let mut door_hit = None;
        for placed in &self.project.component_instances {
            if placed.level_id != self.active_level {
                continue;
            }
            if let Ok(geometry) = self.project.door_geometry(placed) {
                let [a, b] = geometry.opening.map(|[x, y, _]| world_to_screen([x, y]));
                let color = if self.selected_id.as_deref() == Some(&placed.instance.id) {
                    Color32::GOLD
                } else {
                    Color32::LIGHT_BLUE
                };
                painter.line_segment([a, b], Stroke::new(4.0, color));
                let v = b - a;
                if self.project.component_definitions[&placed.instance.definition_id].classification
                    == "IfcWindow"
                {
                    let n = v.rot90().normalized() * 2.;
                    painter.line_segment([a + n, b + n], Stroke::new(1., color));
                    painter.line_segment([a - n, b - n], Stroke::new(1., color));
                } else {
                    let tip = a + v.rot90();
                    painter.line_segment([a, tip], Stroke::new(1., color));
                    let arc = (0..=16)
                        .map(|i| {
                            let angle = std::f32::consts::FRAC_PI_2 * (i as f32) / 16.;
                            a + v * angle.cos() + v.rot90() * angle.sin()
                        })
                        .collect();
                    painter.add(Shape::line(arc, Stroke::new(0.8, color)));
                }
                painter.circle_filled(a, 3.0, color);
                painter.circle_filled(b, 3.0, color);
                if let Some(pointer) = response.interact_pointer_pos() {
                    let distance = segment_distance(pointer, a, b);
                    if distance < 8.0 && door_hit.as_ref().is_none_or(|(best, _)| distance <= *best)
                    {
                        door_hit = Some((distance, placed.instance.id.clone()));
                    }
                }
            }
        }
        if let Some(first) = self.pending_start {
            painter.circle_filled(world_to_screen(first), 4.5, Color32::from_rgb(251, 190, 80));
            if let Some(pointer) = response.hover_pos() {
                painter.line_segment(
                    [
                        world_to_screen(first),
                        world_to_screen(screen_to_world(pointer)),
                    ],
                    Stroke::new(2.0, Color32::from_rgb(251, 190, 80)),
                );
            }
        }
        painter.text(
            rect.left_top() + Vec2::new(12.0, 12.0),
            Align2::LEFT_TOP,
            format!(
                "1 m = {:.0} px  |  Snap {}  |  Nível {}",
                scale,
                if self.snap { "25 cm" } else { "desligado" },
                self.active_level
            ),
            FontId::monospace(12.0),
            Color32::from_rgb(203, 218, 224),
        );

        if self.tool == Tool::Select
            && let Some(Element::Wall(wall)) = self
                .project
                .elements
                .iter()
                .find(|e| Some(e.id()) == self.selected_id.as_deref())
        {
            for (index, point) in [wall.start, wall.end].into_iter().enumerate() {
                let position = world_to_screen(point);
                let handle = ui.interact(
                    egui::Rect::from_center_size(position, Vec2::splat(12.)),
                    ui.scope_id().with((&wall.id, index)),
                    Sense::drag(),
                );
                painter.circle_stroke(position, 4., Stroke::new(1.5, Color32::GOLD));
                if (handle.dragged() || handle.drag_stopped())
                    && let Some(pointer) = handle.interact_pointer_pos()
                {
                    let target = screen_to_world(pointer);
                    let other = if index == 0 { wall.end } else { wall.start };
                    painter.line_segment(
                        [world_to_screen(other), world_to_screen(target)],
                        Stroke::new(2., Color32::GOLD),
                    );
                    if handle.drag_stopped() {
                        let mut draft = wall.clone();
                        if index == 0 {
                            draft.start = target;
                        } else {
                            draft.end = target;
                        }
                        self.apply_wall(draft);
                        return;
                    }
                }
            }
        }
        if response.clicked()
            && let Some(pointer) = response.interact_pointer_pos()
        {
            let world = screen_to_world(pointer);
            match self.tool {
                Tool::Wall => {
                    if let Some(start) = self.pending_start.take() {
                        let snapshot = self.project.clone();
                        match self.project.add_wall(
                            &self.active_level,
                            start,
                            world,
                            self.thickness_m,
                            self.height_m,
                        ) {
                            Ok(id) => {
                                self.save_snapshot_from(snapshot);
                                self.selected_id = Some(id);
                                self.pending_start = self.chain_walls.then_some(world);
                                self.status = "Parede criada".into();
                            }
                            Err(err) => self.status = err,
                        }
                    } else {
                        self.pending_start = Some(world);
                        self.status = "Selecione o ponto final da parede".into();
                    }
                }
                Tool::Door => self.insert_door(world),
                Tool::Window => {
                    let before = self.project.clone();
                    let result = self
                        .nearest_host(world)
                        .ok_or_else(|| "Selecione uma parede para a janela".to_string())
                        .and_then(|(id, d)| self.project.add_window(&id, d));
                    match result {
                        Ok(id) => {
                            self.save_snapshot_from(before);
                            self.selected_id = Some(id);
                            self.tool = Tool::Select;
                            self.status = "Janela inserida".into();
                        }
                        Err(err) => self.status = err,
                    }
                }
                Tool::Select => {
                    self.door_width_draft = None;
                    if let Some((_, id)) = door_hit {
                        self.selected_id = Some(id);
                        return;
                    }
                    self.selected_id = faces
                        .iter()
                        .rev()
                        .find(|face| {
                            let points: Vec<_> =
                                face.polygon.iter().copied().map(world_to_screen).collect();
                            let mut positive = false;
                            let mut negative = false;
                            for i in 0..points.len() {
                                let edge = points[(i + 1) % points.len()] - points[i];
                                let v = pointer - points[i];
                                let cross = edge.x * v.y - edge.y * v.x;
                                positive |= cross > 0.;
                                negative |= cross < 0.;
                                if segment_distance(
                                    pointer,
                                    points[i],
                                    points[(i + 1) % points.len()],
                                ) < 3.
                                {
                                    return true;
                                }
                            }
                            !(positive && negative)
                        })
                        .map(|f| f.wall_id.clone());
                }
            }
        }
    }

    fn save_snapshot_from(&mut self, before: Project) {
        self.opening_dims = None;
        self.host_draft = None;
        self.level_draft = None;
        self.plan_cache = None;
        self.wall_editor = None;
        if self.undo_stack.len() >= 50 {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(before);
        self.redo_stack.clear();
    }

    fn draw_iso(&self, ui: &mut egui::Ui) {
        ui.label("Prévia isométrica do modelo");
        let available = ui.available_size();
        let desired = Vec2::new(available.x.max(1.0), available.y.max(1.0));
        let (response, painter) = ui.allocate_painter(desired, Sense::hover());
        let rect = response.rect;
        painter.rect_filled(
            rect,
            egui::CornerRadius::ZERO,
            Color32::from_rgb(32, 40, 51),
        );
        let origin = rect.center() + Vec2::new(0.0, (rect.height() * 0.12).min(72.0));
        let scale = self.zoom * 0.77;
        let iso = |point: [f64; 3]| -> Pos2 {
            let x = point[0] as f32;
            let y = point[1] as f32;
            let z = point[2] as f32;
            egui::pos2(
                origin.x + (x - y) * scale * 0.90,
                origin.y + (x + y) * scale * 0.43 - z * scale * 1.17,
            )
        };
        for axis in [
            (
                [0.0, 0.0, 0.0],
                [3.0, 0.0, 0.0],
                Color32::from_rgb(204, 90, 90),
            ),
            (
                [0.0, 0.0, 0.0],
                [0.0, 3.0, 0.0],
                Color32::from_rgb(93, 190, 117),
            ),
            (
                [0.0, 0.0, 0.0],
                [0.0, 0.0, 3.0],
                Color32::from_rgb(92, 146, 223),
            ),
        ] {
            painter.line_segment([iso(axis.0), iso(axis.1)], Stroke::new(2.0, axis.2));
        }

        for element in &self.project.elements {
            if let Element::Slab(slab) = element {
                let z = self.project.level_elevation(&slab.level_id);
                let polygon: Vec<Pos2> =
                    slab.polygon.iter().map(|p| iso([p[0], p[1], z])).collect();
                painter.add(self.model_polygon(
                    polygon,
                    Color32::from_rgb(82, 101, 118),
                    Stroke::new(1.3, Color32::from_rgb(151, 170, 181)),
                ));
            }
        }
        // Wireframe / faces simplificadas: não há ocultação, booleanas nem malha B-Rep nesta versão.
        for element in &self.project.elements {
            if let Element::Wall(wall) = element {
                let z0 = self.project.level_elevation(&wall.level_id);
                let height = self.project.wall_height(wall).unwrap_or(wall.height_m);
                let mut spans = Vec::new();
                let mut start = 0.;
                for (a, b, sill, lintel) in self.project.openings(wall) {
                    spans.push((start, a, 0., height));
                    spans.push((a, b, 0., sill));
                    spans.push((a, b, lintel, height));
                    start = b;
                }
                spans.push((start, wall.length_m(), 0., height));
                let mut offset = 0.;
                for layer in wall.effective_layers() {
                    let next = offset + layer.thickness_m;
                    let fill = if self.selected_id.as_deref() == Some(&wall.id) {
                        Color32::GOLD
                    } else {
                        layer_color(layer.role)
                    };
                    for &(a, b, bottom, height) in &spans {
                        if a >= b || bottom >= height {
                            continue;
                        }
                        let footprint = wall.strip(offset, next, a, b);
                        let base = footprint.map(|p| iso([p[0], p[1], z0 + bottom]));
                        let top = footprint.map(|p| iso([p[0], p[1], z0 + height]));
                        let stroke = Stroke::new(0.8, Color32::from_rgb(38, 54, 66));
                        for i in 0..4 {
                            let j = (i + 1) % 4;
                            painter.add(self.model_polygon(
                                vec![base[i], base[j], top[j], top[i]],
                                fill,
                                stroke,
                            ));
                        }
                        painter.add(self.model_polygon(top.to_vec(), fill, stroke));
                    }
                    offset = next;
                }
            }
        }
        for placed in &self.project.component_instances {
            if let Ok(geometry) = self.project.door_geometry(placed) {
                let color = if self.selected_id.as_deref() == Some(&placed.instance.id) {
                    Color32::GOLD
                } else {
                    Color32::LIGHT_BLUE
                };
                painter.add(Shape::closed_line(
                    geometry.leaf.map(iso).to_vec(),
                    Stroke::new(2.0, color),
                ));
            }
        }
    }
}

fn layer_color(role: LayerRole) -> Color32 {
    match role {
        LayerRole::Structure => Color32::from_rgb(168, 201, 201),
        LayerRole::Finish => Color32::from_rgb(207, 181, 154),
        LayerRole::Insulation => Color32::from_rgb(201, 196, 133),
    }
}

fn write_project(path: &Path, json: &str, replace: bool) -> std::io::Result<()> {
    let directory = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temporary = tempfile::NamedTempFile::new_in(directory)?;
    temporary.write_all(json.as_bytes())?;
    temporary.as_file().sync_all()?;
    if replace {
        temporary.persist(path)?;
    } else {
        temporary.persist_noclobber(path)?;
    }
    Ok(())
}

fn segment_distance(point: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let ap = point - a;
    let denom = ab.length_sq();
    if denom < 1e-8 {
        return ap.length();
    }
    let t = (ap.dot(ab) / denom).clamp(0.0, 1.0);
    (point - (a + ab * t)).length()
}

impl StudioApp {
    fn workspace(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("menus").show(ui, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("Arquivo", |ui| {
                    if ui.button("Novo").clicked() {
                        self.replace_project(Project::default());
                        self.status = "Novo projeto".into();
                        ui.close();
                    }
                    if ui.button("Abrir exemplo").clicked() {
                        self.replace_project(Project::example());
                        self.status = "Projeto demonstrativo carregado".into();
                        ui.close();
                    }
                    ui.separator();
                    ui.label("Projeto CIVIL CRAFT (.CCW)");
                    ui.text_edit_singleline(&mut self.file_path);
                    if ui.button("Abrir").clicked() {
                        self.open_file();
                        ui.close();
                    }
                    if ui.button("Salvar").clicked() {
                        self.save_file();
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Consultar IFC…").clicked() {
                        self.ifc.open = true;
                        ui.close();
                    }
                });
                ui.menu_button("Câmeras", |ui| {
                    for (view, label) in [
                        (View::Plan, "Planta"),
                        (View::Isometric, "Isometria"),
                        (View::Split, "Duas vistas"),
                    ] {
                        if ui.selectable_value(&mut self.view, view, label).clicked() {
                            ui.close();
                        }
                    }
                    ui.separator();
                    ui.add(egui::Slider::new(&mut self.zoom, 12.0..=65.0).text("Zoom"));
                });
                ui.menu_button("Ferramentas", |ui| {
                    for (tool, label) in [
                        (Tool::Select, "Selecionar"),
                        (Tool::Wall, "Parede"),
                        (Tool::Door, "Porta"),
                        (Tool::Window, "Janela"),
                    ] {
                        if tool != Tool::Select && self.modeling_mode == ModelingMode::Free {
                            continue;
                        }
                        if ui.selectable_label(self.tool == tool, label).clicked() {
                            self.activate_tool(tool);
                            ui.close();
                        }
                    }
                    ui.checkbox(&mut self.snap, "Snap 0,25 m");
                    ui.separator();
                    if ui
                        .add_enabled(!self.undo_stack.is_empty(), egui::Button::new("Desfazer"))
                        .clicked()
                    {
                        self.undo();
                        ui.close();
                    }
                    if ui
                        .add_enabled(!self.redo_stack.is_empty(), egui::Button::new("Refazer"))
                        .clicked()
                    {
                        self.redo();
                        ui.close();
                    }
                });
                ui.menu_button("Extensões", |ui| {
                    ui.label("Nenhuma extensão disponível nesta versão.");
                });
                ui.menu_button("Janelas", |ui| {
                    ui.checkbox(&mut self.show_navigator, "Mapa do projeto");
                    ui.checkbox(&mut self.show_inspector, "Propriedades");
                });
                ui.menu_button("Ajuda", |ui| {
                    if ui.button("Normas Regulamentadoras (NR)…").clicked() {
                        self.nr.aberta = true;
                        ui.close();
                    }
                    if ui.button("Biblioteca de normas ABNT…").clicked() {
                        self.normas.aberta = true;
                        ui.close();
                    }
                    ui.separator();
                    ui.strong("Guia rápido");
                    ui.label("Escolha o modo de modelagem na faixa inferior.");
                    ui.label("Para desenhar uma parede, marque o início e o fim na planta.");
                    ui.label("Selecione um elemento para consultar suas propriedades.");
                    ui.label("Para salvar, informe o caminho do projeto em Arquivo.");
                });
            });
        });
        self.nr.mostrar(ui.ctx());
        self.normas.mostrar(ui.ctx());
        self.ifc.show(ui.ctx());
        egui::Panel::top("quick_tools").show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.menu_button("+ Ferramentas", |ui| {
                    for (index, (_, label)) in TOOLS.iter().enumerate() {
                        ui.checkbox(&mut self.pinned_tools[index], *label);
                    }
                });
                for (index, (tool, label)) in TOOLS.iter().copied().enumerate() {
                    if self.pinned_tools[index]
                        && (tool == Tool::Select || self.modeling_mode == ModelingMode::Parametric)
                        && ui.selectable_label(self.tool == tool, label).clicked()
                    {
                        self.activate_tool(tool);
                    }
                }
            });
        });
        if self.show_inspector {
            egui::Panel::top("property_strip").show(ui, |ui| {
                egui::ScrollArea::horizontal()
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        ui.horizontal(|ui| self.inspector(ui));
                    });
            });
        }
        egui::Panel::bottom("modeling_mode").show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.menu_button("OVM", |ui| {
                    ui.checkbox(&mut self.show_grid, "Grade da planta");
                    for (view, label) in [
                        (View::Plan, "Planta"),
                        (View::Isometric, "Isometria"),
                        (View::Split, "Duas vistas"),
                    ] {
                        if ui.selectable_value(&mut self.view, view, label).clicked() {
                            ui.close();
                        }
                    }
                });
                ui.menu_button("Estilos", |ui| {
                    if ui
                        .selectable_value(&mut self.outlines, false, "Sombreado")
                        .clicked()
                        || ui
                            .selectable_value(&mut self.outlines, true, "Contornos")
                            .clicked()
                    {
                        ui.close();
                    }
                });
                ui.separator();
                ui.strong("Modelagem");
                for (mode, label) in [
                    (ModelingMode::Parametric, "Paramétrica + visual por nós"),
                    (ModelingMode::Free, "Livre"),
                ] {
                    if ui
                        .selectable_label(self.modeling_mode == mode, label)
                        .clicked()
                    {
                        self.switch_mode(mode);
                    }
                }
            });
        });
        egui::Panel::bottom("status").show(ui, |ui| {
            ui.label(&self.status);
        });
        egui::Panel::left("tool_palette")
            .exact_size(126.0)
            .show(ui, |ui| {
                ui.strong("Ferramentas");
                ui.separator();
                self.palette_tool(ui, Tool::Select, "Selecionar");
                ui.separator();
                if self.modeling_mode == ModelingMode::Parametric {
                    egui::CollapsingHeader::new("Arquitetura")
                        .default_open(true)
                        .show(ui, |ui| {
                            self.palette_tool(ui, Tool::Wall, "Parede");
                            self.palette_tool(ui, Tool::Door, "Porta");
                            self.palette_tool(ui, Tool::Window, "Janela");
                        });
                } else {
                    ui.strong("Geometria");
                    ui.small("Edição de faces e arestas em desenvolvimento.");
                }
                if self.pending_start.is_some() && ui.button("Cancelar").clicked() {
                    self.pending_start = None;
                }
            });
        if self.show_navigator {
            egui::Panel::right("project_sidebar")
                .resizable(true)
                .default_size(236.0)
                .size_range(180.0..=320.0)
                .show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| self.navigator(ui));
                });
        }

        self.wall_dialog(ui.ctx());
        let canvas = egui::CentralPanel::default().show(ui, |ui| match self.view {
            View::Plan => self.draw_plan(ui),
            View::Isometric => self.draw_iso(ui),
            View::Split => self.split_views(ui),
        });
        if let Some(id) = self.selected_id.clone() {
            egui::Window::new("Seleção")
                .default_pos(canvas.response.rect.left_top() + Vec2::new(12.0, 46.0))
                .resizable(false)
                .collapsible(false)
                .show(ui.ctx(), |ui| {
                    ui.horizontal(|ui| {
                        ui.strong(&id);
                        if ui.button("Propriedades").clicked() {
                            self.show_inspector = true;
                        }
                        if ui.button("Desmarcar").clicked() {
                            self.selected_id = None;
                        }
                        if ui.button("Excluir").clicked() {
                            self.delete_element(&id);
                        }
                    });
                });
        }
    }

    fn model_polygon(&self, points: Vec<Pos2>, fill: Color32, stroke: Stroke) -> Shape {
        if self.outlines {
            Shape::closed_line(points, Stroke::new(stroke.width.max(1.2), fill))
        } else {
            Shape::convex_polygon(points, fill, stroke)
        }
    }

    fn switch_mode(&mut self, mode: ModelingMode) {
        if self.modeling_mode != mode {
            self.modeling_mode = mode;
            self.tool = Tool::Select;
            self.pending_start = None;
        }
    }

    fn palette_tool(&mut self, ui: &mut egui::Ui, tool: Tool, label: &str) {
        if ui
            .add_sized(
                [ui.available_width(), 30.0],
                egui::Button::selectable(self.tool == tool, label),
            )
            .clicked()
        {
            self.activate_tool(tool);
        }
    }

    fn activate_tool(&mut self, tool: Tool) {
        self.tool = tool;
        self.pending_start = None;
        if self.view == View::Isometric {
            self.view = View::Plan;
        }
        if tool == Tool::Wall {
            self.show_inspector = true;
        }
    }

    fn split_views(&mut self, ui: &mut egui::Ui) {
        let (area, _) = ui.allocate_exact_size(ui.available_size(), Sense::hover());
        let mut plan = area;
        let mut iso = area;
        let gap = 4.0;
        if area.width() >= 620.0 {
            plan.max.x = area.center().x - gap;
            iso.min.x = area.center().x + gap;
        } else {
            plan.max.y = area.center().y - gap;
            iso.min.y = area.center().y + gap;
        }
        let mut plan_ui = ui.new_child(egui::UiBuilder::new().id_salt("split_plan").max_rect(plan));
        plan_ui.set_clip_rect(ui.clip_rect().intersect(plan));
        self.draw_plan(&mut plan_ui);
        let mut iso_ui = ui.new_child(egui::UiBuilder::new().id_salt("split_iso").max_rect(iso));
        iso_ui.set_clip_rect(ui.clip_rect().intersect(iso));
        self.draw_iso(&mut iso_ui);
    }

    fn navigator(&mut self, ui: &mut egui::Ui) {
        ui.heading("Mapa do projeto");
        ui.label("Projeto");
        ui.text_edit_singleline(&mut self.project.name);
        ui.separator();
        ui.strong("Pavimentos");
        if ui.button("Adicionar pavimento").clicked() {
            let mut number = self.project.levels.len();
            while self
                .project
                .levels
                .iter()
                .any(|l| l.id == format!("P{number}"))
            {
                number += 1;
            }
            let elevation = self
                .project
                .levels
                .iter()
                .map(|l| l.elevation_m)
                .fold(f64::NEG_INFINITY, f64::max)
                + 3.;
            self.save_snapshot();
            self.project.levels.push(civil_core::Level {
                id: format!("P{number}"),
                name: format!("Pavimento {number}"),
                elevation_m: elevation,
            });
        }
        if self
            .level_draft
            .as_ref()
            .is_none_or(|(id, _)| id != &self.active_level)
        {
            self.level_draft = Some((
                self.active_level.clone(),
                self.project.level_elevation(&self.active_level),
            ));
        }
        if let Some((id, z)) = &mut self.level_draft {
            let before_z = self.project.level_elevation(id);
            ui.add(
                egui::DragValue::new(z)
                    .prefix(format!("Cota {id} "))
                    .suffix(" m")
                    .speed(0.05),
            );
            if ui
                .add_enabled(*z != before_z, egui::Button::new("Aplicar cota"))
                .clicked()
            {
                let before = self.project.clone();
                match self.project.set_level_elevation(id, *z) {
                    Ok(true) => self.save_snapshot_from(before),
                    Ok(false) => {}
                    Err(error) => self.status = error,
                }
            }
        }
        for level in &self.project.levels {
            egui::CollapsingHeader::new(format!("{}  •  {:.2} m", level.name, level.elevation_m))
                .id_salt(("project_level", &level.id))
                .default_open(self.active_level == level.id)
                .show(ui, |ui| {
                    if ui
                        .selectable_label(self.active_level == level.id, "Planta do pavimento")
                        .clicked()
                    {
                        self.active_level = level.id.clone();
                        self.pending_start = None;
                        self.selected_id = None;
                        if self.view == View::Isometric {
                            self.view = View::Plan;
                        }
                    }
                    for element in self
                        .project
                        .elements
                        .iter()
                        .filter(|element| element.level_id() == level.id)
                    {
                        let kind = match element {
                            Element::Wall(_) => "Parede",
                            Element::Slab(_) => "Laje",
                        };
                        if ui
                            .selectable_label(
                                self.selected_id.as_deref() == Some(element.id()),
                                format!("{}  {}", element.id(), kind),
                            )
                            .clicked()
                        {
                            self.selected_id = Some(element.id().to_owned());
                            self.active_level = element.level_id().into();
                            self.pending_start = None;
                            self.tool = Tool::Select;
                        }
                    }
                    for placed in self
                        .project
                        .component_instances
                        .iter()
                        .filter(|placed| placed.level_id == level.id)
                    {
                        let id = &placed.instance.id;
                        let definition =
                            &self.project.component_definitions[&placed.instance.definition_id];
                        if ui
                            .selectable_label(
                                self.selected_id.as_ref() == Some(id),
                                format!("{id}  {}", definition.name),
                            )
                            .clicked()
                        {
                            self.selected_id = Some(id.clone());
                            self.active_level = placed.level_id.clone();
                            self.tool = Tool::Select;
                            self.pending_start = None;
                        }
                    }
                });
        }
        ui.separator();
        ui.collapsing("Quantitativos brutos", |ui| {
            let q = self.project.quantities();
            ui.label(format!(
                "Área dos vãos hospedados: {:.2} m²",
                q.hosted_opening_area_m2
            ));
            ui.label(format!("Paredes: {:.2} m", q.wall_length_m));
            ui.label(format!("Área de paredes: {:.2} m²", q.wall_gross_area_m2));
            ui.label(format!(
                "Volume de paredes: {:.2} m³",
                q.wall_gross_volume_m3
            ));
            ui.label(format!("Área de lajes: {:.2} m²", q.slab_area_m2));
            ui.label(format!("Volume de lajes: {:.2} m³", q.slab_volume_m3));
        });
    }

    fn inspector(&mut self, ui: &mut egui::Ui) {
        ui.heading("Propriedades");
        if self.tool == Tool::Wall {
            ui.strong("Próxima parede");
            ui.checkbox(&mut self.chain_walls, "Em sequência");
            ui.label("Espessura (m)");
            ui.add(
                egui::DragValue::new(&mut self.thickness_m)
                    .speed(0.01)
                    .range(0.05..=2.0),
            );
            ui.label("Altura (m)");
            ui.add(
                egui::DragValue::new(&mut self.height_m)
                    .speed(0.1)
                    .range(0.2..=20.0),
            );
            ui.separator();
        }
        if let Some(id) = self.selected_id.clone() {
            self.selection_properties(ui, &id);
        } else if self.tool != Tool::Wall {
            ui.label("Selecione um elemento.");
        }
    }

    fn selection_properties(&mut self, ui: &mut egui::Ui, id: &str) {
        if let Some(element) = self
            .project
            .elements
            .iter()
            .find(|element| element.id() == id)
        {
            match element {
                Element::Wall(wall) => {
                    ui.strong(format!("Parede {id}"));
                    ui.label(format!("Comprimento: {:.2} m", wall.length_m()));
                    ui.label(format!("Espessura: {:.2} m", wall.thickness_m));
                    ui.label(format!(
                        "Altura: {:.2} m",
                        self.project.wall_height(wall).unwrap_or(wall.height_m)
                    ));
                    if ui.button("Editar parede…").clicked() {
                        let mut draft = wall.clone();
                        draft.layers = wall.effective_layers();
                        self.wall_editor = Some(draft);
                    }
                }
                Element::Slab(slab) => {
                    ui.strong(format!("Laje {id}"));
                    ui.label(format!("Área: {:.2} m²", slab.area_m2()));
                    ui.label(format!("Espessura: {:.2} m", slab.thickness_m));
                }
            }
            ui.label(format!("Pavimento: {}", element.level_id()));
        } else if let Some(placed) = self
            .project
            .component_instances
            .iter()
            .find(|placed| placed.instance.id == id)
        {
            let definition = &self.project.component_definitions[&placed.instance.definition_id];
            ui.strong(format!("{} · {id}", definition.name));
            ui.label(format!("Pavimento: {}", placed.level_id));
            if self.modeling_mode == ModelingMode::Parametric {
                if let Some(graph) = &definition.graph {
                    ui.collapsing("Grafo do componente", |ui| {
                        ui.label(format!("{} nós", graph.nodes.len()));
                        for (output, node) in &graph.outputs {
                            ui.label(format!("{node} → {output}"));
                        }
                    });
                }
                self.door_inspector(ui, id);
            }
        }
    }

    fn delete_element(&mut self, id: &str) {
        if self
            .project
            .component_instances
            .iter()
            .any(|p| p.host.as_ref().is_some_and(|h| h.wall_id == id))
        {
            self.status =
                "Remova ou transfira as esquadrias hospedadas antes de excluir a parede".into();
            return;
        }
        self.save_snapshot();
        self.project.elements.retain(|element| element.id() != id);
        self.project
            .component_instances
            .retain(|placed| placed.instance.id != id);
        self.door_width_draft = None;
        self.selected_id = None;
        self.status = "Elemento excluído".into();
    }
}

impl eframe::App for StudioApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.workspace(ui);
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1460.0, 900.0])
            .with_min_inner_size([900.0, 580.0]),
        ..Default::default()
    };
    eframe::run_native(
        "CIVIL CRAFT CAD/BIM",
        options,
        Box::new(|cc| {
            let mut visuals = egui::Visuals::dark();
            visuals.panel_fill = Color32::from_rgb(24, 31, 41);
            visuals.selection.bg_fill = Color32::from_rgb(28, 87, 93);
            cc.egui_ctx.set_visuals(visuals);
            Ok(Box::<StudioApp>::default())
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn width(app: &StudioApp) -> f64 {
        app.project
            .door_geometry(&app.project.component_instances[0])
            .unwrap()
            .width_m
    }

    #[test]
    fn workspace_renders_views_and_selected_door_at_minimum_size() {
        let context = egui::Context::default();
        let mut app = StudioApp::default();
        app.insert_door([0.0, 0.0]);
        let selected = app.selected_id.clone();
        let project = app.project.clone();

        for view in [View::Plan, View::Isometric, View::Split] {
            app.view = view;
            let mut output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        Pos2::ZERO,
                        Vec2::new(900.0, 580.0),
                    )),
                    ..Default::default()
                },
                |ui| app.workspace(ui),
            );
            output.textures_delta.clear();
            assert!(!output.shapes.is_empty());
            assert_eq!(app.selected_id, selected);
            assert_eq!(app.project, project);
        }
    }

    #[test]
    fn split_views_stack_when_docks_leave_less_space() {
        let context = egui::Context::default();
        let mut app = StudioApp {
            view: View::Split,
            ..StudioApp::default()
        };
        let mut render = |show_docks| {
            app.show_navigator = show_docks;
            app.show_inspector = show_docks;
            let mut output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        Pos2::ZERO,
                        Vec2::new(900.0, 580.0),
                    )),
                    ..Default::default()
                },
                |ui| app.workspace(ui),
            );
            output.textures_delta.clear();
            let background = |color| {
                output
                    .shapes
                    .iter()
                    .find_map(|entry| match &entry.shape {
                        Shape::Rect(shape) if shape.fill == color => Some(shape.rect),
                        _ => None,
                    })
                    .expect("fundo da vista")
            };
            (
                background(Color32::from_rgb(25, 32, 42)),
                background(Color32::from_rgb(32, 40, 51)),
            )
        };

        let (plan, iso) = render(true);
        assert!(plan.bottom() < iso.top());
        let (plan, iso) = render(false);
        assert!(plan.right() < iso.left());
    }

    #[test]
    fn switching_modes_cancels_pending_wall_and_preserves_document() {
        let mut app = StudioApp::default();
        app.insert_door([1.0, 2.0]);
        let project = app.project.clone();
        let selection = app.selected_id.clone();
        app.tool = Tool::Wall;
        app.pending_start = Some([3.0, 4.0]);
        app.switch_mode(ModelingMode::Free);
        assert!(app.tool == Tool::Select && app.pending_start.is_none());
        app.switch_mode(ModelingMode::Parametric);
        assert_eq!(app.project, project);
        assert_eq!(app.selected_id, selection);
    }

    #[test]
    fn plan_click_inserts_and_reselects_door() {
        let context = egui::Context::default();
        let mut app = StudioApp {
            tool: Tool::Door,
            snap: false,
            ..StudioApp::default()
        };
        let pointer = egui::pos2(400.0, 320.0);
        let frame = |app: &mut StudioApp, events| {
            context
                .run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            Pos2::ZERO,
                            Vec2::new(800.0, 600.0),
                        )),
                        events,
                        ..Default::default()
                    },
                    |ui| app.draw_plan(ui),
                )
                .textures_delta
                .clear();
        };
        frame(&mut app, vec![egui::Event::PointerMoved(pointer)]);
        let click_event = |pressed| egui::Event::PointerButton {
            pos: pointer,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        frame(&mut app, vec![click_event(true)]);
        frame(&mut app, vec![click_event(false)]);
        let id = app.selected_id.take().expect("porta inserida");
        assert_eq!(app.project.component_instances.len(), 1);
        assert!(app.tool == Tool::Select);

        for pressed in [true, false] {
            frame(&mut app, vec![click_event(pressed)]);
        }
        assert_eq!(app.selected_id.as_deref(), Some(id.as_str()));
    }

    #[test]
    fn legacy_file_saves_as_ccw_without_overwriting_an_existing_copy() {
        let directory = tempfile::tempdir().unwrap();
        let legacy = directory.path().join("casa.BIM.JSON");
        let ccw = directory.path().join("casa.CCW");
        let source = include_str!("../../../samples/casa_piloto.bim.json");
        fs::write(&legacy, source).unwrap();
        let mut app = StudioApp {
            file_path: legacy.display().to_string(),
            ..Default::default()
        };
        app.open_file();
        assert!(app.status.starts_with("Projeto aberto"));
        fs::write(&ccw, "existente").unwrap();
        app.save_file();
        assert_eq!(fs::read_to_string(&ccw).unwrap(), "existente");
        fs::remove_file(&ccw).unwrap();
        app.save_file();
        assert_eq!(Path::new(&app.file_path), ccw);
        assert_eq!(fs::read_to_string(legacy).unwrap(), source);
        let saved: Project = serde_json::from_slice(&fs::read(ccw).unwrap()).unwrap();
        assert_eq!(saved, app.project);
    }

    #[test]
    fn hosted_wall_edits_round_trip_in_editor_history() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = StudioApp {
            project: serde_json::from_str(include_str!("../../../samples/oficina_paredes.CCW"))
                .unwrap(),
            file_path: dir.path().join("paredes.CCW").display().to_string(),
            ..Default::default()
        };
        let original = app.project.clone();
        let Element::Wall(mut w) = app.project.elements[0].clone() else {
            panic!()
        };
        w.start[1] -= 1.;
        w.layers[0].thickness_m = 0.25;
        assert!(app.apply_wall(w));
        let edited = app.project.clone();
        assert_ne!(edited.wall_plan("P0"), original.wall_plan("P0"));
        app.undo();
        assert_eq!(app.project, original);
        app.redo();
        assert_eq!(app.project, edited);
        app.delete_element("W1");
        assert_eq!(app.project, edited);
        app.delete_element("D1");
        assert_ne!(app.project.wall_plan("P0"), edited.wall_plan("P0"));
        app.undo();
        app.save_file();
        app.replace_project(Project::default());
        app.open_file();
        assert_eq!(app.project, edited);
    }

    #[test]
    fn door_edit_history_and_file_round_trip() {
        let directory = tempfile::tempdir().unwrap();
        let mut app = StudioApp {
            file_path: directory.path().join("project").to_str().unwrap().into(),
            ..StudioApp::default()
        };
        app.insert_door([1.0, 2.0]);
        let id = app.selected_id.clone().unwrap();
        assert_eq!(width(&app), 0.9);
        app.apply_door_width(&id, 1.2);
        assert_eq!(width(&app), 1.2);
        app.undo();
        assert_eq!(width(&app), 0.9);
        app.apply_door_width(&id, 9.0);
        assert_eq!(width(&app), 0.9);
        app.redo();
        assert_eq!(width(&app), 1.2);
        app.undo();
        app.undo();
        assert!(app.project.component_instances.is_empty());
        app.redo();
        app.redo();
        let expected = app.project.clone();
        app.save_file();
        assert!(app.file_path.ends_with("project.CCW"));
        // A second save exercises replacement of an existing document.
        app.save_file();
        assert!(app.status.starts_with("Projeto salvo"));
        app.replace_project(Project::default());
        app.open_file();
        assert_eq!(app.project, expected);
        assert_eq!(width(&app), 1.2);
    }
}
