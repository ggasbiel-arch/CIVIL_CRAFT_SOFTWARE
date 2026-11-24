//! Model-derived linework, associative sheet placements and small vector PDF exporter.
//! This is NOT a general CAD B-Rep, hidden-line engine, or BIM authoring tool.
use serde::{Deserialize, Serialize};
use crate::{Element, Project};
use crate::navigation::{SavedView, Viewpoint, ViewpointKind};

pub type Point2 = [f64; 2];
pub type Point3 = [f64; 3];

#[derive(Clone, Debug, PartialEq)]
pub struct ModelEdge { pub start: Point3, pub end: Point3, pub element_id: String }

#[derive(Clone, Debug, PartialEq)]
pub struct DrawingLine { pub start: Point2, pub end: Point2, pub element_id: String }

#[derive(Clone, Debug, PartialEq)]
pub struct VectorDrawing { pub title: String, pub lines: Vec<DrawingLine> }

impl VectorDrawing {
    pub fn bounds(&self) -> Option<(Point2, Point2)> {
        let first = self.lines.first()?;
        let mut low = first.start;
        let mut high = first.start;
        for line in &self.lines {
            for p in [line.start, line.end] {
                for axis in 0..2 {
                    low[axis] = low[axis].min(p[axis]);
                    high[axis] = high[axis].max(p[axis]);
                }
            }
        }
        Some((low, high))
    }
    fn edge(&mut self, a: Point2, b: Point2, id: &str) {
        if a != b && [a[0], a[1], b[0], b[1]].iter().all(|x|x.is_finite()) {
            self.lines.push(DrawingLine { start: a, end: b, element_id: id.to_owned() });
        }
    }
    fn polygon(&mut self, poly: &[Point2], id: &str) {
        if poly.len() < 3 { return; }
        for idx in 0..poly.len() {
            self.edge(poly[idx], poly[(idx + 1) % poly.len()], id);
        }
    }
}

fn base_z(project: &Project, level_id: &str) -> f64 {
    project.levels.iter().find(|l|l.id == level_id)
        .map(|l|l.elevation_m).unwrap_or(0.0)
}

fn footprint_wall(start: Point2, end: Point2, thickness: f64) -> Vec<Point2> {
    let len = (end[0]-start[0]).hypot(end[1]-start[1]);
    if len <= 1e-8 { return Vec::new(); }
    let normal = [-(end[1]-start[1])*thickness*0.5/len,
                    (end[0]-start[0])*thickness*0.5/len];
    vec![
        [start[0]+normal[0],start[1]+normal[1]],
        [end[0]+normal[0],end[1]+normal[1]],
        [end[0]-normal[0],end[1]-normal[1]],
        [start[0]-normal[0],start[1]-normal[1]],
    ]
}

fn outline(element: &Element) -> Vec<Point2> {
    match element {
        Element::Wall(w) => footprint_wall(w.start, w.end, w.thickness_m),
        Element::Slab(s) => s.polygon.clone(),
    }
}

fn vertical_limits(project: &Project, element: &Element) -> (f64, f64) {
    let z = base_z(project, element.level_id());
    match element {
        Element::Wall(w) => (z, z+w.height_m),
        Element::Slab(s) => (z-s.thickness_m, z),
    }
}

pub fn model_wireframe(project: &Project) -> Vec<ModelEdge> {
    let mut out = Vec::new();
    for element in &project.elements {
        let poly = outline(element);
        if poly.len() < 3 { continue; }
        let (low,high) = vertical_limits(project, element);
        let id = element.id().to_owned();
        let n = poly.len();
        for i in 0..n {
            let j=(i+1)%n;
            let a=poly[i]; let b=poly[j];
            out.push(ModelEdge{start:[a[0],a[1],low],end:[b[0],b[1],low],element_id:id.clone()});
            out.push(ModelEdge{start:[a[0],a[1],high],end:[b[0],b[1],high],element_id:id.clone()});
            out.push(ModelEdge{start:[a[0],a[1],low],end:[a[0],a[1],high],element_id:id.clone()});
        }
    }
    out
}

