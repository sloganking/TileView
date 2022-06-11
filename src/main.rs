use std::{collections::HashMap, path::PathBuf};
use glob::{glob, GlobError};

use macroquad::prelude::*;

const IMAGE_TILE_WIDTH: i32 = 512;
const IMAGE_TILE_HEIGHT: i32 = 512;

#[derive(Debug)]
struct Bounds {
    max_x: i32,
    max_z: i32,
    min_x: i32,
    min_z: i32,
}

struct FilenameAndNumbers {
    file_name: PathBuf,
    x: i32,
    z: i32,
}

/// Returns a list of all files in a directory and it's subdirectories
fn get_files_in_dir(path: &str, filetype: &str) -> Result<Vec<PathBuf>, GlobError> {
    //> get list of all files and dirs in path, using glob
        let mut paths = Vec::new();

        let mut potential_slash = "";
        if PathBuf::from(path).is_dir() && !path.ends_with('/') {
            potential_slash = "/";
        }

        let search_params = String::from(path) + potential_slash + "**/*" + filetype;

        for entry in glob(&search_params).expect("Failed to read glob pattern") {
            match entry {
                Ok(path) => {
                    paths.push(path);
                }
                Err(e) => return Err(e),
            }
        }

    //<> filter out directories
        let paths = paths.into_iter().filter(|e| e.is_file());
    //<

    let paths: Vec<PathBuf> = paths.into_iter().collect();
    Ok(paths)
}

#[macroquad::main("BasicShapes")]
async fn main() {
    let texture: Texture2D = load_texture("./input/0,0.png").await.unwrap();
    let texture2: Texture2D = load_texture("./input/-1,0.png").await.unwrap();

    // positive X is right
    let mut x_offset = 0.;
    // positive Y is down
    let mut y_offset = 0.;

    let files = get_files_in_dir("./input/", "").unwrap();

    let mut bounds = Bounds {
        max_x: i32::MIN,
        max_z: i32::MIN,
        min_x: i32::MAX,
        min_z: i32::MAX,
    };

    let mut filename_and_numbers_vec: Vec<FilenameAndNumbers> = Vec::new();

    let mut sector_to_texture_z0 = HashMap::new();

    // find max and min dimensions
    for file in files {
        let file_name = file.file_stem().unwrap().to_str().unwrap();
        let split: Vec<&str> = file_name.split(',').collect();

        let x: i32 = split[0].parse().unwrap();
        let z: i32 = split[1].parse().unwrap();

        filename_and_numbers_vec.push(FilenameAndNumbers {
            file_name: file.clone(),
            x,
            z,
        });

        if x > bounds.max_x {
            bounds.max_x = x;
        }
        if z > bounds.max_z {
            bounds.max_z = z;
        }
        if x < bounds.min_x {
            bounds.min_x = x;
        }
        if z < bounds.min_z {
            bounds.min_z = z;
        }

        let texture: Texture2D = load_texture(file.to_str().unwrap()).await.unwrap();
        sector_to_texture_z0.insert((x, z), texture);
    }

    loop {
        clear_background(GRAY);

        draw_line(40.0, 40.0, 100.0, 200.0, 15.0, BLUE);
        draw_rectangle(screen_width() / 2.0 - 60.0, 100.0, 120.0, 60.0, GREEN);
        draw_circle(screen_width() - 30.0, screen_height() - 30.0, 15.0, YELLOW);

        draw_text("IT WORKS!", 20.0, 20.0, 30.0, DARKGRAY);

        let speed = if is_key_down(KeyCode::LeftShift){
            3.
        } else {
            1.
        };

        if is_key_down(KeyCode::Right) || is_key_down(KeyCode::D) {
            x_offset -= speed;
        }
        if is_key_down(KeyCode::Left) || is_key_down(KeyCode::A) {
            x_offset += speed;
        }
        if is_key_down(KeyCode::Up) || is_key_down(KeyCode::W) {
            y_offset += speed;
        }
        if is_key_down(KeyCode::Down) || is_key_down(KeyCode::S) {
            y_offset -= speed;
        }

        //> draw all textures
            for (sector, texture) in &sector_to_texture_z0 {
                draw_texture(
                    *texture,
                    screen_width() / 2. + x_offset + sector.0 as f32 * IMAGE_TILE_WIDTH as f32,
                    screen_height() / 2. + y_offset + sector.1 as f32 * IMAGE_TILE_HEIGHT as f32,
                    WHITE,
                );
            }
        //<

        // draw_texture(
        //     texture,
        //     screen_width() / 2. + x_offset,
        //     screen_height() / 2. + y_offset,
        //     WHITE,
        // );

        // draw_texture(
        //     texture2,
        //     screen_width() / 2. + x_offset - 512.,
        //     screen_height() / 2. + y_offset,
        //     WHITE,
        // );

        // draw_texture_ex(
        //     texture,
        //     screen_width() / 2. + x_offset,
        //     screen_height() / 2. + y_offset,
        //     WHITE,

        // );

        next_frame().await
    }
}
