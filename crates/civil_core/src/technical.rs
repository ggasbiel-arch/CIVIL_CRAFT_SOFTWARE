//! Original CIVIL CRAFT technical documentation primitives.
//! Scales in mm/m, graphic overrides, hatch vectors, derived dimensions,
//! and sampled occlusion for the current prismatic legacy schema.
use serde::{Deserialize, Serialize};
use crate::{Element, Project};
use crate::documentation::{DrawingLine, VectorDrawing, Point2, Point3, model_wireframe, render_saved_view};
use crate::navigation::{SavedView, Viewpoint, ViewpointKind, ProjectionMode};
use std::collections::BTreeMap;

const EPS: f64 = 1e-8;
pub const PT_PER_MM: f64 = 72.0/25.4;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all="snake_case")]
pub enum HatchPattern { None, Diagonal, Cross }
impl HatchPattern {
    pub fn label(self)->&'static str {
        match self {Self::None=>"Sem hachura",Self::Diagonal=>"Diagonal 45°",Self::Cross=>"Cruzada"}
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all="snake_case")]
pub enum GraphicPreset { Technical, Monochrome, Conceptual }
impl GraphicPreset {
    pub fn label(self)->&'static str {
        match self { Self::Technical=>"Técnico", Self::Monochrome=>"Preto e branco", Self::Conceptual=>"Conceitual" }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GraphicSettings {
    pub preset: GraphicPreset,
    pub line_width_mm: f64,
    pub line_rgb: [u8;3],
    pub hatch: HatchPattern,
    /// Distance between hatch strokes on paper, not distance in model coordinates.
    pub hatch_spacing_mm: f64,
    pub auto_dimensions: bool,
    pub hide_occluded_edges: bool,
}
impl Default for GraphicSettings {
    fn default()->Self {
        Self {
            preset:GraphicPreset::Technical,
            line_width_mm:0.18,
            line_rgb:[28,41,56],
            hatch:HatchPattern::Diagonal,
            hatch_spacing_mm:2.2,
            auto_dimensions:false,
            hide_occluded_edges:false,
        }
    }
}
impl GraphicSettings {
    pub fn validate(&self)->Result<(),String>{
        if !self.line_width_mm.is_finite() || !(0.05..=2.0).contains(&self.line_width_mm)
           || !self.hatch_spacing_mm.is_finite() || !(0.3..=20.0).contains(&self.hatch_spacing_mm) {
            return Err("Espessura de linha ou espaçamento de hachura inválido".into());
        }
        Ok(())
    }
    pub fn color(&self)->[f64;3] {
        match self.preset {
            GraphicPreset::Monochrome => [0.0,0.0,0.0],
            GraphicPreset::Conceptual => [0.16,0.36,0.53],
            GraphicPreset::Technical=>self.line_rgb.map(|c| f64::from(c)/255.0),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PrintScale { pub denominator: f64 }
impl PrintScale {
    pub fn parse(text:&str)->Result<Self,String>{
        let (numerator,denominator)=text.trim().split_once(':')
            .ok_or("Formato de escala inválido. Use 1:50 ou 1:100")?;
        let n=numerator.trim().parse::<f64>().map_err(|_|"Escala inválida")?;
        let d=denominator.trim().parse::<f64>().map_err(|_|"Escala inválida")?;
        if !n.is_finite() || !d.is_finite() || (n-1.0).abs()>1e-9
           || !(1.0..=20_000.0).contains(&d) {
            return Err("Escala fora do intervalo 1:1 a 1:20000".into());
        }
        Ok(Self{denominator:d})
    }
    pub fn points_per_model_meter(self)->f64{
        (1000.0/self.denominator)*PT_PER_MM
    }
    pub fn paper_mm_from_model_m(self,length_m:f64)->f64{
        length_m*1000.0/self.denominator
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Dimension {
    pub start:Point2,
    pub end:Point2,
    pub length_m:f64,
    pub element_id:String,
}
impl Dimension {
    pub fn label(&self)->String {format!("{:.2} m",self.length_m)}
}

/// Real geometry dimensions, not hard-coded example labels.
/// Floor plans measure wall centerline and the longest slab edge.
/// Sections/elevations measure element's projected height.
pub fn derived_dimensions(project:&Project,point:&Viewpoint)->Vec<Dimension>{
    let mut result=Vec::new();
    match point.kind {
        ViewpointKind::FloorPlan|ViewpointKind::Detail=>{
            for e in &project.elements {
                if point.level_id.as_deref().is_some_and(|id|id!=e.level_id()){continue;}
                match e {
                    Element::Wall(w)=>{
                        let d=(w.end[0]-w.start[0]).hypot(w.end[1]-w.start[1]);
                        if d>0.001 { result.push(Dimension{
                            start:w.start,end:w.end,length_m:d,element_id:w.id.clone()
                        });}
                    },
                    Element::Slab(s)=>{
                        if s.polygon.len()<2{continue;}
                        let mut best=0.0;
                        let mut a=[0.0,0.0];
                        let mut b=[0.0,0.0];
                        for i in 0..s.polygon.len(){
                            let p=s.polygon[i];let q=s.polygon[(i+1)%s.polygon.len()];
                            let len=(q[0]-p[0]).hypot(q[1]-p[1]);
                            if len>best {best=len;a=p;b=q;}
                        }
                        if best>0.001 {result.push(Dimension{
                            start:a,end:b,length_m:best,element_id:s.id.clone()
                        });}
                    },
                }
            }
        }
        ViewpointKind::Section|ViewpointKind::Elevation=>{
            let lines=crate::documentation::render_viewpoint(project,point).lines;
            let mut grouped:BTreeMap<String,(f64,f64,f64)>=BTreeMap::new();
            for line in lines {
                let entry=grouped.entry(line.element_id).or_insert(
                    (line.start[0],f64::INFINITY,f64::NEG_INFINITY));
                for p in [line.start,line.end]{
                    entry.0=entry.0.min(p[0]);
                    entry.1=entry.1.min(p[1]);
                    entry.2=entry.2.max(p[1]);
                }
            }
            for (id,(x,zmin,zmax)) in grouped {
                if zmax-zmin>0.001 {
                    result.push(Dimension{
                        start:[x,zmin],end:[x,zmax],length_m:zmax-zmin,element_id:id,
                    });
                }
            }
        }
        ViewpointKind::ThreeD=>{},
    }
    result
}

fn diag_segments(
    min:Point2,max:Point2,spacing:f64,ascending:bool,id:&str,out:&mut Vec<DrawingLine>
){
    if max[0]-min[0] < EPS || max[1]-min[1] < EPS {return;}
    let (first,last)=if ascending {
        (min[1]-max[0],max[1]-min[0])
    }else{
        (min[1]+min[0],max[1]+max[0])
    };
    let mut c=first;
    let mut count=0;
    while c<=last && count<2000 {
        let mut hits=Vec::new();
        for x in [min[0],max[0]] {
            let y=if ascending{x+c}else{c-x};
            if y>=min[1]-EPS&&y<=max[1]+EPS {hits.push([x,y]);}
        }
        for y in [min[1],max[1]] {
            let x=if ascending{y-c}else{c-y};
            if x>=min[0]-EPS&&x<=max[0]+EPS {
                if hits.iter().all(|p: &Point2| (p[0]-x).abs()>EPS||(p[1]-y).abs()>EPS) {
                    hits.push([x,y]);
                }
            }
        }
        if hits.len()>=2 {out.push(DrawingLine{
            start:hits[0],end:hits[1],element_id:id.to_owned()
        });}
        c+=spacing;
        count+=1;
    }
}

/// Section cut rectangles use model coordinates; fills remain vector line segments.
/// The spacing argument is in model metres, computed from requested print scale.
pub fn section_hatches(
    drawing:&VectorDrawing, pattern:HatchPattern, spacing_model_m:f64
)->Vec<DrawingLine>{
    if pattern==HatchPattern::None || !spacing_model_m.is_finite()
        || spacing_model_m<=0.0 {return Vec::new();}
    let mut groups:BTreeMap<&str,(Point2,Point2)>=BTreeMap::new();
    for line in &drawing.lines {
        let g=groups.entry(line.element_id.as_str()).or_insert(
            ([f64::INFINITY,f64::INFINITY],
             [f64::NEG_INFINITY,f64::NEG_INFINITY])
        );
        for p in [line.start,line.end]{
            for axis in 0..2{
                g.0[axis]=g.0[axis].min(p[axis]);
                g.1[axis]=g.1[axis].max(p[axis]);
            }
        }
    }
    let mut out=Vec::new();
    for (id,(lo,hi)) in groups {
        diag_segments(lo,hi,spacing_model_m,false,id,&mut out);
        if pattern==HatchPattern::Cross {
            diag_segments(lo,hi,spacing_model_m,true,id,&mut out);
        }
    }
    out
}
fn cross(a:Point2,b:Point2)->f64{a[0]*b[1]-a[1]*b[0]}
fn point_in_polygon(p:Point2,polygon:&[Point2])->bool{
    let mut inside=false;
    let mut j=polygon.len().saturating_sub(1);
    for i in 0..polygon.len(){
        let a=polygon[i];let b=polygon[j];
        if (a[1]>p[1])!=(b[1]>p[1]) {
            let x=(b[0]-a[0])*(p[1]-a[1])/(b[1]-a[1])+a[0];
            if p[0]<x {inside=!inside;}
        }
        j=i;
    }
    inside
}
fn prism(element:&Element,z:f64)->(Vec<Point2>,f64,f64){
    match element {
        Element::Wall(w)=>{
            let dx=w.end[0]-w.start[0];
            let dy=w.end[1]-w.start[1];
            let length=dx.hypot(dy);
            if length<EPS {return(Vec::new(),z,z);}
            let nx=-dy*w.thickness_m/(2.0*length);
            let ny=dx*w.thickness_m/(2.0*length);
            (vec![
                [w.start[0]+nx,w.start[1]+ny],
                [w.end[0]+nx,w.end[1]+ny],
                [w.end[0]-nx,w.end[1]-ny],
                [w.start[0]-nx,w.start[1]-ny],
            ],z,z+w.height_m)
        }
        Element::Slab(s)=>(s.polygon.clone(),z-s.thickness_m,z),
    }
}
fn segment_hits_prism(origin:Point3,target:Point3,polygon:&[Point2],low:f64,high:f64)->bool{
    if polygon.len()<3{return false;}
    let dir=[target[0]-origin[0],target[1]-origin[1],target[2]-origin[2]];
    // Side faces (vertical planes through every polygon edge).
    for i in 0..polygon.len(){
        let a=polygon[i];let b=polygon[(i+1)%polygon.len()];
        let side=[b[0]-a[0],b[1]-a[1]];
        let denom=cross([dir[0],dir[1]],side);
        if denom.abs()<EPS {continue;}
        let offset=[a[0]-origin[0],a[1]-origin[1]];
        let t=cross(offset,side)/denom;
        let u=cross(offset,[dir[0],dir[1]])/denom;
        if t>1e-6 && t<1.0-1e-5 && u>=-EPS && u<=1.0+EPS {
            let height=origin[2]+t*dir[2];
            if height>=low-EPS && height<=high+EPS {return true;}
        }
    }
    // Caps may hide top/bottom edges even if no side face is crossed.
    if dir[2].abs()>EPS {
        for plane in [low,high] {
            let t=(plane-origin[2])/dir[2];
            if t>1e-6 && t<1.0-1e-5 {
                let p=[origin[0]+t*dir[0],origin[1]+t*dir[1]];
                if point_in_polygon(p,polygon){return true;}
            }
        }
    }
    false
}
fn occluded(project:&Project, sample:Point3, eye:Point3)->bool{
    for element in &project.elements{
        let z=project.levels.iter().find(|l|l.id==element.level_id())
            .map_or(0.0,|l|l.elevation_m);
        let (polygon,low,high)=prism(element,z);
        if segment_hits_prism(eye,sample,&polygon,low,high){return true;}
    }
    false
}
fn interpolate(a:Point3,b:Point3,t:f64)->Point3{
    [a[0]+(b[0]-a[0])*t,a[1]+(b[1]-a[1])*t,a[2]+(b[2]-a[2])*t]
}

/// Sampled visibility for prismatic elements. This is not exact B-Rep hidden-line
/// removal; it has a finite sample resolution and no model face topology.
pub fn hidden_line_drawing(project:&Project,saved:&SavedView,view:&Viewpoint)
    ->VectorDrawing{
    let mut drawing=VectorDrawing{title:saved.title.clone(),lines:Vec::new()};
    let is_3d=view.kind==ViewpointKind::ThreeD;
    let yaw=view.camera.yaw_deg.to_radians();
    let normal=[yaw.sin(),yaw.cos(),0.0];
    let right=[yaw.cos(),-yaw.sin()];
    let camera_eye=saved.camera.axes().0;
    let camera_forward=saved.camera.axes().1;
    let projection=|p:Point3|->Option<Point2>{
        if is_3d {
            saved.camera.project(p,1000.0,1000.0).map(|pt|[pt[0],-pt[1]])
        }else{
            Some([(p[0]-view.marker[0])*right[0]+
                (p[1]-view.marker[1])*right[1],p[2]])
        }
    };
    let segments=12;
    for edge in model_wireframe(project) {
        let mut open:Option<Point3>=None;
        let mut last= edge.start;
        for i in 0..segments {
            let t0=i as f64/segments as f64;
            let t1=(i+1) as f64/segments as f64;
            let a=interpolate(edge.start,edge.end,t0);
            let b=interpolate(edge.start,edge.end,t1);
            let sample=interpolate(a,b,0.5);
            let eye=if is_3d {
                if saved.camera.projection==ProjectionMode::Orthographic {
                    [sample[0]-camera_forward[0]*10_000.0,
                     sample[1]-camera_forward[1]*10_000.0,
                     sample[2]-camera_forward[2]*10_000.0]
                }else{camera_eye}
            }else{
                [sample[0]-normal[0]*10_000.0,
                 sample[1]-normal[1]*10_000.0,sample[2]]
            };
            if !occluded(project,sample,eye) {
                if open.is_none(){open=Some(a);}
                last=b;
            } else if let Some(start)=open.take() {
                if let (Some(pa),Some(pb))=(projection(start),projection(a)) {
                    drawing.lines.push(DrawingLine{
                        start:pa,end:pb,element_id:edge.element_id.clone()
                    });
                }
            }
        }
        if let Some(start)=open.take(){
            if let (Some(pa),Some(pb))=(projection(start),projection(last)){
                drawing.lines.push(DrawingLine{
                    start:pa,end:pb,element_id:edge.element_id.clone()
                });
            }
        }
    }
    drawing
}

pub fn styled_view(project:&Project,saved:&SavedView)->Result<VectorDrawing,String>{
    saved.graphics.validate()?;
    let view=project.workspace.viewpoints.iter()
        .find(|v|v.id==saved.viewpoint_id).ok_or("Ponto de vista inexistente")?;
    if saved.graphics.hide_occluded_edges &&
        matches!(view.kind,ViewpointKind::ThreeD|ViewpointKind::Elevation){
        Ok(hidden_line_drawing(project,saved,view))
    }else{
        render_saved_view(project,saved)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Wall,Element};
    use crate::navigation::{Camera,ViewpointKind};

    #[test]
    fn print_scale_is_physical_and_validated(){
        assert_eq!(PrintScale::parse("1:100").unwrap()
            .paper_mm_from_model_m(10.0),100.0);
        assert_eq!(PrintScale::parse("1:50").unwrap()
            .paper_mm_from_model_m(5.0),100.0);
        assert!(PrintScale::parse("2:100").is_err());
        assert!(PrintScale::parse("1:0").is_err());
        assert!(PrintScale::parse("abc").is_err());
    }

    #[test]
    fn derived_dimension_measures_wall_from_live_model(){
        let mut p=Project::default();
        p.elements.push(Element::Wall(Wall{id:"W".into(),level_id:"P0".into(),
            start:[0.0,0.0],end:[4.0,0.0],thickness_m:0.2,height_m:3.0}));
        let view=p.workspace.viewpoints.iter()
            .find(|v|v.kind==ViewpointKind::FloorPlan).unwrap();
        assert!((derived_dimensions(&p,view)[0].length_m-4.0).abs()<1e-9);
        if let Element::Wall(w)=&mut p.elements[0]{w.end[0]=6.0;}
        assert!((derived_dimensions(&p,view)[0].length_m-6.0).abs()<1e-9);
    }

    #[test]
    fn section_hatches_are_clipped_to_cut_rectangle(){
        let drawing=VectorDrawing{title:"Corte".into(),lines:vec![
            DrawingLine{element_id:"A".into(),start:[0.0,0.0],end:[3.0,0.0]},
            DrawingLine{element_id:"A".into(),start:[3.0,0.0],end:[3.0,2.0]},
            DrawingLine{element_id:"A".into(),start:[3.0,2.0],end:[0.0,2.0]},
            DrawingLine{element_id:"A".into(),start:[0.0,2.0],end:[0.0,0.0]},
        ]};
        let lines=section_hatches(&drawing,HatchPattern::Cross,0.2);
        assert!(!lines.is_empty());
        assert!(lines.iter().all(|l|[l.start,l.end].iter()
            .all(|p|p[0]>=-EPS&&p[0]<=3.0+EPS&&p[1]>=-EPS&&p[1]<=2.0+EPS)));
    }

    #[test]
    fn occluded_back_wall_is_filtered_in_elevation() {
        let mut project=Project::default();
        project.elements.push(Element::Wall(Wall{
            id:"near".into(),level_id:"P0".into(),
            start:[-4.0,0.0],end:[4.0,0.0],
            thickness_m:0.4,height_m:4.0,
        }));
        project.elements.push(Element::Wall(Wall{
            id:"far".into(),level_id:"P0".into(),
            start:[-1.0,3.0],end:[1.0,3.0],
            thickness_m:0.2,height_m:2.5,
        }));
        let mut view=project.workspace.viewpoints[0].clone();
        view.id="elev-test".into();
        view.kind=ViewpointKind::Elevation;
        view.camera.yaw_deg=0.0;
        view.marker=[0.0,-5.0,0.0];
        project.workspace.viewpoints.push(view.clone());
        let id=project.workspace.save_view(&view.id,"Elevação".into(),Camera::default()).unwrap();
        let saved=project.workspace.saved_views.iter().find(|v|v.id==id).unwrap();
        let unfiltered=styled_view(&project,saved).unwrap();
        assert!(unfiltered.lines.iter().any(|line|line.element_id=="far"));
        let mut styled=saved.clone();
        styled.graphics.hide_occluded_edges=true;
        let hidden=styled_view(&project,&styled).unwrap();
        assert!(hidden.lines.iter().all(|line|line.element_id!="far"),
            "The front wall should fully occlude the smaller back wall");
        assert!(hidden.lines.iter().any(|line|line.element_id=="near"));
    }

    #[test]
    fn hidden_edge_pass_does_not_add_new_model_geometry(){
        let mut p=Project::default();
        p.elements.push(Element::Wall(Wall{id:"W".into(),level_id:"P0".into(),
            start:[0.0,0.0],end:[4.0,0.0],thickness_m:0.2,height_m:3.0}));
        let vp=p.workspace.viewpoints.iter().find(|v|v.kind==ViewpointKind::ThreeD)
            .unwrap().id.clone();
        let id=p.workspace.save_view(&vp,"Vista".into(),Camera::default()).unwrap();
        let mut saved=p.workspace.saved_views.iter().find(|s|s.id==id).unwrap().clone();
        saved.graphics.hide_occluded_edges=true;
        let drawing=styled_view(&p,&saved).unwrap();
        assert!(!drawing.lines.is_empty());
        assert!(drawing.lines.len()<=model_wireframe(&p).len()*12);
    }
}
