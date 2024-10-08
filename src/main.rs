use futures::executor::LocalPool;
use futures::task::LocalSpawnExt;
use macroquad::prelude::*;
use std::collections::VecDeque;
use std::fs;
use std::path::Path;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::mpsc::{self, Sender},
};
use tempdir::TempDir;
use tileproc::args::GenTilesArgs;
use tileproc::tiler::{gen_tiles_to_dir, generate_lods};
mod options;
use clap::Parser;

const LOD_FUZZYNESS: f32 = 1.0;

fn world_pos_to_screen_pos(x: f32, y: f32, camera: &CameraSettings) -> (f32, f32) {
    let out_x = screen_width() / 2. + ((x - camera.x_offset) * camera.zoom_multiplier);
    let out_y = screen_height() / 2. + ((y - camera.y_offset) * camera.zoom_multiplier);
    (out_x, out_y)
}

fn screen_pos_to_world_pos(x: f32, y: f32, camera: &CameraSettings) -> (f32, f32) {
    let x_out = camera.x_offset + (x - screen_width() / 2.) / camera.zoom_multiplier;
    let y_out = camera.y_offset + (y - screen_height() / 2.) / camera.zoom_multiplier;
    (x_out, y_out)
}

fn sector_at_screen_pos(
    x: f32,
    y: f32,
    camera: &CameraSettings,
    tile_dimensions: (f32, f32),
    lod: usize,
) -> (i32, i32) {
    let two: f32 = 2.0;
    let screen_point_coords = screen_pos_to_world_pos(x, y, camera);

    // get sector x
    let tile_world_x_size = tile_dimensions.0 * two.powf(lod as f32);
    let screen_point_sector_x = if screen_point_coords.0 < 0.0 {
        (screen_point_coords.0 / tile_world_x_size) as i32 - 1
    } else {
        (screen_point_coords.0 / tile_world_x_size) as i32
    };

    // get sector y
    let tile_world_y_size = tile_dimensions.1 * two.powf(lod as f32);
    let screen_point_sector_y = if screen_point_coords.1 < 0.0 {
        (screen_point_coords.1 / tile_world_y_size) as i32 - 1
    } else {
        (screen_point_coords.1 / tile_world_y_size) as i32
    };

    (screen_point_sector_x, screen_point_sector_y)
}

/// stores texture in texture_cache. Does not check if it is already there.
async fn cache_texture(
    tile_dir: PathBuf,
    tile_data: (i32, i32, usize),
    results_tx: Sender<((i32, i32, usize), Option<Texture2D>)>,
) {
    let (sector_x, sector_y, lod) = tile_data;

    let texture_dir = tile_dir
        .to_path_buf()
        .join(lod.to_string())
        .join(sector_x.to_string() + "," + &sector_y.to_string() + ".png");

    let texture_option =
        match load_texture(&texture_dir.into_os_string().into_string().unwrap()).await {
            Ok(texture) => Some(texture),
            _ => None,
        };

    results_tx.send((tile_data, texture_option)).unwrap();
}

fn tile_on_screen(
    tile_data: (i32, i32, usize),
    camera: &CameraSettings,
    tile_dimensions: (f32, f32),
) -> bool {
    let (tile_x, tile_y, render_lod) = tile_data;
    // determine what sectors we need to render
    let (top_left_sector, bottom_right_sector) =
        get_screen_sectors(camera, tile_dimensions, render_lod);

    // if tile on screen
    tile_x >= top_left_sector.0
        && tile_y >= top_left_sector.1
        && tile_x <= bottom_right_sector.0
        && tile_y <= bottom_right_sector.1
}

fn lod_from_zoom(zoom_multiplier: f32, max_lod: usize) -> usize {
    let two: f32 = 2.0;
    let mut lod: usize = 0;
    for level in 0..=max_lod {
        if zoom_multiplier < LOD_FUZZYNESS / two.powf(level as f32) {
            lod = level;
        } else {
            break;
        }
    }
    lod
}

/// determine if current desired view is fully cached and ready to be rendered
fn current_view_cached(
    texture_cache: &HashMap<(i32, i32, usize), Option<Texture2D>>,
    render_lod: usize,
    camera: &CameraSettings,
    tile_dimensions: (f32, f32),
) -> bool {
    // determine what sectors we need to render
    let (top_left_sector, bottom_right_sector) =
        get_screen_sectors(camera, tile_dimensions, render_lod);

    let mut fully_rendered = true;
    for sector_y in top_left_sector.1..=bottom_right_sector.1 {
        for sector_x in top_left_sector.0..=bottom_right_sector.0 {
            // render texture
            if texture_cache
                .get(&(sector_x, sector_y, render_lod))
                .is_none()
            {
                fully_rendered = false;
                break;
            }
        }
        if !fully_rendered {
            break;
        }
    }

    fully_rendered
}

