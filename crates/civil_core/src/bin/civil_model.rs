//! One document and one operation per request. Only a validated result reaches stdout.
use civil_core::{Project, Wall};
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Deserialize)]
struct Request {
    project: Project,
    command: Command,
}

#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
enum Command {
    Validate,
    Summary,
    UpdateWall {
        wall: Wall,
    },
    AddWall {
        level_id: String,
        start: [f64; 2],
        end: [f64; 2],
        thickness_m: f64,
        height_m: f64,
    },
    HostOpening {
        id: String,
        wall_id: String,
        distance_m: f64,
    },
    SetOpeningWidth {
        id: String,
        width_m: f64,
    },
}

fn execute(
    Request {
        mut project,
        command,
    }: Request,
) -> Result<Value, String> {
    project.validate()?;
    let id = match command {
        Command::Validate => return Ok(json!({"valid":true})),
        Command::Summary => return Ok(json!(project.quantities())),
        Command::UpdateWall { wall } => {
            project.update_wall(wall)?;
            None
        }
        Command::AddWall {
            level_id,
            start,
            end,
            thickness_m,
            height_m,
        } => Some(project.add_wall(&level_id, start, end, thickness_m, height_m)?),
        Command::HostOpening {
            id,
            wall_id,
            distance_m,
        } => {
            project.host_opening(&id, &wall_id, distance_m)?;
            None
        }
        Command::SetOpeningWidth { id, width_m } => {
            project.set_door_width(&id, width_m)?;
            None
        }
    };
    project.validate()?;
    Ok(json!({"project":project,"id":id}))
}
fn main() {
    let result = serde_json::from_reader(std::io::stdin().lock())
        .map_err(|e| e.to_string())
        .and_then(execute);
    match result {
        Ok(value) => println!("{value}"),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}
