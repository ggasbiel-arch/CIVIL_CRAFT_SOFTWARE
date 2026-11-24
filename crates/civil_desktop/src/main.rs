use civil_core::{Level, Project};
use civil_core::documentation::{self, Sheet, SheetPlacement};
use civil_core::navigation::{
    Camera, ProjectionMode, PublishFormat, PublishItem, PublishSource, PublisherSet, SavedView,
    Schedule, ScheduleKind, ScreenBounds, ViewpointKind, Workspace, select_projected,
    selection_mode, SelectionMode,
};
use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, Vec2, PointerButton};
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq)]
enum NavigatorTab { ProjectMap, SavedViews, Publisher }

struct StudioApp {
    project: Project,
    file_path: String,
    status: String,
    tab: NavigatorTab,
    active_level: String,
    active_viewpoint: String,
    selected_view: String,
    selected_schedule: String,
    selected_sheet: String,
    sheet_name: String,
    sheet_number: String,
    preview_sheet: bool,
    active_publisher: String,
    selected_markers: Vec<String>,
    drag_start: Option<Pos2>,
    drag_end: Option<Pos2>,
    orbit_tool: bool,
    project_info_open: bool,
    story_name: String,
    story_elevation: f64,
    viewpoint_name: String,
    saved_view_name: String,
    schedule_name: String,
    schedule_kind: ScheduleKind,
    publisher_name: String,
    publisher_dir: String,
    custom_key: String,
    custom_value: String,
}

impl Default for StudioApp {
    fn default() -> Self {
        let project = Project::default();
        let active_viewpoint = project.workspace.viewpoints.first()
            .map(|v| v.id.clone()).unwrap_or_default();
        Self {
            project,
            file_path: "meu_projeto.bim.json".into(),
            status: "Projeto novo. Use Arquivo > Informações do projeto para cadastrar os dados.".into(),
            tab: NavigatorTab::ProjectMap,
            active_level: "P0".into(),
            active_viewpoint,
            selected_view: String::new(),
            selected_schedule: String::new(),
            selected_sheet: String::new(),
            sheet_name: "Prancha de arquitetura".into(),
            sheet_number: "A-01".into(),
            preview_sheet: false,
            active_publisher: String::new(),
            selected_markers: Vec::new(),
            drag_start: None, drag_end: None,
            orbit_tool: false, project_info_open: false,
            story_name: "Novo pavimento".into(), story_elevation: 3.0,
            viewpoint_name: String::new(),
            saved_view_name: "Vista 3D".into(),
            schedule_name: "Tabela de pavimentos".into(),
            schedule_kind: ScheduleKind::Levels,
            publisher_name: "Emissão 01".into(), publisher_dir: "publicacoes".into(),
            custom_key: String::new(), custom_value: String::new(),
        }
    }
}

impl StudioApp {
    fn replace_project(&mut self, mut project: Project) {
        if project.workspace.viewpoints.is_empty() {
            // Projects saved with the original schema had no navigator section.
            project.workspace = Workspace::new_project(&project.levels);
        }
        self.active_level = project.levels.first().map(|l| l.id.clone()).unwrap_or_default();
        self.active_viewpoint = project.workspace.viewpoints.first().map(|v| v.id.clone()).unwrap_or_default();
        self.active_publisher.clear();
        self.selected_markers.clear();
        self.selected_view.clear();
        self.selected_schedule.clear();
        self.selected_sheet.clear();
        self.preview_sheet = false;
        self.project = project;
    }

    fn new_project(&mut self) {
        self.replace_project(Project::default());
        self.status = "Novo projeto vazio criado".into();
    }

    fn save_project(&mut self) {
        let result = self.project.validate()
            .and_then(|_| serde_json::to_string_pretty(&self.project).map_err(|e| e.to_string()))
            .and_then(|text| fs::write(self.file_path.trim(), text).map_err(|e| e.to_string()));
        self.status = match result {
            Ok(()) => format!("Projeto salvo: {}", self.file_path.trim()),
            Err(err) => format!("Erro ao salvar projeto: {err}"),
        };
    }

    fn open_project(&mut self) {
        let result = fs::read_to_string(self.file_path.trim())
            .map_err(|e| e.to_string())
            .and_then(|text| serde_json::from_str::<Project>(&text).map_err(|e| e.to_string()))
            .and_then(|mut project| {
                if project.workspace.viewpoints.is_empty() {
                    project.workspace = Workspace::new_project(&project.levels);
                }
                project.validate()?;
                Ok(project)
            });
        match result {
            Ok(project) => {
                self.replace_project(project);
                self.status = format!("Projeto aberto: {}", self.file_path.trim());
            }
            Err(err) => self.status = format!("Erro ao abrir projeto: {err}"),
        }
    }

    fn add_story(&mut self) {
        let name = self.story_name.trim();
        if name.is_empty() || !self.story_elevation.is_finite() {
            self.status = "Informe nome e cota válidos para o pavimento".into();
            return;
        }
        let mut num = self.project.levels.len();
        let id = loop {
            let candidate = format!("P{num}");
            if self.project.levels.iter().all(|l| l.id != candidate) { break candidate; }
            num += 1;
        };
        self.project.levels.push(Level {
            id: id.clone(), name: name.to_owned(), elevation_m: self.story_elevation,
        });
        self.project.workspace.new_viewpoint(
            format!("Planta — {name}"), ViewpointKind::FloorPlan, Some(id.clone())
        );
        self.active_level = id;
        self.story_name = "Novo pavimento".into();
        self.story_elevation += 3.0;
        self.status = "Pavimento e ponto de vista da planta criados".into();
    }

