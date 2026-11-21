use civil_core::{Element, Project};
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Sense, Shape, Stroke, Vec2};
use std::{fs, io::Write, path::Path};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tool {
    Select,
    Wall,
    Door,
}

struct StudioApp {
    project: Project,
    tool: Tool,
    pending_start: Option<[f64; 2]>,
    selected_id: Option<String>,
    active_level: String,
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
            project: Project::example(),
            tool: Tool::Wall,
            pending_start: None,
            selected_id: None,
            active_level: "P0".into(),
            door_width_draft: None,
            thickness_m: 0.20,
            height_m: 2.80,
            zoom: 29.0,
            snap: true,
            file_path: "meu_projeto.bim.json".into(),
            status: "Protótipo 0.1 • Planta 2D e vista isométrica experimental".into(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }
}

impl StudioApp {
    fn save_snapshot(&mut self) {
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

    fn insert_door(&mut self, point: [f64; 2]) {
        let before = self.project.clone();
        match self.project.add_door(&self.active_level, point) {
            Ok(id) => {
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
                self.status = "Largura da porta atualizada".into();
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
        let draft = &mut self.door_width_draft;
        if draft.as_ref().is_none_or(|(owner, _)| owner != id) {
            *draft = Some((id.into(), geometry.width_m.to_string()));
        }
        if let Some((_, text)) = draft {
            ui.label("Largura da porta (m)");
            ui.text_edit_singleline(text);
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
        match self
            .project
            .validate()
            .and_then(|_| serde_json::to_string_pretty(&self.project).map_err(|e| e.to_string()))
            .and_then(|json| {
                write_project(Path::new(self.file_path.trim()), &json).map_err(|e| e.to_string())
            }) {
            Ok(()) => self.status = format!("Projeto salvo: {}", self.file_path),
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
        ui.heading("Planta 2D");
        ui.label(match self.tool {
            Tool::Wall => "Marque o início e o fim da parede.",
            Tool::Door => "Clique para posicionar a porta no pavimento ativo.",
            Tool::Select => "Clique em um elemento para selecioná-lo.",
        });
        let available = ui.available_size();
        let desired = Vec2::new(available.x.max(180.0), available.y.max(180.0));
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

        for element in &self.project.elements {
            if element.level_id() != self.active_level {
                continue;
            }
            match element {
                Element::Slab(slab) => {
                    let polygon: Vec<Pos2> =
                        slab.polygon.iter().copied().map(world_to_screen).collect();
                    painter.add(Shape::convex_polygon(
                        polygon,
                        Color32::from_rgb(54, 65, 74),
                        Stroke::new(1.2, Color32::from_rgb(100, 119, 132)),
                    ));
                }
                Element::Wall(wall) => {
                    let polygon: Vec<Pos2> =
                        wall.footprint().into_iter().map(world_to_screen).collect();
                    let selected = self.selected_id.as_deref() == Some(wall.id.as_str());
                    let fill = if selected {
                        Color32::from_rgb(226, 163, 64)
                    } else {
                        Color32::from_rgb(168, 201, 201)
                    };
                    painter.add(Shape::convex_polygon(
                        polygon,
                        fill,
                        Stroke::new(1.0, Color32::from_rgb(9, 22, 27)),
                    ));
                }
            }
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
                Tool::Select => {
                    self.door_width_draft = None;
                    if let Some((_, id)) = door_hit {
                        self.selected_id = Some(id);
                        return;
                    }
                    let mut closest: Option<(f32, String)> = None;
                    for element in &self.project.elements {
                        if let Element::Wall(wall) = element {
                            if wall.level_id != self.active_level {
                                continue;
                            }
                            let dist = segment_distance(
                                pointer,
                                world_to_screen(wall.start),
                                world_to_screen(wall.end),
                            );
                            if dist < 14.0 && closest.as_ref().is_none_or(|(best, _)| dist < *best)
                            {
                                closest = Some((dist, wall.id.clone()));
                            }
                        }
                    }
                    self.selected_id = closest.map(|(_, id)| id);
                }
            }
        }
    }

    fn save_snapshot_from(&mut self, before: Project) {
        if self.undo_stack.len() >= 50 {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(before);
        self.redo_stack.clear();
    }

    fn draw_iso(&self, ui: &mut egui::Ui) {
        ui.heading("Vista isométrica");
        ui.label("Prévia geométrica; ainda não é um viewport 3D completo.");
        let available = ui.available_size();
        let desired = Vec2::new(available.x.max(180.0), available.y.max(180.0));
        let (_response, painter) = ui.allocate_painter(desired, Sense::hover());
        let rect = painter.clip_rect();
        painter.rect_filled(
            rect,
            egui::CornerRadius::ZERO,
            Color32::from_rgb(32, 40, 51),
        );
        let origin = rect.center() + Vec2::new(0.0, 72.0);
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
                painter.add(Shape::convex_polygon(
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
                let z1 = z0 + wall.height_m;
                let footprint = wall.footprint();
                let base: Vec<Pos2> = footprint.iter().map(|p| iso([p[0], p[1], z0])).collect();
                let top: Vec<Pos2> = footprint.iter().map(|p| iso([p[0], p[1], z1])).collect();
                let selected = self.selected_id.as_deref() == Some(wall.id.as_str());
                let fill = if selected {
                    Color32::from_rgb(196, 147, 79)
                } else {
                    Color32::from_rgb(140, 162, 174)
                };
                for i in 0..4 {
                    let next = (i + 1) % 4;
                    painter.add(Shape::convex_polygon(
                        vec![base[i], base[next], top[next], top[i]],
                        fill,
                        Stroke::new(0.8, Color32::from_rgb(38, 54, 66)),
                    ));
                }
                painter.add(Shape::convex_polygon(
                    top,
                    Color32::from_rgb(190, 212, 220),
                    Stroke::new(1.1, Color32::from_rgb(42, 61, 70)),
                ));
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
        painter.text(
            rect.left_top() + Vec2::new(12.0, 12.0),
            Align2::LEFT_TOP,
            "MODELO CONCEITUAL • SEM CÁLCULO ESTRUTURAL",
            FontId::monospace(11.0),
            Color32::from_rgb(202, 218, 224),
        );
    }
}

fn write_project(path: &Path, json: &str) -> std::io::Result<()> {
    let directory = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temporary = tempfile::NamedTempFile::new_in(directory)?;
    temporary.write_all(json.as_bytes())?;
    temporary.as_file().sync_all()?;
    temporary.persist(path)?;
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

impl eframe::App for StudioApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::top("toolbar").show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.heading("PROJETO 2  /  CAD+BIM");
                ui.separator();
                if ui.button("Novo").clicked() {
                    self.replace_project(Project::default());
                    self.status = "Novo projeto".into();
                }
                if ui.button("Exemplo").clicked() {
                    self.replace_project(Project::example());
                    self.status = "Projeto demonstrativo carregado".into();
                }
                if ui.button("Abrir").clicked() {
                    self.open_file();
                }
                if ui.button("Salvar").clicked() {
                    self.save_file();
                }
                ui.separator();
                if ui
                    .add_enabled(!self.undo_stack.is_empty(), egui::Button::new("↶ Desfazer"))
                    .clicked()
                {
                    self.undo();
                }
                if ui
                    .add_enabled(!self.redo_stack.is_empty(), egui::Button::new("↷ Refazer"))
                    .clicked()
                {
                    self.redo();
                }
            });
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.tool, Tool::Select, "Selecionar");
                if ui
                    .selectable_value(&mut self.tool, Tool::Wall, "Parede")
                    .clicked()
                {
                    self.pending_start = None;
                }
                ui.selectable_value(&mut self.tool, Tool::Door, "Porta");
                if self.tool != Tool::Wall {
                    self.pending_start = None;
                }
                ui.separator();
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
                ui.checkbox(&mut self.snap, "Snap 0,25 m");
                ui.add(egui::Slider::new(&mut self.zoom, 12.0..=65.0).text("Zoom"));
                if ui.button("Cancelar desenho").clicked() {
                    self.pending_start = None;
                }
            });
        });

