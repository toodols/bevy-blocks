use bevy::prelude::*;
use rand::{
    distributions::{Distribution, Standard},
    Rng,
};
use std::fmt;

#[derive(PartialEq, Eq, Clone, Copy, Default)]
pub enum TileColor {
    #[default]
    Gray,
    Red,
    Green,
    Blue,
    Transparent,
}
impl Distribution<TileColor> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> TileColor {
        match rng.gen_range(0..3) {
            0 => TileColor::Red,
            1 => TileColor::Green,
            2 => TileColor::Blue,
            _ => unreachable!(),
        }
    }
}
impl From<TileColor> for Color {
    fn from(value: TileColor) -> Self {
        match value {
            TileColor::Red => Color::rgb(1.0, 0.0, 0.0),
            TileColor::Green => Color::rgb(0.0, 1.0, 0.0),
            TileColor::Blue => Color::rgb(0.0, 0.0, 1.0),
            TileColor::Gray => Color::rgb(0.3, 0.3, 0.3),
            TileColor::Transparent => Color::rgba(0.0, 0.0, 0.0, 0.0),
        }
    }
}
impl TileColor {
    const DEFAULT: TileColor = TileColor::Gray;
}

#[derive(PartialEq, Eq, Clone, Copy, Component)]
pub struct Shape {
    pub color: TileColor,
    pub fields: [[bool; 8]; 8],
}
impl Shape {
    pub fn bounds(&self) -> (usize, usize) {
        self.fields
            .iter()
            .enumerate()
            .fold((0, 0), |acc, (i, row)| {
                let max_x_in_row = row.iter().enumerate().fold(0, |max_x, (j, &val)| {
                    if val {
                        usize::max(max_x, j + 1)
                    } else {
                        max_x
                    }
                });
                (
                    usize::max(acc.0, max_x_in_row),
                    usize::max(acc.1, if max_x_in_row > 0 { i + 1 } else { acc.1 }),
                )
            })
    }
    pub fn rotate_90(&self) -> Shape {
        let mut new_fields = [[false; 8]; 8];
        let (width, height) = self.bounds();

        for i in 0..height {
            for j in 0..width {
                new_fields[j][height - i - 1] = self.fields[i][j];
            }
        }

        Shape {
            fields: new_fields,
            ..*self
        }
    }

    pub fn equivalents(&self) -> Vec<Shape> {
        let mut shapes = vec![*self];
        let rot90 = self.rotate_90();
        if !shapes.contains(&rot90) {
            shapes.push(rot90);
        }
        let rot180 = rot90.rotate_90();
        if !shapes.contains(&rot180) {
            shapes.push(rot180);
        }
        let rot270 = rot180.rotate_90();
        if !shapes.contains(&rot270) {
            shapes.push(rot270);
        }
        shapes
    }

    pub fn from_pattern(w: usize, h: usize, pat: &str) -> Self {
        if pat.len() != w * h {
            panic!("Pattern length does not match given dimensions");
        }

        let mut fields = [[false; 8]; 8];
        for (i, c) in pat.chars().enumerate() {
            let x = i % w; // x-coordinate (column)
            let y = i / w; // y-coordinate (row)

            // Ensure we don't go out of bounds if the shape's w or h is greater than 8
            if x >= 8 || y >= 8 {
                panic!("Pattern dimensions out of bounds");
            }

            fields[y][x] = match c {
                '#' => true,
                '.' => false,
                _ => panic!("Invalid character in pattern: {}", c),
            };
        }

        // Initialize with a default color for this demonstration; adjust as needed
        Self {
            color: TileColor::DEFAULT,
            fields,
        }
    }
}

impl fmt::Display for Shape {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (width, height) = self.bounds();

        for i in 0..height {
            for j in 0..width {
                // Write '#' for true and '.' for false
                let c = if self.fields[i][j] { '#' } else { '.' };
                write!(f, "{}", c)?;
            }
            // After each row except the last one, add a newline
            if i < height - 1 {
                write!(f, "\n")?;
            }
        }

        Ok(())
    }
}

#[macro_export]
macro_rules! shapes {
    // Match one or more shape definitions, separated by semicolons
    ($(($x:expr, $y:expr) $pattern:literal);+ $(;)?) => {
        {
            // Create a mutable vector to hold the generated shapes
            let mut temp = Vec::new();

            // Process each shape definition
            $(
                // Extend the temp vector with the generated shapes from the provided patterns
                temp.extend(crate::board::Shape::from_pattern($x, $y, $pattern).equivalents());
            )+

            // Return the filled temp vector
            temp
        }
    };
}

pub struct Grid<T, const W: usize, const H: usize>(pub [[T; W]; H]);

pub const BOARD_WIDTH: usize = 20;
pub const BOARD_HEIGHT: usize = 20;
pub type Board = Grid<Option<TileColor>, BOARD_WIDTH, BOARD_HEIGHT>;

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for i in 0..BOARD_HEIGHT {
            for j in 0..BOARD_WIDTH {
                // Write '#' for true and '.' for false
                write!(f, "{}", if self.0[i][j].is_some() { '#' } else { '.' })?;
            }
            // After each row except the last one, add a newline
            if i < BOARD_HEIGHT - 1 {
                write!(f, "\n")?;
            }
        }

        Ok(())
    }
}

impl<T: Default + Copy, const W: usize, const H: usize> Default for Grid<T, W, H> {
    fn default() -> Self {
        Self([[T::default(); W]; H])
    }
}
impl<T: Clone, const W: usize, const H: usize> Clone for Grid<T, W, H> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<T: Copy, const W: usize, const H: usize> Copy for Grid<T, W, H> {}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SuperimpositionState {
    Fits,
    Intersects,
    Blank,
}

#[derive(Clone, Copy)]
pub struct Superimposition {
    pub fields: Grid<SuperimpositionState, BOARD_WIDTH, BOARD_HEIGHT>,
    pub success: bool,
}

impl Board {
    pub fn superimpose(&self, shape: &Shape, translation: (f32, f32)) -> Superimposition {
        let shape_bounds = shape.bounds();
        let shape_center = (shape_bounds.0 as f32 * 0.5, shape_bounds.1 as f32 * 0.5);

        let cursor_center = (
            ((self.0[0].len()) as f32) * translation.0,
            ((self.0.len()) as f32) * translation.1,
        );

        let shape_offset_to_board = (
            cursor_center.0 - shape_center.0,
            cursor_center.1 - shape_center.1,
        );

        let mut superimposition = Grid::<SuperimpositionState, BOARD_HEIGHT, BOARD_WIDTH>(
            [[SuperimpositionState::Blank; BOARD_WIDTH]; BOARD_HEIGHT],
        );
        let mut success = true;

        for (y, row) in shape.fields.iter().enumerate() {
            for (x, &cell) in row.iter().enumerate() {
                if cell {
                    let board_x = (x as f32 + shape_offset_to_board.0).round() as isize;
                    let board_y = (y as f32 + shape_offset_to_board.1).round() as isize;

                    if board_x < 0
                        || board_x >= BOARD_WIDTH as isize
                        || board_y < 0
                        || board_y >= BOARD_HEIGHT as isize
                    {
                        success = false;
                    } else if let Some(_color) = self.0[board_y as usize][board_x as usize] {
                        superimposition.0[board_y as usize][board_x as usize] =
                            SuperimpositionState::Intersects;
                        success = false;
                    } else {
                        superimposition.0[board_y as usize][board_x as usize] =
                            SuperimpositionState::Fits;
                    }
                }
            }
        }

        Superimposition {
            fields: superimposition,
            success,
        }
    }
}