    fn add_viewpoint(&mut self, kind: ViewpointKind) {
        let title = if self.viewpoint_name.trim().is_empty() {
            format!("{} {}", kind.label(), self.project.workspace.viewpoints.len() + 1)
        } else { self.viewpoint_name.trim().to_owned() };
        let level_id = if kind == ViewpointKind::ThreeD { None }
            else { Some(self.active_level.clone()) };
        let id = self.project.workspace.new_viewpoint(title.clone(), kind, level_id);
        let n = self.project.workspace.viewpoints.len();
        if let Some(item) = self.project.workspace.viewpoints.last_mut() {
            let z = self.project.levels.iter()
                .find(|l| item.level_id.as_ref() == Some(&l.id))
                .map_or(0.0, |l| l.elevation_m);
            item.marker = [
                -6.0 + ((n - 1) % 7) as f64 * 2.0,
                -4.0 + ((n - 1) / 7) as f64 * 2.0,
                z,
            ];
        }
        self.active_viewpoint = id;
        self.viewpoint_name.clear();
        self.status = format!("Ponto de vista criado: {title}");
    }

    fn activate_viewpoint(&mut self, id: &str) {
        if let Some(item) = self.project.workspace.viewpoints.iter().find(|v|v.id == id) {
            self.active_viewpoint = id.to_owned();
            self.preview_sheet = false;
            self.project.workspace.camera = item.camera.clone();
            if let Some(level) = &item.level_id { self.active_level = level.clone(); }
            self.status = format!("Ponto de vista: {}", item.title);
        }
    }

    fn capture_saved_view(&mut self) {
        let title = self.saved_view_name.trim();
        if title.is_empty() {
            self.status = "Informe um nome para a vista salva".into();
            return;
        }
        let id = self.active_viewpoint.clone();
        let camera = self.project.workspace.camera.clone();
        match self.project.workspace.save_view(&id, title.to_owned(), camera) {
            Ok(new_id) => {
                self.selected_view = new_id;
                self.tab = NavigatorTab::SavedViews;
                self.status = format!("Vista salva: {title}");
            }
            Err(err) => self.status = err,
        }
    }

    fn activate_saved_view(&mut self, id: &str) {
        if let Some(view) = self.project.workspace.saved_views.iter().find(|v|v.id == id) {
            self.project.workspace.camera = view.camera.clone();
            self.active_viewpoint = view.viewpoint_id.clone();
            self.selected_view = id.into();
            self.preview_sheet = false;
            self.status = format!("Cena restaurada: {}", view.title);
        }
    }

    fn add_schedule(&mut self) {
        let title = self.schedule_name.trim().to_owned();
        if title.is_empty() { self.status = "Informe um nome para a tabela".into(); return; }
        let id = self.project.workspace.allocate_id("table");
        self.project.workspace.schedules.push(Schedule {
            id: id.clone(), title: title.clone(), kind: self.schedule_kind,
        });
        self.selected_schedule = id;
        self.status = format!("Tabela criada: {title}");
    }

    fn add_sheet(&mut self) {
        let title = self.sheet_name.trim();
        let number = self.sheet_number.trim();
        if title.is_empty() || number.is_empty() {
            self.status = "Informe nome e número da prancha".into();
            return;
        }
        let id = self.project.workspace.allocate_id("sheet");
        self.project.workspace.sheets.push(Sheet::a3_landscape(
            id.clone(), number.to_owned(), title.to_owned(),
        ));
        self.selected_sheet = id;
        self.preview_sheet = true;
        self.status = "Prancha A3 criada. Adicione uma vista salva para preencher.".into();
    }

    fn add_view_to_sheet(&mut self) {
        let view_id = self.selected_view.clone();
        let Some(saved) = self.project.workspace.saved_views.iter()
            .find(|v|v.id == view_id) else {
            self.status = "Selecione primeiro uma vista salva".into();
            return;
        };
        let id = self.project.workspace.allocate_id("placement");
        let Some(sheet) = self.project.workspace.sheets.iter_mut()
            .find(|s|s.id == self.selected_sheet) else {
            self.status = "Crie ou selecione uma prancha".into();
            return;
        };
        let count = sheet.placements.len();
        if count >= 4 {
            self.status = "Limite inicial: quatro vistas por prancha A3".into();
            return;
        }
        let frame = SheetPlacement {
            id, saved_view_id:view_id,
            x_mm: 18.0 + (count%2) as f64 * 190.0,
            y_mm: 22.0 + (count/2) as f64 * 110.0,
            width_mm: 178.0,
            height_mm: 102.0,
        };
        if let Err(err) = sheet.validate() {
            self.status = format!("Prancha inválida: {err}");
            return;
        }
        sheet.placements.push(frame);
        self.preview_sheet = true;
        self.status = "Vista vinculada à prancha. Prévia associativa atualizada.".into();
    }

