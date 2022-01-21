use std::fs::File;

use anyhow::Result;
use macroquad::prelude::*;

use macroquad_tiled as tiled;

use macroquad_platformer::*;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};

use crate::{
    fighter::{FighterData, HitBox, HitData},
    fighter_ai::Setting,
    fighter_wrapper::FighterWrapper,
};

mod fighter;
mod fighter_ai;
mod fighter_wrapper;

fn window_conf() -> Conf {
    Conf {
        window_title: "Window Conf".to_owned(),
        fullscreen: false,
        window_width: 1000,
        window_height: 500,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    env_logger::init();

    let path = format!("{}/assets/", env!("CARGO_MANIFEST_DIR"));
    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher = RecommendedWatcher::new(tx).unwrap();
    watcher
        .watch(path.as_ref(), RecursiveMode::Recursive)
        .unwrap();

    let mut pause = false;

    let tileset = load_texture("examples/tileset.png").await.unwrap();
    tileset.set_filter(FilterMode::Nearest);

    let tiled_map_json = load_string("examples/map.json").await.unwrap();
    let tiled_map = tiled::load_map(&tiled_map_json, &[("tileset.png", tileset)], &[]).unwrap();

    let mut static_colliders = vec![];
    for (_x, _y, tile) in tiled_map.tiles("main layer", None) {
        static_colliders.push(if tile.is_some() {
            Tile::Solid
        } else {
            Tile::Empty
        });
    }

    let mut world = World::new();
    world.add_static_tiled_layer(static_colliders, 100., 100., 20, 1);

    let data_path = format!("{}/assets/fighter.ron", env!("CARGO_MANIFEST_DIR"));
    let ai_path = format!("{}/assets/ai.ron", env!("CARGO_MANIFEST_DIR"));
    let mut fighters = vec![
        FighterWrapper::new(&mut world, 500.0, 300.0, data_path.clone(), None),
        FighterWrapper::new(&mut world, 1500.0, 300.0, data_path.clone(), Some(ai_path)),
    ];
    for fighter in &mut fighters {
        fighter.load_resource().await.unwrap();
    }

    let camera = Camera2D::from_display_rect(Rect::new(0.0, 0.0, 2000.0, 1000.0));

    let mut overlappings: Vec<OverlapEvent> = Vec::new();
    loop {
        clear_background(DARKGRAY);

        set_camera(&camera);

        tiled_map.draw_tiles("main layer", Rect::new(0.0, 0.0, 2000.0, 1000.0), None);

        // Draw FPS
        {
            // debug!("FPS: {}", get_fps());
            draw_text(&format!("FPS: {}", get_fps()), 100.0, 100.0, 64.0, GREEN);
        }

        // Hot Reload Data
        {
            for e in rx.try_iter() {
                let e = e.unwrap();
                match e.kind {
                    notify::EventKind::Modify(mk) => {
                        debug!("Modify: {:?}, Path: {:?}", mk, e.paths);
                        //  Reload everything
                        {
                            let input_path =
                                format!("{}/assets/fighter.ron", env!("CARGO_MANIFEST_DIR"));
                            debug!("Reloading: {}", input_path);
                            let f = File::open(&input_path).expect("Failed opening file");
                            let data: Result<FighterData, ron::Error> = ron::de::from_reader(f);
                            match data {
                                Ok(data) => {
                                    for fighter in &mut fighters {
                                        fighter.fighter.data = data.clone();
                                    }
                                }
                                Err(error) => {
                                    error!("Cannot reload data! {:?}", error);
                                }
                            }
                        }
                        {
                            let input_path =
                                format!("{}/assets/ai.ron", env!("CARGO_MANIFEST_DIR"));
                            debug!("Reloading: {}", input_path);
                            let f = File::open(&input_path).expect("Failed opening file");
                            let data: Result<Vec<Setting>, ron::Error> = ron::de::from_reader(f);
                            match data {
                                Ok(data) => {
                                    if let Some(ai) = &mut fighters[1].ai {
                                        ai.settings = data;
                                    }
                                }
                                Err(error) => {
                                    error!("Cannot reload data! {:?}", error);
                                }
                            }
                        }
                        {
                            for fighter in &mut fighters {
                                fighter.load_resource().await.unwrap();
                            }
                        }
                    }
                    _ => (),
                }
            }
        }

        // Draw fighter
        {
            for fighter in &mut fighters {
                let t = fighter.fighter.time;
                let s = fighter
                    .fighter
                    .data
                    .states
                    .get(&fighter.fighter.state)
                    .unwrap();
                let len = s.animation.len();
                if len != 0 {
                    let d = s.duration;
                    let i = (t / d) * s.animation.len() as f32;
                    let u = i as usize;
                    let u = u % s.animation.len();
                    let key = &s.animation[u];

                    let flip = fighter.fighter.facing == -1.0;

                    let texture = fighter.resources.get(key).unwrap();
                    let w = texture.width();
                    let h = texture.height();
                    let (x, y) = fighter.position(&world);
                    let x = x - w / 2.0;
                    let y = -(y + h);

                    draw_texture_ex(
                        *texture,
                        x,
                        y,
                        WHITE,
                        DrawTextureParams {
                            flip_x: flip,
                            ..Default::default()
                        },
                    );
                }
            }
        }

        // Draw state
        {
            for fighter in &mut fighters {
                let pos = world.actor_pos(fighter.collider);
                let state = &fighter.fighter.state;
                draw_text(state, pos.x, pos.y, 64.0, RED);
            }
        }

        // Draw Box
        {
            for fighter in &mut fighters {
                for (_, b) in fighter.fighter.collision_boxes().unwrap() {
                    let (x, y, w, h) = fighter.world_xywh_by_rect(&world, b.into());
                    draw_rectangle_lines(x, y, w, h, 1.0, YELLOW);
                    draw_rectangle(x, y, w, h, Color::new(0.99, 0.98, 0.00, 0.10));
                }
                for (_, b) in fighter.fighter.hurt_boxes().unwrap() {
                    let (x, y, w, h) = fighter.world_xywh_by_rect(&world, b.into());
                    draw_rectangle_lines(x, y, w, h, 1.0, GREEN);
                    draw_rectangle(x, y, w, h, Color::new(0.00, 0.89, 0.19, 0.10));
                }
                for (_, b) in fighter.fighter.hit_boxes().unwrap() {
                    let (x, y, w, h) = fighter.world_xywh_by_rect(&world, b.into());
                    draw_rectangle_lines(x, y, w, h, 1.0, RED);
                    draw_rectangle(x, y, w, h, Color::new(0.90, 0.16, 0.22, 0.10));
                }
            }
        }
        // Draw AI
        {
            let fighter = &fighters[1];
            let ai = fighter.ai.as_ref().unwrap();
            for setting in &ai.settings {
                let (x, y, w, h) = fighter.world_xywh_by_x1x2y1y2(
                    &world,
                    setting.distance_x.0,
                    setting.distance_x.1,
                    setting.distance_y.0,
                    setting.distance_y.1,
                );
                draw_rectangle_lines(x, y, w, h, 1.0, ORANGE);
                draw_rectangle(x, y, w, h, Color::new(1.00, 0.63, 0.00, 0.10));
            }
        }
        // player movement control
        // {
        //     let pos = world.actor_pos(player.collider);
        //     let on_ground = world.collide_check(player.collider, pos + vec2(0., 1.));

        //     if on_ground == false {
        //         player.speed.y += 500. * get_frame_time();
        //     }

        //     if is_key_pressed(KeyCode::Space) {
        //         if on_ground {
        //             player.speed.y = -120.;
        //         }
        //     }

        //     world.move_h(player.collider, player.speed.x * get_frame_time());
        //     world.move_v(player.collider, player.speed.y * get_frame_time());
        // }

        // platform movement
        // {
        //     world.solid_move(platform.collider, platform.speed * get_frame_time(), 0.0);
        //     let pos = world.solid_pos(platform.collider);
        // }

        if is_key_pressed(KeyCode::Escape) {
            pause = !pause;
        }
        if pause {
            draw_text("Pause", 0.0, 0.0, 64.0, RED);
        }
        // update fighter
        if (pause && is_key_down(KeyCode::Space))
            || (!pause && !is_key_down(KeyCode::Space))
            || is_key_pressed(KeyCode::Enter)
        {
            {
                // auto facing
                let player_pos = world.actor_pos(fighters[0].collider);
                let ai_pos = world.actor_pos(fighters[1].collider);
                {
                    // Ai facing
                    let ai = &mut fighters[1];
                    if let Some(s) = ai.fighter.data.states.get(&ai.fighter.state) {
                        if s.auto_facing {
                            ai.fighter.facing = if ai_pos.x < player_pos.x { 1.0 } else { -1.0 };
                        }
                    }
                }
                {
                    // Player facing
                    let player = &mut fighters[0];
                    if let Some(s) = player.fighter.data.states.get(&player.fighter.state) {
                        if s.auto_facing {
                            player.fighter.facing =
                                if ai_pos.x < player_pos.x { -1.0 } else { 1.0 };
                        }
                    }
                }
            }
            {
                // Knockback on wall
                let player = &fighters[0];
                let collider = player.collider;
                let pos = world.actor_pos(collider);
                let right = world.collide_check(collider, pos + vec2(1.0, 0.0));
                let left = world.collide_check(collider, pos + vec2(-1.0, 0.0));
                if (right && player.fighter.velocity_x > 0.0)
                    || (left && player.fighter.velocity_x < 0.0)
                {
                    let v = player.fighter.velocity_x;
                    let ai = &mut fighters[1];
                    ai.fighter.velocity_x = v * -1.0;
                    let player = &mut fighters[0];
                    player.fighter.velocity_x = 0.0;
                }
                let ai = &fighters[1];
                let collider = ai.collider;
                let pos = world.actor_pos(collider);
                let right = world.collide_check(collider, pos + vec2(1.0, 0.0));
                let left = world.collide_check(collider, pos + vec2(-1.0, 0.0));
                if (right && ai.fighter.velocity_x > 0.0) || (left && ai.fighter.velocity_x < 0.0) {
                    let v = ai.fighter.velocity_x;
                    let player = &mut fighters[0];
                    player.fighter.velocity_x = v * -1.0;
                    let ai = &mut fighters[1];
                    ai.fighter.velocity_x = 0.0;
                }
            }
            {
                // Ai
                let f2 = &fighters[0];
                let pos2 = world.actor_pos(f2.collider);
                let f = &mut fighters[1];
                let ai = f.ai.as_ref().unwrap();
                let pos1 = world.actor_pos(f.collider);
                let d = (pos1 - pos2).abs();
                let input = ai.input(d.x, d.y);
                f.fighter.input_state.left = if f.fighter.facing == 1.0 {
                    input.backward
                } else {
                    input.forward
                };
                f.fighter.input_state.right = if f.fighter.facing == 1.0 {
                    input.forward
                } else {
                    input.backward
                };
                f.fighter.input_state.up = input.up;
                f.fighter.input_state.down = input.down;
                f.fighter.input_state.throw = input.throw;
                f.fighter.input_state.light = input.light;
                f.fighter.input_state.heavy = input.heavy;
            }
            {
                // Player Input
                let f = &mut fighters[0];
                f.fighter.input_state.right = is_key_down(KeyCode::Right);
                f.fighter.input_state.left = is_key_down(KeyCode::Left);
                f.fighter.input_state.up = is_key_down(KeyCode::Up);
                f.fighter.input_state.down = is_key_down(KeyCode::Down);
                f.fighter.input_state.light = is_key_down(KeyCode::Z);
                f.fighter.input_state.heavy = is_key_down(KeyCode::X);
                f.fighter.input_state.throw = is_key_down(KeyCode::C);
            }
            for fighter in &mut fighters {
                fighter.update(&mut world).unwrap();
            }
            let mut on_hit: Vec<HitEvent> = Vec::new();
            let mut prev_overlappings = Vec::new();
            prev_overlappings.append(&mut overlappings);
            // check hit box!
            for (i, fighter) in fighters.iter().enumerate() {
                let hit_boxes: Vec<HitBox> = fighter
                    .fighter
                    .hit_boxes()
                    .unwrap()
                    .values()
                    .filter(|b| !b.disable)
                    .cloned()
                    .collect();
                let hit_data = &fighter.fighter.data.states[&fighter.fighter.state].hit_data;
                let hit_box_xys: Vec<(f32, f32, f32, f32)> = hit_boxes
                    .iter()
                    .map(|hit| fighter.x1x2y1y2_by_rect(&world, hit.into()))
                    .collect();
                let mut cs: Vec<OverlapEvent> = fighters
                    .iter()
                    .enumerate()
                    .filter(|(u, _)| i != *u)
                    .filter(|(_, victim)| {
                        victim
                            .fighter
                            .hurt_boxes()
                            .unwrap()
                            .values()
                            .filter(|b| !b.disable)
                            .any(|hurt| {
                                let (ax1, ax2, ay1, ay2) =
                                    victim.x1x2y1y2_by_rect(&world, hurt.into());
                                hit_box_xys.iter().any(|(bx1, bx2, by1, by2)| {
                                    ax1 < *bx2 && ax2 > *bx1 && ay1 < *by2 && ay2 > *by1
                                })
                            })
                    })
                    .map(|(u, _victim)| OverlapEvent { a: i, b: u })
                    .collect();
                let mut hits = cs
                    .iter()
                    .filter(|e| prev_overlappings.iter().all(|p| e != &p))
                    .map(|c| HitEvent {
                        fighter: c.b,
                        data: hit_data.clone(),
                        facing: fighter.fighter.facing,
                    })
                    .collect();
                on_hit.append(&mut hits);
                overlappings.append(&mut cs);
                // Check throw box
                // ...
            }
            // handle on hit
            for e in on_hit {
                let fighter = &mut fighters[e.fighter];
                fighter.fighter.on_hit(e.data, e.facing).unwrap();
            }
            // handle throw
            // ...
        }

        next_frame().await
    }
}

pub struct HitEvent {
    pub fighter: usize,
    pub data: HitData,
    pub facing: f32,
}

#[derive(Eq, PartialEq)]
pub struct OverlapEvent {
    pub a: usize,
    pub b: usize,
}
