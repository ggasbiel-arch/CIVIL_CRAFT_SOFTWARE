use super::*;

impl StudioApp {
    pub(super) fn apply_wall(&mut self, wall: Wall) -> bool {
        let before = self.project.clone();
        match self.project.update_wall(wall) {
            Ok(changed) => {
                if changed {
                    self.save_snapshot_from(before);
                }
                self.status = "Parede atualizada".into();
                true
            }
            Err(error) => {
                self.status = error;
                false
            }
        }
    }
    pub(super) fn wall_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut wall) = self.wall_editor.take() else {
            return;
        };
        if self.selected_id.as_deref() != Some(&wall.id) {
            return;
        }
        let mut open = true;
        let mut apply = false;
        egui::Window::new(format!("Editar parede {}", wall.id))
            .open(&mut open)
            .vscroll(true)
            .show(ctx, |ui| {
                for (label, point) in [("Início", &mut wall.start), ("Fim", &mut wall.end)] {
                    ui.horizontal(|ui| {
                        ui.label(label);
                        for (coord, label) in point.iter_mut().zip(["X", "Y"]) {
                            ui.add(
                                egui::DragValue::new(coord)
                                    .prefix(format!("{label} "))
                                    .suffix(" m")
                                    .speed(0.05),
                            );
                        }
                    });
                }
                ui.horizontal(|ui| {
                    ui.label("Eixo");
                    for (axis, label) in [
                        (ReferenceAxis::Center, "Centro"),
                        (ReferenceAxis::LeftFace, "Face esquerda"),
                        (ReferenceAxis::RightFace, "Face direita"),
                    ] {
                        ui.selectable_value(&mut wall.axis, axis, label);
                    }
                });
                ui.add(
                    egui::DragValue::new(&mut wall.offset_m)
                        .prefix("Deslocamento ")
                        .suffix(" m")
                        .speed(0.01),
                );
                egui::ComboBox::from_label("Topo")
                    .selected_text(wall.top_level_id.as_deref().unwrap_or("Altura fixa"))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut wall.top_level_id, None, "Altura fixa");
                        for level in &self.project.levels {
                            if level.elevation_m > self.project.level_elevation(&wall.level_id) {
                                ui.selectable_value(
                                    &mut wall.top_level_id,
                                    Some(level.id.clone()),
                                    &level.name,
                                );
                            }
                        }
                    });
                if wall.top_level_id.is_none() {
                    ui.add(
                        egui::DragValue::new(&mut wall.height_m)
                            .prefix("Altura ")
                            .suffix(" m")
                            .speed(0.05),
                    );
                }
                ui.separator();
                ui.label("Camadas da esquerda para a direita, no sentido do eixo");
                let mut remove = None;
                let removable = wall.layers.len() > 1;
                for (index, layer) in wall.layers.iter_mut().enumerate() {
                    ui.push_id(index, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Material");
                            ui.text_edit_singleline(&mut layer.material);
                            if removable && ui.button("Remover").clicked() {
                                remove = Some(index);
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::DragValue::new(&mut layer.thickness_m)
                                    .prefix("Espessura ")
                                    .suffix(" m")
                                    .speed(0.005),
                            );
                            ui.add(egui::DragValue::new(&mut layer.priority).prefix("Prioridade "));
                        });
                        ui.horizontal(|ui| {
                            for (role, label) in [
                                (LayerRole::Structure, "Estrutura"),
                                (LayerRole::Finish, "Acabamento"),
                                (LayerRole::Insulation, "Isolamento"),
                            ] {
                                ui.selectable_value(&mut layer.role, role, label);
                            }
                        });
                    });
                }
                if let Some(index) = remove {
                    wall.layers.remove(index);
                }
                if ui.button("Adicionar camada").clicked() {
                    wall.layers.push(WallLayer::base(0.02));
                }
                apply = ui.button("Aplicar alterações").clicked();
            });
        if apply && self.apply_wall(wall.clone()) {
            return;
        }
        if open {
            self.wall_editor = Some(wall);
        }
    }
}
