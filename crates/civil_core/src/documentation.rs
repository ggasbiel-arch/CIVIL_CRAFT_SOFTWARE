//! Model-derived linework, associative sheet placements and small vector PDF exporter.
//! This is NOT a general CAD B-Rep, hidden-line engine, or BIM authoring tool.
use serde::{Deserialize, Serialize};
use crate::{Element, Project};
use crate::navigation::{Viewpoint, ViewpointKind};

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
        let b=match c as u32 {
            0..=255 => c as u8,
            0x2013|0x2014 => b'-',
            0x2018|0x2019=>b'\'',
            0x201C|0x201D=>b'"',
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

/// Export a linked sheet on ISO-style A3 paper as a simple vector PDF.
///
/// Exports existing BIM records (currently legacy walls and slabs) dynamically
/// through the saved viewpoint ID. Fit-to-frame is used, NOT print scale 1:N.
/// Limited to 2D plans/sections/elevations; no hatches, occlusion, fonts embedding or PDF/A.
pub fn export_sheet_pdf(project:&Project,sheet:&Sheet)->Result<Vec<u8>,String>{
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
        let vp=project.workspace.viewpoints.iter()
            .find(|v|v.id==saved.viewpoint_id)
            .ok_or("Ponto de vista vinculado não encontrado")?;
        if vp.kind==ViewpointKind::ThreeD {
            return Err("Publicação PDF de vista 3D ainda não implementada".into());
        }
        let drawing=render_viewpoint(project,vp);
        let left=placement.x_mm*PT_PER_MM;
        let top=paper_h-placement.y_mm*PT_PER_MM;
        let width=placement.width_mm*PT_PER_MM;
        let height=placement.height_mm*PT_PER_MM;
        pdf_line(&mut cmd,left,top,left+width,top);
        pdf_line(&mut cmd,left+width,top,left+width,top-height);
        pdf_line(&mut cmd,left+width,top-height,left,top-height);
        pdf_line(&mut cmd,left,top-height,left,top);
        pdf_text(&mut cmd,left+5.0,top-12.0,9.0,
            &format!("{} [{}]",saved.title,saved.scale));
        if let Some((minimum,maximum))=drawing.bounds() {
            let span_x=(maximum[0]-minimum[0]).max(0.05);
            let span_y=(maximum[1]-minimum[1]).max(0.05);
            let drawable_w=(width-16.0).max(1.0);
            let drawable_h=(height-35.0).max(1.0);
            let scale=(drawable_w/span_x).min(drawable_h/span_y);
            let center_x=(minimum[0]+maximum[0])*0.5;
            let center_y=(minimum[1]+maximum[1])*0.5;
            let draw_x=left+width/2.0;
            let draw_y=top-height/2.0-5.0;
            cmd.push_str("0 0 0 RG 0.48 w\n");
            for seg in &drawing.lines {
                let a=[draw_x+(seg.start[0]-center_x)*scale,
                       draw_y+(seg.start[1]-center_y)*scale];
                let b=[draw_x+(seg.end[0]-center_x)*scale,
                       draw_y+(seg.end[1]-center_y)*scale];
                pdf_line(&mut cmd,a[0],a[1],b[0],b[1]);
            }
        } else {
            pdf_text(&mut cmd,left+7.0,top-height/2.0,9.0,
                "Sem elementos nesta vista");
        }
    }
    // Minimal PDF 1.4 with five indirect objects, one page, WinAnsi Helvetica.
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
    fn rejects_sheet_outside_paper() {
        let mut sheet=Sheet::a3_landscape("id".into(),"A-01".into(),"Planta".into());
        sheet.placements.push(SheetPlacement {
            id:"p".into(),saved_view_id:"view".into(),
            x_mm:300.0,y_mm:10.0,width_mm:200.0,height_mm:100.0,
        });
        assert!(sheet.validate().is_err());
    }
}