    fn export_selected_sheet(&mut self) {
        let Some(sheet) = self.project.workspace.sheets.iter()
            .find(|s|s.id == self.selected_sheet) else {
            self.status = "Selecione uma prancha".into();
            return;
        };
        match documentation::export_sheet_pdf(&self.project,sheet) {
            Ok(bytes) => {
                let dir = PathBuf::from(self.publisher_dir.trim());
                if let Err(err) = fs::create_dir_all(&dir) {
                    self.status = format!("Erro ao criar diretório: {err}");
                    return;
                }
                let safe_name: String = sheet.number.chars().map(|ch|
                    if ch.is_ascii_alphanumeric() || ch=='-' || ch=='_' {ch} else {'_'}
                ).collect();
                let path = dir.join(format!("Prancha_{safe_name}.pdf"));
                match fs::write(&path,bytes) {
                    Ok(()) => self.status=format!("PDF vetorial exportado: {}",path.display()),
                    Err(err)=>self.status=format!("Erro ao exportar PDF: {err}"),
                }
            }
            Err(err)=>self.status=format!("Falha ao publicar prancha: {err}"),
        }
    }

    fn add_publisher(&mut self) {
        let title = self.publisher_name.trim().to_owned();
        if title.is_empty() { self.status = "Informe o nome do conjunto".into(); return; }
        let id = self.project.workspace.allocate_id("publish");
        self.project.workspace.publisher_sets.push(PublisherSet {
            id: id.clone(), title: title.clone(), items: Vec::new(),
        });
        self.active_publisher = id;
        self.status = format!("Conjunto criado: {title}");
    }

    fn append_publisher_item(&mut self, source: PublishSource) {
        let source_id = match source {
            PublishSource::SavedView => self.selected_view.clone(),
            PublishSource::Schedule => self.selected_schedule.clone(),
            PublishSource::Sheet => self.selected_sheet.clone(),
        };
        let exists = match source {
            PublishSource::SavedView => self.project.workspace.saved_views.iter()
                .any(|v| v.id == source_id),
            PublishSource::Schedule => self.project.workspace.schedules.iter()
                .any(|v| v.id == source_id),
            PublishSource::Sheet => self.project.workspace.sheets.iter()
                .any(|v| v.id == source_id),
        };
        if !exists {
            self.status = "Selecione primeiro uma vista ou tabela na aba Vistas e Tabelas".into();
            return;
        }
        let id = self.project.workspace.allocate_id("item");
        let set = self.project.workspace.publisher_sets.iter_mut()
            .find(|set|set.id == self.active_publisher);
        if let Some(set) = set {
            set.items.push(PublishItem {
                id, source, source_id,
                format: match source {
                    PublishSource::SavedView=>PublishFormat::Json,
                    PublishSource::Schedule=>PublishFormat::Csv,
                    PublishSource::Sheet=>PublishFormat::Pdf,
                },
            });
            self.status = "Item vinculado ao conjunto publicador".into();
        } else {
            self.status = "Crie ou selecione um conjunto publicador".into();
        }
    }

    fn publish_active_set(&mut self) {
        if let Err(err) = self.project.validate() {
            self.status = format!("Publicação bloqueada: {err}");
            return;
        }
        let Some(set) = self.project.workspace.publisher_sets.iter()
            .find(|s|s.id == self.active_publisher).cloned() else {
            self.status = "Selecione um conjunto publicador".into(); return;
        };
        let dest = PathBuf::from(self.publisher_dir.trim());
        if self.publisher_dir.trim().is_empty() {
            self.status = "Informe a pasta de publicação".into(); return;
        }
        if let Err(err) = fs::create_dir_all(&dest) {
            self.status = format!("Erro ao criar pasta: {err}"); return;
        }
        let mut count = 0;
        for item in &set.items {
            let output: Result<(String, Vec<u8>), String> = match item.source {
                PublishSource::SavedView => {
                    self.project.workspace.saved_views.iter()
                        .find(|v|v.id == item.source_id)
                        .ok_or("Vista inexistente".to_owned())
                        .and_then(|v| {
                            serde_json::to_string_pretty(v)
                                .map(|data|(String::from("json"), data.into_bytes()))
                                .map_err(|err|err.to_string())
                        })
                }
                PublishSource::Schedule => {
                    self.project.workspace.schedules.iter()
                        .find(|v|v.id == item.source_id)
                        .ok_or("Tabela inexistente".to_owned())
                        .map(|s| (String::from("csv"), schedule_csv(&self.project, s.kind).into_bytes()))
                }
                PublishSource::Sheet => self.project.workspace.sheets.iter()
                    .find(|s|s.id==item.source_id)
                    .ok_or("Prancha não encontrada".to_owned())
                    .and_then(|s| documentation::export_sheet_pdf(&self.project,s)
                        .map(|pdf|("pdf".to_owned(),pdf))),
            };
            match output {
                Ok((ext, data)) => {
                    let safe_id: String = item.id.chars().map(|ch| {
                        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' { ch }
                        else { '_' }
                    }).collect();
                    let filepath = dest.join(format!("{safe_id}.{ext}"));
                    if let Err(err) = fs::write(&filepath, data) {
                        self.status = format!("Publicação interrompida: {err}");
                        return;
                    }
                    count += 1;
                }
                Err(err) => { self.status = err; return; }
            }
        }
        self.status = format!("{count} arquivo(s) exportado(s) para {}", dest.display());
    }