fn get_screen_sectors(
    camera: &CameraSettings,
    tile_dimensions: (f32, f32),
    lod: usize,
) -> ((i32, i32), (i32, i32)) {
    let top_left_sector = sector_at_screen_pos(0., 0., camera, tile_dimensions, lod);

    let bottom_right_sector = sector_at_screen_pos(
        screen_width(),
        screen_height(),
        camera,
        tile_dimensions,
        lod,
    );

    (top_left_sector, bottom_right_sector)
}

fn average(numbers: &VecDeque<f64>) -> f64 {
    numbers.iter().sum::<f64>() / numbers.len() as f64
}

fn new_rolling_average(new_value: f64, rolling_decode_buffer: &mut VecDeque<f64>) -> f64 {
    rolling_decode_buffer.push_back(new_value);

    if rolling_decode_buffer.len() > 100 {
        rolling_decode_buffer.pop_front();
    }

    average(rolling_decode_buffer)
}

/// draws a grid over the screen that outlines the size of tiles being currently rendered
fn _draw_tile_lines(camera: &CameraSettings, lod: usize, tile_dimensions: (f32, f32)) {
    let (top_left_sector, bottom_right_sector) = get_screen_sectors(camera, tile_dimensions, lod);
    let two: f32 = 2.0;

    // for all sectors to render
    for sector_y in top_left_sector.1..=bottom_right_sector.1 {
        let tile_screen_y = screen_height() / 2.
            + (-camera.y_offset * camera.zoom_multiplier)
            + sector_y as f32 * tile_dimensions.1 * camera.zoom_multiplier * two.powf(lod as f32);

        draw_line(0., tile_screen_y, screen_width(), tile_screen_y, 3.0, RED);
    }

    for sector_x in top_left_sector.0..=bottom_right_sector.0 {
        let tile_screen_x = screen_width() / 2.
            + (-camera.x_offset * camera.zoom_multiplier)
            + sector_x as f32 * tile_dimensions.0 * camera.zoom_multiplier * two.powf(lod as f32);

        draw_line(tile_screen_x, 0., tile_screen_x, screen_height(), 3.0, RED);
    }
}

fn median(numbers: &mut [i32]) -> i32 {
    numbers.sort();
    let mid = numbers.len() / 2;
    numbers[mid]
}

async fn infer_target_fps() -> i32 {
    let fps_test_start_time = get_time();
    let mut fps_records: Vec<i32> = Vec::new();
    while get_time() - fps_test_start_time < 0.5 {
        fps_records.push(get_fps());
        next_frame().await;
    }
    median(&mut fps_records)
}

struct CameraSettings {
    x_offset: f32,
    y_offset: f32,
    zoom_multiplier: f32,
}

// Channel types used to send results of retrieving tiles.
type TileSender = std::sync::mpsc::Sender<((i32, i32, usize), Option<Texture2D>)>;
type TileReceiver = std::sync::mpsc::Receiver<((i32, i32, usize), Option<Texture2D>)>;

struct TileViewer {
    texture_cache: HashMap<(i32, i32, usize), Option<Texture2D>>,
    retriving_pools: HashMap<(i32, i32, usize), LocalPool>,
    tile_dimensions: (f32, f32),
    max_lod: usize,
    results_tx: TileSender,
    results_rx: TileReceiver,
    rolling_decode_buffer: VecDeque<f64>,
    rolling_average_decode_time: f64,
    tile_dir: PathBuf,
}

impl TileViewer {
    async fn new(tile_dir: &Path) -> Self {
        let (results_tx, results_rx): (TileSender, TileReceiver) = mpsc::channel();
        TileViewer {
            texture_cache: HashMap::new(),
            retriving_pools: HashMap::new(),
            tile_dimensions: {
                // return dimentions of a tile in lod 0

                // get a tile from lod 0
                let mut paths = fs::read_dir(tile_dir.to_path_buf().join(0.to_string())).unwrap();
                let path = paths.next().unwrap().unwrap().path();
                let path_string = path.to_str().unwrap();
                let initial_texture: Texture2D = load_texture(path_string).await.unwrap();

                // get the dimensions of the tile before it is freed
                let tile_dimensions = (initial_texture.width(), initial_texture.height());
                initial_texture.delete();

                tile_dimensions
            },
            max_lod: max_lod_in_tile_dir(tile_dir),
            results_tx,
            results_rx,
            rolling_decode_buffer: VecDeque::new(),
            rolling_average_decode_time: 0.0,
            tile_dir: tile_dir.to_path_buf(),
        }
    }

