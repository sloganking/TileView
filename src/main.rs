use glob::{glob, GlobError};
use std::{collections::HashMap, path::PathBuf};

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

async fn get_textures_for_zoom_level(level: u32) -> HashMap<(i32, i32), Texture2D> {
    let files = get_files_in_dir(&("./terrain/".to_owned() + &level.to_string()), "").unwrap();

    let mut sector_to_texture = HashMap::new();

    for file in files {
        let file_name = file.file_stem().unwrap().to_str().unwrap();
        let split: Vec<&str> = file_name.split(',').collect();
        let x: i32 = split[0].parse().unwrap();
        let z: i32 = split[1].parse().unwrap();

        let texture: Texture2D = load_texture(file.to_str().unwrap()).await.unwrap();

        sector_to_texture.insert((x, z), texture);
    }

    sector_to_texture
}

#[macroquad::main("Map Renderer")]
async fn main() {
    // positive X is right
    let mut x_offset = 0.;
    // positive Y is down
    let mut y_offset = 0.;

    // load texture cache
    let mut texture_cache: Vec<HashMap<(i32, i32), Texture2D>> = Vec::new();
    for x in 0..7 {
        texture_cache.push(get_textures_for_zoom_level(x.try_into().unwrap()).await);
    }

    let mut zoom_multiplier: f32 = 1.0;

    loop {
        clear_background(GRAY);

        // draw_line(40.0, 40.0, 100.0, 200.0, 15.0, BLUE);
        // draw_rectangle(screen_width() / 2.0 - 60.0, 100.0, 120.0, 60.0, GREEN);
        // draw_circle(screen_width() - 30.0, screen_height() - 30.0, 15.0, YELLOW);

        // draw_text("IT WORKS!", 20.0, 20.0, 30.0, DARKGRAY);

        //> react to key presses
            let speed = if is_key_down(KeyCode::LeftShift) {
                20. / zoom_multiplier
            } else {
                5. / zoom_multiplier
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

            let zoom_speed = if is_key_down(KeyCode::LeftShift) {
                zoom_multiplier / 100. * 4.
            } else {
                zoom_multiplier / 100.
            };

            if is_key_down(KeyCode::E) {
                zoom_multiplier += zoom_speed;
            }
            if is_key_down(KeyCode::Q) {
                zoom_multiplier -= zoom_speed;
            }

            zoom_multiplier = zoom_multiplier.clamp(0.01, 10.);

        //<> get LOD

            let two: f32 = 2.0;
            let lod = if zoom_multiplier < 1. / two.powf(6.) {
                6
            } else if zoom_multiplier < 1. / two.powf(5.) {
                5
            } else if zoom_multiplier < 1. / two.powf(4.) {
                4
            } else if zoom_multiplier < 1. / two.powf(3.) {
                3
            } else if zoom_multiplier < 1. / two.powf(2.) {
                2
            } else if zoom_multiplier < 1. / two.powf(1.) {
                1
            } else {
                0
            };

        //<> draw all textures

            let list_of_sectors_to_render = [(0, 0)];

            for (sector, _) in &texture_cache[lod] {
                // println!("lod: {}",lod);
                // println!("sector: {} {}", sector.0, sector.1);
                let texture = texture_cache[lod].get(&(sector.0, sector.1)).unwrap();

                let params = DrawTextureParams {
                    dest_size: Some(vec2(
                        IMAGE_TILE_WIDTH as f32 * zoom_multiplier * two.powf(lod as f32),
                        IMAGE_TILE_HEIGHT as f32 * zoom_multiplier * two.powf(lod as f32),
                    )),
                    source: None,
                    rotation: 0.,
                    flip_x: false,
                    flip_y: false,
                    pivot: None,
                };

                draw_texture_ex(
                    *texture,
                    screen_width() / 2.
                        + (x_offset * zoom_multiplier)
                        + sector.0 as f32
                            * IMAGE_TILE_WIDTH as f32
                            * zoom_multiplier
                            * two.powf(lod as f32),
                    screen_height() / 2.
                        + (y_offset * zoom_multiplier)
                        + sector.1 as f32
                            * IMAGE_TILE_HEIGHT as f32
                            * zoom_multiplier
                            * two.powf(lod as f32),
                    WHITE,
                    params,
                );
            }
        //<

        draw_text(
            &("lod level: ".to_owned() + &lod.to_string()),
            20.0,
            20.0,
            30.0,
            WHITE,
        );

        next_frame().await
    }
}