    fn project_map(&mut self, ui: &mut egui::Ui) {
        ui.heading("Mapa do Projeto");
        ui.small("Pavimentos e pontos de vista");
        ui.separator();
        egui::CollapsingHeader::new("Pavimentos").default_open(true).show(ui, |ui| {
            let levels: Vec<_> = self.project.levels.iter().map(|l|
                (l.id.clone(),l.name.clone(),l.elevation_m)
            ).collect();
            for (id,name,elevation) in levels {
                if ui.selectable_label(self.active_level == id,
                    format!("{name}  ({elevation:.2} m)")).clicked() {
                    self.active_level = id;
                }
            }
            ui.separator();
            ui.label("Novo pavimento");
            ui.text_edit_singleline(&mut self.story_name);
            ui.horizontal(|ui| {
                ui.label("Cota (m)");
                ui.add(egui::DragValue::new(&mut self.story_elevation).speed(0.1));
            });
            if ui.button("+ Criar pavimento").clicked() { self.add_story(); }
        });
        ui.separator();
        egui::CollapsingHeader::new("Pontos de vista").default_open(true).show(ui, |ui| {
            for kind in [
                ViewpointKind::ThreeD,ViewpointKind::FloorPlan,
                ViewpointKind::Section,ViewpointKind::Elevation,ViewpointKind::Detail
            ] {
                let points:Vec<_> = self.project.workspace.viewpoints.iter()
                    .filter(|v|v.kind == kind)
                    .map(|v|(v.id.clone(),v.title.clone())).collect();
                egui::CollapsingHeader::new(kind.label()).default_open(true).show(ui, |ui| {
                    for (id,title) in points {
                        if ui.selectable_label(self.active_viewpoint == id,title).clicked() {
                            self.activate_viewpoint(&id);
                        }
                    }
                });
            }
            ui.separator();
            if let Some(view) = self.project.workspace.viewpoints.iter_mut()
                .find(|v|v.id==self.active_viewpoint) {
                if matches!(view.kind, ViewpointKind::Section|ViewpointKind::Elevation) {
                    ui.separator();
                    ui.label("Plano de corte / direção de elevação");
                    ui.horizontal(|ui| {
                        ui.label("X (m)");
                        ui.add(egui::DragValue::new(&mut view.marker[0]).speed(0.1));
                        ui.label("Y (m)");
                        ui.add(egui::DragValue::new(&mut view.marker[1]).speed(0.1));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Orientação (°)");
                        ui.add(egui::DragValue::new(&mut view.camera.yaw_deg).speed(1.0));
                    });
                }
            }
            ui.text_edit_singleline(&mut self.viewpoint_name);
            ui.horizontal_wrapped(|ui| {
                if ui.button("+ 3D").clicked() { self.add_viewpoint(ViewpointKind::ThreeD); }
                if ui.button("+ Corte").clicked() { self.add_viewpoint(ViewpointKind::Section); }
                if ui.button("+ Elevação").clicked() { self.add_viewpoint(ViewpointKind::Elevation); }
                if ui.button("+ Detalhe").clicked() { self.add_viewpoint(ViewpointKind::Detail); }
                if ui.button("+ Planta").clicked() { self.add_viewpoint(ViewpointKind::FloorPlan); }
            });
        });
        ui.separator();
        ui.small("Cortes, elevações e plantas geram linhas dos elementos gravados. Ainda sem ocultação de arestas.");
    }

    fn saved_views_map(&mut self, ui: &mut egui::Ui) {
        ui.heading("Vistas e Tabelas");
        ui.separator();
        ui.label("Salvar a câmera atual como vista");
        ui.text_edit_singleline(&mut self.saved_view_name);
        if ui.button("Salvar vista atual").clicked() { self.capture_saved_view(); }
        egui::CollapsingHeader::new("Vistas salvas").default_open(true).show(ui, |ui| {
            let views: Vec<_> = self.project.workspace.saved_views.iter()
                .map(|v|(v.id.clone(),v.title.clone())).collect();
            for (id,title) in views {
                if ui.selectable_label(self.selected_view == id,title).clicked() {
                    self.activate_saved_view(&id);
                }
            }
        });
        if let Some(view) = self.project.workspace.saved_views.iter_mut()
            .find(|v|v.id==self.selected_view) {
            ui.separator();
            ui.label("Definições da vista");
            ui.label("Nome");
            ui.text_edit_singleline(&mut view.title);
            ui.label("Escala");
            ui.text_edit_singleline(&mut view.scale);
            ui.label("Combinação de camadas");
            ui.text_edit_singleline(&mut view.layer_preset);
            ui.label("Observações");
            ui.text_edit_multiline(&mut view.notes);
        }
        ui.separator();
        egui::CollapsingHeader::new("Tabelas").default_open(true).show(ui, |ui| {
            let schedules:Vec<_>=self.project.workspace.schedules.iter().map(|s|
                (s.id.clone(),s.title.clone(),s.kind)).collect();
            for (id,title,kind) in schedules {
                if ui.selectable_label(self.selected_schedule == id,
                    format!("{} · {}",title,kind.label())).clicked() {
                    self.selected_schedule=id;
                }
            }
            ui.separator();
            ui.label("Nova tabela");
            ui.text_edit_singleline(&mut self.schedule_name);
            egui::ComboBox::from_id_salt("schedule_kind")
                .selected_text(self.schedule_kind.label())
                .show_ui(ui,|ui| {
                    for kind in [ScheduleKind::Levels,ScheduleKind::Elements,ScheduleKind::Views] {
                        ui.selectable_value(&mut self.schedule_kind,kind,kind.label());
                    }
                });
            if ui.button("+ Criar tabela").clicked() { self.add_schedule(); }
        });
        if let Some(table)=self.project.workspace.schedules.iter().find(|s|s.id==self.selected_schedule) {
            ui.separator();
            ui.strong(format!("Prévia: {}",table.title));
            for line in schedule_csv(&self.project,table.kind).lines().take(9) { ui.small(line); }
        }
        self.sheet_map(ui);
    }