    /// Queues tiles from the current LOD that should be rendered on screen, for being retrieved and stored in cache, if they are not already.
    fn queue_desired_textures(&mut self, camera: &CameraSettings) {
        let lod = lod_from_zoom(camera.zoom_multiplier, self.max_lod);
        let (top_left_sector, bottom_right_sector) =
            get_screen_sectors(camera, self.tile_dimensions, lod);

        // for all sectors to render
        for sector_y in top_left_sector.1..=bottom_right_sector.1 {
            for sector_x in top_left_sector.0..=bottom_right_sector.0 {
                // if tile not in cache
                if self.texture_cache.get(&(sector_x, sector_y, lod)).is_none() {
                    // if not actively retrieving
                    if self
                        .retriving_pools
                        .get(&(sector_x, sector_y, lod))
                        .is_none()
                    {
                        let f = cache_texture(
                            self.tile_dir.clone(),
                            (sector_x, sector_y, lod),
                            self.results_tx.clone(),
                        );

                        // create LocalPool with one task inside
                        let pool = LocalPool::new();
                        let spawner = pool.spawner();
                        spawner.spawn_local(f).unwrap();

                        self.retriving_pools.insert((sector_x, sector_y, lod), pool);
                    }
                }
            }
        }
    }

    /// Removes unused tiles from texture_cache
    ///
    /// Removes any tiles in cache that are not visible on screen.
    ///
    /// Removes all tiles not in the desired LOD, only when the tile cache contains a full screen of tiles from the desired LOD.
    fn clean_tile_texture_cache(&mut self, camera: &CameraSettings) {
        let lod = lod_from_zoom(camera.zoom_multiplier, self.max_lod);

        // determine what sectors we need to render
        let (top_left_sector, bottom_right_sector) =
            get_screen_sectors(camera, self.tile_dimensions, lod);

        // remove tiles out of view
        {
            let mut to_remove = Vec::new();
            for (tile_data, _) in self.texture_cache.iter() {
                if !tile_on_screen(*tile_data, camera, self.tile_dimensions) {
                    to_remove.push(*tile_data);
                }
            }

            // remove tiles
            for (sec_x, sec_y, sec_lod) in to_remove {
                if let Some(texture) = self.texture_cache.remove(&(sec_x, sec_y, sec_lod)).unwrap()
                {
                    texture.delete();
                }
            }
        }

        //determine if current desired view is fully rendered
        let fully_rendered =
            current_view_cached(&self.texture_cache, lod, camera, self.tile_dimensions);

        // possibly remove tiles in wrong lod
        {
            // clear texture cache only if fully rendering what we want to be
            if fully_rendered {
                // find tiles to remove
                let mut to_remove = Vec::new();
                for ((sec_x, sec_y, sec_lod), _) in self.texture_cache.iter() {
                    if !((lod == *sec_lod)
                        && (*sec_y >= top_left_sector.1 && *sec_y <= bottom_right_sector.1)
                        && (*sec_x >= top_left_sector.0 && *sec_x <= bottom_right_sector.0))
                    {
                        to_remove.push((*sec_x, *sec_y, *sec_lod));
                    }
                }

                // remove tiles
                for (sec_x, sec_y, sec_lod) in to_remove {
                    if let Some(texture) =
                        self.texture_cache.remove(&(sec_x, sec_y, sec_lod)).unwrap()
                    {
                        texture.delete();
                    }
                }
            }
        }
    }

