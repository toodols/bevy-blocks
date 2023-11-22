use std::{cmp, fmt};
mod board;
use bevy::{ecs::system::EntityCommands, prelude::*, window::PrimaryWindow};
use board::{Board as BoardGrid, Shape, TileColor, BOARD_HEIGHT, BOARD_WIDTH};
use rand::Rng;

use crate::board::SuperimpositionState;

#[derive(Component)]
struct Board {
    grid: BoardGrid,
    entities: [[Entity; BOARD_WIDTH]; BOARD_HEIGHT],
    extents: Rect,
}
impl Board {
    fn global_extents(&self, transform: &GlobalTransform) -> Rect {
        let min =
            transform.compute_matrix() * Vec4::new(self.extents.min.x, self.extents.min.y, 0., 1.);
        let max =
            transform.compute_matrix() * Vec4::new(self.extents.max.x, self.extents.max.y, 0., 1.);
        Rect {
            min: min.xy(),
            max: max.xy(),
        }
    }
}

const TILE_SIZE: f32 = 30.;

fn startup(mut commands: Commands) {
    commands.spawn((Camera2dBundle::default(), MainCamera));
    // let map_size = TilemapSize {
    //     x: BOARD_WIDTH as u32,
    //     y: BOARD_HEIGHT as u32,
    // };
    // let mut tile_storage = TileStorage::empty(map_size);
    // let map_type = TilemapType::Square;
    // let tilemap_entity = commands.spawn_empty().id();
    // helpers::filling::fill_tilemap(
    //     TileTextureIndex(0),
    //     map_size,
    //     TilemapId(tilemap_entity),
    //     commands,
    //     &mut tile_storage,
    // )
    fn board<'w, 's, 'a>(
        commands: &'a mut Commands<'w, 's>,
        is_main_board: bool,
    ) -> EntityCommands<'w, 's, 'a> {
        let mut board_entity = commands.spawn(SpatialBundle {
            transform: if is_main_board {
                Transform::from_scale(Vec3::splat(TILE_SIZE))
            } else {
                Transform::default()
            },
            ..default()
        });
        let mut rows: Vec<[Entity; BOARD_HEIGHT]> = Vec::with_capacity(BOARD_HEIGHT);
        board_entity.with_children(|commands| {
            for y in 0..BOARD_HEIGHT {
                let mut row: Vec<Entity> = Vec::with_capacity(BOARD_WIDTH);
                for x in 0..BOARD_WIDTH {
                    let cmds = commands.spawn(SpriteBundle {
                        sprite: Sprite {
                            color: if is_main_board {
                                TileColor::Gray.into()
                            } else {
                                TileColor::Transparent.into()
                            },
                            custom_size: Some(Vec2::new(0.99, 0.99)),
                            ..default()
                        },
                        transform: Transform {
                            translation: (Vec3::new(
                                (x as f32 - (BOARD_WIDTH as f32) * 0.5) + 0.5,
                                (y as f32 - (BOARD_HEIGHT as f32) * 0.5) + 0.5,
                                0.,
                            )),
                            ..default()
                        },
                        ..default()
                    });
                    row.push(cmds.id());
                }
                rows.push(row.try_into().unwrap());
            }
        });
        board_entity.insert(Board {
            grid: BoardGrid::default(),
            entities: rows.try_into().unwrap(),
            extents: Rect {
                min: Vec2::new(-0.5 * BOARD_WIDTH as f32, -0.5 * BOARD_HEIGHT as f32),
                max: Vec2::new(0.5 * BOARD_WIDTH as f32, 0.5 * BOARD_HEIGHT as f32),
            },
        });
        board_entity
    }

    let main_board = board(&mut commands, true).insert(MainBoard).id();
    let overlay_board = board(&mut commands, false).insert(OverlayBoard).id();
    commands.add(AddChild {
        parent: main_board,
        child: overlay_board,
    });

    let mut default_shape = Shape::from_pattern(2, 2, "####");
    default_shape.color = TileColor::Blue;
    let mut selected = build_shape(&mut commands, &default_shape);
    selected.insert(SelectedShape);
}

fn build_shape<'w, 's, 'a>(
    commands: &'a mut Commands<'w, 's>,
    shape: &Shape,
) -> EntityCommands<'w, 's, 'a> {
    let mut shape_entity = commands.spawn((
        *shape,
        SpatialBundle {
            transform: Transform::from_scale(Vec3::splat(TILE_SIZE)),
            ..default()
        },
    ));
    shape_entity.with_children(|commands| {
        for y in 0..shape.bounds().1 {
            for x in 0..shape.bounds().0 {
                if shape.fields[y][x] {
                    commands.spawn(SpriteBundle {
                        sprite: Sprite {
                            color: shape.color.into(),
                            custom_size: Some(Vec2::new(0.99, 0.99)),
                            ..default()
                        },
                        transform: Transform {
                            translation: (Vec3::new(
                                (x as f32 - (shape.bounds().0 as f32) * 0.5) + 0.5,
                                (y as f32 - (shape.bounds().1 as f32) * 0.5) + 0.5,
                                0.,
                            )),
                            ..default()
                        },
                        ..default()
                    });
                }
            }
        }
    });
    shape_entity
}