// Intersects a horizontal polygon with a vertical cutting plane whose XY normal
// points along direction [sin(yaw), cos(yaw)]. No hidden-line removal.
fn section_interval(poly:&[Point2], marker:Point2, yaw_rad:f64)->Option<(f64,f64)>{
    if poly.len()<3{return None;}
    let normal=[yaw_rad.sin(),yaw_rad.cos()];
    let right=[yaw_rad.cos(),-yaw_rad.sin()];
    let distance=|p:Point2| (p[0]-marker[0])*normal[0]+(p[1]-marker[1])*normal[1];
    let projected=|p:Point2| (p[0]-marker[0])*right[0]+(p[1]-marker[1])*right[1];
    let mut hits:Vec<f64>=Vec::new();
    for i in 0..poly.len() {
        let a=poly[i];let b=poly[(i+1)%poly.len()];
        let da=distance(a); let db=distance(b);
        if da.abs()<1e-8 {hits.push(projected(a));}
        if da*db< -1e-16 {
            let t=da/(da-db);
            let intersection=[a[0]+(b[0]-a[0])*t,a[1]+(b[1]-a[1])*t];
            hits.push(projected(intersection));
        }
    }
    if hits.is_empty(){return None;}
    let low=hits.iter().copied().fold(f64::INFINITY,f64::min);
    let high=hits.iter().copied().fold(f64::NEG_INFINITY,f64::max);
    if !low.is_finite() || high-low<1e-8 {None}else{Some((low,high))}
}