    /// Renders image tiles and returns how many are currently being rendered
    ///
    /// Renders all image tiles in tile cache that are on screen. Including tiles with an LOD different from the current one.
    /// Larger LOD tiles are rendered first, so as to fill in holes left by smaller LOD tiles that have not been cached yet.
    fn render_screen_tiles(
        &self,
        camera: &CameraSettings,
        tile_boxes: bool,
        show_culling: bool,
    ) -> u32 {
        let mut num_rendered_tiles: u32 = 0;
        let two: f32 = 2.0;

        for render_lod in (0..=self.max_lod).rev() {
            // determine what sectors we need to render
            let (top_left_sector, bottom_right_sector) =
                get_screen_sectors(camera, self.tile_dimensions, render_lod);

            // for all cached tiles
            for ((tile_x, tile_y, tile_lod), texture_option) in &self.texture_cache {
                // if correct LOD
                if *tile_lod == render_lod {
                    let tile_on_screen = if !show_culling {
                        *tile_x >= top_left_sector.0
                            && *tile_y >= top_left_sector.1
                            && *tile_x <= bottom_right_sector.0
                            && *tile_y <= bottom_right_sector.1
                    } else {
                        // Define a helper closure for culling logic
                        let is_within = |coord: i32, start: i32, end: i32| {
                            if end - start > 1 {
                                // Multiple tiles on this axis; apply culling
                                coord > start && coord < end
                            } else {
                                // Only one or two tiles on this axis; allow both
                                coord == start || coord == end
                            }
                        };

                        // Check both axes
                        is_within(*tile_x, top_left_sector.0, bottom_right_sector.0)
                            && is_within(*tile_y, top_left_sector.1, bottom_right_sector.1)
                    };

                    // if tile on screen
                    if tile_on_screen {
                        // if there's a texture to be rendered
                        if let Some(texture) = texture_option {
                            let tile_world_width =
                                self.tile_dimensions.0 * two.powf(render_lod as f32);
                            let tile_world_height =
                                self.tile_dimensions.1 * two.powf(render_lod as f32);

                            let tile_screen_width = tile_world_width * camera.zoom_multiplier;
                            let tile_screen_height = tile_world_height * camera.zoom_multiplier;

                            let tile_world_x = tile_world_width * *tile_x as f32;
                            let tile_world_y = tile_world_height * *tile_y as f32;

                            let (tile_screen_x, tile_screen_y) =
                                world_pos_to_screen_pos(tile_world_x, tile_world_y, camera);

                            let params = DrawTextureParams {
                                dest_size: Some(vec2(tile_screen_width, tile_screen_height)),
                                source: None,
                                rotation: 0.,
                                flip_x: false,
                                flip_y: false,
                                pivot: None,
                            };

                            if camera.zoom_multiplier >= 1.0 {
                                texture.set_filter(FilterMode::Nearest);
                            } else {
                                texture.set_filter(FilterMode::Linear);
                            }

                            draw_texture_ex(*texture, tile_screen_x, tile_screen_y, WHITE, params);

                            if tile_boxes {
                                // draw red box around newly rendered tile

                                // top
                                draw_line(
                                    tile_screen_x,
                                    tile_screen_y,
                                    tile_screen_x + tile_screen_width,
                                    tile_screen_y,
                                    3.0,
                                    RED,
                                );
                                // bottom
                                draw_line(
                                    tile_screen_x,
                                    tile_screen_y + tile_screen_height,
                                    tile_screen_x + tile_screen_width,
                                    tile_screen_y + tile_screen_height,
                                    3.0,
                                    RED,
                                );
                                // left
                                draw_line(
                                    tile_screen_x,
                                    tile_screen_y,
                                    tile_screen_x,
                                    tile_screen_y + tile_screen_height,
                                    3.0,
                                    RED,
                                );
                                // right
                                draw_line(
                                    tile_screen_x + tile_screen_width,
                                    tile_screen_y,
                                    tile_screen_x + tile_screen_width,
                                    tile_screen_y + tile_screen_height,
                                    3.0,
                                    RED,
                                );
                            }

                            num_rendered_tiles += 1;
                        }
                    }
                }
            }
        }
        num_rendered_tiles
    }

