use glob::{glob, GlobError};
use std::{collections::HashMap, path::PathBuf};

use macroquad::prelude::*;

// #[derive(Debug)]
// struct Bounds {
//     max_x: i32,
//     max_z: i32,
//     min_x: i32,
//     min_z: i32,
// }

// struct FilenameAndNumbers {
//     file_name: PathBuf,
//     x: i32,
//     z: i32,
// }

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

const TILE_DIR: &str = "./terrain/";

async fn get_textures_for_zoom_level(
    level: u32,
    directory: &str,
    tile_dimensions: (f32, f32),
) -> HashMap<(i32, i32), Texture2D> {
    let files = get_files_in_dir(&(directory.to_owned() + &level.to_string()), "").unwrap();

    let mut sector_to_texture = HashMap::new();

    for file in files {
        // get sector from filename
        let file_name = file.file_stem().unwrap().to_str().unwrap();
        let split: Vec<&str> = file_name.split(',').collect();
        let x: i32 = split[0].parse().unwrap();
        let z: i32 = split[1].parse().unwrap();

        // map sector to texture
        let texture: Texture2D = load_texture(file.to_str().unwrap()).await.unwrap();
        texture.set_filter(FilterMode::Nearest);
        if texture.width() != tile_dimensions.0 || texture.height() != tile_dimensions.1 {
            panic!("File: \"{}\" has differing dimensions", file_name)
        }
        sector_to_texture.insert((x, z), texture);
    }

    sector_to_texture
}

fn coord_to_screen_pos(x: i32, y: i32, camera: &CameraSettings) -> (f32, f32) {
    let out_x = screen_width() / 2. + ((camera.x_offset + x as f32) * camera.zoom_multiplier);
    let out_y = screen_height() / 2. + ((camera.y_offset + y as f32) * camera.zoom_multiplier);
    (out_x, out_y)
}

fn screen_pos_to_coord(x: f32, y: f32, camera: &CameraSettings) -> (f32, f32) {
    let x_out = -camera.x_offset + (x as f32 - screen_width() / 2.) / camera.zoom_multiplier;
    let y_out = -camera.y_offset + (y as f32 - screen_height() / 2.) / camera.zoom_multiplier;
    (x_out, y_out)
}