/// Generate actual linework from the currently stored model elements.
/// A floor plan filters by level; a section cuts the extruded legacy solids;
/// an elevation shows the projected wireframe, WITHOUT occlusion calculation.
pub fn render_viewpoint(project:&Project, viewpoint:&Viewpoint)->VectorDrawing{
    let mut drawing=VectorDrawing{title:viewpoint.title.clone(),lines:Vec::new()};
    match viewpoint.kind {
        ViewpointKind::FloorPlan|ViewpointKind::Detail => {
            for element in &project.elements {
                if viewpoint.level_id.as_deref()
                    .is_some_and(|level|element.level_id()!=level) {continue;}
                drawing.polygon(&outline(element),element.id());
            }
        }
        ViewpointKind::Section => {
            let yaw=viewpoint.camera.yaw_deg.to_radians();
            let marker=[viewpoint.marker[0],viewpoint.marker[1]];
            for element in &project.elements {
                let poly=outline(element);
                if let Some((min,max))=section_interval(&poly,marker,yaw) {
                    let (z0,z1)=vertical_limits(project,element);
                    drawing.polygon(&[[min,z0],[max,z0],[max,z1],[min,z1]],element.id());
                }
            }
        }
        ViewpointKind::Elevation => {
            let yaw=viewpoint.camera.yaw_deg.to_radians();
            let right=[yaw.cos(),-yaw.sin()];
            let marker=[viewpoint.marker[0],viewpoint.marker[1]];
            for edge in model_wireframe(project) {
                let proj=|p:Point3| {
                    [(p[0]-marker[0])*right[0]+(p[1]-marker[1])*right[1],p[2]]
                };
                drawing.edge(proj(edge.start),proj(edge.end),&edge.element_id);
            }
        }
        ViewpointKind::ThreeD => {} // 3D rendered by viewport camera, not flattened in sheets.
    }
    drawing
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SheetPlacement {
    pub id:String,
    pub saved_view_id:String,
    /// Upper-left corner of view frame, in millimetres from page upper-left.
    pub x_mm:f64,
    pub y_mm:f64,
    pub width_mm:f64,
    pub height_mm:f64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Sheet {
    pub id:String,
    pub number:String,
    pub title:String,
    pub width_mm:f64,
    pub height_mm:f64,
    #[serde(default)]
    pub placements:Vec<SheetPlacement>,
}
impl Sheet {
    pub fn a3_landscape(id:String,number:String,title:String)->Self{
        Self{id,number,title,width_mm:420.0,height_mm:297.0,placements:Vec::new()}
    }
    pub fn validate(&self)->Result<(),String>{
        if self.id.trim().is_empty()||self.title.trim().is_empty()||self.number.trim().is_empty()
            || !self.width_mm.is_finite()||!self.height_mm.is_finite()
            || !(100.0..=2000.0).contains(&self.width_mm)
            || !(100.0..=2000.0).contains(&self.height_mm){
            return Err("Prancha inválida".into());
        }
        let mut ids=std::collections::HashSet::new();
        for p in &self.placements {
            if p.id.trim().is_empty()||!ids.insert(p.id.as_str())
                || p.saved_view_id.trim().is_empty()
                || ![p.x_mm,p.y_mm,p.width_mm,p.height_mm].iter().all(|n|n.is_finite())
                || p.x_mm<0.0||p.y_mm<0.0||p.width_mm<=0.0||p.height_mm<=0.0
                || p.x_mm+p.width_mm>self.width_mm+1e-6
                || p.y_mm+p.height_mm>self.height_mm+1e-6 {
                return Err("Janela de prancha inválida".into());
            }
        }
        Ok(())
    }
}

fn pdf_literal(text:&str)->String {
    let mut result=String::from("(");
    for c in text.chars() {
        // Base-14 Helvetica has no embedded Unicode font; transliterate
        // Latin accents rather than writing UTF-8 bytes as WinAnsi PDF text.
        let b=match c {
            'á'|'à'|'â'|'ã'|'ä'|'Á'|'À'|'Â'|'Ã'|'Ä'=>b'a',
            'é'|'è'|'ê'|'ë'|'É'|'È'|'Ê'|'Ë'=>b'e',
            'í'|'ì'|'î'|'ï'|'Í'|'Ì'|'Î'|'Ï'=>b'i',
            'ó'|'ò'|'ô'|'õ'|'ö'|'Ó'|'Ò'|'Ô'|'Õ'|'Ö'=>b'o',
            'ú'|'ù'|'û'|'ü'|'Ú'|'Ù'|'Û'|'Ü'=>b'u',
            'ç'|'Ç'=>b'c',
            'ñ'|'Ñ'=>b'n',
            '–'|'—'=>b'-',
            '‘'|'’'=>b'\'',
            '“'|'”'=>b'"',
            ascii if ascii.is_ascii()=>ascii as u8,
            _=>b'?',
        };
        if b==b'(' || b==b')'||b==b'\\' {
            result.push('\\');result.push(b as char);
        } else if (32..=126).contains(&b){
            result.push(b as char);
        } else {
            result.push_str(&format!("\\{:03o}",b));
        }
    }
    result.push(')');
    result
}

const PT_PER_MM:f64=72.0/25.4;

fn pdf_text(cmd:&mut String,x:f64,y:f64,size:f64,label:&str){
    cmd.push_str(&format!("BT /F1 {size:.2} Tf {x:.3} {y:.3} Td {} Tj ET\n",
        pdf_literal(label)));
}
fn pdf_line(cmd:&mut String,x1:f64,y1:f64,x2:f64,y2:f64){
    cmd.push_str(&format!("{x1:.3} {y1:.3} m {x2:.3} {y2:.3} l S\n"));
}

/// Resolve a saved view against live model data.
/// 3D views use the saved camera snapshot, other views use their source viewpoint
/// and its live cutting position. Linework is deliberately simplified.
pub fn render_saved_view(project:&Project,saved:&SavedView)->Result<VectorDrawing,String>{
    let viewpoint=project.workspace.viewpoints.iter()
        .find(|v|v.id==saved.viewpoint_id)
        .ok_or("Ponto de vista salvo não encontrado")?;
    if viewpoint.kind!=ViewpointKind::ThreeD {
        return Ok(render_viewpoint(project,viewpoint));
    }
    let mut drawing=VectorDrawing{title:saved.title.clone(),lines:Vec::new()};
    for edge in model_wireframe(project) {
        let a=saved.camera.project(edge.start,1000.0,1000.0);
        let b=saved.camera.project(edge.end,1000.0,1000.0);
        if let (Some(a),Some(b))=(a,b) {
            // Viewport pixel Y points down; drawing coordinates point up.
            drawing.edge([a[0],-a[1]],[b[0],-b[1]],&edge.element_id);
        }
    }
    Ok(drawing)
}

/// Generate a vector PDF of live linked views with true physical scales for
/// 2D model-derived views, per-view styles, line hatches and measured dimensions.
///
/// Sheets are not drawings for construction: no exact B-Rep, no lineweight
/// standard preset, no PDF/A and occlusion is sampled rather than analytic.
pub fn export_sheet_pdf(project:&Project,sheet:&Sheet)->Result<Vec<u8>,String>{
    use crate::technical::{PrintScale,PT_PER_MM,styled_view,section_hatches,derived_dimensions};
    project.validate()?;
    sheet.validate()?;
    let paper_w=sheet.width_mm*PT_PER_MM;
    let paper_h=sheet.height_mm*PT_PER_MM;
    let mut cmd=String::from("0 0 0 RG 0.6 w\n");
    pdf_line(&mut cmd,12.0*PT_PER_MM,12.0*PT_PER_MM,
        (sheet.width_mm-12.0)*PT_PER_MM,12.0*PT_PER_MM);
    pdf_line(&mut cmd,12.0*PT_PER_MM,12.0*PT_PER_MM,
        12.0*PT_PER_MM,(sheet.height_mm-12.0)*PT_PER_MM);
    pdf_line(&mut cmd,12.0*PT_PER_MM,(sheet.height_mm-12.0)*PT_PER_MM,
        (sheet.width_mm-12.0)*PT_PER_MM,(sheet.height_mm-12.0)*PT_PER_MM);
    pdf_line(&mut cmd,(sheet.width_mm-12.0)*PT_PER_MM,12.0*PT_PER_MM,
        (sheet.width_mm-12.0)*PT_PER_MM,(sheet.height_mm-12.0)*PT_PER_MM);
    let band=35.0*PT_PER_MM;
    pdf_line(&mut cmd,12.0*PT_PER_MM,band,(sheet.width_mm-12.0)*PT_PER_MM,band);
    pdf_text(&mut cmd,18.0*PT_PER_MM,24.0*PT_PER_MM,13.0,&project.name);
    pdf_text(&mut cmd,18.0*PT_PER_MM,18.0*PT_PER_MM,10.0,&sheet.title);
    pdf_text(&mut cmd,18.0*PT_PER_MM,13.0*PT_PER_MM,8.0,
        &format!("Codigo: {} | Revisao: {} | Prancha: {}",
            project.information.project_code,project.information.revision,sheet.number));

    for placement in &sheet.placements {
        let saved=project.workspace.saved_views.iter()
            .find(|v|v.id==placement.saved_view_id)
            .ok_or("Vista vinculada à prancha não encontrada")?;
        let source=project.workspace.viewpoints.iter()
            .find(|v|v.id==saved.viewpoint_id)
            .ok_or("Ponto de vista vinculado não encontrado")?;
        let drawing=styled_view(project,saved)?;
        let left=placement.x_mm*PT_PER_MM;
        let top=paper_h-placement.y_mm*PT_PER_MM;
        let width=placement.width_mm*PT_PER_MM;
        let height=placement.height_mm*PT_PER_MM;
        let clip_bottom=top-height;
        pdf_line(&mut cmd,left,top,left+width,top);
        pdf_line(&mut cmd,left+width,top,left+width,clip_bottom);
        pdf_line(&mut cmd,left+width,clip_bottom,left,clip_bottom);
        pdf_line(&mut cmd,left,clip_bottom,left,top);
        let is_3d=source.kind==ViewpointKind::ThreeD;
        let notation=if is_3d{"SEM ESCALA".to_string()}else{saved.scale.clone()};
        pdf_text(&mut cmd,left+5.0,top-12.0,9.0,
            &format!("{} [{}]",saved.title,notation));

        if let Some((minimum,maximum))=drawing.bounds() {
            let print_scale=PrintScale::parse(&saved.scale)?;
            let span_x=(maximum[0]-minimum[0]).max(0.05);
            let span_y=(maximum[1]-minimum[1]).max(0.05);
            let drawable_w=(width-18.0*PT_PER_MM).max(1.0);
            let drawable_h=(height-27.0*PT_PER_MM).max(1.0);
            let scale=if is_3d {
                (drawable_w/span_x).min(drawable_h/span_y)
            }else{
                let true_scale=print_scale.points_per_model_meter();
                if span_x*true_scale>drawable_w+1e-6
                    || span_y*true_scale>drawable_h+1e-6 {
                    return Err(format!(
                        "Vista '{}' não cabe na prancha em {}. Selecione outra escala ou aumente o quadro.",
                        saved.title,saved.scale));
                }
                true_scale
            };
            let center_x=(minimum[0]+maximum[0])*0.5;
            let center_y=(minimum[1]+maximum[1])*0.5;
            let draw_x=left+width/2.0;
            let draw_y=top-height/2.0-5.0*PT_PER_MM;
            let to_pdf=|p:Point2|->Point2 {
                [draw_x+(p[0]-center_x)*scale,
                 draw_y+(p[1]-center_y)*scale]
            };

            // Clip hatches and dimensions to their own view frame.
            cmd.push_str(&format!("q\n{left:.3} {clip_bottom:.3} {width:.3} {height:.3} re W n\n"));
            let color=saved.graphics.color();
            let line_weight=saved.graphics.line_width_mm*PT_PER_MM;
            if source.kind==ViewpointKind::Section && !is_3d {
                let hatch_spacing_m=saved.graphics.hatch_spacing_mm
                    *print_scale.denominator/1000.0;
                let hatches=section_hatches(&drawing,saved.graphics.hatch,hatch_spacing_m);
                cmd.push_str(&format!(
                    "0.56 0.58 0.60 RG {:.3} w\n",(line_weight*0.55).max(0.08)));
                for hatch in hatches.iter().take(4000) {
                    let a=to_pdf(hatch.start);let b=to_pdf(hatch.end);
                    pdf_line(&mut cmd,a[0],a[1],b[0],b[1]);
                }
            }
            cmd.push_str(&format!(
                "{:.4} {:.4} {:.4} RG {:.3} w\n",
                color[0],color[1],color[2],line_weight));
            for segment in &drawing.lines {
                let a=to_pdf(segment.start);
                let b=to_pdf(segment.end);
                pdf_line(&mut cmd,a[0],a[1],b[0],b[1]);
            }

            // Dimension values are computed in model metres, not PDF pixels.
            if !is_3d && saved.graphics.auto_dimensions {
                let dimensions=derived_dimensions(project,source);
                cmd.push_str(&format!("0.15 0.15 0.15 RG {:.3} w\n",
                    (0.12*PT_PER_MM).max(0.12)));
                for dim in dimensions.iter().take(80) {
                    let a=to_pdf(dim.start);let b=to_pdf(dim.end);
                    let dx=b[0]-a[0];let dy=b[1]-a[1];
                    let len=dx.hypot(dy);
                    if len < 2.0*PT_PER_MM {continue;}
                    let offset=6.0*PT_PER_MM;
                    let normal=[-dy/len,dx/len];
                    let shift=|p:Point2,k:f64| -> Point2 {
                        [p[0]+normal[0]*k,p[1]+normal[1]*k]
                    };
                    let c=shift(a,offset);
                    let d=shift(b,offset);
                    let ca=shift(a,offset+2.0*PT_PER_MM);
                    let db=shift(b,offset+2.0*PT_PER_MM);
                    pdf_line(&mut cmd,a[0],a[1],ca[0],ca[1]);
                    pdf_line(&mut cmd,b[0],b[1],db[0],db[1]);
                    pdf_line(&mut cmd,c[0],c[1],d[0],d[1]);
                    let mid=[(c[0]+d[0])/2.0,(c[1]+d[1])/2.0];
                    pdf_text(&mut cmd,mid[0]+1.5,mid[1]+3.0,7.5,&dim.label());
                }
            }
            cmd.push_str("Q\n");
        }else{
            pdf_text(&mut cmd,left+7.0,top-height/2.0,9.0,
                "Sem elementos nesta vista");
        }
    }
    // Tiny, deterministic PDF 1.4 writer for vector paths and annotations.
    let stream_bytes=cmd.as_bytes();
    let objects=vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {paper_w:.3} {paper_h:.3}] /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>"),
        format!("<< /Length {} >>\nstream\n{}\nendstream",stream_bytes.len(),cmd),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_owned(),
    ];
    let mut output=b"%PDF-1.4\n".to_vec();
    let mut offsets=vec![0usize];
    for (idx,body) in objects.iter().enumerate() {
        offsets.push(output.len());
        output.extend_from_slice(format!("{} 0 obj\n{}\nendobj\n",idx+1,body).as_bytes());
    }
    let startxref=output.len();
    output.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n",
        offsets.len()).as_bytes());
    for offset in offsets.iter().skip(1) {
        output.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    output.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
        offsets.len(),startxref).as_bytes());
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Wall, Slab, Element};
    use crate::navigation::Camera;

    fn project_with_legacy_data()->Project {
        let mut p=Project::default();
        p.elements.push(Element::Wall(Wall {
            id:"wall-1".into(),level_id:"P0".into(),start:[-2.0,0.0],end:[2.0,0.0],
            thickness_m:0.2,height_m:3.0,
        }));
        p.elements.push(Element::Slab(Slab {
            id:"slab-1".into(),level_id:"P0".into(),
            polygon:vec![[-2.0,-1.0],[2.0,-1.0],[2.0,1.0],[-2.0,1.0]],
            thickness_m:0.25,
        }));
        p
    }

    #[test]
    fn floor_plan_is_derived_from_stored_geometry() {
        let p=project_with_legacy_data();
        let vp=p.workspace.viewpoints.iter()
            .find(|v|v.kind==ViewpointKind::FloorPlan).unwrap();
        assert_eq!(render_viewpoint(&p,vp).lines.len(),8);
    }

    #[test]
    fn section_is_actual_intersection_not_an_illustration() {
        let p=project_with_legacy_data();
        let mut vp=p.workspace.viewpoints[0].clone();
        vp.kind=ViewpointKind::Section;
        vp.marker=[0.0,0.0,0.0];
        vp.camera.yaw_deg=0.0;
        let cut=render_viewpoint(&p,&vp);
        assert!(!cut.lines.is_empty());
        vp.marker=[0.0,50.0,0.0];
        assert!(render_viewpoint(&p,&vp).lines.is_empty());
    }

    #[test]
    fn elevation_contains_real_edges() {
        let p=project_with_legacy_data();
        let mut vp=p.workspace.viewpoints[0].clone();
        vp.kind=ViewpointKind::Elevation;
        assert!(!render_viewpoint(&p,&vp).lines.is_empty());
    }

    #[test]
    fn sheet_pdf_has_correct_cross_reference() {
        let mut p=project_with_legacy_data();
        let vp=p.workspace.viewpoints.iter()
            .find(|v|v.kind==ViewpointKind::FloorPlan).unwrap().id.clone();
        let id=p.workspace.save_view(&vp,"Planta terreo".into(),Camera::default()).unwrap();
        let mut sheet=Sheet::a3_landscape("sheet-1".into(),"A-01".into(),"Arquitetura".into());
        sheet.placements.push(SheetPlacement {
            id:"slot-1".into(),saved_view_id:id,x_mm:20.0,y_mm:25.0,
            width_mm:300.0,height_mm:200.0,
        });
        p.workspace.sheets.push(sheet.clone());
        let pdf=export_sheet_pdf(&p,&sheet).unwrap();
        assert!(pdf.starts_with(b"%PDF-1.4\n"));
        assert!(pdf.ends_with(b"%%EOF\n"));
        let text=String::from_utf8(pdf).unwrap();
        assert!(text.contains("xref\n0 6\n"));
        assert!(text.contains("Planta terreo"));
    }

    #[test]
    fn linked_pdf_updates_after_model_geometry_changes() {
        let mut project=project_with_legacy_data();
        let source=project.workspace.viewpoints.iter()
            .find(|v|v.kind==ViewpointKind::FloorPlan).unwrap().id.clone();
        let id=project.workspace.save_view(&source,
            "Planta associativa".into(),Camera::default()).unwrap();
        let mut sheet=Sheet::a3_landscape("linked".into(),"A-02".into(),"Associativa".into());
        sheet.placements.push(SheetPlacement {
            id:"v".into(),saved_view_id:id,x_mm:20.0,y_mm:20.0,
            width_mm:300.0,height_mm:210.0,
        });
        project.workspace.sheets.push(sheet.clone());
        let before=export_sheet_pdf(&project,&sheet).unwrap();
        if let Element::Wall(w)=&mut project.elements[0] {w.end[0]=3.0;}
        let after=export_sheet_pdf(&project,&sheet).unwrap();
        assert_ne!(before,after,"PDF must be generated from live model data");
    }

    #[test]
    fn saved_3d_camera_produces_pdf_wireframe() {
        let mut project=project_with_legacy_data();
        let source=project.workspace.viewpoints.iter()
            .find(|v|v.kind==ViewpointKind::ThreeD).unwrap().id.clone();
        let saved=project.workspace.save_view(&source,"3D perspectiva".into(),Camera::default()).unwrap();
        let drawing=render_saved_view(&project,project.workspace.saved_views.iter()
            .find(|v|v.id==saved).unwrap()).unwrap();
        assert!(!drawing.lines.is_empty());
        let mut sheet=Sheet::a3_landscape("s3".into(),"A-03".into(),"Perspectiva".into());
        sheet.placements.push(SheetPlacement {
            id:"p3".into(),saved_view_id:saved,x_mm:20.0,y_mm:24.0,
            width_mm:320.0,height_mm:200.0,
        });
        project.workspace.sheets.push(sheet.clone());
        let pdf=export_sheet_pdf(&project,&sheet).unwrap();
        assert!(String::from_utf8_lossy(&pdf).contains("3D perspectiva"));
    }

    #[test]
    fn pdf_uses_real_scale_and_computed_dimension_labels() {
        use crate::technical::HatchPattern;
        let mut p=project_with_legacy_data();
        let vp=p.workspace.viewpoints.iter()
            .find(|v|v.kind==ViewpointKind::FloorPlan).unwrap().id.clone();
        let view_id=p.workspace.save_view(&vp,"Planta 1:100".into(),Camera::default()).unwrap();
        let view=p.workspace.saved_views.iter_mut().find(|v|v.id==view_id).unwrap();
        view.scale="1:100".into();
        view.graphics.auto_dimensions=true;
        view.graphics.hatch=HatchPattern::None;
        let mut sheet=Sheet::a3_landscape("scale-sheet".into(),"A-11".into(),"Escalas".into());
        sheet.placements.push(SheetPlacement{
            id:"slot".into(),saved_view_id:view_id,x_mm:15.0,y_mm:15.0,
            width_mm:320.0,height_mm:220.0,
        });
        p.workspace.sheets.push(sheet.clone());
        let first=String::from_utf8(export_sheet_pdf(&p,&sheet).unwrap()).unwrap();
        assert!(first.contains("1:100"));
        assert!(first.contains("4.00 m"));
        if let Element::Wall(w)=&mut p.elements[0]{w.end[0]=3.0;}
        let second=String::from_utf8(export_sheet_pdf(&p,&sheet).unwrap()).unwrap();
        assert!(second.contains("5.00 m"));
        assert_ne!(first,second);
    }

    #[test]
    fn refuses_to_mislabel_drawing_when_model_exceeds_print_frame() {
        let mut p=project_with_legacy_data();
        if let Element::Wall(w)=&mut p.elements[0]{w.end[0]=70.0;}
        let vp=p.workspace.viewpoints.iter()
            .find(|v|v.kind==ViewpointKind::FloorPlan).unwrap().id.clone();
        let id=p.workspace.save_view(&vp,"Implantação".into(),Camera::default()).unwrap();
        let view=p.workspace.saved_views.iter_mut().find(|v|v.id==id).unwrap();
        view.scale="1:50".into();
        let mut sheet=Sheet::a3_landscape("overflow".into(),"A-12".into(),"Teste".into());
        sheet.placements.push(SheetPlacement{
            id:"slot".into(),saved_view_id:id,x_mm:20.0,y_mm:20.0,
            width_mm:300.0,height_mm:220.0,
        });
        p.workspace.sheets.push(sheet.clone());
        let error=export_sheet_pdf(&p,&sheet).unwrap_err();
        assert!(error.contains("não cabe"));
    }

    #[test]
    fn section_hatch_changes_vector_pdf_stream() {
        use crate::technical::HatchPattern;
        let mut p=project_with_legacy_data();
        let mut vp=p.workspace.viewpoints.iter().find(|v|v.kind==ViewpointKind::ThreeD)
            .unwrap().clone();
        vp.id="section-for-fill".into();
        vp.kind=ViewpointKind::Section;
        vp.camera.yaw_deg=0.0;
        vp.marker=[0.0,0.0,0.0];
        p.workspace.viewpoints.push(vp.clone());
        let id=p.workspace.save_view(&vp.id,"Corte".into(),Camera::default()).unwrap();
        let mut sheet=Sheet::a3_landscape("hatched".into(),"A-13".into(),"Seção".into());
        sheet.placements.push(SheetPlacement{
            id:"slot".into(),saved_view_id:id.clone(),x_mm:20.0,y_mm:20.0,
            width_mm:300.0,height_mm:200.0,
        });
        p.workspace.sheets.push(sheet.clone());
        let none=p.workspace.saved_views.iter_mut().find(|v|v.id==id).unwrap();
        none.graphics.hatch=HatchPattern::None;
        let plain=export_sheet_pdf(&p,&sheet).unwrap();
        p.workspace.saved_views.iter_mut().find(|v|v.id==id).unwrap()
            .graphics.hatch=HatchPattern::Cross;
        let hatched=export_sheet_pdf(&p,&sheet).unwrap();
        assert!(hatched.len()>plain.len(),"Hatch should add line commands to the PDF");
    }

    #[test]
    fn rejects_sheet_outside_paper() {
        let mut sheet=Sheet::a3_landscape("id".into(),"A-01".into(),"Planta".into());
        sheet.placements.push(SheetPlacement {
            id:"p".into(),saved_view_id:"view".into(),
            x_mm:300.0,y_mm:10.0,width_mm:200.0,height_mm:100.0,
        });
        assert!(sheet.validate().is_err());
    }
}
