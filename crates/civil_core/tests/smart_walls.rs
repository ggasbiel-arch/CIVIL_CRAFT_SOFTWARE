use civil_core::{Element, Project, ReferenceAxis, Wall, WallFace, WallLayer};

fn wall(project: &Project, index: usize) -> Wall {
    let Element::Wall(w) = &project.elements[index] else {
        panic!("parede esperada")
    };
    w.clone()
}
fn area(faces: &[WallFace], id: Option<&str>) -> f64 {
    faces
        .iter()
        .filter(|face| id.is_none_or(|id| id == face.wall_id))
        .map(|face| {
            let p = &face.polygon;
            (0..p.len())
                .map(|i| {
                    let q = p[(i + 1) % p.len()];
                    p[i][0] * q[1] - p[i][1] * q[0]
                })
                .sum::<f64>()
                .abs()
                * 0.5
        })
        .sum()
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-8, "{a} vs {b}");
}
#[test]
fn l_t_x_butt_junctions_partition_the_union_and_respect_priority() {
    for (start, end, coverage) in [
        ([2., 0.], [2., 2.], 0.79),
        ([1., 0.], [1., 2.], 0.78),
        ([1., -1.], [1., 1.], 0.76),
    ] {
        let mut p = Project::default();
        let first = p.add_wall("P0", [0., 0.], [2., 0.], 0.2, 3.).unwrap();
        let second = p.add_wall("P0", start, end, 0.2, 3.).unwrap();
        let plan = p.wall_plan("P0");
        let expected = coverage;
        close(area(&plan, None), expected);
        close(area(&plan, Some(&first)), 0.4);
        let mut w = wall(&p, 1);
        let mut layer = WallLayer::base(0.2);
        layer.priority = 3;
        w.layers = vec![layer];
        p.update_wall(w).unwrap();
        let plan = p.wall_plan("P0");
        close(area(&plan, Some(&second)), 0.4);
        close(area(&plan, None), expected);
        p.elements.reverse();
        assert_eq!(p.wall_plan("P0"), plan);
    }
}
#[test]
fn moving_reversed_and_nearly_collinear_strips_preserves_layers_and_bounds() {
    let mut p = Project::default();
    p.add_wall("P0", [0., 0.], [4., 0.], 0.2, 3.).unwrap();
    p.add_wall("P0", [4., 1e-11], [0., 0.], 0.2, 3.).unwrap();
    close(area(&p.wall_plan("P0"), None), 0.8);
    let mut w = wall(&p, 1);
    w.start[1] = 1.;
    w.end[1] = 1.;
    w.layers = vec![WallLayer::base(0.1), WallLayer::base(0.2)];
    w.layers[1].material = "Gesso".into();
    w.axis = ReferenceAxis::LeftFace;
    p.update_wall(w).unwrap();
    close(area(&p.wall_plan("P0"), None), 2.);
    close(wall(&p, 1).thickness_m, 0.3);
    let previous = p.clone();
    let mut w = wall(&p, 1);
    w.layers[0].thickness_m = -0.1;
    assert!(p.update_wall(w).is_err());
    assert_eq!(p, previous);
}
#[test]
fn hosted_openings_follow_rotated_axis_and_reject_conflicts_atomically() {
    let mut p = Project::default();
    let id = p.add_wall("P0", [0., 0.], [4., 0.], 0.2, 3.).unwrap();
    let door = p.add_door("P0", [0., 0.]).unwrap();
    p.host_opening(&door, &id, 0.).unwrap();
    let window = p.add_window(&id, 1.5).unwrap();
    p.set_door_width(&door, 1.2).unwrap();
    close(area(&p.wall_plan("P0"), None), (4. - 2.4) * 0.2);
    let mut w = wall(&p, 0);
    w.start = [2., 1.];
    w.end = [2., 5.];
    p.update_wall(w).unwrap();
    let g = p.door_geometry(&p.component_instances[1]).unwrap();
    assert_eq!(g.opening[0], [2., 2.5, 1.]);
    close(g.leaf[2][2], 2.);
    let before = p.clone();
    assert!(p.set_door_width(&door, 1.8).is_err());
    assert!(p.host_opening(&window, &id, 3.).is_err());
    assert!(p.add_window(&id, -0.1).is_err());
    let mut short = wall(&p, 0);
    short.end = [2., 3.];
    assert!(p.update_wall(short).is_err());
    assert_eq!(p, before);
    let id2 = p.add_wall("P0", [0., 0.], [5., 0.], 0.2, 3.).unwrap();
    p.host_opening(&window, &id2, 3.8).unwrap();
    assert_eq!(p.openings(&wall(&p, 0)).len(), 1);
    assert_eq!(p.openings(&wall(&p, 1)).len(), 1);
}
#[test]
fn fixture_levels_history_data_and_legacy_migration() {
    let mut p: Project =
        serde_json::from_str(include_str!("../../../samples/oficina_paredes.CCW")).unwrap();
    p.validate().unwrap();
    assert_eq!(p.component_instances.len(), 4);
    let before = p.quantities();
    p.set_level_elevation("P1", 4.1).unwrap();
    close(
        p.quantities().wall_gross_area_m2 - before.wall_gross_area_m2,
        28.,
    );
    let saved = p.clone();
    assert!(p.set_level_elevation("P1", 1.).is_err());
    assert_eq!(p, saved);
    let opened: Project = serde_json::from_slice(&serde_json::to_vec(&p).unwrap()).unwrap();
    assert_eq!(opened, p);
    assert_eq!(opened.wall_plan("P0"), p.wall_plan("P0"));
    let old: Project =
        serde_json::from_str(include_str!("../../../samples/casa_piloto.bim.json")).unwrap();
    assert_eq!(old.schema_version, 2);
    close(old.quantities().wall_length_m, 28.);
}

