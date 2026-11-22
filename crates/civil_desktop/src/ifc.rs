use eframe::egui;
use std::{process::Command, sync::mpsc, thread};

#[derive(Default)]
pub struct IfcInspector {
    pub open: bool,
    path: String,
    report: String,
    pending: Option<mpsc::Receiver<String>>,
}

impl IfcInspector {
    pub fn show(&mut self, ctx: &egui::Context) {
        if let Some(receiver) = &self.pending {
            match receiver.try_recv() {
                Ok(report) => {
                    self.report = report;
                    self.pending = None;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.report = "A consulta IFC foi interrompida.".into();
                    self.pending = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        egui::Window::new("Consultar IFC")
            .open(&mut self.open)
            .show(ctx, |ui| {
                ui.label("Caminho do arquivo .ifc");
                ui.text_edit_singleline(&mut self.path);
                if ui
                    .add_enabled(
                        self.pending.is_none() && !self.path.trim().is_empty(),
                        egui::Button::new("Ler elementos"),
                    )
                    .clicked()
                {
                    let path = self.path.trim().to_owned();
                    let (sender, receiver) = mpsc::channel();
                    self.pending = Some(receiver);
                    self.report.clear();
                    let ctx = ctx.clone();
                    thread::spawn(move || {
                        let result = inspect(&path).unwrap_or_else(|error| error);
                        let _ = sender.send(result);
                        ctx.request_repaint();
                    });
                }
                if self.pending.is_some() {
                    ui.spinner();
                }
                egui::ScrollArea::both().max_height(400.0).show(ui, |ui| {
                    ui.label(&self.report);
                });
            });
    }
}

fn inspect(path: &str) -> Result<String, String> {
    let python = std::env::var_os("CIVIL_CRAFT_PYTHON").unwrap_or_else(|| {
        if cfg!(windows) {
            "python".into()
        } else {
            "python3".into()
        }
    });
    let mut command = Command::new(python);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    let output = command
        .args([
            "-I",
            "-c",
            include_str!("../../../python/ifc_inspect.py"),
            path,
        ])
        .output()
        .map_err(|error| format!("Falha ao iniciar Python. Confira CIVIL_CRAFT_PYTHON. {error}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
    } else {
        let message = String::from_utf8_lossy(&output.stderr);
        Err(if message.trim().is_empty() {
            format!("O leitor IFC terminou com {}", output.status)
        } else {
            message.trim().to_owned()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requer Python com IfcOpenShell"]
    fn reads_ifc_step_through_desktop_bridge() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("modelo de arquitetura.ifc");
        std::fs::write(
            &path,
            "ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('ViewDefinition [CoordinationView]'),'2;1');
FILE_NAME('example.ifc','2026-11-22T00:00:00',(''),(''),'','','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1=IFCWALL('0JWd5xXvL8GQPKGvIGa000',$,'Parede A',$,$,$,$,$,.STANDARD.);
ENDSEC;
END-ISO-10303-21;",
        )
        .unwrap();
        let report = inspect(path.to_str().unwrap()).unwrap();
        assert!(report.contains("Esquema: IFC4"));
        assert!(report.contains("IfcWall: 1"));
        assert!(report.contains("Parede A  ·  0JWd5xXvL8GQPKGvIGa000"));
        std::fs::write(&path, "arquivo inválido").unwrap();
        assert!(inspect(path.to_str().unwrap()).is_err());
    }
}
