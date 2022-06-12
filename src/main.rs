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

fn coord_to_2d_pos(x: i32, y: i32, camera: &CameraSettings) -> (f32, f32) {
    let out_x = screen_width() / 2. + ((camera.x_offset + x as f32 * 2.) * camera.zoom_multiplier);

    let out_y = screen_height() / 2. + ((camera.y_offset + y as f32 * 2.) * camera.zoom_multiplier);

    (out_x, out_y)
}

struct CameraSettings {
    x_offset: f32,
    y_offset: f32,
    zoom_multiplier: f32,
}

#[macroquad::main("Map Renderer")]
async fn main() {
    let mut camera = CameraSettings {
        x_offset: 0.,
        y_offset: 0.,
        zoom_multiplier: 1.0,
    };

    // load texture cache
    let mut texture_cache: Vec<HashMap<(i32, i32), Texture2D>> = Vec::new();
    for x in 0..7 {
        texture_cache.push(get_textures_for_zoom_level(x.try_into().unwrap()).await);
    }

    loop {
        clear_background(GRAY);

        // draw_line(40.0, 40.0, 100.0, 200.0, 15.0, BLUE);
        // draw_rectangle(screen_width() / 2.0 - 60.0, 100.0, 120.0, 60.0, GREEN);
        // draw_circle(screen_width() - 30.0, screen_height() - 30.0, 15.0, YELLOW);

        // draw_text("IT WORKS!", 20.0, 20.0, 30.0, DARKGRAY);

        //> react to key presses
            let speed = if is_key_down(KeyCode::LeftShift) {
                20. / camera.zoom_multiplier
            } else {
                5. / camera.zoom_multiplier
            };

            if is_key_down(KeyCode::Right) || is_key_down(KeyCode::D) {
                camera.x_offset -= speed;
            }
            if is_key_down(KeyCode::Left) || is_key_down(KeyCode::A) {
                camera.x_offset += speed;
            }
            if is_key_down(KeyCode::Up) || is_key_down(KeyCode::W) {
                camera.y_offset += speed;
            }
            if is_key_down(KeyCode::Down) || is_key_down(KeyCode::S) {
                camera.y_offset -= speed;
            }

            let zoom_speed = if is_key_down(KeyCode::LeftShift) {
                camera.zoom_multiplier / 100. * 4.
            } else {
                camera.zoom_multiplier / 100.
            };

            if is_key_down(KeyCode::E) {
                camera.zoom_multiplier += zoom_speed;
            }
            if is_key_down(KeyCode::Q) {
                camera.zoom_multiplier -= zoom_speed;
            }

            camera.zoom_multiplier = camera.zoom_multiplier.clamp(0.01, 10.);

        //<> get LOD

            let two: f32 = 2.0;
            let lod = if camera.zoom_multiplier < 1. / two.powf(6.) {
                6
            } else if camera.zoom_multiplier < 1. / two.powf(5.) {
                5
            } else if camera.zoom_multiplier < 1. / two.powf(4.) {
                4
            } else if camera.zoom_multiplier < 1. / two.powf(3.) {
                3
            } else if camera.zoom_multiplier < 1. / two.powf(2.) {
                2
            } else if camera.zoom_multiplier < 1. / two.powf(1.) {
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
                        IMAGE_TILE_WIDTH as f32 * camera.zoom_multiplier * two.powf(lod as f32),
                        IMAGE_TILE_HEIGHT as f32 * camera.zoom_multiplier * two.powf(lod as f32),
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
                        + (camera.x_offset * camera.zoom_multiplier)
                        + sector.0 as f32
                            * IMAGE_TILE_WIDTH as f32
                            * camera.zoom_multiplier
                            * two.powf(lod as f32),
                    screen_height() / 2.
                        + (camera.y_offset * camera.zoom_multiplier)
                        + sector.1 as f32
                            * IMAGE_TILE_HEIGHT as f32
                            * camera.zoom_multiplier
                            * two.powf(lod as f32),
                    WHITE,
                    params,
                );
            }
        //<

        draw_text(
            &("camera.x_offset: ".to_owned() + &camera.x_offset.to_string()),
            20.0,
            20.0,
            30.0,
            WHITE,
        );

        // draw circle at 0,0
        let coords = coord_to_2d_pos(0, 13000, &camera);
        draw_circle(coords.0, coords.1, 5.0, YELLOW);

        // lod level matters
        // camera zoom matters
        // offset matters
        // tile width matters

        let sector_x = (camera.x_offset * camera.zoom_multiplier);

        let tile_width = IMAGE_TILE_WIDTH as f32 * camera.zoom_multiplier * two.powf(lod as f32);

        next_frame().await
    }
}