#[test]
fn cached_clips_match_full_regeneration_after_edits_and_removal() {
    let mut cache = civil_core::WallPlanCache::default();
    let mut project = Project::example();
    cache.update(&project, "P0");
    let mut w = wall(&project, 0);
    w.start[1] += 1.;
    w.end[1] += 1.;
    project.update_wall(w).unwrap();
    assert_eq!(cache.update(&project, "P0"), project.wall_plan("P0"));
    project.elements.remove(0);
    assert_eq!(cache.update(&project, "P0"), project.wall_plan("P0"));
    assert!(cache.update(&project, "outro").is_empty());
}

#[test]
fn automation_uses_the_same_geometry_after_python_round_trip() {
    let input = include_str!("../../../samples/oficina_paredes.CCW");
    let mut project: Project = serde_json::from_str(input).unwrap();
    let before = project.quantities().hosted_opening_area_m2;
    project.set_door_width("D1", 1.2).unwrap();
    close(
        project.quantities().hosted_opening_area_m2 - before,
        0.3 * 2.1,
    );
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let script = "from pathlib import Path\nfrom civil_tools import read_project,model_command,quantities\np=read_project(Path('samples/oficina_paredes.CCW'))\np=model_command(p,{'op':'set_opening_width','id':'D1','width_m':1.2})['project']\nprint(quantities(p)['hosted_opening_area_m2'])";
    let output = std::process::Command::new(if cfg!(windows) { "python" } else { "python3" })
        .env("PYTHONPATH", root.join("python"))
        .env("CIVIL_MODEL_BIN", env!("CARGO_BIN_EXE_civil_model"))
        .current_dir(root)
        .args(["-c", script])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    close(
        String::from_utf8(output.stdout)
            .unwrap()
            .trim()
            .parse()
            .unwrap(),
        project.quantities().hosted_opening_area_m2,
    );
}

#[test]
fn regenerates_a_storey_of_one_thousand_disjoint_walls() {
    let mut project = Project::default();
    project
        .add_wall("P0", [0., 0.], [3., 0.], 0.2, 2.8)
        .unwrap();
    let template = wall(&project, 0);
    for index in 1..1000 {
        let mut w = template.clone();
        w.id = format!("W{}", index + 1);
        w.start[1] = index as f64;
        w.end[1] = index as f64;
        project.elements.push(Element::Wall(w));
    }
    let mut cache = civil_core::WallPlanCache::default();
    let t = std::time::Instant::now();
    let faces = cache.update(&project, "P0");
    let first = t.elapsed();
    assert_eq!(faces.len(), 1000);
    let mut edit = template;
    edit.end[0] += 1.;
    project.update_wall(edit).unwrap();
    let t = std::time::Instant::now();
    let updated = cache.update(&project, "P0");
    eprintln!(
        "1000 paredes, primeira planta {first:?}, atualização {:?}",
        t.elapsed()
    );
    close(area(&updated, None) - area(&faces, None), 0.2);
}