    fn sheet_map(&mut self, ui: &mut egui::Ui) {
        ui.separator();
        egui::CollapsingHeader::new("Pranchas / Layouts").default_open(true).show(ui, |ui| {
            ui.label("Nova prancha (A3 paisagem)");
            ui.text_edit_singleline(&mut self.sheet_number);
            ui.text_edit_singleline(&mut self.sheet_name);
            if ui.button("+ Criar prancha").clicked() { self.add_sheet(); }
            let sheets: Vec<_> = self.project.workspace.sheets.iter()
                .map(|s|(s.id.clone(),s.number.clone(),s.title.clone())).collect();
            for (id,number,title) in sheets {
                if ui.selectable_label(self.selected_sheet==id,
                    format!("{number} · {title}")).clicked() {
                    self.selected_sheet=id;
                    self.preview_sheet=true;
                }
            }
            if !self.selected_sheet.is_empty() {
                if let Some(sheet)=self.project.workspace.sheets.iter()
                    .find(|s|s.id==self.selected_sheet) {
                    ui.label(format!("{} vista(s) vinculada(s)",sheet.placements.len()));
                    for place in &sheet.placements {
                        if let Some(view)=self.project.workspace.saved_views.iter()
                            .find(|v|v.id==place.saved_view_id) {
                            ui.small(format!("• {}",view.title));
                        }
                    }
                }
                if ui.button("Adicionar vista selecionada à prancha").clicked() {
                    self.add_view_to_sheet();
                }
                if ui.button("Pré-visualizar prancha").clicked() { self.preview_sheet=true; }
                if ui.button("Exportar prancha PDF").clicked() {
                    self.export_selected_sheet();
                }
            }
        });
    }

    fn publisher_map(&mut self, ui: &mut egui::Ui) {
        ui.heading("Conjuntos Publicadores");
        ui.separator();
        ui.text_edit_singleline(&mut self.publisher_name);
        if ui.button("+ Novo conjunto").clicked() { self.add_publisher(); }
        let sets:Vec<_>=self.project.workspace.publisher_sets.iter()
            .map(|s|(s.id.clone(),s.title.clone())).collect();
        for (id,title) in sets {
            if ui.selectable_label(self.active_publisher == id,title).clicked() {
                self.active_publisher=id;
            }
        }
        if self.active_publisher.is_empty() { return; }
        ui.separator();
        if ui.button("Adicionar vista selecionada").clicked() {
            self.append_publisher_item(PublishSource::SavedView);
        }
        if ui.button("Adicionar tabela selecionada").clicked() {
            self.append_publisher_item(PublishSource::Schedule);
        }
        if ui.button("Adicionar prancha selecionada").clicked() {
            self.append_publisher_item(PublishSource::Sheet);
        }
        if let Some(set) = self.project.workspace.publisher_sets.iter()
            .find(|s|s.id==self.active_publisher) {
            ui.strong(format!("Itens de {}",set.title));
            for item in &set.items {
                let label = match item.source {
                    PublishSource::SavedView=>"Vista (JSON)",
                    PublishSource::Schedule=>"Tabela (CSV)",
                    PublishSource::Sheet=>"Prancha (PDF)",
                };
                ui.label(format!("{} · {}",label,item.source_id));
            }
        }
        ui.separator();
        ui.label("Diretório de publicação");
        ui.text_edit_singleline(&mut self.publisher_dir);
        if ui.button("Publicar conjunto").clicked() { self.publish_active_set(); }
        ui.small("Exporta vistas JSON, tabelas CSV e pranchas com desenhos 2D/3D vetoriais em PDF. IFC e documentação executiva ainda pendentes.");
    }

