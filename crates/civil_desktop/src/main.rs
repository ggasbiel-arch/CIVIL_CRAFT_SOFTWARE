mod ifc;
mod normas;
use civil_core::{Element, Project};
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Sense, Shape, Stroke, Vec2};
use std::{fs, io::Write, path::Path};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tool {
    Select,
    Wall,
    Door,
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

struct StudioApp {
    normas: normas::Biblioteca,
    ifc: ifc::IfcInspector,
    modeling_mode: ModelingMode,
    project: Project,
    tool: Tool,
    view: View,
    show_navigator: bool,
    show_inspector: bool,
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
            normas: Default::default(),
            ifc: Default::default(),
            modeling_mode: ModelingMode::Parametric,
            project: Project::example(),
            tool: Tool::Select,
            view: View::Plan,
            show_navigator: true,
            show_inspector: true,
            pending_start: None,
            selected_id: None,
            active_level: "P0".into(),
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
            Tool::Door => "Clique para posicionar a porta no pavimento ativo.",
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

        let (slabs, walls) = (
            self.project
                .elements
                .iter()
                .filter(|element| matches!(element, Element::Slab(_))),
            self.project
                .elements
                .iter()
                .filter(|element| matches!(element, Element::Wall(_))),
        );
        for element in slabs.chain(walls) {
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
                    ui.checkbox(&mut self.show_navigator, "Navegador BIM");
                    ui.checkbox(&mut self.show_inspector, "Propriedades");
                });
                ui.menu_button("Ajuda", |ui| {
                    if ui.button("Biblioteca de normas ABNT…").clicked() {
                        self.normas.aberta = true;
                        ui.close();
                    }
                    ui.separator();
                    ui.strong("Guia rápido");
                    ui.label("Escolha o modo de modelagem na faixa superior.");
                    ui.label("Para desenhar uma parede, marque o início e o fim na planta.");
                    ui.label("Selecione um elemento para consultar suas propriedades.");
                    ui.label("Para salvar, informe o caminho do projeto em Arquivo.");
                });
            });
        });
        self.normas.mostrar(ui.ctx());
        self.ifc.show(ui.ctx());
        egui::Panel::top("modeling_mode").show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
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
            self.navigator(ui);
        }
        if self.show_inspector {
            self.inspector(ui);
        }

        egui::CentralPanel::default().show(ui, |ui| match self.view {
            View::Plan => self.draw_plan(ui),
            View::Isometric => self.draw_iso(ui),
            View::Split => self.split_views(ui),
        });
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
        egui::Panel::left("navigator")
            .resizable(true)
            .default_size(192.0)
            .size_range(160.0..=240.0)
            .show(ui, |ui| {
                ui.heading("Navegador BIM");
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
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
                                self.active_level = element.level_id().into();
                                self.pending_start = None;
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
                        ui.separator();
                        ui.collapsing("Quantitativos brutos", |ui| {
                            let q = self.project.quantities();
                            ui.label(format!("Paredes: {:.2} m", q.wall_length_m));
                            ui.label(format!("Área de paredes: {:.2} m²", q.wall_gross_area_m2));
                            ui.label(format!(
                                "Volume de paredes: {:.2} m³",
                                q.wall_gross_volume_m3
                            ));
                            ui.label(format!("Área de lajes: {:.2} m²", q.slab_area_m2));
                            ui.label(format!("Volume de lajes: {:.2} m³", q.slab_volume_m3));
                        });
                    });
            });
    }

    fn inspector(&mut self, ui: &mut egui::Ui) {
        egui::Panel::right("inspector")
            .resizable(true)
            .default_size(192.0)
            .size_range(160.0..=240.0)
            .show(ui, |ui| {
                ui.heading("Propriedades");
                ui.separator();
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if self.tool == Tool::Wall {
                            ui.strong("Próxima parede");
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
                    });
            });
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
                    ui.label(format!("Altura: {:.2} m", wall.height_m));
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
        ui.separator();
        if ui.button("Excluir elemento").clicked() {
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