fn update(
    mut commands: Commands,
    q_windows: Query<&Window, With<PrimaryWindow>>,
    q_camera: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut q_board: Query<(&mut Board, &GlobalTransform), (With<MainBoard>, Without<OverlayBoard>)>,
    q_overlay_board: Query<&Board, With<OverlayBoard>>,
    input_mb: Res<Input<MouseButton>>,
    mut q_board_tiles: Query<&mut Sprite>,
    mut q_selected_shape: Query<(&Shape, &mut Transform, Entity), With<SelectedShape>>,
    shape_pool: Res<ShapePool>,
) {
    // Resolve queries
    let (mut board, board_transform) = q_board.single_mut();
    let window = q_windows.single();
    let (camera, camera_transform) = q_camera.single();

    // Clear overlay board
    let overlay_board = q_overlay_board.single();
    for x in overlay_board.entities.iter() {
        for y in x.iter() {
            if let Ok(mut sprite) = q_board_tiles.get_mut(*y) {
                sprite.color = TileColor::Transparent.into();
            }
        }
    }

    // Convert cursor to world
    if let Some(world_position) = window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor))
        .map(|ray| ray.origin.truncate())
    {
        if let Ok((selected_shape, mut selected_shape_transform, selected_shape_entity)) =
            q_selected_shape.get_single_mut()
        {
            let extents = board.global_extents(board_transform);
            let extents_size = extents.size();
            let position_on_board = world_position - extents.min;
            let translation = (
                (position_on_board.x / extents_size.x),
                (position_on_board.y / extents_size.y),
            );
            // Move the selected shape to cursor position
            selected_shape_transform.translation = world_position.extend(0.);

            let superimposition = board.grid.superimpose(selected_shape, translation);

            // Update board if superimposition succeeds
            if input_mb.just_pressed(MouseButton::Left) && superimposition.success {
                for (y, row) in superimposition.fields.0.iter().enumerate() {
                    for (x, state) in row.iter().enumerate() {
                        if *state == SuperimpositionState::Fits {
                            board.grid.0[y][x] = Some(selected_shape.color);
                        }
                    }
                }
                let mut rng = rand::thread_rng();
                let mut new_shape = shape_pool.0[rng.gen_range(0..shape_pool.0.len())];
                new_shape.color = rng.gen();
                commands.entity(selected_shape_entity).despawn_recursive();
                build_shape(&mut commands, &new_shape)
                    .insert(SelectedShape)
                    .insert(Transform {
                        translation: world_position.extend(0.),
                        scale: Vec3::splat(TILE_SIZE),
                        ..default()
                    });
            }

            // Update overlay board to reflect shape over cursor
            for (y, row) in superimposition.fields.0.iter().enumerate() {
                for (x, state) in row.iter().enumerate() {
                    if let Some(entity) = overlay_board.entities.get(y).and_then(|row| row.get(x)) {
                        if let Ok(mut sprite) = q_board_tiles.get_mut(*entity) {
                            match state {
                                SuperimpositionState::Blank => {}
                                SuperimpositionState::Fits => {
                                    sprite.color = Color::from(selected_shape.color).with_a(0.5);
                                }
                                SuperimpositionState::Intersects => {
                                    sprite.color = Color::from(TileColor::Red).with_a(0.5);
                                }
                            };
                        }
                    }
                }
            }
        }
    }
}

fn update_board(
    mut q_board: Query<&mut Board, (With<MainBoard>, Without<OverlayBoard>, Changed<Board>)>,
    mut q_board_tiles: Query<&mut Sprite>,
) {
    for board in q_board.iter_mut() {
        for (y, row) in board.entities.iter().enumerate() {
            for (x, entity) in row.iter().enumerate() {
                if let Ok(mut sprite) = q_board_tiles.get_mut(*entity) {
                    if let Some(color) = board.grid.0[y][x] {
                        sprite.color = color.into();
                    } else {
                        sprite.color = TileColor::Gray.into();
                    }
                }
            }
        }
    }
}

#[derive(Component)]
struct MainBoard;

#[derive(Component)]
struct OverlayBoard;

#[derive(Component)]
struct SelectedShape;

#[derive(Component)]
struct MainCamera;

#[derive(Resource)]
struct ShapePool(Vec<Shape>);

fn main() {
    let generated = shapes! {
        // 2x2 Square
        (2,2) "####";
        // Line 4
        (4,1) "####";
        // Line 3
        (3,1) "###";
        // V
        (2,2) "##.#";
        // L
        (3,2) "###..#";
        // Dot
        (1,1) "#";
        // Line 2
        (1,2) "##";
        // 3x3 Square
        (3,3) "#########";
        // 3x2 Rectangle
        (2,3) "######";
        // T
        (3,2) "###.#.";
        // S
        (3,2) "##..##";
    };

    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, startup)
        .add_systems(Update, (update, update_board))
        .insert_resource(ShapePool(generated))
        .run();
    println!("Hello, world!");
}
