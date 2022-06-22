use glob::{glob, GlobError};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex}, fs::File, io::Read,
};

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


const TILE_DIR: &str = "./tile_images/terrain/";

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

fn coord_to_screen_pos(x: f32, y: f32, camera: &CameraSettings) -> Point {
    let out_x = screen_width() / 2. + ((camera.x_offset + x) * camera.zoom_multiplier);
    let out_y = screen_height() / 2. + ((camera.y_offset + y) * camera.zoom_multiplier);
    Point{
        x: out_x,
        y: out_y
    }
}

fn screen_pos_to_coord(x: f32, y: f32, camera: &CameraSettings) -> Point {
    let x_out = -camera.x_offset + (x as f32 - screen_width() / 2.) / camera.zoom_multiplier;
    let y_out = -camera.y_offset + (y as f32 - screen_height() / 2.) / camera.zoom_multiplier;
    Point{
        x: x_out,
        y: y_out,
    }
}

struct Rectangle {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

// fn value_in_range(value: f32, min: f32, max: f32) -> bool {
//     (value >= min) && (value <= max)
// }

// /// returns true if two rectangles overlap
// ///
// /// Resources:
// ///
// /// https://stackoverflow.com/questions/306316/determine-if-two-rectangles-overlap-each-other
// ///
// /// https://silentmatt.com/rectangle-intersection/
// fn rectangle_overlap(a: Rectangle, b: Rectangle) -> bool {
//     let x_overlap =
//         value_in_range(a.x, b.x, b.x + b.width) || value_in_range(b.x, a.x, a.x + a.width);

//     let y_overlap =
//         value_in_range(a.y, b.y, b.y + b.height) || value_in_range(b.y, a.y, a.y + a.height);

//     x_overlap && y_overlap
// }

fn sector_at_screen_pos(
    x: f32,
    y: f32,
    camera: &CameraSettings,
    tile_dimensions: (f32, f32),
    lod: usize,
) -> (i32, i32) {
    let two: f32 = 2.0;
    let screen_point_coords = screen_pos_to_coord(x, y, camera);

    // get sector x
    let tile_world_x_size = tile_dimensions.0 as f32 * two.powf(lod as f32);
    let screen_point_sector_x = if screen_point_coords.x < 0. {
        (screen_point_coords.x / tile_world_x_size) as i32 - 1
    } else {
        (screen_point_coords.x / tile_world_x_size) as i32
    };

    // get sector y
    let tile_world_y_size = tile_dimensions.1 as f32 * two.powf(lod as f32);
    let screen_point_sector_y = if screen_point_coords.y < 0.0 {
        (screen_point_coords.y / tile_world_y_size) as i32 - 1
    } else {
        (screen_point_coords.y / tile_world_y_size) as i32
    };

    (screen_point_sector_x, screen_point_sector_y)
}

/// stores texture in hdd_texture_cache. Does not check if it is already there.
async fn cache_texture(
    tile_data: (i32, i32, usize),
    mutex_hdd_texture_cache: Arc<Mutex<HashMap<(i32, i32, usize), Option<Texture2D>>>>,
    mutex_retrieving_tile_map: Arc<Mutex<HashMap<(i32, i32, usize), bool>>>,
) {
    let (sector_x, sector_y, lod) = tile_data;

    let texture_dir = TILE_DIR.to_owned()
        + &lod.to_string()
        + "/"
        + &sector_x.to_string()
        + ","
        + &sector_y.to_string()
        + ".png";

    match load_texture(&texture_dir).await {
        Ok(texture) => {
            texture.set_filter(FilterMode::Nearest);
            let mut hdd_texture_cache = mutex_hdd_texture_cache.lock().unwrap();
            hdd_texture_cache.insert((sector_x, sector_y, lod), Some(texture));
        }

        _ => {
            let mut hdd_texture_cache = mutex_hdd_texture_cache.lock().unwrap();
            hdd_texture_cache.insert((sector_x, sector_y, lod), None);
        }
    };

    // mark tile as no longer activly being retrieved
    let mut retrieving_tile_map = mutex_retrieving_tile_map.lock().unwrap();
    retrieving_tile_map.remove(&tile_data);
    drop(retrieving_tile_map);
}

fn pathtype_to_color(pathtype: &str) -> Color{
    match pathtype {
        "iceroad" => BLUE,
        "roofless iceroad" => SKYBLUE,
        "rail" => GRAY,
        "normal" => GREEN,
        _ => GREEN,
    }
}

fn point_on_screen(point: &Point) -> bool {
    !(point.x < 0. || point.x > screen_width() - 0. || point.y < 0. || point.y > screen_height() - 0.)
}

fn get_path_lines(path_data: &serde_json::Value) -> (Vec<PathLine>, Vec<Intersection>){

    let mut path_lines = Vec::new();
    let mut intersections = Vec::new();

    let nodes = path_data.as_object().unwrap();
    let mut already_rendered: HashMap<String, bool> = HashMap::new();

    // for every existing
    for (node, node_value) in nodes{
        // println!("node: {}",node);
        if let Some(neighbors) = node_value["connections"].as_object(){
            for (neighbor, _) in neighbors {

                // if line not already rendered
                if already_rendered.get(&(neighbor.to_owned() + node)) == None{

                    // mark line as rendered
                    already_rendered.insert(node.to_owned() + neighbor, true);

                    let node_pathtype = nodes[node]["pathType"].as_str().unwrap();
                    let neighbor_pathtype = nodes[neighbor]["pathType"].as_str().unwrap();

                    let path_type = if node_pathtype == neighbor_pathtype{
                        node_pathtype
                    } else{
                        "normal"
                    };

                    path_lines.push(PathLine{
                        point1: Point { x: nodes[node]["x"].as_f64().unwrap() as f32, y: nodes[node]["z"].as_f64().unwrap() as f32 },
                        point2: Point { x: nodes[neighbor]["x"].as_f64().unwrap() as f32, y: nodes[neighbor]["z"].as_f64().unwrap() as f32 },
                        color: pathtype_to_color(path_type),
                    });
                }
            }
            if neighbors.len() > 2{
               
                
                intersections.push(Intersection{
                    point:  Point { x: nodes[node]["x"].as_f64().unwrap() as f32, y: nodes[node]["z"].as_f64().unwrap() as f32 },
                    color: pathtype_to_color(nodes[node]["pathType"].as_str().unwrap()),
                });
            }
        }
    }

    (path_lines, intersections)
}

fn render_lines(path_lines: &[PathLine], camera: &CameraSettings){
    for line in path_lines{
        let coords1 = coord_to_screen_pos(line.point1.x + 0.5, line.point1.y + 0.5, &camera);
        let coords2 = coord_to_screen_pos(line.point2.x + 0.5, line.point2.y + 0.5, &camera);

        let line_line = Line{
            point1: Point { x: coords1.x, y: coords1.y },
            point2: Point { x: coords2.x, y: coords2.y },
        };

        let screen_rectangle = Rectangle{
            x: 0.,
            y: 0.,
            width: screen_width() - 0.,
            height: screen_height() - 0.,
        };

        if point_on_screen(&coords1) || point_on_screen(&coords2) || line_intersects_rectangle(&line_line, &screen_rectangle) {
            draw_line(coords1.x, coords1.y, coords2.x, coords2.y, 5.0, line.color);
        }
        
    }
}

fn render_intersections(intersections: &[Intersection], camera: &CameraSettings){
    for intersection in intersections{
        let intersection_coords = coord_to_screen_pos(intersection.point.x + 0.5, intersection.point.y + 0.5, &camera);
        if point_on_screen(&intersection_coords){
            draw_circle(intersection_coords.x, intersection_coords.y, 8.0, intersection.color);
        }
    }
}

fn lines_intersect(line1: &Line, line2: &Line) -> bool {

    let x1 = line1.point1.x;
    let x2 = line1.point2.x;
    let x3 = line2.point1.x;
    let x4 = line2.point2.x;

    let y1 = line1.point1.y;
    let y2 = line1.point2.y;
    let y3 = line2.point1.y;
    let y4 = line2.point2.y;

    let u_a: f32 = ((x4-x3)*(y1-y3) - (y4-y3)*(x1-x3)) / ((y4-y3)*(x2-x1) - (x4-x3)*(y2-y1));
    let u_b: f32 = ((x2-x1)*(y1-y3) - (y2-y1)*(x1-x3)) / ((y4-y3)*(x2-x1) - (x4-x3)*(y2-y1));

    u_a >= 0.0 && u_a <= 1.0 && u_b >= 0.0 && u_b <= 1.0
}

fn line_intersects_rectangle(line: &Line, rectangle: &Rectangle) -> bool {
    let top_line = Line{
        point1: Point{
            x: rectangle.x,
            y: rectangle.y,
        },
        point2: Point{
            x: rectangle.x + rectangle.width,
            y: rectangle.y,
        },
    };

    let left_line = Line{
        point1: Point{
            x: rectangle.x,
            y: rectangle.y,
        },
        point2: Point{
            x: rectangle.x,
            y: rectangle.y + rectangle.height,
        },
    };

    let bottom_line = Line{
        point1: Point{
            x: rectangle.x,
            y: rectangle.y + rectangle.height,
        },
        point2: Point{
            x: rectangle.x + rectangle.width,
            y: rectangle.y + rectangle.height,
        },
    };

    let right_line = Line{
        point1: Point{
            x: rectangle.x + rectangle.width,
            y: rectangle.y,
        },
        point2: Point{
            x: rectangle.x + rectangle.width,
            y: rectangle.y + rectangle.height,
        },
    };

    let left = lines_intersect(&left_line, line);
    let right = lines_intersect(&right_line, line);
    let bottom = lines_intersect(&bottom_line, line);
    let top = lines_intersect(&top_line, line);

    left || right || bottom || top
}

fn distance_between_points(point1: Point, point2: Point) -> f32 {
    ((point1.x - point2.x).abs().powf(2.) + (point1.y - point2.y).abs().powf(2.)).sqrt()
}

struct Point{
    x: f32,
    y: f32,
}

struct Line {
    point1: Point,
    point2: Point,
}

struct PathLine {
    point1: Point,
    point2: Point,
    color: Color,
}

struct Intersection{
    point: Point,
    color: Color,
}

struct CameraSettings {
    x_offset: f32,
    y_offset: f32,
    zoom_multiplier: f32,
}

#[macroquad::main("Map Renderer")]
async fn main() {
    // get initial tile dimensions
    let mut tile_dimensions: (f32, f32) = (0., 0.);
    let files = get_files_in_dir(&(TILE_DIR.to_owned() + &0.to_string()), "").unwrap();
    let texture: Texture2D = load_texture(files[0].to_str().unwrap()).await.unwrap();
    tile_dimensions.0 = texture.width();
    tile_dimensions.1 = texture.height();
    texture.delete();

    // load texture cache
    let mut max_lod: u32 = 0;
    // let mut texture_cache: Vec<HashMap<(i32, i32), Texture2D>> = Vec::new();
    for x in 0.. {
        if PathBuf::from(TILE_DIR.to_owned() + &x.to_string()).is_dir() {
            // texture_cache.push(
            //     get_textures_for_zoom_level(x.try_into().unwrap(), TILE_DIR, tile_dimensions).await,
            // );
            max_lod = x;
        } else {
            break;
        }
    }

    let two: f32 = 2.0;
    let default_zoom = 1.0 / two.powf(max_lod as f32 - 1.0) as f32;

    let mut camera = CameraSettings {
        x_offset: 0.,
        y_offset: 0.,
        zoom_multiplier: default_zoom,
    };

    let mut mouse_clicked_in_position: Option<(f32, f32)> = None;
    let mut clicked_in_x_offset: f32 = 0.0;
    let mut clicked_in_y_offset: f32 = 0.0;

    // stores cached textures
    let arc_mutex_hdd_texture_cache: Arc<Mutex<HashMap<(i32, i32, usize), Option<Texture2D>>>> =
        Arc::new(Mutex::new(HashMap::new()));

    // keeps track of which tiles are currently being retreived
    let mutex_retrieving_tile_map: Arc<Mutex<HashMap<(i32, i32, usize), bool>>> =
        Arc::new(Mutex::new(HashMap::new()));

    use futures::executor::LocalPool;
    use futures::future::{pending, ready};
    use futures::task::LocalSpawnExt;

    let mut pool = LocalPool::new();
    let spawner = pool.spawner();

    // retrieve paths from json
    let mut file = File::open("./paths/nodes.json").expect("Failed to open file");
    let mut contents = String::new();
    file.read_to_string(&mut contents).expect("Failed to read to string");
    let path_data: serde_json::Value = serde_json::from_str(&contents).expect("JSON was not well-formatted");

    let (path_lines, intersections) = get_path_lines(&path_data);

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

            let min_zoom = 1.0 / two.powf(max_lod as f32 + 1.0) as f32;
            let max_zoom = 20.0;

            // limit the zoom
            camera.zoom_multiplier = camera.zoom_multiplier.clamp(min_zoom, 20.);

            // zoom via scroll wheel
            let (_, mouse_scroll) = mouse_wheel();
            if mouse_scroll == 1.0 && camera.zoom_multiplier < max_zoom {
                // record mouse positions
                let mouse_screen_pos = mouse_position();
                let mouse_world_pos =
                    screen_pos_to_coord(mouse_screen_pos.0, mouse_screen_pos.1, &camera);

                // zoom in
                camera.zoom_multiplier += zoom_speed * 10.;

                // limit the zoom
                camera.zoom_multiplier = camera.zoom_multiplier.clamp(min_zoom, 20.);

                // center camera on where mouse was in world
                camera.x_offset = -mouse_world_pos.x;
                camera.y_offset = -mouse_world_pos.y;

                let screen_x_to_change = mouse_screen_pos.0 - screen_width() / 2.;
                let screen_y_to_change = mouse_screen_pos.1 - screen_height() / 2.;

                // move camera by screen_x_to_change
                camera.x_offset += screen_x_to_change / camera.zoom_multiplier;
                camera.y_offset += screen_y_to_change / camera.zoom_multiplier;
            } else if mouse_scroll == -1.0 && camera.zoom_multiplier > min_zoom {
                // record mouse positions
                let mouse_screen_pos = mouse_position();
                let mouse_world_pos =
                    screen_pos_to_coord(mouse_screen_pos.0, mouse_screen_pos.1, &camera);

                // zoom out
                camera.zoom_multiplier -= zoom_speed * 10.;

                // limit the zoom
                camera.zoom_multiplier = camera.zoom_multiplier.clamp(min_zoom, 20.);

                // center camera on where mouse was in world
                camera.x_offset = -mouse_world_pos.x;
                camera.y_offset = -mouse_world_pos.y;

                let screen_x_to_change = mouse_screen_pos.0 - screen_width() / 2.;
                let screen_y_to_change = mouse_screen_pos.1 - screen_height() / 2.;

                // move camera by screen_x_to_change
                camera.x_offset += screen_x_to_change / camera.zoom_multiplier;
                camera.y_offset += screen_y_to_change / camera.zoom_multiplier;
            }

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
            for level in 0..=max_lod {
                if camera.zoom_multiplier < 1. / two.powf(level as f32) {
                    lod = level as usize;
                } else {
                    break;
                }
            }
            // re-make immutable
            let lod = lod;
        //<> determine what sectors we need to render
            //get top left sector to render
            let top_left_sector = sector_at_screen_pos(0., 0., &camera, tile_dimensions, lod);

            //get bottom right sector to render
            let bottom_right_sector = sector_at_screen_pos(
                screen_width(),
                screen_height(),
                &camera,
                tile_dimensions,
                lod,
            );
        //<> cache uncached textures
            // for all sectors to render
            // for sector_y in top_left_sector.1..=bottom_right_sector.1 {
            //     for sector_x in top_left_sector.0..=bottom_right_sector.0 {

            //         // // futures::run(fut);

            //         // use futures::executor::LocalPool;

            //         // let mut pool = LocalPool::new();

            //         // pool.spawn_local()
            //         // pool.run();
            //     }
            // }
        //<> draw all textures

            let mut rendered_tiles = 0;

            // for all sectors to render
            for sector_y in top_left_sector.1..=bottom_right_sector.1 {
                for sector_x in top_left_sector.0..=bottom_right_sector.0 {
                    // determine texture
                    let texture_option = {
                        let arc_mutex_hdd_texture_cache2 = arc_mutex_hdd_texture_cache.clone();
                        let hdd_texture_cache = arc_mutex_hdd_texture_cache.lock().unwrap();
                        let mutex_retrieving_tile_map2 = mutex_retrieving_tile_map.clone();

                        let texture_option = match hdd_texture_cache.get(&(sector_x, sector_y, lod)) {
                            Some(texture_option) => *texture_option,
                            None => {
                                drop(hdd_texture_cache);

                                let mut retrieving_tile_map = mutex_retrieving_tile_map.lock().unwrap();
                                if retrieving_tile_map.get(&(sector_x, sector_y, lod)) == None {
                                    retrieving_tile_map.insert((sector_x, sector_y, lod), true);
                                    drop(retrieving_tile_map);

                                    let f = cache_texture(
                                        (sector_x, sector_y, lod),
                                        arc_mutex_hdd_texture_cache2,
                                        mutex_retrieving_tile_map2,
                                    );

                                    spawner.spawn_local(f).unwrap();
                                }

                                None
                            }
                        };
                        texture_option
                    };

                    // render texture
                    if let Some(texture) = texture_option {
                      
                        let tile_world_width = tile_dimensions.0 as f32 * two.powf(lod as f32);
                        let tile_world_height = tile_dimensions.1 as f32 * two.powf(lod as f32);

                        let tile_screen_width = tile_world_width * camera.zoom_multiplier;
                        let tile_screen_height = tile_world_height * camera.zoom_multiplier;


                        let tile_world_x = tile_world_width * sector_x as f32;
                        let tile_world_y = tile_world_height * sector_y as f32;

                        let tile_screen_point =
                            coord_to_screen_pos(tile_world_x, tile_world_y, &camera);

                        let params = DrawTextureParams {
                            dest_size: Some(vec2(tile_screen_width, tile_screen_height)),
                            source: None,
                            rotation: 0.,
                            flip_x: false,
                            flip_y: false,
                            pivot: None,
                        };

                        draw_texture_ex(texture, tile_screen_point.x, tile_screen_point.y, WHITE, params);
                        rendered_tiles += 1;
                    }
                }
            }

            pool.try_run_one();
            // pool.run_until_stalled();

        //<>  clean up any unrendered textures
            {
                let mut hdd_texture_cache = arc_mutex_hdd_texture_cache.lock().unwrap();

                // find tiles to remove
                let mut to_remove = Vec::new();
                for ((sec_x, sec_y, sec_lod), _) in &*hdd_texture_cache {
                    if !((lod == *sec_lod)
                        && (*sec_y >= top_left_sector.1 && *sec_y <= bottom_right_sector.1)
                        && (*sec_x >= top_left_sector.0 && *sec_x <= bottom_right_sector.0))
                    {
                        to_remove.push((*sec_x, *sec_y, *sec_lod));
                    }
                }

                // remove tiles
                for (sec_x, sec_y, sec_lod) in to_remove {
                    if let Some(texture) = hdd_texture_cache.remove(&(sec_x, sec_y, sec_lod)).unwrap() {
                        texture.delete();
                    }
                }
            }

        //<> draw tile lines
            // if true {
            //     // for all sectors to render
            //     for sector_y in top_left_sector.1..=bottom_right_sector.1 {
            //         let tile_screen_y = screen_height() / 2.
            //             + (camera.y_offset * camera.zoom_multiplier)
            //             + sector_y as f32
            //                 * tile_dimensions.1 as f32
            //                 * camera.zoom_multiplier
            //                 * two.powf(lod as f32);

            //         draw_line(0., tile_screen_y, screen_width(), tile_screen_y, 3.0, GRAY);
            //     }

            //     for sector_x in top_left_sector.0..=bottom_right_sector.0 {
            //         let tile_screen_x = screen_width() / 2.
            //             + (camera.x_offset * camera.zoom_multiplier)
            //             + sector_x as f32
            //                 * tile_dimensions.0 as f32
            //                 * camera.zoom_multiplier
            //                 * two.powf(lod as f32);

            //         draw_line(tile_screen_x, 0., tile_screen_x, screen_height(), 3.0, GRAY);
            //     }
            // }
        //<

        render_lines(&path_lines, &camera);
        render_intersections(&intersections, &camera);

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
            &("mouse.x: ".to_owned() + &mouse_coord.x.to_string()),
            20.0,
            100.0,
            30.0,
            WHITE,
        );

        draw_text(
            &("mouse.y: ".to_owned() + &mouse_coord.y.to_string()),
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