struct Rectangle {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

fn value_in_range(value: f32, min: f32, max: f32) -> bool {
    (value >= min) && (value <= max)
}

/// returns true if two rectangles overlap
///
/// Resources:
///
/// https://stackoverflow.com/questions/306316/determine-if-two-rectangles-overlap-each-other
///
/// https://silentmatt.com/rectangle-intersection/
fn rectangle_overlap(a: Rectangle, b: Rectangle) -> bool {
    let x_overlap =
        value_in_range(a.x, b.x, b.x + b.width) || value_in_range(b.x, a.x, a.x + a.width);

    let y_overlap =
        value_in_range(a.y, b.y, b.y + b.height) || value_in_range(b.y, a.y, a.y + a.height);

    x_overlap && y_overlap
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

    let mut tile_dimensions: (f32, f32) = (0., 0.);

    // load texture cache
    let mut max_zoom_level: u32 = 0;
    let mut texture_cache: Vec<HashMap<(i32, i32), Texture2D>> = Vec::new();
    for x in 0.. {
        if PathBuf::from(TILE_DIR.to_owned() + &x.to_string()).is_dir() {
            // get initial tile dimensions
            if x == 0 {
                let files = get_files_in_dir(&(TILE_DIR.to_owned() + &0.to_string()), "").unwrap();

                let texture: Texture2D = load_texture(files[0].to_str().unwrap()).await.unwrap();
                tile_dimensions.0 = texture.width();
                tile_dimensions.1 = texture.height();
            }

            texture_cache.push(
                get_textures_for_zoom_level(x.try_into().unwrap(), TILE_DIR, tile_dimensions).await,
            );
            max_zoom_level = x;
        } else {
            break;
        }
    }

    let mut mouse_clicked_in_position: Option<(f32, f32)> = None;
    let mut clicked_in_x_offset: f32 = 0.0;
    let mut clicked_in_y_offset: f32 = 0.0;

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

            // zoom via buttons
            if is_key_down(KeyCode::E) {
                camera.zoom_multiplier += zoom_speed;
            }
            if is_key_down(KeyCode::Q) {
                camera.zoom_multiplier -= zoom_speed;
            }

            // zoom via scroll wheel
            let (_, mouse_scroll) = mouse_wheel();
            if mouse_scroll == 1.0 {
                camera.zoom_multiplier += zoom_speed * 10.;
            } else if mouse_scroll == -1.0 {
                camera.zoom_multiplier -= zoom_speed * 10.;
            }

            // limit the zoom
            camera.zoom_multiplier = camera.zoom_multiplier.clamp(0.01, 20.);

            // mouse drag screen
            if is_mouse_button_down(MouseButton::Left) {
                if mouse_clicked_in_position == None {
                    mouse_clicked_in_position = Some(mouse_position());
                    clicked_in_x_offset = camera.x_offset;
                    clicked_in_y_offset = camera.y_offset;
                } else {
                    let cur_mouse_pos = mouse_position();

                    // calc new x_offset
                    let mouse_x_diff = cur_mouse_pos.0 - mouse_clicked_in_position.unwrap().0;
                    camera.x_offset = clicked_in_x_offset + mouse_x_diff / camera.zoom_multiplier;

                    // calc new y_offset
                    let mouse_y_diff = cur_mouse_pos.1 - mouse_clicked_in_position.unwrap().1;
                    camera.y_offset = clicked_in_y_offset + mouse_y_diff / camera.zoom_multiplier;
                }
            } else {
                mouse_clicked_in_position = None;
            }

        //<> get LOD

            let two: f32 = 2.0;
            let mut lod: usize = 0;
            for level in 0..=max_zoom_level {
                if camera.zoom_multiplier < 1. / two.powf(level as f32) {
                    lod = level as usize;
                } else {
                    break;
                }
            }
            // re-make immutable
            let lod = lod;

        //<> draw all textures

            // let list_of_sectors_to_render = [(0, 0)];

            let mut rendered_tiles = 0;
            for (sector, _) in &texture_cache[lod] {
                let sx = screen_width() / 2.
                    + (camera.x_offset * camera.zoom_multiplier)
                    + sector.0 as f32
                        * tile_dimensions.0 as f32
                        * camera.zoom_multiplier
                        * two.powf(lod as f32);

                let sy = screen_height() / 2.
                    + (camera.y_offset * camera.zoom_multiplier)
                    + sector.1 as f32
                        * tile_dimensions.1 as f32
                        * camera.zoom_multiplier
                        * two.powf(lod as f32);

                let tile_width =
                    tile_dimensions.0 as f32 * camera.zoom_multiplier * two.powf(lod as f32);
                let tile_height =
                    tile_dimensions.1 as f32 * camera.zoom_multiplier * two.powf(lod as f32);

                let tile_rect = Rectangle {
                    x: sx,
                    y: sy,
                    width: tile_width,
                    height: tile_height,
                };

                let screen_rect = Rectangle {
                    x: 0.0,
                    y: 0.0,
                    width: screen_width(),
                    height: screen_height(),
                };

                //if texture pos on screen
                if rectangle_overlap(screen_rect, tile_rect) {
                    rendered_tiles += 1;
                    let texture = texture_cache[lod].get(&(sector.0, sector.1)).unwrap();

                    let params = DrawTextureParams {
                        dest_size: Some(vec2(tile_width, tile_height)),
                        source: None,
                        rotation: 0.,
                        flip_x: false,
                        flip_y: false,
                        pivot: None,
                    };

                    draw_texture_ex(*texture, sx, sy, WHITE, params);
                }
            }
        //<

        draw_text(
            &("fps: ".to_owned() + &get_fps().to_string()),
            20.0,
            20.0,
            30.0,
            WHITE,
        );

        draw_text(
            &("zoom_multiplier: ".to_owned() + &camera.zoom_multiplier.to_string()),
            20.0,
            40.0,
            30.0,
            WHITE,
        );

        draw_text(
            &("LOD: ".to_owned() + &lod.to_string()),
            20.0,
            60.0,
            30.0,
            WHITE,
        );

        draw_text(
            &("rendered_tiles: ".to_owned() + &rendered_tiles.to_string()),
            20.0,
            80.0,
            30.0,
            WHITE,
        );

        let mouse = mouse_position();
        let mouse_coord = screen_pos_to_coord(mouse.0, mouse.1, &camera);
        draw_text(
            &("mouse.x: ".to_owned() + &mouse_coord.0.to_string()),
            20.0,
            100.0,
            30.0,
            WHITE,
        );

        draw_text(
            &("mouse.y: ".to_owned() + &mouse_coord.1.to_string()),
            20.0,
            120.0,
            30.0,
            WHITE,
        );

        // draw_text(
        //     &("camera.x_offset: ".to_owned() + &(camera.x_offset).to_string()),
        //     20.0,
        //     140.0,
        //     30.0,
        //     WHITE,
        // );

        // // draw beacon
        // let coords = coord_to_screen_pos(0, 13000, &camera);
        // draw_circle(coords.0, coords.1, 5.0, YELLOW);
        // draw_text("Test beacon", coords.0, coords.1, 30.0, WHITE);

        // // draw blue line on map
        // let coords1 = coord_to_screen_pos(-4800, -5200, &camera);
        // let coords2 = coord_to_screen_pos(13000, 0, &camera);
        // draw_line(coords1.0, coords1.1, coords2.0, coords2.1, 15.0, BLUE);

        // // draw dot at end of line
        // let coords = coord_to_screen_pos(-4800, -5200, &camera);
        // draw_circle(coords.0, coords.1, 15.0, BLUE);

        next_frame().await
    }
}