    fn project_info_window(&mut self, ctx: &egui::Context) {
        if !self.project_info_open { return; }
        egui::Window::new("Arquivo / Informações do Projeto")
            .open(&mut self.project_info_open)
            .default_width(480.0)
            .resizable(true)
            .show(ctx, |ui| {
                ui.label("Nome do projeto");
                ui.text_edit_singleline(&mut self.project.name);
                let info = &mut self.project.information;
                ui.label("Código do projeto");
                ui.text_edit_singleline(&mut info.project_code);
                ui.label("Endereço / Local");
                ui.text_edit_singleline(&mut info.address);
                ui.label("Cliente");
                ui.text_edit_singleline(&mut info.client);
                ui.label("Arquiteto / Responsável");
                ui.text_edit_singleline(&mut info.architect);
                ui.label("Empresa / Engenharia");
                ui.text_edit_singleline(&mut info.engineering_company);
                ui.label("Fase");
                ui.text_edit_singleline(&mut info.phase);
                ui.label("Revisão");
                ui.text_edit_singleline(&mut info.revision);
                ui.label("Descrição");
                ui.text_edit_multiline(&mut info.description);
                ui.separator();
                ui.heading("Campos personalizados");
                let entries: Vec<_> = info.custom.iter()
                    .map(|(k,v)|(k.clone(),v.clone())).collect();
                for (key,value) in entries {
                    ui.horizontal(|ui| {
                        ui.label(format!("{key}: {value}"));
                        if ui.small_button("Excluir").clicked() { info.custom.remove(&key); }
                    });
                }
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut self.custom_key);
                    ui.text_edit_singleline(&mut self.custom_value);
                    if ui.button("Adicionar").clicked() && !self.custom_key.trim().is_empty() {
                        info.custom.insert(self.custom_key.trim().to_owned(),
                            self.custom_value.clone());
                        self.custom_key.clear();
                        self.custom_value.clear();
                    }
                });
                ui.small("As informações são gravadas no arquivo do projeto ao clicar em Salvar.");
            });
    }

    fn top_bar(&mut self, root:&mut egui::Ui) {
        egui::Panel::top("main_menus").show(root, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.strong("CIVIL CRAFT · Projeto 2");
                ui.separator();
                ui.menu_button("Arquivo", |ui| {
                    if ui.button("Novo projeto").clicked() { self.new_project(); ui.close(); }
                    if ui.button("Abrir projeto").clicked() { self.open_project(); ui.close(); }
                    if ui.button("Salvar projeto").clicked() { self.save_project(); ui.close(); }
                    ui.separator();
                    if ui.button("Informações do projeto...").clicked() {
                        self.project_info_open = true;
                        ui.close();
                    }
                });
                if ui.button("Salvar vista").clicked() { self.capture_saved_view(); }
                ui.separator();
                ui.label("Câmera");
                let camera = &mut self.project.workspace.camera;
                egui::ComboBox::from_id_salt("projection_mode")
                    .selected_text(camera.projection.label())
                    .show_ui(ui, |ui| {
                        for mode in [ProjectionMode::Orthographic,
                            ProjectionMode::Perspective,ProjectionMode::TwoPoint] {
                            if ui.selectable_value(&mut camera.projection,mode,mode.label()).clicked()
                                && mode == ProjectionMode::TwoPoint { camera.pitch_deg = 0.0; }
                        }
                    });
                if ui.button("Enquadrar").clicked() { *camera = Camera::default(); }
                ui.toggle_value(&mut self.orbit_tool, "Orbitar [O]");
            });
        });
    }

    fn navigator(&mut self, root:&mut egui::Ui) {
        egui::Panel::left("navigator").resizable(true)
            .default_size(295.0).size_range(235.0..=480.0).show(root, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.selectable_value(&mut self.tab,NavigatorTab::ProjectMap,"Projeto");
                ui.selectable_value(&mut self.tab,NavigatorTab::SavedViews,"Vistas/Tabelas");
                ui.selectable_value(&mut self.tab,NavigatorTab::Publisher,"Publicador");
            });
            ui.separator();
            egui::ScrollArea::vertical().id_salt("navigator-scroll").show(ui, |ui| {
                match self.tab {
                    NavigatorTab::ProjectMap=>self.project_map(ui),
                    NavigatorTab::SavedViews=>self.saved_views_map(ui),
                    NavigatorTab::Publisher=>self.publisher_map(ui),
                }
            });
        });
    }

    fn draw_sheet_preview(&mut self, ui: &mut egui::Ui) {
        let Some(sheet)=self.project.workspace.sheets.iter()
            .find(|s|s.id==self.selected_sheet) else {
            ui.label("Selecione uma prancha no mapa Vistas/Tabelas");
            return;
        };
        ui.heading(format!("Prancha {} — {}",sheet.number,sheet.title));
        ui.small("Pré-visualização vetorial dos elementos armazenados; dimensões do papel em mm.");
        let space=ui.available_size();
        let scale=(space.x/sheet.width_mm as f32)
            .min((space.y-14.0).max(30.0)/sheet.height_mm as f32).max(0.1);
        let target=egui::vec2(sheet.width_mm as f32*scale,
            sheet.height_mm as f32*scale);
        let (paper,_response)=ui.allocate_exact_size(target,Sense::hover());
        let painter=ui.painter().with_clip_rect(paper);
        painter.rect_filled(paper,0.0,Color32::WHITE);
        painter.rect_stroke(paper,0.0,Stroke::new(1.0,Color32::DARK_GRAY),
            egui::StrokeKind::Inside);
        let title_y=paper.bottom()-35.0*scale;
        painter.line_segment([Pos2::new(paper.left()+12.0*scale,title_y),
                Pos2::new(paper.right()-12.0*scale,title_y)],
            Stroke::new(1.0,Color32::BLACK));
        painter.text(Pos2::new(paper.left()+18.0*scale,paper.bottom()-24.0*scale),
            egui::Align2::LEFT_TOP,format!("{} — {}",sheet.number,self.project.name),
            egui::FontId::proportional(12.0),Color32::BLACK);
        for place in &sheet.placements {
            let bounds=Rect::from_min_size(
                Pos2::new(paper.left()+place.x_mm as f32*scale,
                    paper.top()+place.y_mm as f32*scale),
                egui::vec2(place.width_mm as f32*scale,
                    place.height_mm as f32*scale),
            );
            painter.rect_stroke(bounds,0.0,Stroke::new(1.0,Color32::GRAY),
                egui::StrokeKind::Inside);
            let Some(saved)=self.project.workspace.saved_views.iter()
                .find(|v|v.id==place.saved_view_id) else {continue;};
            painter.text(bounds.left_top()+egui::vec2(5.0,4.0),
                egui::Align2::LEFT_TOP,&saved.title,
                egui::FontId::proportional(10.0),Color32::DARK_GRAY);
            let Ok(draw)=documentation::render_saved_view(&self.project,saved) else {continue;};
            if let Some((min,max))=draw.bounds() {
                let w=(max[0]-min[0]).max(0.05);
                let h=(max[1]-min[1]).max(0.05);
                let factor=((bounds.width()-16.0)/w as f32)
                    .min((bounds.height()-32.0)/h as f32);
                let center=Pos2::new(bounds.center().x,bounds.center().y+7.0);
                let cx=(min[0]+max[0])*0.5;
                let cy=(min[1]+max[1])*0.5;
                for line in &draw.lines {
                    let project=|p:[f64;2]| Pos2::new(
                        center.x+((p[0]-cx) as f32)*factor,
                        center.y-((p[1]-cy) as f32)*factor,
                    );
                    painter.line_segment([project(line.start),project(line.end)],
                        Stroke::new(1.0,Color32::from_rgb(38,53,70)));
                }
            }
        }
    }

    fn draw_viewport(&mut self, ui: &mut egui::Ui) {
        let size = ui.available_size().max(Vec2::new(100.0,100.0));
        let (rect, response) = ui.allocate_exact_size(size,Sense::click_and_drag());
        let painter = ui.painter().with_clip_rect(rect);
        painter.rect_filled(rect,0.0,Color32::from_rgb(25,30,37));
        let camera = &self.project.workspace.camera;
        let w=rect.width() as f64;
        let h=rect.height() as f64;
        let to_pos=|xy:[f64;2]| -> Pos2 { Pos2::new(rect.left()+xy[0] as f32,rect.top()+xy[1] as f32) };

        for n in -15..=15 {
            let k=n as f64;
            let gray=if n==0{Color32::from_rgb(88,98,108)}
                else{Color32::from_rgb(49,58,68)};
            let line=(camera.project([-15.0,k,0.0],w,h),
                      camera.project([15.0,k,0.0],w,h));
            if let (Some(a),Some(b))=line {
                painter.line_segment([to_pos(a),to_pos(b)],Stroke::new(1.0,gray));
            }
            let line=(camera.project([k,-15.0,0.0],w,h),
                      camera.project([k,15.0,0.0],w,h));
            if let (Some(a),Some(b))=line {
                painter.line_segment([to_pos(a),to_pos(b)],Stroke::new(1.0,gray));
            }
        }
        for (to,color) in [
            ([4.0,0.0,0.0],Color32::from_rgb(218,93,92)),
            ([0.0,4.0,0.0],Color32::from_rgb(93,182,103)),
            ([0.0,0.0,4.0],Color32::from_rgb(103,155,229)),
        ] {
            if let (Some(origin),Some(dest)) =
                (camera.project([0.0,0.0,0.0],w,h),camera.project(to,w,h)) {
                painter.line_segment([to_pos(origin),to_pos(dest)],Stroke::new(2.5,color));
            }
        }

        // Read-only model linework is derived from stored BIM records, not demo tools.
        if let Some(active) = self.project.workspace.viewpoints.iter()
            .find(|v|v.id == self.active_viewpoint) {
            if active.kind == ViewpointKind::ThreeD {
                for edge in documentation::model_wireframe(&self.project) {
                    if let (Some(a),Some(b))=(camera.project(edge.start,w,h),
                        camera.project(edge.end,w,h)) {
                        painter.line_segment([to_pos(a),to_pos(b)],
                            Stroke::new(1.4,Color32::from_rgb(225,226,231)));
                    }
                }
            } else {
                let drawing=documentation::render_viewpoint(&self.project,active);
                if let Some((minimum,maximum))=drawing.bounds() {
                    let width=(maximum[0]-minimum[0]).max(0.05);
                    let height=(maximum[1]-minimum[1]).max(0.05);
                    let factor=((w-100.0)/width).min((h-100.0)/height);
                    let center=[(minimum[0]+maximum[0])*0.5,
                        (minimum[1]+maximum[1])*0.5];
                    for edge in &drawing.lines {
                        let to_screen=|p:[f64;2]| Pos2::new(
                            rect.center().x+((p[0]-center[0])*factor) as f32,
                            rect.center().y-((p[1]-center[1])*factor) as f32);
                        painter.line_segment([to_screen(edge.start),to_screen(edge.end)],
                            Stroke::new(1.6,Color32::from_rgb(221,223,230)));
                    }
                }
            }
        }

        // Viewpoint markers are real navigator entities, not placeholder walls or floors.
        let mut marker_bounds: Vec<(String,ScreenBounds)> = Vec::new();
        for view in &self.project.workspace.viewpoints {
            if let Some(xy)=camera.project(view.marker,w,h) {
                let point=to_pos(xy);
                if rect.contains(point) {
                    let selected=self.selected_markers.contains(&view.id);
                    painter.circle_filled(point,if selected {7.0}else{4.5},
                        if selected {Color32::YELLOW}else{Color32::from_rgb(102,205,217)});
                    marker_bounds.push((view.id.clone(),ScreenBounds::from_points(
                        [xy[0]-5.0,xy[1]-5.0],[xy[0]+5.0,xy[1]+5.0]
                    )));
                }
            }
        }

        // Camera gestures: Shift+middle or right drag -> orbit; middle -> pan.
        // Left drag -> CAD window/crossing selection, unless Orbit tool is activated.
        let mods=ui.input(|input|input.modifiers);
        if response.dragged_by(PointerButton::Secondary)
            || (response.dragged_by(PointerButton::Middle) && mods.shift)
            || (response.dragged_by(PointerButton::Primary) && self.orbit_tool) {
            let delta=ui.input(|i|i.pointer.delta());
            self.project.workspace.camera.orbit(delta.x as f64,delta.y as f64);
        } else if response.dragged_by(PointerButton::Middle) {
            let delta=ui.input(|i|i.pointer.delta());
            self.project.workspace.camera.pan(delta.x as f64,delta.y as f64,h);
        }

        if response.hovered() {
            let dy=ui.input(|i|i.smooth_scroll_delta.y);
            if dy.abs()>0.01 {
                self.project.workspace.camera.zoom((dy / 140.0) as f64);
            }
        }
        if !self.orbit_tool {
            if response.drag_started_by(PointerButton::Primary) {
                self.drag_start = response.interact_pointer_pos();
                self.drag_end = self.drag_start;
            }
            if response.dragged_by(PointerButton::Primary) {
                self.drag_end = response.interact_pointer_pos();
            }
            if response.drag_stopped_by(PointerButton::Primary) {
                if let (Some(a),Some(b))=(self.drag_start,self.drag_end) {
                    let local = |p:Pos2| [f64::from(p.x-rect.left()),f64::from(p.y-rect.top())];
                    self.selected_markers=select_projected(local(a),local(b),
                        marker_bounds.iter().map(|(id,b)|(id.as_str(),*b)));
                    self.status = format!("{} ponto(s) de vista selecionado(s)",
                        self.selected_markers.len());
                }
                self.drag_start=None;
                self.drag_end=None;
            }
            if response.clicked_by(PointerButton::Primary) {
                if let Some(p)=response.interact_pointer_pos() {
                    let local=[f64::from(p.x-rect.left()),f64::from(p.y-rect.top())];
                    self.selected_markers = marker_bounds.iter()
                        .filter(|(_,b)|b.intersects(ScreenBounds::from_points(local,local)))
                        .map(|(id,_)|id.clone()).collect();
                }
            }
        }

        if let (Some(a),Some(b))=(self.drag_start,self.drag_end) {
            let selection=Rect::from_two_pos(a,b);
            let mode=selection_mode(a.x as f64,b.x as f64);
            let color=match mode {SelectionMode::Window=>Color32::from_rgb(79,168,245),
                SelectionMode::Crossing=>Color32::from_rgb(109,203,145)};
            painter.rect_filled(selection,0.0,color.gamma_multiply(0.13));
            painter.rect_stroke(selection,0.0,Stroke::new(1.0,color),egui::StrokeKind::Inside);
        }
        painter.text(rect.left_top()+egui::vec2(15.0,15.0),
            egui::Align2::LEFT_TOP,
            format!("3D · {}",self.project.workspace.camera.projection.label()),
            egui::FontId::proportional(16.0),Color32::WHITE);
        painter.text(rect.left_bottom()-egui::vec2(-15.0,40.0),
            egui::Align2::LEFT_BOTTOM,
            "Seleção: esquerdo | Órbita: Shift+meio / direito / O | Pan: meio | Zoom: roda",
            egui::FontId::proportional(12.0),Color32::LIGHT_GRAY);
    }
}

