use eframe::egui;
use std::{fs, path::Path};

const CATALOGO: &str = "https://www.abntcatalogo.com.br/";
const INICIAL: &str = include_str!("../../../biblioteca/referencias-abnt.tsv");

struct Referencia {
    codigo: String,
    assunto: String,
    edicao: String,
    indice: String,
}

fn carregar(texto: &str) -> Result<Vec<Referencia>, String> {
    let mut linhas = texto.trim_start_matches('\u{feff}').lines();
    if linhas.next() != Some("codigo\tassunto\tedicao") {
        return Err(
            "O cabeçalho TSV deve conter codigo, assunto e edicao, separados por tabulações."
                .into(),
        );
    }
    let mut itens = Vec::new();
    for (i, linha) in linhas.enumerate() {
        if linha.trim().is_empty() {
            continue;
        }
        let campos: Vec<_> = linha.split('\t').map(str::trim).collect();
        if campos.len() != 3 || campos[0].is_empty() || campos[1].is_empty() {
            return Err(format!("Referência inválida na linha {}.", i + 2));
        }
        itens.push(Referencia {
            codigo: campos[0].into(),
            assunto: campos[1].into(),
            edicao: campos[2].into(),
            indice: campos.join(" ").to_lowercase(),
        });
    }
    Ok(itens)
}

pub struct Biblioteca {
    pub aberta: bool,
    busca: String,
    caminho: String,
    origem: String,
    erro: String,
    itens: Vec<Referencia>,
}

impl Default for Biblioteca {
    fn default() -> Self {
        let mut biblioteca = Self {
            aberta: false,
            busca: String::new(),
            caminho: String::new(),
            origem: "Seleção inicial de referências".into(),
            erro: String::new(),
            itens: carregar(INICIAL).expect("referências incluídas no aplicativo"),
        };
        if let Ok(caminho) = std::env::var("CIVIL_CRAFT_ABNT_TSV") {
            biblioteca.caminho = caminho;
            biblioteca.abrir_catalogo();
        }
        biblioteca
    }
}

impl Biblioteca {
    fn abrir_catalogo(&mut self) {
        let caminho = Path::new(self.caminho.trim());
        match fs::read_to_string(caminho)
            .map_err(|e| e.to_string())
            .and_then(|texto| carregar(&texto))
        {
            Ok(itens) => {
                self.itens = itens;
                self.origem = caminho.display().to_string();
                self.erro.clear();
            }
            Err(e) => self.erro = format!("Falha ao carregar catálogo: {e}"),
        }
    }

    pub fn mostrar(&mut self, ctx: &egui::Context) {
        let mut aberta = self.aberta;
        egui::Window::new("Biblioteca de normas ABNT")
            .open(&mut aberta)
            .default_width(540.0)
            .show(ctx, |ui| {
                ui.hyperlink_to("Consultar catálogo oficial da ABNT", CATALOGO);
                ui.collapsing("Carregar catálogo local", |ui| {
                    ui.label("Caminho do catálogo TSV");
                    ui.text_edit_singleline(&mut self.caminho);
                    if ui.button("Carregar referências").clicked() {
                        self.abrir_catalogo();
                    }
                    if !self.erro.is_empty() {
                        ui.label(&self.erro);
                    }
                });
                ui.separator();
                ui.label(&self.origem);
                ui.add(
                    egui::TextEdit::singleline(&mut self.busca)
                        .hint_text("Buscar código, assunto ou edição"),
                );
                let busca = self.busca.to_lowercase();
                let resultados: Vec<_> = self
                    .itens
                    .iter()
                    .filter(|r| {
                        busca
                            .split_whitespace()
                            .all(|termo| r.indice.contains(termo))
                    })
                    .collect();
                ui.label(format!(
                    "{} de {} referências",
                    resultados.len(),
                    self.itens.len()
                ));
                let altura = ui.text_style_height(&egui::TextStyle::Body);
                egui::ScrollArea::vertical().max_height(360.0).show_rows(
                    ui,
                    altura,
                    resultados.len(),
                    |ui, faixa| {
                        for item in &resultados[faixa] {
                            let texto =
                                format!("{}   {}   {}", item.codigo, item.assunto, item.edicao);
                            ui.add(egui::Label::new(&texto).truncate())
                                .on_hover_text(texto);
                        }
                    },
                );
            });
        self.aberta = aberta;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_edicoes_distintas_e_rejeita_linha_incompleta() {
        let dados =
            "\u{feff}codigo\tassunto\tedicao\r\nNBR A\tReferência\t2010\r\nNBR A\tReferência\t2020";
        let itens = carregar(dados).unwrap();
        assert_eq!(itens.len(), 2);
        assert!(itens[1].indice.contains("2020"));
        assert!(carregar(&format!("{dados}\nsem campos")).is_err());
        assert!(!carregar(INICIAL).unwrap().is_empty());
    }
}