    /// Retrieves tiles requested by queue_desired_textures() and stores them in texture_cache
    ///
    /// Always retrieves at least one tile, assuming at least one needs to be retrieved.
    ///
    /// Retrieves more tiles if there is time to do so before the next frame needs to be rendered.
    fn retrieve_tiles_till_out_of_work_or_time(
        &mut self,
        camera: &CameraSettings,
        frame_start_time: f64,
        frame_time_limit: f64,
    ) {
        let lod = lod_from_zoom(camera.zoom_multiplier, self.max_lod);

        // stop retrieving any tiles that are not current desired lod
        self.retriving_pools
            .retain(|(_, _, tile_lod), _| *tile_lod == lod);

        // possibly prepair one tile
        let mut finished_tiles = Vec::new();
        let mut textures_decoded = 0;
        for ((tile_x, tile_y, tile_lod), pool) in &mut self.retriving_pools {
            // stop if out of time. But only if have decoded at least one texture
            if textures_decoded != 0 {
                let time_since_last_frame = get_time() - frame_start_time;
                if time_since_last_frame + self.rolling_average_decode_time > frame_time_limit * 0.7
                {
                    break;
                }
            }

            if *tile_lod == lod {
                let tile_decode_start_time = get_time();
                if pool.try_run_one() {
                    // don't break unless texture was sent back
                    if let Ok((details, texture_option)) = self.results_rx.try_recv() {
                        // mark for removal from self.retriving_pools
                        finished_tiles.push((*tile_x, *tile_y, *tile_lod));

                        // store in
                        self.texture_cache.insert(details, texture_option);

                        if texture_option.is_some() {
                            textures_decoded += 1;

                            let last_time_to_decode = get_time() - tile_decode_start_time;

                            self.rolling_average_decode_time = new_rolling_average(
                                last_time_to_decode,
                                &mut self.rolling_decode_buffer,
                            );
                        }
                    }
                }
            }
        }

        // remove any finished tiles
        for (tile_x, tile_y, tile_lod) in finished_tiles {
            self.retriving_pools.remove(&(tile_x, tile_y, tile_lod));
        }
    }
}

// finds max_lod in a directory containing tile lods
fn max_lod_in_tile_dir(dir: &Path) -> usize {
    let mut max_lod: usize = 0;
    for x in 0.. {
        if dir.to_path_buf().join(&x.to_string()).is_dir() {
            max_lod = x;
        } else {
            break;
        }
    }
    max_lod
}