fn csv_cell(text:&str)->String {
    format!("\"{}\"",text.replace('"',"\"\""))
}
fn schedule_csv(project:&Project,kind:ScheduleKind)->String {
    let mut out=String::new();
    match kind {
        ScheduleKind::Levels=>{
            out.push_str("id;nome;cota_m\n");
            for level in &project.levels {
                out.push_str(&format!("{};{};{}\n",
                    csv_cell(&level.id),csv_cell(&level.name),level.elevation_m));
            }
        }
        ScheduleKind::Elements=>{
            out.push_str("id;tipo;pavimento\n");
            for element in &project.elements {
                let kind=match element{civil_core::Element::Wall(_)=>"wall",
                    civil_core::Element::Slab(_)=>"slab"};
                out.push_str(&format!("{};{};{}\n",
                    csv_cell(element.id()),csv_cell(kind),csv_cell(element.level_id())));
            }
        }
        ScheduleKind::Views=>{
            out.push_str("id;nome;ponto_de_vista;escala\n");
            for view in &project.workspace.saved_views {
                out.push_str(&format!("{};{};{};{}\n",
                    csv_cell(&view.id),csv_cell(&view.title),
                    csv_cell(&view.viewpoint_id),csv_cell(&view.scale)));
            }
        }
    }
    out
}

