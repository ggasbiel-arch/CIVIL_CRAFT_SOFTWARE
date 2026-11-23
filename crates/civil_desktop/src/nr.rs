use eframe::egui;

const INDICE: &str = "https://www.gov.br/trabalho-e-emprego/pt-br/acesso-a-informacao/participacao-social/conselhos-e-orgaos-colegiados/comissao-tripartite-partitaria-permanente/normas-regulamentadora/normas-regulamentadoras-vigentes";
const CATALOGO: &str = include_str!("../../../biblioteca/normas-regulamentadoras.tsv");

struct Norma {
    numero: u16,
    titulo: &'static str,
    situacao: &'static str,
    url: &'static str,
}

fn normas() -> impl Iterator<Item = Norma> {
    CATALOGO.lines().map(|linha| {
        let mut campos = linha.split('\t');
        Norma {
            numero: campos.next().unwrap().parse().expect("número NR"),
            titulo: campos.next().expect("título NR"),
            situacao: campos.next().expect("situação NR"),
            url: campos.next().expect("página NR"),
        }
    })
}

impl Norma {
    fn corresponde(&self, busca: &str) -> bool {
        let busca = busca.trim().to_lowercase();
        let codigo = busca
            .strip_prefix("nr")
            .unwrap_or(&busca)
            .trim_start_matches(['-', ' ']);
        if let Ok(numero) = codigo.parse::<u16>() {
            return self.numero == numero;
        }
        let texto = format!("nr-{} {} {}", self.numero, self.titulo, self.situacao).to_lowercase();
        busca.split_whitespace().all(|termo| texto.contains(termo))
    }
}

#[derive(Default)]
pub struct Biblioteca {
    pub aberta: bool,
    busca: String,
    somente_vigentes: bool,
}

impl Biblioteca {
    pub fn mostrar(&mut self, ctx: &egui::Context) {
        egui::Window::new("Normas Regulamentadoras (NR)").open(&mut self.aberta)
            .default_size([580.0, 420.0]).show(ctx, |ui| {
                ui.hyperlink_to("Índice oficial do Ministério do Trabalho e Emprego", INDICE);
                ui.small("Catálogo consultado em 23/11/2026. Abra a página da norma para consultar o texto oficial.");
                ui.add(egui::TextEdit::singleline(&mut self.busca).hint_text("Número ou palavras do título"));
                ui.checkbox(&mut self.somente_vigentes, "Somente vigentes");
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let mut resultados = normas().filter(|n|
                        (!self.somente_vigentes || n.situacao == "Vigente") && n.corresponde(&self.busca)
                    ).peekable();
                    if resultados.peek().is_none() { ui.label("Nenhuma norma encontrada."); }
                    for n in resultados {
                        ui.hyperlink_to(format!("NR-{} · {} · {}", n.numero, n.situacao, n.titulo), n.url);
                    }
                });
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indice_do_mte_inclui_historico_e_links_individuais() {
        let normas: Vec<_> = normas().collect();
        assert_eq!(
            normas.iter().map(|n| n.numero).collect::<Vec<_>>(),
            (1..=38).collect::<Vec<_>>()
        );
        assert_eq!(
            normas
                .iter()
                .filter(|n| n.situacao == "Revogada")
                .map(|n| n.numero)
                .collect::<Vec<_>>(),
            [2, 27]
        );
        assert!(
            normas.iter().all(|n| !n.titulo.is_empty()
                && n.url.starts_with("https://www.gov.br/trabalho-e-emprego/"))
        );
        assert!(normas[0].corresponde("NR 01"));
        assert!(!normas[17].corresponde("1"));
        assert!(normas[17].corresponde("construção indústria"));
    }
}