#[macroquad::main("TileView")]
async fn main() {
    let args = options::Args::parse();
    let tile_dir = args.image_path;

    let (mut tile_viewer, max_lod) = if tile_dir.is_dir() {
        (
            TileViewer::new(&tile_dir).await,
            max_lod_in_tile_dir(&tile_dir),
        )
    } else {
        let tmp_dir = TempDir::new("tile-viewer").unwrap().path().to_path_buf();
        fs::create_dir(&tmp_dir).unwrap();

        let mut output_dir = tmp_dir.clone();
        output_dir.push("0/");

        gen_tiles_to_dir(&GenTilesArgs {
            input: tile_dir.clone(),
            output: output_dir,
            tile_dimensions: 256,
            x_offset: None,
            y_offset: None,
        });

        generate_lods(&tmp_dir);

        (
            TileViewer::new(&tmp_dir).await,
            max_lod_in_tile_dir(&tmp_dir),
        )
    };

    let two: f32 = 2.0;
    let default_zoom = 1.0 / two.powf(max_lod as f32 - 1.0);

    let mut camera = CameraSettings {
        x_offset: 0.,
        y_offset: 0.,
        zoom_multiplier: default_zoom,
    };

    let mut mouse_clicked_in_position: Option<(f32, f32)> = None;
    let mut clicked_in_x_offset: f32 = 0.0;
    let mut clicked_in_y_offset: f32 = 0.0;

    let target_fps = infer_target_fps().await;
    let frame_time_limit = 1. / target_fps as f64;

    loop {
        let frame_start_time = get_time();

        // react to key presses
        {
            let fps_speed_multiplier = 144. / target_fps as f32;
            let speed = if is_key_down(KeyCode::LeftShift) {
                20. / camera.zoom_multiplier * fps_speed_multiplier
            } else {
                5. / camera.zoom_multiplier * fps_speed_multiplier
            };

            if is_key_down(KeyCode::Right) || is_key_down(KeyCode::D) {
                camera.x_offset += speed;
            }
            if is_key_down(KeyCode::Left) || is_key_down(KeyCode::A) {
                camera.x_offset -= speed;
            }
            if is_key_down(KeyCode::Up) || is_key_down(KeyCode::W) {
                camera.y_offset -= speed;
            }
            if is_key_down(KeyCode::Down) || is_key_down(KeyCode::S) {
                camera.y_offset += speed;
            }

            let zoom_speed = if is_key_down(KeyCode::LeftShift) {
                camera.zoom_multiplier / 100. * 4. * fps_speed_multiplier
            } else {
                camera.zoom_multiplier / 100. * fps_speed_multiplier
            };

            // zoom via buttons
            if is_key_down(KeyCode::E) {
                camera.zoom_multiplier += zoom_speed;
            }
            if is_key_down(KeyCode::Q) {
                camera.zoom_multiplier -= zoom_speed;
            }

            let min_zoom = LOD_FUZZYNESS / two.powf(max_lod as f32 + 1.0);
            let max_zoom = 20.0;

            // limit the zoom
            camera.zoom_multiplier = camera.zoom_multiplier.clamp(min_zoom, max_zoom);

            // zoom via scroll wheel
            let (_, mouse_scroll) = mouse_wheel();
            if mouse_scroll == 1.0 && camera.zoom_multiplier < max_zoom {
                // record mouse positions
                let mouse_screen_pos = mouse_position();
                let mouse_world_pos =
                    screen_pos_to_world_pos(mouse_screen_pos.0, mouse_screen_pos.1, &camera);

                // zoom in
                camera.zoom_multiplier += zoom_speed * 10.;

                // limit the zoom
                camera.zoom_multiplier = camera.zoom_multiplier.clamp(min_zoom, max_zoom);

                // center camera on where mouse was in world
                camera.x_offset = mouse_world_pos.0;
                camera.y_offset = mouse_world_pos.1;

                let screen_x_to_change = mouse_screen_pos.0 - screen_width() / 2.;
                let screen_y_to_change = mouse_screen_pos.1 - screen_height() / 2.;

                // move camera by screen_x_to_change
                camera.x_offset -= screen_x_to_change / camera.zoom_multiplier;
                camera.y_offset -= screen_y_to_change / camera.zoom_multiplier;
            } else if mouse_scroll == -1.0 && camera.zoom_multiplier > min_zoom {
                // record mouse positions
                let mouse_screen_pos = mouse_position();
                let mouse_world_pos =
                    screen_pos_to_world_pos(mouse_screen_pos.0, mouse_screen_pos.1, &camera);

                // zoom out
                camera.zoom_multiplier -= zoom_speed * 10.;

                // limit the zoom
                camera.zoom_multiplier = camera.zoom_multiplier.clamp(min_zoom, max_zoom);

                // center camera on where mouse was in world
                camera.x_offset = mouse_world_pos.0;
                camera.y_offset = mouse_world_pos.1;

                let screen_x_to_change = mouse_screen_pos.0 - screen_width() / 2.;
                let screen_y_to_change = mouse_screen_pos.1 - screen_height() / 2.;

                // move camera by screen_x_to_change
                camera.x_offset -= screen_x_to_change / camera.zoom_multiplier;
                camera.y_offset -= screen_y_to_change / camera.zoom_multiplier;
            }

            // mouse drag screen
            if is_mouse_button_down(MouseButton::Left) {
                match mouse_clicked_in_position {
                    None => {
                        mouse_clicked_in_position = Some(mouse_position());
                        clicked_in_x_offset = -camera.x_offset;
                        clicked_in_y_offset = -camera.y_offset;
                    }
                    Some(x) => {
                        let cur_mouse_pos = mouse_position();

                        // calc new x_offset
                        let mouse_x_diff = cur_mouse_pos.0 - x.0;
                        camera.x_offset =
                            -(clicked_in_x_offset + mouse_x_diff / camera.zoom_multiplier);

                        // calc new y_offset
                        let mouse_y_diff = cur_mouse_pos.1 - x.1;
                        camera.y_offset =
                            -(clicked_in_y_offset + mouse_y_diff / camera.zoom_multiplier);
                    }
                };
            } else {
                mouse_clicked_in_position = None;
            }
        }
        // render tile_viewer
        let num_rendered_tiles = {
            clear_background(GRAY);

            // tile_viewer.recieve_retrieved_tiles();
            tile_viewer.clean_tile_texture_cache(&camera);
            tile_viewer.queue_desired_textures(&camera);
            let num_rendered_tiles =
                tile_viewer.render_screen_tiles(&camera, args.tiles, args.show_culling);
            tile_viewer.retrieve_tiles_till_out_of_work_or_time(
                &camera,
                frame_start_time,
                frame_time_limit,
            );
            num_rendered_tiles
        };
        // draw text in top left corner
        if args.stats {
            let lod = lod_from_zoom(camera.zoom_multiplier, max_lod);
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
                &("rendered_tiles: ".to_owned() + &num_rendered_tiles.to_string()),
                20.0,
                80.0,
                30.0,
                WHITE,
            );

            let mouse = mouse_position();
            let mouse_coord = screen_pos_to_world_pos(mouse.0, mouse.1, &camera);
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
        }

        next_frame().await
    }
}