impl eframe::App for StudioApp {
    fn ui(&mut self,root:&mut egui::Ui,_frame:&mut eframe::Frame) {
        let ctx = root.ctx().clone();
        if !ctx.egui_wants_keyboard_input() && ctx.input(|i|i.key_pressed(egui::Key::O)) {
            self.orbit_tool=!self.orbit_tool;
        }
        self.top_bar(root);
        self.navigator(root);
        egui::Panel::bottom("status").show(root,|ui|{
            ui.horizontal_wrapped(|ui|{
                ui.label("Arquivo:");
                ui.add_sized([220.0,22.0],egui::TextEdit::singleline(&mut self.file_path));
                ui.separator();
                ui.label(&self.status);
            });
        });
        egui::CentralPanel::default().show(root,|ui| {
            if self.preview_sheet {self.draw_sheet_preview(ui)}
            else {self.draw_viewport(ui)}
        });
        self.project_info_window(&ctx);
    }
}

fn main()->eframe::Result {
    let options=eframe::NativeOptions{
        viewport:egui::ViewportBuilder::default()
            .with_inner_size([1550.0,900.0])
            .with_min_inner_size([960.0,600.0]),
        ..Default::default()
    };
    eframe::run_native("CIVIL CRAFT · CAD/BIM — Câmeras e Navigator",options,
        Box::new(|cc|{
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            Ok(Box::<StudioApp>::default())
        }))
}