        egui::Panel::left("navigator")
            .resizable(true)
            .default_size(225.0)
            .show(ui, |ui| {
                ui.heading("Navegador BIM");
                ui.label("Projeto");
                ui.text_edit_singleline(&mut self.project.name);
                ui.separator();
                ui.strong("Pavimentos");
                for level in &self.project.levels {
                    if ui
                        .selectable_label(
                            self.active_level == level.id,
                            format!("{}  •  {:.2} m", level.name, level.elevation_m),
                        )
                        .clicked()
                    {
                        self.active_level = level.id.clone();
                        self.pending_start = None;
                    }
                }
                ui.separator();
                ui.strong("Elementos");
                egui::ScrollArea::vertical()
                    .max_height(280.0)
                    .show(ui, |ui| {
                        for element in &self.project.elements {
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
                                self.tool = Tool::Select;
                            }
                        }
                        for placed in &self.project.component_instances {
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
                if let Some(selected_id) = self.selected_id.clone() {
                    self.door_inspector(ui, &selected_id);
                    if ui.button(format!("Excluir {selected_id}")).clicked() {
                        self.save_snapshot();
                        self.project.elements.retain(|e| e.id() != selected_id);
                        self.project
                            .component_instances
                            .retain(|placed| placed.instance.id != selected_id);
                        self.door_width_draft = None;
                        self.selected_id = None;
                        self.status = "Elemento excluído".into();
                    }
                }
                ui.separator();
                ui.strong("Quantitativos brutos");
                let q = self.project.quantities();
                ui.label(format!("Paredes: {:.2} m", q.wall_length_m));
                ui.label(format!("Área bruta: {:.2} m²", q.wall_gross_area_m2));
                ui.label(format!("Volume paredes: {:.2} m³", q.wall_gross_volume_m3));
                ui.label(format!("Área de lajes: {:.2} m²", q.slab_area_m2));
                ui.label(format!("Volume lajes: {:.2} m³", q.slab_volume_m3));
                ui.small("Sem dedução de encontros ou aberturas.");
            });

        egui::Panel::bottom("status").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("Arquivo:");
                ui.text_edit_singleline(&mut self.file_path);
            });
            ui.label(&self.status);
        });
        egui::CentralPanel::default().show(ui, |ui| {
            ui.columns(2, |columns| {
                self.draw_plan(&mut columns[0]);
                self.draw_iso(&mut columns[1]);
            });
        });
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
        "Projeto 2 | CAD+BIM — Protótipo 0.1",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
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
    fn door_edit_history_and_file_round_trip() {
        let directory = tempfile::tempdir().unwrap();
        let mut app = StudioApp {
            file_path: directory
                .path()
                .join("project.bim.json")
                .to_str()
                .unwrap()
                .into(),
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
        // A second save exercises replacement of an existing document.
        app.save_file();
        assert!(app.status.starts_with("Projeto salvo"));
        app.replace_project(Project::default());
        app.open_file();
        assert_eq!(app.project, expected);
        assert_eq!(width(&app), 1.2);
    }
}
