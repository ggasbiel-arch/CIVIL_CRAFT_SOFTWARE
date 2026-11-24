use civil_core::Project;
use eframe::egui;
use std::fs;

// Neutral CAD/BIM shell: no temporary wall, window or floor tools.
struct StudioApp {
    project: Project,
    active_level: String,
    file_path: String,
    status: String,
}

impl Default for StudioApp {
    fn default() -> Self {
        Self {
            project: Project::default(),
            active_level: "P0".into(),
            file_path: "meu_projeto.bim.json".into(),
            status: "Área de trabalho pronta".into(),
        }
    }
}

impl StudioApp {
    fn replace_project(&mut self, project: Project) {
        self.active_level = project.levels.first()
            .map_or_else(|| "P0".into(), |level| level.id.clone());
        self.project = project;
    }

    fn save_file(&mut self) {
        match self.project.validate()
            .and_then(|_| serde_json::to_string_pretty(&self.project).map_err(|e| e.to_string()))
            .and_then(|json| fs::write(self.file_path.trim(), json).map_err(|e| e.to_string())) {
            Ok(()) => self.status = format!("Projeto salvo: {}", self.file_path),
            Err(err) => self.status = format!("Erro ao salvar: {err}"),
        }
    }

    fn open_file(&mut self) {
        let loaded = fs::read_to_string(self.file_path.trim())
            .map_err(|e| e.to_string())
            .and_then(|data| serde_json::from_str::<Project>(&data).map_err(|e| e.to_string()))
            .and_then(|project| { project.validate()?; Ok(project) });
        match loaded {
            Ok(project) => {
                self.replace_project(project);
                self.status = format!("Projeto aberto: {}", self.file_path);
            }
            Err(err) => self.status = format!("Erro ao abrir: {err}"),
        }
    }
}

impl eframe::App for StudioApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.heading("PROJETO 2 / CAD+BIM");
                ui.separator();
                if ui.button("Novo").clicked() {
                    self.replace_project(Project::default());
                    self.status = "Novo projeto vazio".into();
                }
                if ui.button("Abrir").clicked() { self.open_file(); }
                if ui.button("Salvar").clicked() { self.save_file(); }
            });
        });

        egui::SidePanel::left("navigator")
            .resizable(true).default_width(225.0).show(ctx, |ui| {
                ui.heading("Navegador BIM");
                ui.label("Nome do projeto");
                ui.text_edit_singleline(&mut self.project.name);
                ui.separator();
                ui.strong("Pavimentos");
                for level in &self.project.levels {
                    if ui.selectable_label(
                        self.active_level == level.id,
                        format!("{} • {:.2} m", level.name, level.elevation_m)
                    ).clicked() {
                        self.active_level = level.id.clone();
                    }
                }
                ui.separator();
                ui.strong("Modelo");
                ui.label(format!("{} elementos armazenados", self.project.elements.len()));
                if !self.project.elements.is_empty() {
                    ui.small("Dados legados preservados; viewport definitiva pendente.");
                }
            });

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Arquivo:");
                ui.text_edit_singleline(&mut self.file_path);
            });
            ui.label(&self.status);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.columns(2, |columns| {
                columns[0].heading("Área de trabalho 2D");
                columns[0].label("Reservada ao editor CAD definitivo.");
                columns[1].heading("Área de trabalho 3D");
                columns[1].label("Reservada ao kernel geométrico.");
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
    eframe::run_native("Projeto 2 | CAD+BIM — Base limpa", options, Box::new(|cc| {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        Ok(Box::<StudioApp>::default())
    }))
}
